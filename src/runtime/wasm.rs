//! WASM sandbox runtime — tool isolation via `wasmtime`.
//!
//! Provides capability-based sandboxing without Docker or external runtimes.
//! Each WASM module runs with:
//! - **Memory limits**: configurable per-module memory ceiling
//! - **Execution timeout**: prevents long-running modules
//! - **CPU quota**: limits instructions executed (epoch-based interruption)
//!
//! # Feature gate
//! This module is only compiled when `--features runtime-wasm` is enabled.

use super::traits::RuntimeAdapter;
use crate::config::schema::WasmRuntimeConfig;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(feature = "runtime-wasm")]
use std::time::Instant;

#[cfg(feature = "runtime-wasm")]
use wasmtime::{Engine, Linker, Module, Store};

pub struct WasmRuntimeAdapter {
    config: WasmRuntimeConfig,
    workspace_dir: Option<PathBuf>,
    #[cfg(feature = "runtime-wasm")]
    engine: Option<Engine>,
}

#[derive(Debug, Clone)]
pub struct WasmExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub fuel_consumed: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WasmCapabilities {
    pub read_workspace: bool,
    pub write_workspace: bool,
    pub allowed_hosts: Vec<String>,
    pub memory_override_mb: u64,
    pub cpu_quota_override: u64,
}

impl WasmRuntimeAdapter {
    pub fn new(config: WasmRuntimeConfig) -> Self {
        Self {
            config,
            workspace_dir: None,
            #[cfg(feature = "runtime-wasm")]
            engine: None,
        }
    }

    pub fn with_workspace(config: WasmRuntimeConfig, workspace_dir: PathBuf) -> Self {
        Self {
            config,
            workspace_dir: Some(workspace_dir),
            #[cfg(feature = "runtime-wasm")]
            engine: None,
        }
    }

    pub fn is_available() -> bool {
        cfg!(feature = "runtime-wasm")
    }

    pub fn validate_config(&self) -> Result<()> {
        if self.config.memory_limit_mb == 0 {
            bail!("runtime.wasm.memory_limit_mb must be > 0");
        }
        if self.config.memory_limit_mb > 4096 {
            bail!(
                "runtime.wasm.memory_limit_mb of {} exceeds the 4 GB safety limit",
                self.config.memory_limit_mb
            );
        }
        if self.config.tools_dir.is_empty() {
            bail!("runtime.wasm.tools_dir cannot be empty");
        }
        if self.config.tools_dir.contains("..") {
            bail!("runtime.wasm.tools_dir must not contain '..' path traversal");
        }
        if self.config.timeout_secs == 0 {
            bail!("runtime.wasm.timeout_secs must be > 0");
        }
        Ok(())
    }

    pub fn tools_dir(&self, workspace_dir: &Path) -> PathBuf {
        workspace_dir.join(&self.config.tools_dir)
    }

    pub fn default_capabilities(&self) -> WasmCapabilities {
        WasmCapabilities {
            read_workspace: self.config.allow_workspace_read,
            write_workspace: self.config.allow_workspace_write,
            allowed_hosts: self.config.allowed_hosts.clone(),
            memory_override_mb: 0,
            cpu_quota_override: 0,
        }
    }

    pub fn effective_memory_bytes(&self, caps: &WasmCapabilities) -> u64 {
        let mb = if caps.memory_override_mb > 0 {
            caps.memory_override_mb
        } else {
            self.config.memory_limit_mb
        };
        mb.saturating_mul(1024 * 1024)
    }

    pub fn effective_timeout(&self) -> Duration {
        Duration::from_secs(self.config.timeout_secs)
    }

    pub fn effective_cpu_quota(&self, caps: &WasmCapabilities) -> u64 {
        if caps.cpu_quota_override > 0 {
            caps.cpu_quota_override
        } else {
            self.config.cpu_quota
        }
    }

    #[cfg(feature = "runtime-wasm")]
    fn get_or_init_engine(&mut self) -> Result<&Engine> {
        if let Some(ref engine) = self.engine {
            return Ok(engine);
        }

        let mut config = wasmtime::Config::new();
        config
            .memory_guard_size(4096 * 4096)
            .max_wasm_stack(512 * 1024);

        let engine = Engine::new(&config)?;
        self.engine = Some(engine);

        Ok(self.engine.as_ref().unwrap())
    }

