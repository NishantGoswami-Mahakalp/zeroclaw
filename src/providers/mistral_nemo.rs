use crate::providers::traits::{
    ChatMessage, ChatResponse, Provider, ProviderCapabilities, TokenUsage, ToolCall,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";

pub struct MistralNeMoProvider {
    base_url: String,
    api_key: Option<String>,
    safe_prompt: bool,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    top_p: Option<f64>,
    random_seed: Option<u32>,
    safe_prompt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCallJson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ToolCallJson {
    id: Option<String>,
    #[serde(rename = "type")]
    kind: String,
    function: FunctionJson,
}

#[derive(Debug, Serialize)]
struct FunctionJson {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiChatResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    index: u32,
    message: ResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Vec<ApiToolCall>,
}

#[derive(Debug, Deserialize)]
struct ApiToolCall {
    id: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    function: ApiFunction,
}

#[derive(Debug, Deserialize)]
struct ApiFunction {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

impl MistralNeMoProvider {
    pub fn new(api_key: Option<&str>) -> Self {
        let api_key = api_key.and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

        Self {
            base_url: MISTRAL_BASE_URL.to_string(),
            api_key,
            safe_prompt: false,
        }
    }

    pub fn with_safe_prompt(mut self, safe_prompt: bool) -> Self {
        self.safe_prompt = safe_prompt;
        self
    }

    pub fn with_base_url(mut self, base_url: Option<&str>) -> Self {
        if let Some(url) = base_url {
            let trimmed = url.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                self.base_url = trimmed.to_string();
            }
        }
        self
    }

    fn http_client(&self) -> Client {
        crate::config::build_runtime_proxy_client_with_timeouts("provider.mistral", 300, 10)
    }

    fn convert_messages(&self, messages: &[ChatMessage]) -> Vec<Message> {
        messages
            .iter()
            .map(|msg| {
                let mut tool_calls = None;

                if msg.role == "assistant" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&msg.content) {
                        if let Some(calls) = value.get("tool_calls").and_then(|v| v.as_array()) {
                            let converted: Vec<ToolCallJson> = calls
                                .iter()
                                .filter_map(|c| {
                                    let id = c.get("id").and_then(|v| v.as_str()).map(String::from);
                                    let name = c
                                        .get("function")
                                        .and_then(|f| f.get("name"))
                                        .and_then(|n| n.as_str())
                                        .map(String::from)?;
                                    let args = c
                                        .get("function")
                                        .and_then(|f| f.get("arguments"))
                                        .cloned()
                                        .unwrap_or(serde_json::json!({}));
                                    Some(ToolCallJson {
                                        id,
                                        kind: "function".to_string(),
                                        function: FunctionJson {
                                            name,
                                            arguments: args,
                                        },
                                    })
                                })
                                .collect();
                            if !converted.is_empty() {
                                tool_calls = Some(converted);
                            }
                        }
                    }
                }

                let (tool_call_id, name, content) = if msg.role == "tool" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&msg.content) {
                        let tc_id = value
                            .get("tool_call_id")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let tool_name = value
                            .get("tool_name")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        let text = value
                            .get("content")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        (tc_id, tool_name, text.unwrap_or_default())
                    } else {
                        (None, None, msg.content.clone())
                    }
                } else {
                    (None, None, msg.content.clone())
                };

                Message {
                    role: msg.role.clone(),
                    content,
                    name,
                    tool_calls,
                    tool_call_id,
                }
            })
            .collect()
    }

    async fn send_request(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f64,
        tools: Option<&[serde_json::Value]>,
    ) -> anyhow::Result<ApiChatResponse> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature: Some(temperature),
            max_tokens: None,
            top_p: None,
            random_seed: None,
            safe_prompt: self.safe_prompt,
            tools: tools.map(|t| t.to_vec()),
            parallel_tool_calls: Some(true),
            response_format: None,
            stream: Some(false),
        };

        let url = format!("{}/chat/completions", self.base_url);

        tracing::debug!(
            "Mistral request: url={} model={} message_count={} temperature={}",
            url,
            model,
            request.messages.len(),
            temperature
        );

        let mut request_builder = self.http_client().post(&url).json(&request);

        if let Some(key) = self.api_key.as_ref() {
            request_builder = request_builder.bearer_auth(key);
        }

        let response = request_builder.send().await?;
        let status = response.status();
        tracing::debug!("Mistral response status: {}", status);

        let body = response.bytes().await?;
        tracing::debug!("Mistral response body length: {} bytes", body.len());

        if !status.is_success() {
            let raw = String::from_utf8_lossy(&body);
            let sanitized = crate::providers::sanitize_api_error(&raw);
            tracing::error!(
                "Mistral error response: status={} body_excerpt={}",
                status,
                sanitized
            );
            anyhow::bail!("Mistral API error ({}): {}", status, sanitized);
        }

        let chat_response: ApiChatResponse = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(e) => {
                let raw = String::from_utf8_lossy(&body);
                let sanitized = crate::providers::sanitize_api_error(&raw);
                tracing::error!(
                    "Mistral response deserialization failed: {e}. body_excerpt={}",
                    sanitized
                );
                anyhow::bail!("Failed to parse Mistral response: {e}");
            }
        };

        Ok(chat_response)
    }
}

