//! Audit logging for security events

use crate::config::AuditConfig;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    CommandExecution,
    FileAccess,
    ConfigChange,
    AuthSuccess,
    AuthFailure,
    PolicyViolation,
    SecurityEvent,
    SessionStart,
    SessionEnd,
    ToolExecution,
    PeripheralAccess,
}

/// NTP synchronization status for timestamp verification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NtpSyncStatus {
    Synced,
    Unsynchronized,
    Unknown,
}

impl Default for NtpSyncStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Network context for tracking connection origin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkContext {
    pub source_ip: Option<String>,
    pub source_port: Option<u16>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
}

impl Default for NetworkContext {
    fn default() -> Self {
        Self {
            source_ip: None,
            source_port: None,
            user_agent: None,
            session_id: None,
        }
    }
}

/// Operation target - what resource was accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationTarget {
    pub resource_type: Option<String>,
    pub resource_path: Option<String>,
    pub resource_id: Option<String>,
}

impl Default for OperationTarget {
    fn default() -> Self {
        Self {
            resource_type: None,
            resource_path: None,
            resource_id: None,
        }
    }
}

/// Hash chain entry for tamper detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashChainEntry {
    pub previous_hash: String,
    pub current_hash: String,
    pub event_index: u64,
}

impl HashChainEntry {
    pub fn compute_hash(
        event_json: &str,
        previous_hash: &str,
        event_index: u64,
        secret_key: &[u8],
    ) -> Result<String> {
        let mut mac = HmacSha256::new_from_slice(secret_key)
            .context("failed to create HMAC for hash chain")?;

        mac.update(event_json.as_bytes());
        mac.update(previous_hash.as_bytes());
        mac.update(&event_index.to_le_bytes());

        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

/// Actor information (who performed the action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub channel: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
}

/// Action information (what was done)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub command: Option<String>,
    pub risk_level: Option<String>,
    pub approved: bool,
    pub allowed: bool,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Security context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    pub policy_violation: bool,
    pub rate_limit_remaining: Option<u32>,
    pub sandbox_backend: Option<String>,
}

/// Complete audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub ntp_sync: NtpSyncStatus,
    pub event_id: String,
    pub event_type: AuditEventType,
    pub actor: Option<Actor>,
    pub action: Option<Action>,
    pub target: Option<OperationTarget>,
    pub network: Option<NetworkContext>,
    pub result: Option<ExecutionResult>,
    pub security: SecurityContext,
    pub hash_chain: Option<HashChainEntry>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            timestamp: Utc::now(),
            ntp_sync: NtpSyncStatus::Unknown,
            event_id: Uuid::new_v4().to_string(),
            event_type,
            actor: None,
            action: None,
            target: None,
            network: None,
            result: None,
            security: SecurityContext {
                policy_violation: false,
                rate_limit_remaining: None,
                sandbox_backend: None,
            },
            hash_chain: None,
        }
    }

    /// Set NTP sync status
    pub fn with_ntp_sync(mut self, status: NtpSyncStatus) -> Self {
        self.ntp_sync = status;
        self
    }

    /// Set the actor
    pub fn with_actor(
        mut self,
        channel: String,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Self {
        self.actor = Some(Actor {
            channel,
            user_id,
            username,
        });
        self
    }

    /// Set the action
    pub fn with_action(
        mut self,
        command: String,
        risk_level: String,
        approved: bool,
        allowed: bool,
    ) -> Self {
        self.action = Some(Action {
            command: Some(command),
            risk_level: Some(risk_level),
            approved,
            allowed,
        });
        self
    }

    /// Set the result
    pub fn with_result(
        mut self,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: u64,
        error: Option<String>,
    ) -> Self {
        self.result = Some(ExecutionResult {
            success,
            exit_code,
            duration_ms: Some(duration_ms),
            error,
        });
        self
    }

    /// Set security context
    pub fn with_security(mut self, sandbox_backend: Option<String>) -> Self {
        self.security.sandbox_backend = sandbox_backend;
        self
    }

    /// Set network context (IP, session)
    pub fn with_network(
        mut self,
        source_ip: Option<String>,
        source_port: Option<u16>,
        user_agent: Option<String>,
        session_id: Option<String>,
    ) -> Self {
        self.network = Some(NetworkContext {
            source_ip,
            source_port,
            user_agent,
            session_id,
        });
        self
    }

    /// Set operation target
    pub fn with_target(
        mut self,
        resource_type: Option<String>,
        resource_path: Option<String>,
        resource_id: Option<String>,
    ) -> Self {
        self.target = Some(OperationTarget {
            resource_type,
            resource_path,
            resource_id,
        });
        self
    }

    /// Compute and set hash chain entry
    pub fn with_hash_chain(
        mut self,
        previous_hash: &str,
        event_index: u64,
        secret_key: &[u8],
    ) -> Result<Self> {
        let event_json =
            serde_json::to_string(&self).context("failed to serialize event for hash chain")?;

        let current_hash =
            HashChainEntry::compute_hash(&event_json, previous_hash, event_index, secret_key)?;

        self.hash_chain = Some(HashChainEntry {
            previous_hash: previous_hash.to_string(),
            current_hash,
            event_index,
        });
        Ok(self)
    }
}

