use crate::providers::traits::{
    ChatMessage, ChatRequest as ProviderChatRequest, ChatResponse as ProviderChatResponse,
    Provider, ProviderCapabilities, TokenUsage, ToolCall as ProviderToolCall, ToolsPayload,
};
use crate::tools::ToolSpec;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const QWEN25_DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";

pub struct Qwen25Provider {
    base_url: String,
    credential: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolSpecSerialized>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Content,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    MultiContent(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
struct ContentPart {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<String>,
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

#[derive(Debug, Serialize, Clone)]
struct ToolSpecSerialized {
    #[serde(rename = "type")]
    kind: String,
    function: ToolFunction,
}

#[derive(Debug, Serialize, Clone)]
struct ToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<UsageInfo>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    id: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    function: FunctionCall,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
}

impl Qwen25Provider {
    pub fn new(credential: Option<&str>) -> Self {
        Self::with_base_url(None, credential)
    }

    pub fn with_base_url(base_url: Option<&str>, credential: Option<&str>) -> Self {
        Self {
            base_url: base_url
                .map(|u| u.trim_end_matches('/').to_string())
                .unwrap_or_else(|| QWEN25_DEFAULT_BASE_URL.to_string()),
            credential: credential.map(ToString::to_string),
        }
    }

    fn build_content(&self, text: &str, image_urls: &[String]) -> Content {
        if image_urls.is_empty() {
            Content::Text(text.to_string())
        } else {
            let mut parts = vec![ContentPart {
                kind: "text".to_string(),
                text: Some(text.to_string()),
                image: None,
            }];
            for url in image_urls {
                parts.push(ContentPart {
                    kind: "image_url".to_string(),
                    text: None,
                    image: Some(url.clone()),
                });
            }
            Content::MultiContent(parts)
        }
    }

    fn convert_messages(&self, messages: &[ChatMessage]) -> Vec<Message> {
        messages
            .iter()
            .map(|m| {
                let content = if m.role == "user" {
                    let (text, image_urls) = extract_image_urls(&m.content);
                    self.build_content(&text, &image_urls)
                } else if m.role == "assistant" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&m.content) {
                        if let Some(tool_calls_value) = value.get("tool_calls") {
                            if let Ok(parsed_calls) =
                                serde_json::from_value::<Vec<ProviderToolCall>>(
                                    tool_calls_value.clone(),
                                )
                            {
                                let _tool_calls = parsed_calls
                                    .into_iter()
                                    .map(|tc| ToolCall {
                                        id: Some(tc.id),
                                        kind: Some("function".to_string()),
                                        function: FunctionCall {
                                            name: tc.name,
                                            arguments: tc.arguments,
                                        },
                                    })
                                    .collect::<Vec<_>>();
                                let content = value
                                    .get("content")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToString::to_string);
                                let reasoning_content = value
                                    .get("reasoning_content")
                                    .and_then(serde_json::Value::as_str)
                                    .map(ToString::to_string);

                                let final_content = match (content, reasoning_content) {
                                    (Some(c), _) if !c.is_empty() => Content::Text(c),
                                    (_, Some(r)) if !r.is_empty() => Content::Text(r),
                                    _ => Content::Text(String::new()),
                                };

                                return Message {
                                    role: m.role.clone(),
                                    content: final_content,
                                };
                            }
                        }
                    }
                    Content::Text(m.content.clone())
                } else if m.role == "tool" {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&m.content) {
                        let _tool_call_id = value
                            .get("tool_call_id")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string);
                        let content = value
                            .get("content")
                            .and_then(serde_json::Value::as_str)
                            .map(ToString::to_string)
                            .unwrap_or_default();
                        return Message {
                            role: "tool".to_string(),
                            content: Content::Text(content),
                        };
                    }
                    Content::Text(m.content.clone())
                } else {
                    Content::Text(m.content.clone())
                };

                Message {
                    role: m.role.clone(),
                    content,
                }
            })
            .collect()
    }

    fn serialize_tools(&self, tools: Option<&[ToolSpec]>) -> Option<Vec<ToolSpecSerialized>> {
        tools.map(|items| {
            items
                .iter()
                .map(|tool| ToolSpecSerialized {
                    kind: "function".to_string(),
                    function: ToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect()
        })
    }

    fn parse_response(&self, message: ResponseMessage) -> ProviderChatResponse {
        let text = message
            .content
            .clone()
            .or_else(|| message.reasoning_content.clone())
            .filter(|c| !c.is_empty());
        let reasoning_content = message.reasoning_content;
        let tool_calls = message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|tc| ProviderToolCall {
                id: tc.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                name: tc.function.name,
                arguments: tc.function.arguments,
            })
            .collect::<Vec<_>>();

        ProviderChatResponse {
            text,
            tool_calls,
            usage: None,
            reasoning_content,
        }
    }

    fn http_client(&self) -> Client {
        crate::config::build_runtime_proxy_client_with_timeouts("provider.qwen25", 120, 10)
    }
}

