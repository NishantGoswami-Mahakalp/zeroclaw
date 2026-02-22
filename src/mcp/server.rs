use crate::memory::Memory;
use crate::tools::traits::Tool;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

use crate::mcp::types::*;

pub mod server {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum TransportMode {
        Stdio,
        Http,
    }

    impl Default for TransportMode {
        fn default() -> Self {
            Self::Stdio
        }
    }

    #[derive(Debug, Clone)]
    pub struct ServerConfig {
        pub host: String,
        pub port: u16,
        pub transport_mode: TransportMode,
    }

    impl Default for ServerConfig {
        fn default() -> Self {
            Self {
                host: "127.0.0.1".to_string(),
                port: 8765,
                transport_mode: TransportMode::Stdio,
            }
        }
    }

    pub struct McpServer {
        config: ServerConfig,
        tools: Vec<Arc<dyn Tool>>,
        memory: Arc<dyn Memory>,
    }

    impl McpServer {
        pub fn new(
            config: ServerConfig,
            tools: Vec<Arc<dyn Tool>>,
            memory: Arc<dyn Memory>,
        ) -> Self {
            Self {
                config,
                tools,
                memory,
            }
        }

        pub async fn run(&self) -> Result<()> {
            match self.config.transport_mode {
                TransportMode::Stdio => self.run_stdio().await,
                TransportMode::Http => self.run_http().await,
            }
        }

        async fn run_stdio(&self) -> Result<()> {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();
            let mut writer = stdout;

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }

                let response = self.handle_message(&line).await;
                if let Ok(resp_json) = serde_json::to_string(&response) {
                    writer.write_all(resp_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?;
                }
            }

            Ok(())
        }

        async fn run_http(&self) -> Result<()> {
            let addr = format!("{}:{}", self.config.host, self.config.port);
            let listener = TcpListener::bind(&addr).await?;
            tracing::info!("MCP server listening on {}", addr);

            loop {
                let (stream, _) = listener.accept().await?;
                let tools = self.tools.clone();
                let memory = self.memory.clone();

                tokio::spawn(async move {
                    let mut buffer = [0u8; 65536];
                    use tokio::io::AsyncReadExt;
                    let mut stream = stream;
                    if let Ok(n) = stream.read(&mut buffer).await {
                        if n > 0 {
                            let request = String::from_utf8_lossy(&buffer[..n]);
                            let response =
                                Self::handle_message_static(&tools, &memory, &request).await;
                            if let Ok(resp_json) = serde_json::to_string(&response) {
                                let _ = stream.write_all(resp_json.as_bytes());
                            }
                        }
                    }
                });
            }
        }

        async fn handle_message_static(
            tools: &[Arc<dyn Tool>],
            memory: &Arc<dyn Memory>,
            message: &str,
        ) -> JsonRpcResponse {
            let request: Result<JsonRpcRequest, _> = serde_json::from_str(message);
            match request {
                Ok(req) => {
                    let id = req.id.clone();
                    match req.method.as_str() {
                        "initialize" => Self::handle_initialize(id),
                        "tools/list" => Self::handle_tools_list(tools, id),
                        "tools/call" => Self::handle_tools_call(tools, req.params, id).await,
                        "resources/list" => Self::handle_resources_list(memory, id).await,
                        "resources/read" => {
                            Self::handle_resources_read(memory, req.params, id).await
                        }
                        _ => JsonRpcResponse::error(
                            id,
                            McpError::new(error_codes::METHOD_NOT_FOUND, "Method not found"),
                        ),
                    }
                }
                Err(_) => JsonRpcResponse::error(
                    serde_json::Value::Null,
                    McpError::new(error_codes::PARSE_ERROR, "Invalid JSON"),
                ),
            }
        }

        async fn handle_message(&self, message: &str) -> JsonRpcResponse {
            Self::handle_message_static(&self.tools, &self.memory, message).await
        }