#[async_trait]
impl Provider for MistralNeMoProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: false,
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let mut messages = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: sys.to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        messages.push(Message {
            role: "user".to_string(),
            content: message.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        let response = self
            .send_request(messages, model, temperature, None)
            .await?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let api_messages = self.convert_messages(messages);

        let response = self
            .send_request(api_messages, model, temperature, None)
            .await?;

        let content = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let api_messages = self.convert_messages(messages);

        let tools_opt = if tools.is_empty() { None } else { Some(tools) };

        let response = self
            .send_request(api_messages, model, temperature, tools_opt)
            .await?;

        let usage = TokenUsage {
            input_tokens: response.usage.prompt_tokens,
            output_tokens: response.usage.completion_tokens,
        };

        let choice = response.choices.first();

        if let Some(choice) = choice {
            if !choice.message.tool_calls.is_empty() {
                let tool_calls: Vec<ToolCall> = choice
                    .message
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc
                            .id
                            .clone()
                            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                        name: tc.function.name.clone(),
                        arguments: serde_json::to_string(&tc.function.arguments)
                            .unwrap_or_else(|_| "{}".to_string()),
                    })
                    .collect();

                let text = if choice.message.content.is_empty() {
                    None
                } else {
                    Some(choice.message.content.clone())
                };

                return Ok(ChatResponse {
                    text,
                    tool_calls,
                    usage: Some(usage),
                    reasoning_content: None,
                });
            }

            let content = choice.message.content.clone();
            if !content.is_empty() {
                return Ok(ChatResponse {
                    text: Some(content),
                    tool_calls: vec![],
                    usage: Some(usage),
                    reasoning_content: None,
                });
            }
        }

        Ok(ChatResponse {
            text: Some(String::new()),
            tool_calls: vec![],
            usage: Some(usage),
            reasoning_content: None,
        })
    }

    async fn chat(
        &self,
        request: crate::providers::traits::ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        if let Some(specs) = request.tools {
            if !specs.is_empty() {
                let tools: Vec<serde_json::Value> = specs
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "type": "function",
                            "function": {
                                "name": s.name,
                                "description": s.description,
                                "parameters": s.parameters
                            }
                        })
                    })
                    .collect();
                return self
                    .chat_with_tools(request.messages, &tools, model, temperature)
                    .await;
            }
        }

        let text = self
            .chat_with_history(request.messages, model, temperature)
            .await?;
        Ok(ChatResponse {
            text: Some(text),
            tool_calls: vec![],
            usage: None,
            reasoning_content: None,
        })
    }

    fn supports_native_tools(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_url() {
        let p = MistralNeMoProvider::new(None);
        assert_eq!(p.base_url, MISTRAL_BASE_URL);
    }

    #[test]
    fn custom_url() {
        let p = MistralNeMoProvider::new(None).with_base_url(Some("https://custom.mistral.ai/v1"));
        assert_eq!(p.base_url, "https://custom.mistral.ai/v1");
    }

    #[test]
    fn custom_url_trims_slash() {
        let p = MistralNeMoProvider::new(None).with_base_url(Some("https://custom.mistral.ai/v1/"));
        assert_eq!(p.base_url, "https://custom.mistral.ai/v1");
    }

    #[test]
    fn api_key_handling() {
        let p = MistralNeMoProvider::new(Some("test-key"));
        assert_eq!(p.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn api_key_trims_whitespace() {
        let p = MistralNeMoProvider::new(Some("  test-key  "));
        assert_eq!(p.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn api_key_empty_becomes_none() {
        let p = MistralNeMoProvider::new(Some(""));
        assert!(p.api_key.is_none());
    }

    #[test]
    fn safe_prompt_default_false() {
        let p = MistralNeMoProvider::new(None);
        assert!(!p.safe_prompt);
    }

    #[test]
    fn safe_prompt_can_be_enabled() {
        let p = MistralNeMoProvider::new(None).with_safe_prompt(true);
        assert!(p.safe_prompt);
    }

    #[test]
    fn response_deserializes() {
        let json = r#"{
            "id": "cmpl-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "mistral-nemo",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;
        let resp: ApiChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "Hello!");
        assert_eq!(resp.usage.total_tokens, Some(15));
    }

    #[test]
    fn response_with_tool_calls() {
        let json = r#"{
            "id": "cmpl-test",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "mistral-nemo",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "shell", "arguments": {"command": "date"}}}]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        }"#;
        let resp: ApiChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.tool_calls.len(), 1);
        assert_eq!(resp.choices[0].message.tool_calls[0].function.name, "shell");
    }

    #[test]
    fn convert_messages_user_role() {
        let provider = MistralNeMoProvider::new(None);
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }];
        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[0].content, "Hello");
    }

    #[test]
    fn convert_messages_system_role() {
        let provider = MistralNeMoProvider::new(None);
        let messages = vec![ChatMessage {
            role: "system".to_string(),
            content: "You are helpful".to_string(),
        }];
        let converted = provider.convert_messages(&messages);
        assert_eq!(converted[0].role, "system");
    }

    #[test]
    fn convert_messages_assistant_with_tool_calls() {
        let provider = MistralNeMoProvider::new(None);
        let messages = vec![ChatMessage {
            role: "assistant".to_string(),
            content: r#"{"tool_calls":[{"id":"call_1","function":{"name":"shell","arguments":"{\"command\":\"ls\"}"}}]}"#.to_string(),
        }];
        let converted = provider.convert_messages(&messages);
        assert!(converted[0].tool_calls.is_some());
        assert_eq!(converted[0].tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(
            converted[0].tool_calls.as_ref().unwrap()[0].function.name,
            "shell"
        );
    }

    #[test]
    fn convert_messages_tool_result() {
        let provider = MistralNeMoProvider::new(None);
        let messages = vec![ChatMessage {
            role: "tool".to_string(),
            content: r#"{"tool_call_id":"call_1","content":"result"}"#.to_string(),
        }];
        let converted = provider.convert_messages(&messages);
        assert_eq!(converted[0].role, "tool");
        assert_eq!(converted[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(converted[0].content, "result");
    }

    #[test]
    fn capabilities_has_native_tool_calling() {
        let provider = MistralNeMoProvider::new(None);
        let caps = <MistralNeMoProvider as Provider>::capabilities(&provider);
        assert!(caps.native_tool_calling);
        assert!(!caps.vision);
    }
}
