use crate::agent::a2a::types::*;
use anyhow::Result;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use uuid::Uuid;

const MAX_BODY_SIZE: usize = 65_536;
const REQUEST_TIMEOUT_SECS: u64 = 30;

#[derive(Clone)]
pub struct A2AServerState {
    pub config: Arc<A2AServerConfig>,
    pub agent_card: Arc<AgentCard>,
    pub tasks: Arc<Mutex<HashMap<String, TaskContext>>>,
    pub event_tx: broadcast::Sender<A2AEvent>,
}

#[derive(Clone)]
pub struct A2AServerConfig {
    pub host: String,
    pub port: u16,
    pub api_keys: Vec<String>,
    pub oauth_clients: HashMap<String, OAuthClient>,
    pub require_authentication: bool,
}

impl Default for A2AServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 42618,
            api_keys: Vec::new(),
            oauth_clients: HashMap::new(),
            require_authentication: true,
        }
    }
}

#[derive(Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

pub struct TaskContext {
    pub id: String,
    pub status: TaskStatus,
    pub message: Option<AgentMessage>,
    pub history: Vec<StateTransition>,
    pub created_at: Instant,
    pub updated_at: Instant,
}

impl TaskContext {
    pub fn new(id: String, message: AgentMessage) -> Self {
        let now = Instant::now();
        Self {
            id: id.clone(),
            status: TaskStatus::Submitted,
            message: Some(message),
            history: vec![StateTransition::new(
                TaskStatus::Submitted,
                TaskStatus::Submitted,
                chrono::Utc::now().to_rfc3339(),
            )],
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_status(&mut self, status: TaskStatus, message: Option<AgentMessage>) {
        let now = Instant::now();
        let transition = if let Some(ref msg) = message {
            StateTransition::new(self.status, status, chrono::Utc::now().to_rfc3339())
                .with_message(msg.clone())
        } else {
            StateTransition::new(self.status, status, chrono::Utc::now().to_rfc3339())
        };
        self.history.push(transition);
        self.status = status;
        self.updated_at = now;
        if message.is_some() {
            self.message = message;
        }
    }
}

#[derive(Clone, Debug)]
pub enum A2AEvent {
    TaskStatusUpdate {
        task_id: String,
        status: TaskStatus,
    },
    TaskPushNotification {
        task_id: String,
        message: AgentMessage,
    },
}

pub struct A2AServer {
    config: A2AServerConfig,
    agent_card: AgentCard,
}

impl A2AServer {
    pub fn new(config: A2AServerConfig, agent_card: AgentCard) -> Self {
        Self { config, agent_card }
    }

    pub fn config(&self) -> &A2AServerConfig {
        &self.config
    }

    pub fn agent_card(&self) -> &AgentCard {
        &self.agent_card
    }

    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port).parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;

        let (event_tx, _event_rx) = broadcast::channel::<A2AEvent>(256);

        let state = A2AServerState {
            config: Arc::new(self.config),
            agent_card: Arc::new(self.agent_card),
            tasks: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
        };

        println!("🤖 A2A Server listening on http://{}", addr);
        println!("   GET  /.well-known/agent-card.json — Agent Card");
        println!("   POST / — A2A JSON-RPC endpoint");
        println!("   GET  /tasks/<id> — Get task status");
        println!("   WS   / — A2A WebSocket streaming");
        println!();

        let app = Router::new()
            .route("/.well-known/agent-card.json", get(handle_agent_card))
            .route("/", post(handle_a2a_rpc))
            .route("/", get(handle_ws_upgrade))
            .route("/tasks/:id", get(handle_task_get))
            .route("/tasks/:id/cancel", post(handle_task_cancel))
            .with_state(state)
            .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(REQUEST_TIMEOUT_SECS),
            ));

        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_agent_card(State(state): State<A2AServerState>) -> impl IntoResponse {
    let agent_card = state.agent_card.as_ref().clone();
    let json = serde_json::to_string(&agent_card).unwrap();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json,
    )
}