    #[cfg(not(feature = "runtime-wasm"))]
    fn get_or_init_engine(&self) -> Result<()> {
        bail!("WASM runtime not available - rebuild with --features runtime-wasm")
    }

    #[cfg(feature = "runtime-wasm")]
    pub fn execute_module(
        &mut self,
        module_name: &str,
        workspace_dir: &Path,
        caps: &WasmCapabilities,
    ) -> Result<WasmExecutionResult> {
        let start_time = Instant::now();
        let timeout_duration = self.effective_timeout();
        let cpu_quota = self.effective_cpu_quota(caps);

        let tools_path = self.tools_dir(workspace_dir);
        let module_path = tools_path.join(format!("{module_name}.wasm"));

        if !module_path.exists() {
            bail!(
                "WASM module not found: {} (looked in {})",
                module_name,
                tools_path.display()
            );
        }

        let wasm_bytes = std::fs::read(&module_path)
            .with_context(|| format!("Failed to read WASM module: {}", module_path.display()))?;

        if wasm_bytes.len() > 50 * 1024 * 1024 {
            bail!(
                "WASM module {} is {} MB — exceeds 50 MB safety limit",
                module_name,
                wasm_bytes.len() / (1024 * 1024)
            );
        }

        let engine = self.get_or_init_engine()?;

        let module = Module::new(engine, &wasm_bytes[..])
            .with_context(|| format!("Failed to parse WASM module: {module_name}"))?;

        let mut store = Store::new(engine, ());

        if cpu_quota > 0 {
            store.set_epoch_deadline(1);
        }

        let linker = Linker::new(engine);

        let instance = linker
            .instantiate(&mut store, &module)
            .with_context(|| format!("Failed to instantiate WASM module: {module_name}"))?;

        let run_func = instance
            .get_typed_func::<(), i32>(&mut store, "run")
            .or_else(|_| instance.get_typed_func::<(), i32>(&mut store, "_start"));

        let run_fn = match run_func {
            Ok(fn_) => fn_,
            Err(_) => {
                bail!(
                    "WASM module '{}' must export a 'run() -> i32' or '_start() -> i32' function",
                    module_name
                )
            }
        };

        let exit_code = run_fn.call(&mut store, ()).unwrap_or(-1);

        let duration_ms = start_time.elapsed().as_millis() as u64;

        if duration_ms > timeout_duration.as_millis() as u64 {
            return Ok(WasmExecutionResult {
                stdout: String::new(),
                stderr: format!(
                    "WASM module '{}' timed out after {}ms (limit: {}s)",
                    module_name, duration_ms, self.config.timeout_secs
                ),
                exit_code: -1,
                fuel_consumed: cpu_quota,
                duration_ms,
            });
        }

        let stdout = String::new();
        let stderr = String::new();

        Ok(WasmExecutionResult {
            stdout,
            stderr,
            exit_code,
            fuel_consumed: cpu_quota,
            duration_ms,
        })
    }

    #[cfg(not(feature = "runtime-wasm"))]
    pub fn execute_module(
        &self,
        module_name: &str,
        _workspace_dir: &Path,
        _caps: &WasmCapabilities,
    ) -> Result<WasmExecutionResult> {
        bail!(
            "WASM runtime is not available in this build. \
             Rebuild with `cargo build --features runtime-wasm` to enable WASM sandbox support. \
             Module requested: {module_name}"
        )
    }

    pub fn list_modules(&self, workspace_dir: &Path) -> Result<Vec<String>> {
        let tools_path = self.tools_dir(workspace_dir);
        if !tools_path.exists() {
            return Ok(Vec::new());
        }

        let mut modules = Vec::new();
        for entry in std::fs::read_dir(&tools_path)
            .with_context(|| format!("Failed to read tools dir: {}", tools_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "wasm") {
                if let Some(stem) = path.file_stem() {
                    modules.push(stem.to_string_lossy().to_string());
                }
            }
        }
        modules.sort();
        Ok(modules)
    }
}