fn extract_image_urls(content: &str) -> (String, Vec<String>) {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        if let Some(images) = value.get("images").and_then(|v| v.as_array()) {
            let urls: Vec<String> = images
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
            let text = value
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return (text, urls);
        }
    }
    (content.to_string(), Vec::new())
}

#[async_trait]
impl Provider for Qwen25Provider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            native_tool_calling: true,
            vision: true,
        }
    }

    fn convert_tools(&self, tools: &[ToolSpec]) -> ToolsPayload {
        let native_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters
                    }
                })
            })
            .collect();
        ToolsPayload::OpenAI {
            tools: native_tools,
        }
    }

    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Qwen API key not set. Set QWEN_API_KEY or DASHSCOPE_API_KEY.")
        })?;

        let mut messages = Vec::new();

        if let Some(sys) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: Content::Text(sys.to_string()),
            });
        }

        messages.push(Message {
            role: "user".to_string(),
            content: Content::Text(message.to_string()),
        });

        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens: None,
            top_p: None,
            tools: None,
            tool_choice: None,
        };

        let response = self
            .http_client()
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {credential}"))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(super::api_error("Qwen", response).await);
        }

        let chat_response: ChatResponse = response.json().await?;

        chat_response
            .choices
            .into_iter()
            .next()
            .map(|c| {
                c.message
                    .content
                    .or_else(|| c.message.reasoning_content)
                    .unwrap_or_default()
            })
            .ok_or_else(|| anyhow::anyhow!("No response from Qwen"))
    }

    async fn chat(
        &self,
        request: ProviderChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Qwen API key not set. Set QWEN_API_KEY or DASHSCOPE_API_KEY.")
        })?;

        let tool_payload = request.tools.map(|tools| {
            tools
                .iter()
                .map(|tool| ToolSpecSerialized {
                    kind: "function".to_string(),
                    function: ToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect::<Vec<_>>()
        });

        let native_request = ChatRequest {
            model: model.to_string(),
            messages: self.convert_messages(request.messages),
            temperature,
            max_tokens: None,
            top_p: None,
            tools: tool_payload.clone(),
            tool_choice: tool_payload.as_ref().map(|_| "auto".to_string()),
        };

        let response = self
            .http_client()
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {credential}"))
            .header("Content-Type", "application/json")
            .json(&native_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(super::api_error("Qwen", response).await);
        }

        let native_response: ChatResponse = response.json().await?;
        let usage = native_response.usage.map(|u| TokenUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });
        let message = native_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| anyhow::anyhow!("No response from Qwen"))?;
        let mut result = self.parse_response(message);
        result.usage = usage;
        Ok(result)
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[serde_json::Value],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ProviderChatResponse> {
        let credential = self.credential.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Qwen API key not set. Set QWEN_API_KEY or DASHSCOPE_API_KEY.")
        })?;

        let native_tools: Option<Vec<ToolSpecSerialized>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .filter_map(|t| {
                        let tool_type = t.get("type")?.as_str()?;
                        if tool_type != "function" {
                            return None;
                        }
                        let function = t.get("function")?;
                        Some(ToolSpecSerialized {
                            kind: "function".to_string(),
                            function: ToolFunction {
                                name: function.get("name")?.as_str()?.to_string(),
                                description: function
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                parameters: function
                                    .get("parameters")
                                    .cloned()
                                    .unwrap_or(serde_json::json!({})),
                            },
                        })
                    })
                    .collect(),
            )
        };

        let native_request = ChatRequest {
            model: model.to_string(),
            messages: self.convert_messages(messages),
            temperature,
            max_tokens: None,
            top_p: None,
            tools: native_tools.clone(),
            tool_choice: native_tools.as_ref().map(|_| "auto".to_string()),
        };

        let response = self
            .http_client()
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {credential}"))
            .header("Content-Type", "application/json")
            .json(&native_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(super::api_error("Qwen", response).await);
        }

        let native_response: ChatResponse = response.json().await?;
        let usage = native_response.usage.map(|u| TokenUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
        });
        let message = native_response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or_else(|| anyhow::anyhow!("No response from Qwen"))?;
        let mut result = self.parse_response(message);
        result.usage = usage;
        Ok(result)
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        if let Some(credential) = self.credential.as_ref() {
            self.http_client()
                .get(format!("{}/models", self.base_url))
                .header("Authorization", format!("Bearer {credential}"))
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_with_key() {
        let p = Qwen25Provider::new(Some("qwen-test-credential"));
        assert_eq!(p.credential.as_deref(), Some("qwen-test-credential"));
    }

    #[test]
    fn creates_without_key() {
        let p = Qwen25Provider::new(None);
        assert!(p.credential.is_none());
    }

    #[test]
    fn creates_with_empty_key() {
        let p = Qwen25Provider::new(Some(""));
        assert_eq!(p.credential.as_deref(), Some(""));
    }

    #[tokio::test]
    async fn chat_fails_without_key() {
        let p = Qwen25Provider::new(None);
        let result = p.chat_with_system(None, "hello", "qwen2.5", 0.7).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key not set"));
    }

    #[test]
    fn content_serializes_text_only() {
        let content = Content::Text("Hello".to_string());
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("Hello"));
    }

    #[test]
    fn content_serializes_with_image() {
        let content = Content::MultiContent(vec![
            ContentPart {
                kind: "text".to_string(),
                text: Some("Describe this".to_string()),
                image: None,
            },
            ContentPart {
                kind: "image_url".to_string(),
                text: None,
                image: Some("https://example.com/image.jpg".to_string()),
            },
        ]);
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("image_url"));
        assert!(json.contains("https://example.com/image.jpg"));
    }

    #[test]
    fn message_serializes_user() {
        let msg = Message {
            role: "user".to_string(),
            content: Content::Text("Hello".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("Hello"));
    }

    #[test]
    fn request_serializes_with_temperature() {
        let req = ChatRequest {
            model: "qwen2.5".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: Content::Text("hello".to_string()),
            }],
            temperature: 0.7,
            max_tokens: Some(1000),
            top_p: Some(0.9),
            tools: None,
            tool_choice: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":1000"));
        assert!(json.contains("\"top_p\":0.9"));
    }

    #[test]
    fn response_deserializes_single_choice() {
        let json = r#"{"choices":[{"message":{"content":"Hi!"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content, Some("Hi!".to_string()));
    }

    #[test]
    fn response_deserializes_tool_calls() {
        let json = r#"{"choices":[{"message":{"content":"Let me check","tool_calls":[{"id":"call_123","type":"function","function":{"name":"shell","arguments":"{\"command\":\"ls\"}"}}]}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        let msg = &resp.choices[0].message;
        assert_eq!(msg.content.as_deref(), Some("Let me check"));
        assert!(msg.tool_calls.as_ref().is_some_and(|tc| !tc.is_empty()));
        let tc = &msg.tool_calls.as_ref().unwrap()[0];
        assert_eq!(tc.function.name, "shell");
    }

    #[test]
    fn provider_supports_native_tools() {
        let p = Qwen25Provider::new(Some("test"));
        assert!(p.supports_native_tools());
    }

    #[test]
    fn provider_supports_vision() {
        let p = Qwen25Provider::new(Some("test"));
        assert!(p.supports_vision());
    }

    #[test]
    fn convert_tools_creates_openai_format() {
        let p = Qwen25Provider::new(Some("test"));
        let tools = vec![ToolSpec {
            name: "shell".to_string(),
            description: "Run a shell command".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string"}
                },
                "required": ["command"]
            }),
        }];
        let payload = p.convert_tools(&tools);
        match payload {
            ToolsPayload::OpenAI { tools: native } => {
                assert_eq!(native.len(), 1);
                assert!(native[0].get("function").is_some());
            }
            _ => panic!("Expected OpenAI tools payload"),
        }
    }

    #[test]
    fn extract_image_urls_parses_json() {
        let content = r#"{"text":"What is this?","images":["https://example.com/1.jpg","https://example.com/2.jpg"]}"#;
        let (text, urls) = extract_image_urls(content);
        assert_eq!(text, "What is this?");
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/1.jpg".to_string()));
    }

    #[test]
    fn extract_image_urls_returns_plain_text() {
        let content = "Just plain text without images";
        let (text, urls) = extract_image_urls(content);
        assert_eq!(text, "Just plain text without images");
        assert!(urls.is_empty());
    }
}