async fn handle_a2a_rpc(
    State(state): State<A2AServerState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let auth_result = authenticate(&state, &headers).await;

    let response = if let Err(e) = auth_result {
        A2AResponse::error(
            serde_json::Value::Null,
            ErrorMessage::new(error_codes::AUTHENTICATION_FAILED, e.to_string()),
        )
    } else {
        let body_str = match std::str::from_utf8(&body) {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(A2AResponse::error(
                        serde_json::Value::Null,
                        ErrorMessage::new(error_codes::PARSE_ERROR, "Invalid UTF-8"),
                    )),
                )
            }
        };

        let request: A2ARequest = match serde_json::from_str(body_str) {
            Ok(r) => r,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(A2AResponse::error(
                        serde_json::Value::Null,
                        ErrorMessage::new(error_codes::PARSE_ERROR, "Invalid JSON"),
                    )),
                )
            }
        };

        match handle_method(&state, &request).await {
            Ok(result_json) => A2AResponse::success(request.id, result_json),
            Err(error) => A2AResponse::error(request.id, error),
        }
    };

    (StatusCode::OK, Json(response))
}

async fn handle_task_get(
    State(state): State<A2AServerState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let tasks = state.tasks.lock();
    match tasks.get(&task_id) {
        Some(task) => {
            let status_msg = TaskStatusMessage::new(task.id.clone(), task.status)
                .with_history(task.history.clone());
            (
                StatusCode::OK,
                Json(A2AResponse::success(
                    serde_json::Value::String(task_id),
                    serde_json::to_value(status_msg).unwrap(),
                )),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(A2AResponse::error(
                serde_json::Value::String(task_id),
                ErrorMessage::new(error_codes::TASK_NOT_FOUND, "Task not found"),
            )),
        ),
    }
}

async fn handle_task_cancel(
    State(state): State<A2AServerState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut tasks = state.tasks.lock();
    match tasks.get_mut(&task_id) {
        Some(task) => {
            if task.status == TaskStatus::Completed
                || task.status == TaskStatus::Failed
                || task.status == TaskStatus::Canceled
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(A2AResponse::error(
                        serde_json::Value::String(task_id.clone()),
                        ErrorMessage::new(error_codes::TASK_CANCEL_FAILED, "Task already finished"),
                    )),
                );
            }
            task.update_status(TaskStatus::Canceled, None);
            let _ = state.event_tx.send(A2AEvent::TaskStatusUpdate {
                task_id: task_id.clone(),
                status: TaskStatus::Canceled,
            });
            let status_msg = TaskStatusMessage::new(task_id.clone(), task.status)
                .with_history(task.history.clone());
            (
                StatusCode::OK,
                Json(A2AResponse::success(
                    serde_json::Value::String(task_id),
                    serde_json::to_value(status_msg).unwrap(),
                )),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(A2AResponse::error(
                serde_json::Value::String(task_id),
                ErrorMessage::new(error_codes::TASK_NOT_FOUND, "Task not found"),
            )),
        ),
    }
}

