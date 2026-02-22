use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::{mpsc, RwLock};

use crate::mcp::types::*;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> serde_json::Value {
    serde_json::Value::Number(REQUEST_ID.fetch_add(1, Ordering::SeqCst).into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    Stdio,
    Http,
}

impl Default for TransportMode {
    fn default() -> Self {
        Self::Http
    }
}

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub transport_mode: TransportMode,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            transport_mode: TransportMode::Http,
            command: None,
            args: Vec::new(),
            env: HashMap::new(),
            url: Some("http://localhost:8765".to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

pub struct McpClient {
    config: McpServerConfig,
    state: Arc<RwLock<ConnectionState>>,
    server_capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    server_info: Arc<RwLock<Option<ServerInfo>>>,
    request_sender: Arc<RwLock<Option<mpsc::Sender<String>>>>,
    stdio_process: Arc<RwLock<Option<Child>>>,
    http_client: reqwest::Client,
    max_retries: u32,
    base_delay_ms: u64,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            server_capabilities: Arc::new(RwLock::new(None)),
            server_info: Arc::new(RwLock::new(None)),
            request_sender: Arc::new(RwLock::new(None)),
            stdio_process: Arc::new(RwLock::new(None)),
            http_client: reqwest::Client::new(),
            max_retries: 5,
            base_delay_ms: 1000,
        }
    }

    pub async fn connect(&self) -> Result<()> {
        {
            let state = self.state.read().await;
            if *state == ConnectionState::Connected {
                return Ok(());
            }
        }

        *self.state.write().await = ConnectionState::Connecting;

        let result = match self.config.transport_mode {
            TransportMode::Stdio => self.connect_stdio().await,
            TransportMode::Http => self.connect_http().await,
        };

        match result {
            Ok(_) => {
                *self.state.write().await = ConnectionState::Connected;
                tracing::info!("MCP client connected to server: {}", self.config.name);
                Ok(())
            }
            Err(e) => {
                *self.state.write().await = ConnectionState::Disconnected;
                Err(e)
            }
        }
    }

    async fn connect_stdio(&self) -> Result<()> {
        let command = self
            .config
            .command
            .as_ref()
            .context("stdio transport requires command")?;

        let mut child = tokio::process::Command::new(command)
            .args(&self.config.args)
            .envs(&self.config.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn MCP server process")?;

        let stdin = child.stdin.take().context("Failed to get stdin")?;
        let stdout = child.stdout.take().context("Failed to get stdout")?;

        let (tx, mut rx) = mpsc::channel::<String>(100);

        *self.request_sender.write().await = Some(tx);

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let state = self.state.clone();
        let capabilities = self.server_capabilities.clone();
        let info = self.server_info.clone();

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if let Err(e) = Self::handle_response(&state, &capabilities, &info, &line).await {
                    tracing::warn!("Error handling MCP response: {}", e);
                }
            }
        });

        let writer = Arc::new(RwLock::new(stdin));
        let writer_clone = writer.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mut w = writer_clone.write().await;
                if let Err(e) = w.write_all(msg.as_bytes()).await {
                    tracing::warn!("Failed to write to MCP server: {}", e);
                    break;
                }
                if let Err(e) = w.write_all(b"\n").await {
                    tracing::warn!("Failed to write newline to MCP server: {}", e);
                    break;
                }
                if let Err(e) = w.flush().await {
                    tracing::warn!("Failed to flush MCP server stdin: {}", e);
                    break;
                }
            }
        });

        *self.stdio_process.write().await = Some(child);

        self.initialize().await?;

        Ok(())
    }

    async fn connect_http(&self) -> Result<()> {
        let url = self
            .config
            .url
            .as_ref()
            .context("HTTP transport requires URL")?;

        self.initialize().await?;

        Ok(())
    }

    async fn handle_response(
        _state: &Arc<RwLock<ConnectionState>>,
        capabilities: &Arc<RwLock<Option<ServerCapabilities>>>,
        info: &Arc<RwLock<Option<ServerInfo>>>,
        line: &str,
    ) -> Result<()> {
        let response: JsonRpcResponse = serde_json::from_str(line)?;

        if let Some(result) = response.result {
            if let Ok(init_result) = serde_json::from_value::<InitializeResult>(result.clone()) {
                *capabilities.write().await = Some(init_result.capabilities);
                *info.write().await = Some(init_result.server_info);
            }
        }

        Ok(())
    }

    async fn initialize(&self) -> Result<()> {
        let id = next_id();
        let params = InitializeRequestParams {
            protocol_version: Some("2024-11-05".to_string()),
            capabilities: ClientCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                resources: Some(ResourcesCapability {
                    subscribe: None,
                    list_changed: None,
                }),
            },
            client_info: ClientInfo {
                name: "zeroclaw".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let request = JsonRpcRequest::new(id, "initialize", Some(serde_json::to_value(params)?));

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "MCP initialization failed: {}",
                error.message
            ));
        }

        if let Some(result) = response.result {
            let init_result: InitializeResult =
                serde_json::from_value(result).context("Failed to parse initialize result")?;

            *self.server_capabilities.write().await = Some(init_result.capabilities);
            *self.server_info.write().await = Some(init_result.server_info.clone());

            tracing::debug!(
                "MCP server initialized: {} v{}",
                init_result.server_info.name,
                init_result.server_info.version
            );
        }

        let notification = JsonRpcNotification::new("initialized", None);
        self.send_notification(notification).await?;

        Ok(())
    }

    async fn send_request(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        let request_json = serde_json::to_string(&request)?;
        match self.config.transport_mode {
            TransportMode::Stdio => {
                let sender = self.request_sender.read().await;
                let sender = sender.as_ref().context("Not connected")?;

                sender.send(request_json).await?;

                Ok(JsonRpcResponse::success(
                    request.id,
                    serde_json::json!({"status": "sent"}),
                ))
            }
            TransportMode::Http => {
                let url = self
                    .config
                    .url
                    .as_ref()
                    .context("HTTP URL not configured")?;
                let response = self
                    .http_client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(request_json)
                    .send()
                    .await?
                    .text()
                    .await?;

                let response: JsonRpcResponse =
                    serde_json::from_str(&response).context("Failed to parse MCP response")?;
                Ok(response)
            }
        }
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<()> {
        let notification_json = serde_json::to_string(&notification)?;

        match self.config.transport_mode {
            TransportMode::Stdio => {
                let sender = self.request_sender.read().await;
                if let Some(sender) = sender.as_ref() {
                    sender.send(notification_json).await?;
                }
            }
            TransportMode::Http => {
                // Notifications don't expect responses for HTTP
            }
        }

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.state.write().await = ConnectionState::Disconnected;
        *self.server_capabilities.write().await = None;
        *self.server_info.write().await = None;
        *self.request_sender.write().await = None;

        if let Some(mut child) = self.stdio_process.write().await.take() {
            let _ = child.kill().await;
        }

        tracing::info!("MCP client disconnected from server: {}", self.config.name);
        Ok(())
    }

    pub async fn reconnect(&self) -> Result<()> {
        let mut retries = 0;
        let mut delay = self.base_delay_ms;

        while retries < self.max_retries {
            tracing::debug!(
                "MCP reconnection attempt {} to {}",
                retries + 1,
                self.config.name
            );

            match self.connect().await {
                Ok(_) => {
                    tracing::info!(
                        "MCP client reconnected to {} after {} attempts",
                        self.config.name,
                        retries + 1
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("MCP reconnection attempt {} failed: {}", retries + 1, e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            delay = delay * 2;
            retries += 1;
        }

        Err(anyhow::anyhow!(
            "Failed to reconnect to MCP server after {} attempts",
            self.max_retries
        ))
    }

    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }

    pub async fn get_server_info(&self) -> Option<ServerInfo> {
        self.server_info.read().await.clone()
    }

    pub async fn get_capabilities(&self) -> Option<ServerCapabilities> {
        self.server_capabilities.read().await.clone()
    }

    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        let id = next_id();
        let request = JsonRpcRequest::new(id, "tools/list", None);

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Failed to list tools: {}", error.message));
        }

        if let Some(result) = response.result {
            let list_result: ToolsListResult = serde_json::from_value(result)?;
            Ok(list_result.tools)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolsCallResult> {
        if !self.is_connected().await {
            self.reconnect().await?;
        }

        let id = next_id();
        let params = ToolsCallRequestParams {
            name: name.to_string(),
            arguments,
            _meta: None,
        };
        let request = JsonRpcRequest::new(id, "tools/call", Some(serde_json::to_value(params)?));

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!("Tool call failed: {}", error.message));
        }

        if let Some(result) = response.result {
            let call_result: ToolsCallResult = serde_json::from_value(result)?;
            Ok(call_result)
        } else {
            Err(anyhow::anyhow!("No result from tool call"))
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<ResourceDefinition>> {
        let id = next_id();
        let request = JsonRpcRequest::new(id, "resources/list", None);

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "Failed to list resources: {}",
                error.message
            ));
        }

        if let Some(result) = response.result {
            let list_result: ResourcesListResult = serde_json::from_value(result)?;
            Ok(list_result.resources)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContent>> {
        if !self.is_connected().await {
            self.reconnect().await?;
        }

        let id = next_id();
        let params = ResourcesReadRequestParams {
            uri: uri.to_string(),
            _meta: None,
        };
        let request =
            JsonRpcRequest::new(id, "resources/read", Some(serde_json::to_value(params)?));

        let response = self.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "Failed to read resource: {}",
                error.message
            ));
        }

        if let Some(result) = response.result {
            let read_result: ResourcesReadResult = serde_json::from_value(result)?;
            Ok(read_result.contents)
        } else {
            Err(anyhow::anyhow!("No result from resource read"))
        }
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new(McpServerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_server_config_default() {
        let config = McpServerConfig::default();
        assert_eq!(config.transport_mode, TransportMode::Http);
        assert_eq!(config.url, Some("http://localhost:8765".to_string()));
    }

    #[tokio::test]
    async fn mcp_client_creation() {
        let config = McpServerConfig {
            name: "test-server".to_string(),
            transport_mode: TransportMode::Http,
            url: Some("http://localhost:8080".to_string()),
            ..Default::default()
        };
        let client = McpClient::new(config);
        assert!(!client.is_connected().await);
    }
}