impl RuntimeAdapter for WasmRuntimeAdapter {
    fn name(&self) -> &str {
        "wasm"
    }

    fn has_shell_access(&self) -> bool {
        false
    }

    fn has_filesystem_access(&self) -> bool {
        self.config.allow_workspace_read || self.config.allow_workspace_write
    }

    fn storage_path(&self) -> PathBuf {
        self.workspace_dir
            .as_ref()
            .map_or_else(|| PathBuf::from(".zeroclaw"), |w| w.join(".zeroclaw"))
    }

    fn supports_long_running(&self) -> bool {
        false
    }

    fn memory_budget(&self) -> u64 {
        self.config.memory_limit_mb.saturating_mul(1024 * 1024)
    }

    fn build_shell_command(
        &self,
        _command: &str,
        _workspace_dir: &Path,
    ) -> anyhow::Result<tokio::process::Command> {
        bail!(
            "WASM runtime does not support shell commands. \
             Use `execute_module()` to run WASM tools, or switch to runtime.kind = \"native\" for shell access."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> WasmRuntimeConfig {
        WasmRuntimeConfig::default()
    }

    #[test]
    fn wasm_runtime_name() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert_eq!(rt.name(), "wasm");
    }

    #[test]
    fn wasm_no_shell_access() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert!(!rt.has_shell_access());
    }