async fn handle_ws_upgrade(
    State(state): State<A2AServerState>,
    headers: HeaderMap,
    ws: axum::extract::WebSocketUpgrade,
) -> impl IntoResponse {
    let auth_result = authenticate(&state, &headers).await;
    if auth_result.is_err() {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    ws.on_upgrade(|socket| handle_websocket(socket, state))
        .into_response()
}

async fn handle_websocket(socket: axum::extract::ws::WebSocket, state: A2AServerState) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(axum::extract::ws::Message::Text(text)) => text,
            Ok(axum::extract::ws::Message::Close(_)) => break,
            Err(_) => break,
            _ => continue,
        };

        let body_str = match std::str::from_utf8(msg.as_bytes()) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let request: A2ARequest = match serde_json::from_str(body_str) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let result = handle_method(&state, &request).await;

        let response = match result {
            Ok(result_json) => A2AResponse::success(request.id, result_json),
            Err(error) => A2AResponse::error(request.id, error),
        };

        let json = serde_json::to_string(&response).unwrap();
        if sender
            .send(axum::extract::ws::Message::Text(json.into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

async fn authenticate(state: &A2AServerState, headers: &HeaderMap) -> Result<()> {
    if !state.config.require_authentication {
        return Ok(());
    }

    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if let Some(api_key) = auth_header.strip_prefix("Bearer ") {
        if state.config.api_keys.contains(&api_key.to_string()) {
            return Ok(());
        }
        anyhow::bail!("Invalid API key");
    }

    if let Some(_api_key) = auth_header.strip_prefix("ApiKey ") {
        if state.config.api_keys.contains(&_api_key.to_string()) {
            return Ok(());
        }
        anyhow::bail!("Invalid API key");
    }

    anyhow::bail!("Authentication required")
}

async fn handle_method(
    state: &A2AServerState,
    request: &A2ARequest,
) -> Result<serde_json::Value, ErrorMessage> {
    match request.method.as_str() {
        "tasks/submit" => handle_task_submit(state, request),
        "tasks/get" => handle_task_get_rpc(state, request),
        "tasks/cancel" => handle_task_cancel_rpc(state, request),
        "agent/card" => handle_agent_card_rpc(state),
        _ => Err(ErrorMessage::new(
            error_codes::METHOD_NOT_FOUND,
            "Method not found",
        )),
    }
}

fn handle_task_submit(
    state: &A2AServerState,
    request: &A2ARequest,
) -> Result<serde_json::Value, ErrorMessage> {
    let params = request
        .params
        .as_ref()
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing params"))?;

    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let message: AgentMessage = params
        .get("message")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing message"))?;

    let task = TaskContext::new(task_id.clone(), message);
    let mut tasks = state.tasks.lock();
    tasks.insert(task_id.clone(), task);

    let submit_result = serde_json::json!({
        "id": task_id,
        "status": "submitted"
    });

    Ok(submit_result)
}

fn handle_task_get_rpc(
    state: &A2AServerState,
    request: &A2ARequest,
) -> Result<serde_json::Value, ErrorMessage> {
    let params = request
        .params
        .as_ref()
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing params"))?;

    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing task id"))?;

    let tasks = state.tasks.lock();
    let task = tasks
        .get(&task_id)
        .ok_or_else(|| ErrorMessage::new(error_codes::TASK_NOT_FOUND, "Task not found"))?;

    let mut status_msg =
        TaskStatusMessage::new(task.id.clone(), task.status).with_history(task.history.clone());
    if let Some(ref msg) = task.message {
        status_msg = status_msg.with_message(msg.clone());
    }

    Ok(serde_json::to_value(status_msg).unwrap())
}

fn handle_task_cancel_rpc(
    state: &A2AServerState,
    request: &A2ARequest,
) -> Result<serde_json::Value, ErrorMessage> {
    let params = request
        .params
        .as_ref()
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing params"))?;

    let task_id = params
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| ErrorMessage::new(error_codes::INVALID_PARAMS, "Missing task id"))?;

    let mut tasks = state.tasks.lock();
    let task = tasks
        .get_mut(&task_id)
        .ok_or_else(|| ErrorMessage::new(error_codes::TASK_NOT_FOUND, "Task not found"))?;

    if task.status == TaskStatus::Completed
        || task.status == TaskStatus::Failed
        || task.status == TaskStatus::Canceled
    {
        return Err(ErrorMessage::new(
            error_codes::TASK_CANCEL_FAILED,
            "Task already finished",
        ));
    }

    task.update_status(TaskStatus::Canceled, None);

    let status_msg =
        TaskStatusMessage::new(task.id.clone(), task.status).with_history(task.history.clone());

    Ok(serde_json::to_value(status_msg).unwrap())
}

fn handle_agent_card_rpc(state: &A2AServerState) -> Result<serde_json::Value, ErrorMessage> {
    Ok(serde_json::to_value(state.agent_card.as_ref().clone()).unwrap())
}
