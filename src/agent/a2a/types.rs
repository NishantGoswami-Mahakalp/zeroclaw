use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: AgentCapabilities,
    pub provider: Option<ProviderInfo>,
    pub authentication: AuthenticationRequirements,
    pub endpoint: String,
}

impl AgentCard {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: None,
            capabilities: AgentCapabilities::default(),
            provider: None,
            authentication: AuthenticationRequirements::default(),
            endpoint: endpoint.into(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_capabilities(mut self, capabilities: AgentCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_provider(mut self, provider: ProviderInfo) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn with_authentication(mut self, auth: AuthenticationRequirements) -> Self {
        self.authentication = auth;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_transition_history: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_uploads: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_upload: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

impl AgentCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.streaming = Some(enabled);
        self
    }

    pub fn with_push_notifications(mut self, enabled: bool) -> Self {
        self.push_notifications = Some(enabled);
        self
    }

    pub fn with_state_history(mut self, enabled: bool) -> Self {
        self.state_transition_history = Some(enabled);
        self
    }

    pub fn with_file_uploads(mut self, enabled: bool) -> Self {
        self.file_uploads = Some(enabled);
        self
    }

    pub fn with_image_upload(mut self, enabled: bool) -> Self {
        self.image_upload = Some(enabled);
        self
    }

    pub fn with_tools(mut self, tools: ToolsCapability) -> Self {
        self.tools = Some(tools);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl ToolsCapability {
    pub fn new() -> Self {
        Self { list_changed: None }
    }

    pub fn with_list_changed(mut self, enabled: bool) -> Self {
        self.list_changed = Some(enabled);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub organization: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl ProviderInfo {
    pub fn new(organization: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            organization: organization.into(),
            provider: provider.into(),
            model: None,
            version: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthenticationRequirements {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemes: Option<Vec<AuthScheme>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<String>>,
}

impl AuthenticationRequirements {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn with_schemes(mut self, schemes: Vec<AuthScheme>) -> Self {
        self.schemes = Some(schemes);
        self
    }

    pub fn with_credentials(mut self, credentials: Vec<String>) -> Self {
        self.credentials = Some(credentials);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthScheme {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "basic")]
    Basic,
    #[serde(rename = "bearer")]
    Bearer,
    #[serde(rename = "api_key")]
    ApiKey,
    #[serde(rename = "oauth2")]
    OAuth2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    pub id: String,
    pub type_: MessageType,
    pub headers: MessageHeaders,
    pub body: MessageBody,
}

impl MessageEnvelope {
    pub fn new(id: impl Into<String>, type_: MessageType, body: MessageBody) -> Self {
        Self {
            id: id.into(),
            type_,
            headers: MessageHeaders::default(),
            body,
        }
    }

    pub fn with_headers(mut self, headers: MessageHeaders) -> Self {
        self.headers = headers;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageHeaders {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<serde_json::Value>,
}

impl MessageHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    pub fn with_timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    pub fn with_message_id(mut self, id: impl Into<String>) -> Self {
        self.message_id = Some(id.into());
        self
    }

    pub fn with_return_url(mut self, url: impl Into<String>) -> Self {
        self.return_url = Some(url.into());
        self
    }

    pub fn with_custom(mut self, custom: serde_json::Value) -> Self {
        self.custom = Some(custom);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Request,
    Response,
    Notification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageBody {
    TaskSubmit(TaskSubmitMessage),
    TaskGet(TaskGetMessage),
    TaskCancel(TaskCancelMessage),
    TaskStatus(TaskStatusMessage),
    TaskResult(TaskResultMessage),
    Error(ErrorMessage),
    Authentication(AuthenticationMessage),
    PushNotification(PushNotificationMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSubmitMessage {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub message: AgentMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_content_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification: Option<bool>,
}

impl TaskSubmitMessage {
    pub fn new(id: impl Into<String>, message: AgentMessage) -> Self {
        Self {
            id: id.into(),
            session_id: None,
            message,
            accepted_content_types: None,
            push_notification: None,
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_content_types(mut self, types: Vec<String>) -> Self {
        self.accepted_content_types = Some(types);
        self
    }

    pub fn with_push_notification(mut self, enabled: bool) -> Self {
        self.push_notification = Some(enabled);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGetMessage {
    pub id: String,
}

impl TaskGetMessage {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelMessage {
    pub id: String,
}

impl TaskCancelMessage {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusMessage {
    pub id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AgentMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<StateTransition>>,
}

impl TaskStatusMessage {
    pub fn new(id: impl Into<String>, status: TaskStatus) -> Self {
        Self {
            id: id.into(),
            status,
            message: None,
            history: None,
        }
    }

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_history(mut self, history: Vec<StateTransition>) -> Self {
        self.history = Some(history);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Submitted,
    Queued,
    Working,
    InputRequired,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: TaskStatus,
    pub to: TaskStatus,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AgentMessage>,
}

impl StateTransition {
    pub fn new(from: TaskStatus, to: TaskStatus, timestamp: impl Into<String>) -> Self {
        Self {
            from,
            to,
            timestamp: timestamp.into(),
            message: None,
        }
    }

    pub fn with_message(mut self, message: AgentMessage) -> Self {
        self.message = Some(message);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultMessage {
    pub id: String,
    pub result: AgentMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl TaskResultMessage {
    pub fn new(id: impl Into<String>, result: AgentMessage) -> Self {
        Self {
            id: id.into(),
            result,
            error: None,
        }
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AgentMessage {
    pub fn new(role: MessageRole, parts: Vec<MessagePart>) -> Self {
        Self {
            role,
            parts,
            metadata: None,
        }
    }

    pub fn user(parts: Vec<MessagePart>) -> Self {
        Self::new(MessageRole::User, parts)
    }

    pub fn agent(parts: Vec<MessagePart>) -> Self {
        Self::new(MessageRole::Agent, parts)
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn text_part(content: impl Into<String>) -> MessagePart {
        MessagePart::Text {
            text: content.into(),
        }
    }

    pub fn file_part(file: FilePart) -> MessagePart {
        MessagePart::File { file }
    }

    pub fn tool_use_part(tool_use: ToolUsePart) -> MessagePart {
        MessagePart::ToolUse { tool_use }
    }

    pub fn tool_result_part(tool_result: ToolResultPart) -> MessagePart {
        MessagePart::ToolResult { tool_result }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessagePart {
    Text { text: String },
    File { file: FilePart },
    ToolUse { tool_use: ToolUsePart },
    ToolResult { tool_result: ToolResultPart },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePart {
    pub name: Option<String>,
    pub mime_type: Option<String>,
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<String>,
}

impl FilePart {
    pub fn new() -> Self {
        Self {
            name: None,
            mime_type: None,
            uri: None,
            bytes: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn with_bytes(mut self, bytes: impl Into<String>) -> Self {
        self.bytes = Some(bytes.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsePart {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

impl ToolUsePart {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultPart {
    pub tool_use_id: String,
    pub content: ToolResultContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResultPart {
    pub fn new(tool_use_id: impl Into<String>, content: ToolResultContent) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content,
            is_error: None,
        }
    }

    pub fn with_error(mut self, is_error: bool) -> Self {
        self.is_error = Some(is_error);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ToolResultContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { uri: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ErrorMessage {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const TASK_NOT_FOUND: i32 = -32001;
    pub const TASK_CANCEL_FAILED: i32 = -32002;
    pub const AUTHENTICATION_FAILED: i32 = -32003;
    pub const INVALID_AUTHENTICATION: i32 = -32004;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationMessage {
    pub schemes: Vec<AuthScheme>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<serde_json::Value>,
}

impl AuthenticationMessage {
    pub fn new(schemes: Vec<AuthScheme>) -> Self {
        Self {
            schemes,
            credentials: None,
        }
    }

    pub fn with_credentials(mut self, credentials: serde_json::Value) -> Self {
        self.credentials = Some(credentials);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationMessage {
    pub id: String,
    pub message: AgentMessage,
}

impl PushNotificationMessage {
    pub fn new(id: impl Into<String>, message: AgentMessage) -> Self {
        Self {
            id: id.into(),
            message,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ARequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl A2ARequest {
    pub fn new(
        id: serde_json::Value,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorMessage>,
}

impl A2AResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, error: ErrorMessage) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ANotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl A2ANotification {
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_card_builder() {
        let card = AgentCard::new("test-agent", "1.0.0", "https://example.com/a2a")
            .with_description("A test agent")
            .with_capabilities(AgentCapabilities::new().with_streaming(true))
            .with_provider(ProviderInfo::new("TestOrg", "openai").with_model("gpt-4"))
            .with_authentication(AuthenticationRequirements::new().with_required(true));

        assert_eq!(card.name, "test-agent");
        assert_eq!(card.version, "1.0.0");
        assert_eq!(card.endpoint, "https://example.com/a2a");
        assert_eq!(card.description, Some("A test agent".to_string()));
        assert!(card.capabilities.streaming.unwrap_or(false));
        assert_eq!(
            card.provider.as_ref().unwrap().model,
            Some("gpt-4".to_string())
        );
    }

    #[test]
    fn agent_capabilities_default() {
        let caps = AgentCapabilities::default();
        assert!(caps.streaming.is_none());
        assert!(caps.tools.is_none());
    }

    #[test]
    fn agent_capabilities_builder() {
        let caps = AgentCapabilities::new()
            .with_streaming(true)
            .with_push_notifications(true)
            .with_state_history(true)
            .with_tools(ToolsCapability::new().with_list_changed(true));

        assert_eq!(caps.streaming, Some(true));
        assert_eq!(caps.push_notifications, Some(true));
        assert_eq!(caps.state_transition_history, Some(true));
        assert!(caps.tools.is_some());
    }

    #[test]
    fn message_envelope_builder() {
        let envelope = MessageEnvelope::new(
            "msg-123",
            MessageType::Request,
            MessageBody::TaskSubmit(TaskSubmitMessage::new(
                "task-1",
                AgentMessage::user(vec![AgentMessage::text_part("Hello")]),
            )),
        )
        .with_headers(MessageHeaders::new().with_correlation_id("corr-456"));

        assert_eq!(envelope.id, "msg-123");
        assert!(matches!(envelope.type_, MessageType::Request));
        assert_eq!(
            envelope.headers.correlation_id,
            Some("corr-456".to_string())
        );
    }

    #[test]
    fn task_submit_message_builder() {
        let msg = TaskSubmitMessage::new(
            "task-1",
            AgentMessage::user(vec![
                AgentMessage::text_part("Hello"),
                AgentMessage::file_part(FilePart::new().with_name("test.txt")),
            ]),
        )
        .with_session_id("session-1")
        .with_content_types(vec!["text/plain".to_string()])
        .with_push_notification(true);

        assert_eq!(msg.id, "task-1");
        assert_eq!(msg.session_id, Some("session-1".to_string()));
        assert_eq!(
            msg.accepted_content_types,
            Some(vec!["text/plain".to_string()])
        );
        assert_eq!(msg.push_notification, Some(true));
    }

    #[test]
    fn task_status_message() {
        let msg = TaskStatusMessage::new("task-1", TaskStatus::Working)
            .with_message(AgentMessage::agent(vec![AgentMessage::text_part(
                "Processing...",
            )]))
            .with_history(vec![StateTransition::new(
                TaskStatus::Submitted,
                TaskStatus::Queued,
                "2024-01-01T00:00:00Z",
            )]);

        assert_eq!(msg.status, TaskStatus::Working);
        assert!(msg.message.is_some());
        assert!(msg.history.is_some());
    }

    #[test]
    fn agent_message_parts() {
        let msg = AgentMessage::user(vec![
            AgentMessage::text_part("Hello"),
            AgentMessage::file_part(FilePart::new().with_uri("file://test.txt")),
            AgentMessage::tool_use_part(ToolUsePart::new(
                "tool-1",
                "shell",
                serde_json::json!({"command": "ls"}),
            )),
            AgentMessage::tool_result_part(ToolResultPart::new(
                "tool-1",
                ToolResultContent::Text {
                    text: "result".to_string(),
                },
            )),
        ]);

        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.parts.len(), 4);
    }

    #[test]
    fn a2a_request_serialization() {
        let req = A2ARequest::new(
            serde_json::Value::Number(1.into()),
            "tasks/submit",
            Some(serde_json::json!({"id": "task-1"})),
        );
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""method":"tasks/submit""#));
        assert!(json.contains(r#""id":1"#));
    }

    #[test]
    fn a2a_response_success() {
        let resp = A2AResponse::success(
            serde_json::Value::Number(1.into()),
            serde_json::json!({"status": "completed"}),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""result":{"status":"completed"}"#));
    }

    #[test]
    fn a2a_response_error() {
        let error = ErrorMessage::new(error_codes::TASK_NOT_FOUND, "Task not found");
        let resp = A2AResponse::error(serde_json::Value::Number(1.into()), error);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""error":{"code":-32001,"message":"Task not found"}"#));
    }

    #[test]
    fn a2a_notification() {
        let notification = A2ANotification::new(
            "tasks/statusUpdate",
            Some(serde_json::json!({"id": "task-1", "status": "working"})),
        );
        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains(r#""method":"tasks/statusUpdate""#));
    }

    #[test]
    fn auth_scheme_variants() {
        let schemes = vec![
            AuthScheme::None,
            AuthScheme::Basic,
            AuthScheme::Bearer,
            AuthScheme::ApiKey,
            AuthScheme::OAuth2,
        ];
        let json = serde_json::to_string(&schemes).unwrap();
        assert!(json.contains("\"none\""));
        assert!(json.contains("\"basic\""));
        assert!(json.contains("\"bearer\""));
    }

    #[test]
    fn task_status_variants() {
        let statuses = vec![
            TaskStatus::Submitted,
            TaskStatus::Queued,
            TaskStatus::Working,
            TaskStatus::InputRequired,
            TaskStatus::Completed,
            TaskStatus::Failed,
            TaskStatus::Canceled,
        ];
        let json = serde_json::to_string(&statuses).unwrap();
        assert!(json.contains("\"submitted\""));
        assert!(json.contains("\"completed\""));
        assert!(json.contains("\"failed\""));
    }

    #[test]
    fn message_part_serialization() {
        let text_part = AgentMessage::text_part("Hello world");
        let json = serde_json::to_string(&text_part).unwrap();
        assert!(json.contains(r#""type":"Text""#));
        assert!(json.contains("Hello world"));
    }

    #[test]
    fn file_part_builder() {
        let file = FilePart::new()
            .with_name("document.pdf")
            .with_mime_type("application/pdf")
            .with_uri("file:///documents/document.pdf");

        assert_eq!(file.name, Some("document.pdf".to_string()));
        assert_eq!(file.mime_type, Some("application/pdf".to_string()));
        assert_eq!(file.uri, Some("file:///documents/document.pdf".to_string()));
    }

    #[test]
    fn error_message_with_data() {
        let error = ErrorMessage::new(error_codes::INVALID_PARAMS, "Invalid parameters")
            .with_data(serde_json::json!({"field": "id", "reason": "required"}));

        assert_eq!(error.code, -32602);
        assert_eq!(error.message, "Invalid parameters");
        assert!(error.data.is_some());
    }
}