        fn handle_initialize(id: serde_json::Value) -> JsonRpcResponse {
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: Some(ToolsServerCapability {
                        list_changed: Some(true),
                    }),
                    resources: Some(ResourcesServerCapability {
                        subscribe: Some(false),
                        list_changed: Some(false),
                    }),
                },
                server_info: ServerInfo {
                    name: "zeroclaw".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            };
            JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
        }

        fn handle_tools_list(tools: &[Arc<dyn Tool>], id: serde_json::Value) -> JsonRpcResponse {
            let tool_definitions: Vec<ToolDefinition> = tools
                .iter()
                .map(|t| ToolDefinition::new(t.name(), t.description(), t.parameters_schema()))
                .collect();

            let result = ToolsListResult {
                tools: tool_definitions,
                next_cursor: None,
            };

            JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
        }

        async fn handle_tools_call(
            tools: &[Arc<dyn Tool>],
            params: Option<serde_json::Value>,
            id: serde_json::Value,
        ) -> JsonRpcResponse {
            let Some(params) = params else {
                return JsonRpcResponse::error(
                    id,
                    McpError::new(error_codes::INVALID_PARAMS, "Missing params"),
                );
            };

            let call_params: Result<ToolsCallRequestParams, _> = serde_json::from_value(params);
            let Ok(call_params) = call_params else {
                return JsonRpcResponse::error(
                    id,
                    McpError::new(error_codes::INVALID_PARAMS, "Invalid params"),
                );
            };

            let tool_name = call_params.name;
            let tool = tools.iter().find(|t| t.name() == tool_name);

            match tool {
                Some(tool) => match tool.execute(call_params.arguments).await {
                    Ok(result) => {
                        let tool_result = if result.success {
                            ToolsCallResult::text(result.output)
                        } else {
                            ToolsCallResult::error(
                                result.error.unwrap_or_else(|| "Unknown error".to_string()),
                            )
                        };
                        JsonRpcResponse::success(id, serde_json::to_value(tool_result).unwrap())
                    }
                    Err(e) => JsonRpcResponse::error(
                        id,
                        McpError::new(error_codes::INTERNAL_ERROR, e.to_string()),
                    ),
                },
                None => JsonRpcResponse::error(
                    id,
                    McpError::new(
                        error_codes::TOOL_NOT_FOUND,
                        format!("Tool not found: {}", tool_name),
                    ),
                ),
            }
        }

        async fn handle_resources_list(
            memory: &Arc<dyn Memory>,
            id: serde_json::Value,
        ) -> JsonRpcResponse {
            match memory.list(None, None).await {
                Ok(entries) => {
                    let resources: Vec<ResourceDefinition> = entries
                        .iter()
                        .map(|entry| {
                            ResourceDefinition::new(
                                format!("memory://{}", entry.key),
                                entry.key.clone(),
                            )
                            .with_description(format!("{} - {}", entry.category, entry.id))
                        })
                        .collect();

                    let result = ResourcesListResult {
                        resources,
                        next_cursor: None,
                    };

                    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
                }
                Err(e) => JsonRpcResponse::error(
                    id,
                    McpError::new(error_codes::INTERNAL_ERROR, e.to_string()),
                ),
            }
        }

        async fn handle_resources_read(
            memory: &Arc<dyn Memory>,
            params: Option<serde_json::Value>,
            id: serde_json::Value,
        ) -> JsonRpcResponse {
            let Some(params) = params else {
                return JsonRpcResponse::error(
                    id,
                    McpError::new(error_codes::INVALID_PARAMS, "Missing params"),
                );
            };

            let read_params: Result<ResourcesReadRequestParams, _> = serde_json::from_value(params);
            let Ok(read_params) = read_params else {
                return JsonRpcResponse::error(
                    id,
                    McpError::new(error_codes::INVALID_PARAMS, "Invalid params"),
                );
            };

            let uri = read_params.uri;
            if let Some(key) = uri.strip_prefix("memory://") {
                match memory.get(key).await {
                    Ok(Some(entry)) => {
                        let content = ResourceContent {
                            uri: uri.clone(),
                            mime_type: Some("text/plain".to_string()),
                            text: Some(entry.content),
                            blob: None,
                        };
                        let result = ResourcesReadResult {
                            contents: vec![content],
                        };
                        JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
                    }
                    Ok(None) => JsonRpcResponse::error(
                        id,
                        McpError::new(error_codes::RESOURCE_NOT_FOUND, "Memory entry not found"),
                    ),
                    Err(e) => JsonRpcResponse::error(
                        id,
                        McpError::new(error_codes::INTERNAL_ERROR, e.to_string()),
                    ),
                }
            } else {
                JsonRpcResponse::error(
                    id,
                    McpError::new(
                        error_codes::RESOURCE_NOT_FOUND,
                        "Unknown resource URI scheme",
                    ),
                )
            }
        }
    }

    pub async fn create_mcp_server(
        config: ServerConfig,
        tool_registry: Vec<Arc<dyn Tool>>,
        memory: Arc<dyn Memory>,
    ) -> Result<McpServer> {
        Ok(McpServer::new(config, tool_registry, memory))
    }
}

pub use server::*;