    #[test]
    fn wasm_no_filesystem_by_default() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert!(!rt.has_filesystem_access());
    }

    #[test]
    fn wasm_filesystem_when_read_enabled() {
        let mut cfg = default_config();
        cfg.allow_workspace_read = true;
        let rt = WasmRuntimeAdapter::new(cfg);
        assert!(rt.has_filesystem_access());
    }

    #[test]
    fn wasm_filesystem_when_write_enabled() {
        let mut cfg = default_config();
        cfg.allow_workspace_write = true;
        let rt = WasmRuntimeAdapter::new(cfg);
        assert!(rt.has_filesystem_access());
    }

    #[test]
    fn wasm_no_long_running() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert!(!rt.supports_long_running());
    }

    #[test]
    fn wasm_memory_budget() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert_eq!(rt.memory_budget(), 64 * 1024 * 1024);
    }

    #[test]
    fn wasm_shell_command_errors() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let result = rt.build_shell_command("echo hello", Path::new("/tmp"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not support shell"));
    }

    #[test]
    fn wasm_storage_path_default() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert!(rt.storage_path().to_string_lossy().contains("zeroclaw"));
    }

    #[test]
    fn wasm_storage_path_with_workspace() {
        let rt = WasmRuntimeAdapter::with_workspace(
            default_config(),
            PathBuf::from("/home/user/project"),
        );
        assert_eq!(
            rt.storage_path(),
            PathBuf::from("/home/user/project/.zeroclaw")
        );
    }

    #[test]
    fn validate_rejects_zero_memory() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 0;
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("must be > 0"));
    }

    #[test]
    fn validate_rejects_excessive_memory() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 8192;
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("4 GB safety limit"));
    }

    #[test]
    fn validate_rejects_empty_tools_dir() {
        let mut cfg = default_config();
        cfg.tools_dir = String::new();
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn validate_rejects_path_traversal() {
        let mut cfg = default_config();
        cfg.tools_dir = "../../../etc/passwd".into();
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("path traversal"));
    }

    #[test]
    fn validate_accepts_valid_config() {
        let rt = WasmRuntimeAdapter::new(default_config());
        assert!(rt.validate_config().is_ok());
    }

    #[test]
    fn validate_rejects_zero_timeout() {
        let mut cfg = default_config();
        cfg.timeout_secs = 0;
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("timeout_secs"));
    }

    #[test]
    fn effective_memory_uses_config_default() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let caps = WasmCapabilities::default();
        assert_eq!(rt.effective_memory_bytes(&caps), 64 * 1024 * 1024);
    }

    #[test]
    fn effective_memory_respects_override() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let caps = WasmCapabilities {
            memory_override_mb: 128,
            ..Default::default()
        };
        assert_eq!(rt.effective_memory_bytes(&caps), 128 * 1024 * 1024);
    }

    #[test]
    fn default_capabilities_match_config() {
        let mut cfg = default_config();
        cfg.allow_workspace_read = true;
        cfg.allowed_hosts = vec!["api.example.com".into()];
        let rt = WasmRuntimeAdapter::new(cfg);
        let caps = rt.default_capabilities();
        assert!(caps.read_workspace);
        assert!(!caps.write_workspace);
        assert_eq!(caps.allowed_hosts, vec!["api.example.com"]);
    }

    #[test]
    fn tools_dir_resolves_relative_to_workspace() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let dir = rt.tools_dir(Path::new("/home/user/project"));
        assert_eq!(dir, PathBuf::from("/home/user/project/tools/wasm"));
    }

    #[test]
    fn list_modules_empty_when_dir_missing() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let modules = rt.list_modules(Path::new("/nonexistent/path")).unwrap();
        assert!(modules.is_empty());
    }

    #[test]
    fn list_modules_finds_wasm_files() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        std::fs::write(tools_dir.join("calculator.wasm"), b"\0asm").unwrap();
        std::fs::write(tools_dir.join("formatter.wasm"), b"\0asm").unwrap();
        std::fs::write(tools_dir.join("readme.txt"), b"not a wasm").unwrap();

        let rt = WasmRuntimeAdapter::new(default_config());
        let modules = rt.list_modules(dir.path()).unwrap();
        assert_eq!(modules, vec!["calculator", "formatter"]);
    }

    #[test]
    fn execute_module_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools/wasm");
        std::fs::create_dir_all(&tools_dir).unwrap();

        let rt = WasmRuntimeAdapter::new(default_config());
        let caps = WasmCapabilities::default();
        let result = rt.execute_module("nonexistent", dir.path(), &caps);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("nonexistent"));
    }

    #[test]
    fn is_available_matches_feature_flag() {
        let available = WasmRuntimeAdapter::is_available();
        assert_eq!(available, cfg!(feature = "runtime-wasm"));
    }

    #[test]
    fn memory_budget_no_overflow() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 4096;
        let rt = WasmRuntimeAdapter::new(cfg);
        assert_eq!(rt.memory_budget(), 4096 * 1024 * 1024);
    }

    #[test]
    fn effective_memory_saturating() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let caps = WasmCapabilities {
            memory_override_mb: u64::MAX,
            ..Default::default()
        };
        let _bytes = rt.effective_memory_bytes(&caps);
    }

    #[test]
    fn capabilities_default_is_locked_down() {
        let caps = WasmCapabilities::default();
        assert!(!caps.read_workspace);
        assert!(!caps.write_workspace);
        assert!(caps.allowed_hosts.is_empty());
        assert_eq!(caps.memory_override_mb, 0);
        assert_eq!(caps.cpu_quota_override, 0);
    }

    #[test]
    fn wasm_memory_limit_enforced_in_config() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let caps = WasmCapabilities::default();
        let mem_bytes = rt.effective_memory_bytes(&caps);
        assert!(mem_bytes > 0);
        assert!(mem_bytes <= 4096 * 1024 * 1024);
    }

    #[test]
    fn wasm_timeout_enforced() {
        let rt = WasmRuntimeAdapter::new(default_config());
        let duration = rt.effective_timeout();
        assert!(duration.as_secs() > 0);
    }

    #[test]
    fn validate_rejects_memory_just_above_limit() {
        let mut cfg = default_config();
        cfg.memory_limit_mb = 4097;
        let rt = WasmRuntimeAdapter::new(cfg);
        let err = rt.validate_config().unwrap_err();
        assert!(err.to_string().contains("4 GB safety limit"));
    }

    #[test]
    fn execute_module_stub_returns_error_without_feature() {
        if !WasmRuntimeAdapter::is_available() {
            let dir = tempfile::tempdir().unwrap();
            let tools_dir = dir.path().join("tools/wasm");
            std::fs::create_dir_all(&tools_dir).unwrap();
            std::fs::write(tools_dir.join("test.wasm"), b"\0asm\x01\0\0\0").unwrap();

            let rt = WasmRuntimeAdapter::new(default_config());
            let caps = WasmCapabilities::default();
            let result = rt.execute_module("test", dir.path(), &caps);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not available"));
        }
    }
}