/// Audit log export backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditExportBackend {
    File,
    Syslog,
    Http,
}

/// Audit log export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExportConfig {
    pub backend: AuditExportBackend,
    pub endpoint: Option<String>,
    pub enabled: bool,
}

impl Default for AuditExportConfig {
    fn default() -> Self {
        Self {
            backend: AuditExportBackend::File,
            endpoint: None,
            enabled: false,
        }
    }
}

/// Audit retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRetentionPolicy {
    pub max_files: u32,
    pub max_age_days: Option<u32>,
}

impl Default for AuditRetentionPolicy {
    fn default() -> Self {
        Self {
            max_files: 10,
            max_age_days: Some(90),
        }
    }
}

/// Hash chain state for tamper detection
pub struct HashChainState {
    last_hash: String,
    event_index: u64,
    secret_key: Vec<u8>,
}

impl HashChainState {
    pub fn new(secret_key: Vec<u8>) -> Self {
        Self {
            last_hash: "genesis".to_string(),
            event_index: 0,
            secret_key,
        }
    }

    pub fn get_last_hash(&self) -> &str {
        &self.last_hash
    }

    pub fn get_event_index(&self) -> u64 {
        self.event_index
    }

    pub fn advance(&mut self, new_hash: String) {
        self.last_hash = new_hash;
        self.event_index += 1;
    }
}

/// Audit logger
pub struct AuditLogger {
    log_path: PathBuf,
    config: AuditConfig,
    buffer: Mutex<Vec<AuditEvent>>,
    hash_chain: Mutex<Option<HashChainState>>,
    retention_policy: AuditRetentionPolicy,
    export_configs: Vec<AuditExportConfig>,
}

/// Structured command execution details for audit logging.
#[derive(Debug, Clone)]
pub struct CommandExecutionLog<'a> {
    pub channel: &'a str,
    pub command: &'a str,
    pub risk_level: &'a str,
    pub approved: bool,
    pub allowed: bool,
    pub success: bool,
    pub duration_ms: u64,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditConfig, zeroclaw_dir: PathBuf) -> Result<Self> {
        let log_path = zeroclaw_dir.join(&config.log_path);

        let hash_chain = if config.sign_events {
            let secret_key = std::env::var("ZEROCLAW_AUDIT_SECRET")
                .unwrap_or_else(|_| "default-audit-key-change-in-production".to_string())
                .into_bytes();
            Some(HashChainState::new(secret_key))
        } else {
            None
        };

        Ok(Self {
            log_path,
            config,
            buffer: Mutex::new(Vec::new()),
            hash_chain: Mutex::new(hash_chain),
            retention_policy: AuditRetentionPolicy::default(),
            export_configs: Vec::new(),
        })
    }

    /// Configure export backends
    pub fn with_export_backends(mut self, configs: Vec<AuditExportConfig>) -> Self {
        self.export_configs = configs;
        self
    }

    /// Configure retention policy
    pub fn with_retention_policy(mut self, policy: AuditRetentionPolicy) -> Self {
        self.retention_policy = policy;
        self
    }

    /// Log an event with hash chain support
    pub fn log(&self, event: &AuditEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        self.rotate_if_needed()?;
        self.enforce_retention()?;

        let event_with_hash = {
            let mut hc = self.hash_chain.lock();
            if let Some(ref mut state) = *hc {
                let previous_hash = state.get_last_hash().to_string();
                let event_index = state.get_event_index();

                let event_json = serde_json::to_string(event)?;
                let current_hash = HashChainEntry::compute_hash(
                    &event_json,
                    &previous_hash,
                    event_index,
                    &state.secret_key,
                )?;

                state.advance(current_hash.clone());

                let mut event = event.clone();
                event.hash_chain = Some(HashChainEntry {
                    previous_hash,
                    current_hash,
                    event_index,
                });
                event
            } else {
                event.clone()
            }
        };

        self.write_event(&event_with_hash)?;
        self.export_to_backends(&event_with_hash)?;

        Ok(())
    }

    /// Write event to file
    fn write_event(&self, event: &AuditEvent) -> Result<()> {
        let line = serde_json::to_string(event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", line)?;
        file.sync_all()?;
        Ok(())
    }

    /// Export event to configured backends
    fn export_to_backends(&self, event: &AuditEvent) -> Result<()> {
        for config in &self.export_configs {
            if !config.enabled {
                continue;
            }

            match config.backend {
                AuditExportBackend::Syslog => {
                    self.export_to_syslog(event)?;
                }
                AuditExportBackend::Http => {
                    if let Some(ref endpoint) = config.endpoint {
                        self.export_to_http(event, endpoint)?;
                    }
                }
                AuditExportBackend::File => {
                    // Already handled by write_event
                }
            }
        }
        Ok(())
    }

    /// Export to syslog
    fn export_to_syslog(&self, event: &AuditEvent) -> Result<()> {
        #[cfg(unix)]
        {
            let syslog_msg = format!(
                "<{}> zeroclaw: {:?} - {}",
                if event.result.as_ref().map_or(false, |r| r.success) {
                    14 // info
                } else {
                    10 // alert
                },
                event.event_type,
                event.event_id
            );

            tracing::info!("{}", syslog_msg);
        }
        Ok(())
    }

    /// Export to HTTP endpoint
    fn export_to_http(&self, event: &AuditEvent, endpoint: &str) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let _ = client
            .post(endpoint)
            .json(event)
            .timeout(std::time::Duration::from_secs(5))
            .send();
        Ok(())
    }

    /// Enforce retention policy
    fn enforce_retention(&self) -> Result<()> {
        if let Ok(entries) = fs::read_dir(&self.log_path.parent().unwrap_or(&self.log_path)) {
            let mut log_files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().to_string_lossy().contains("audit.log"))
                .collect();

            log_files.sort_by_key(|e| std::cmp::Reverse(e.path()));

            if log_files.len() > self.retention_policy.max_files as usize {
                for file in log_files
                    .iter()
                    .skip(self.retention_policy.max_files as usize)
                {
                    let _ = fs::remove_file(file.path());
                }
            }
        }
        Ok(())
    }

    /// Log a command execution event.
    pub fn log_command_event(&self, entry: CommandExecutionLog<'_>) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor(entry.channel.to_string(), None, None)
            .with_action(
                entry.command.to_string(),
                entry.risk_level.to_string(),
                entry.approved,
                entry.allowed,
            )
            .with_result(entry.success, None, entry.duration_ms, None);

        self.log(&event)
    }

    /// Backward-compatible helper to log a command execution event.
    #[allow(clippy::too_many_arguments)]
    pub fn log_command(
        &self,
        channel: &str,
        command: &str,
        risk_level: &str,
        approved: bool,
        allowed: bool,
        success: bool,
        duration_ms: u64,
    ) -> Result<()> {
        self.log_command_event(CommandExecutionLog {
            channel,
            command,
            risk_level,
            approved,
            allowed,
            success,
            duration_ms,
        })
    }

    /// Rotate log if it exceeds max size
    fn rotate_if_needed(&self) -> Result<()> {
        if let Ok(metadata) = std::fs::metadata(&self.log_path) {
            let current_size_mb = metadata.len() / (1024 * 1024);
            if current_size_mb >= u64::from(self.config.max_size_mb) {
                self.rotate()?;
            }
        }
        Ok(())
    }

    /// Rotate the log file
    fn rotate(&self) -> Result<()> {
        for i in (1..10).rev() {
            let old_name = format!("{}.{}.log", self.log_path.display(), i);
            let new_name = format!("{}.{}.log", self.log_path.display(), i + 1);
            let _ = std::fs::rename(&old_name, &new_name);
        }

        let rotated = format!("{}.1.log", self.log_path.display());
        std::fs::rename(&self.log_path, &rotated)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn audit_event_new_creates_unique_id() {
        let event1 = AuditEvent::new(AuditEventType::CommandExecution);
        let event2 = AuditEvent::new(AuditEventType::CommandExecution);
        assert_ne!(event1.event_id, event2.event_id);
    }

    #[test]
    fn audit_event_with_actor() {
        let event = AuditEvent::new(AuditEventType::CommandExecution).with_actor(
            "telegram".to_string(),
            Some("123".to_string()),
            Some("@alice".to_string()),
        );

        assert!(event.actor.is_some());
        let actor = event.actor.as_ref().unwrap();
        assert_eq!(actor.channel, "telegram");
        assert_eq!(actor.user_id, Some("123".to_string()));
        assert_eq!(actor.username, Some("@alice".to_string()));
    }

    #[test]
    fn audit_event_with_action() {
        let event = AuditEvent::new(AuditEventType::CommandExecution).with_action(
            "ls -la".to_string(),
            "low".to_string(),
            false,
            true,
        );

        assert!(event.action.is_some());
        let action = event.action.as_ref().unwrap();
        assert_eq!(action.command, Some("ls -la".to_string()));
        assert_eq!(action.risk_level, Some("low".to_string()));
    }

    #[test]
    fn audit_event_serializes_to_json() {
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor("telegram".to_string(), None, None)
            .with_action("ls".to_string(), "low".to_string(), false, true)
            .with_result(true, Some(0), 15, None);

        let json = serde_json::to_string(&event);
        assert!(json.is_ok());
        let json = json.expect("serialize");
        let parsed: AuditEvent = serde_json::from_str(json.as_str()).expect("parse");
        assert!(parsed.actor.is_some());
        assert!(parsed.action.is_some());
        assert!(parsed.result.is_some());
    }

    #[test]
    fn audit_logger_disabled_does_not_create_file() -> Result<()> {
        let tmp = TempDir::new()?;
        let config = AuditConfig {
            enabled: false,
            ..Default::default()
        };
        let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
        let event = AuditEvent::new(AuditEventType::CommandExecution);

        logger.log(&event)?;

        // File should not exist since logging is disabled
        assert!(!tmp.path().join("audit.log").exists());
        Ok(())
    }

    // ── §8.1 Log rotation tests ─────────────────────────────

    #[tokio::test]
    async fn audit_logger_writes_event_when_enabled() -> Result<()> {
        let tmp = TempDir::new()?;
        let config = AuditConfig {
            enabled: true,
            max_size_mb: 10,
            ..Default::default()
        };
        let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;
        let event = AuditEvent::new(AuditEventType::CommandExecution)
            .with_actor("cli".to_string(), None, None)
            .with_action("ls".to_string(), "low".to_string(), false, true);

        logger.log(&event)?;

        let log_path = tmp.path().join("audit.log");
        assert!(log_path.exists(), "audit log file must be created");

        let content = tokio::fs::read_to_string(&log_path).await?;
        assert!(!content.is_empty(), "audit log must not be empty");

        let parsed: AuditEvent = serde_json::from_str(content.trim())?;
        assert!(parsed.action.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn audit_log_command_event_writes_structured_entry() -> Result<()> {
        let tmp = TempDir::new()?;
        let config = AuditConfig {
            enabled: true,
            max_size_mb: 10,
            ..Default::default()
        };
        let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;

        logger.log_command_event(CommandExecutionLog {
            channel: "telegram",
            command: "echo test",
            risk_level: "low",
            approved: false,
            allowed: true,
            success: true,
            duration_ms: 42,
        })?;

        let log_path = tmp.path().join("audit.log");
        let content = tokio::fs::read_to_string(&log_path).await?;
        let parsed: AuditEvent = serde_json::from_str(content.trim())?;

        let action = parsed.action.unwrap();
        assert_eq!(action.command, Some("echo test".to_string()));
        assert_eq!(action.risk_level, Some("low".to_string()));
        assert!(action.allowed);

        let result = parsed.result.unwrap();
        assert!(result.success);
        assert_eq!(result.duration_ms, Some(42));
        Ok(())
    }

    #[test]
    fn audit_rotation_creates_numbered_backup() -> Result<()> {
        let tmp = TempDir::new()?;
        let config = AuditConfig {
            enabled: true,
            max_size_mb: 0, // Force rotation on first write
            ..Default::default()
        };
        let logger = AuditLogger::new(config, tmp.path().to_path_buf())?;

        // Write initial content that triggers rotation
        let log_path = tmp.path().join("audit.log");
        std::fs::write(&log_path, "initial content\n")?;

        let event = AuditEvent::new(AuditEventType::CommandExecution);
        logger.log(&event)?;

        let rotated = format!("{}.1.log", log_path.display());
        assert!(
            std::path::Path::new(&rotated).exists(),
            "rotation must create .1.log backup"
        );
        Ok(())
    }
}
