use crate::config::db::{Agent, Channel, ConfigDatabase, Provider};
use axum::{
    extract::{Path, State},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub fn create_db_router(db: Arc<ConfigDatabase>) -> Router {
    Router::new()
        .route("/api/db/profiles", get(list_profiles))
        .route("/api/db/profiles", post(create_profile))
        .route("/api/db/profiles/:id", get(get_profile))
        .route("/api/db/profiles/:id", put(update_profile))
        .route("/api/db/profiles/:id", delete(delete_profile))
        .route("/api/db/profiles/:id/set-active", post(set_active_profile))
        .route("/api/db/providers", get(list_providers_all))
        .route("/api/db/providers", post(create_provider))
        .route("/api/db/providers/:id", get(get_provider))
        .route("/api/db/providers/:id", put(update_provider))
        .route("/api/db/providers/:id", delete(delete_provider))
        .route("/api/db/channels", get(list_channels_all))
        .route("/api/db/channels", post(create_channel))
        .route("/api/db/channels/:id", get(get_channel))
        .route("/api/db/channels/:id", put(update_channel))
        .route("/api/db/channels/:id", delete(delete_channel))
        .route("/api/db/agents", get(list_agents_all))
        .route("/api/db/agents", post(create_agent))
        .route("/api/db/agents/:id", get(get_agent))
        .route("/api/db/agents/:id", put(update_agent))
        .route("/api/db/agents/:id", delete(delete_agent))
        .with_state(db)
}

// ==================== Profiles ====================

async fn list_profiles(State(db): State<Arc<ConfigDatabase>>) -> Json<Vec<ProfileResponse>> {
    let profiles = db.get_profiles().unwrap_or_default();
    Json(profiles.into_iter().map(ProfileResponse::from).collect())
}

async fn get_profile(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<ProfileResponse> {
    match db.get_profile(&id).ok().flatten() {
        Some(p) => Json(ProfileResponse::from(p)),
        None => Json(ProfileResponse {
            id: String::new(),
            name: String::new(),
            description: None,
            is_active: false,
            created_at: String::new(),
            updated_at: String::new(),
        }),
    }
}

async fn create_profile(
    State(db): State<Arc<ConfigDatabase>>,
    Json(input): Json<ProfileInput>,
) -> Json<ProfileResponse> {
    let profile = crate::config::db::Profile {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        description: input.description,
        is_active: input.is_active.unwrap_or(false),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.create_profile(&profile);
    Json(ProfileResponse::from(profile))
}

async fn update_profile(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
    Json(input): Json<ProfileInput>,
) -> Json<MessageResponse> {
    let profile = crate::config::db::Profile {
        id: id.clone(),
        name: input.name,
        description: input.description,
        is_active: input.is_active.unwrap_or(false),
        created_at: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let result = db.update_profile(&profile);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Profile updated".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

async fn delete_profile(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<MessageResponse> {
    let result = db.delete_profile(&id);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Profile deleted".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

async fn set_active_profile(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<MessageResponse> {
    let result = db.set_active_profile(&id);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Profile activated".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

// ==================== Providers ====================

async fn list_providers_all(State(db): State<Arc<ConfigDatabase>>) -> Json<Vec<Provider>> {
    match db.get_default_provider("default").ok().flatten() {
        Some(_) => Json(db.get_providers("default").unwrap_or_default()),
        None => Json(vec![]),
    }
}

async fn get_provider(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<Provider> {
    let provider = db.get_provider(&id).ok().flatten();
    Json(provider.unwrap_or(Provider {
        id: String::new(),
        profile_id: String::new(),
        name: String::new(),
        api_key: None,
        api_url: None,
        default_model: None,
        temperature: None,
        is_enabled: true,
        is_default: false,
        priority: 0,
        metadata: None,
        created_at: String::new(),
        updated_at: String::new(),
    }))
}

async fn create_provider(
    State(db): State<Arc<ConfigDatabase>>,
    Json(input): Json<ProviderInput>,
) -> Json<Provider> {
    let provider = Provider {
        id: uuid::Uuid::new_v4().to_string(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        name: input.name,
        api_key: input.api_key,
        api_url: input.api_url,
        default_model: input.default_model,
        temperature: input.temperature,
        is_enabled: input.is_enabled.unwrap_or(true),
        is_default: input.is_default.unwrap_or(false),
        priority: input.priority.unwrap_or(0),
        metadata: input.metadata,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.create_provider(&provider);
    Json(provider)
}

async fn update_provider(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
    Json(input): Json<ProviderInput>,
) -> Json<MessageResponse> {
    let provider = Provider {
        id: id.clone(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        name: input.name,
        api_key: input.api_key,
        api_url: input.api_url,
        default_model: input.default_model,
        temperature: input.temperature,
        is_enabled: input.is_enabled.unwrap_or(true),
        is_default: input.is_default.unwrap_or(false),
        priority: input.priority.unwrap_or(0),
        metadata: input.metadata,
        created_at: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let result = db.update_provider(&provider);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Provider updated".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

async fn delete_provider(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<MessageResponse> {
    let result = db.delete_provider(&id);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Provider deleted".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

// ==================== Channels ====================

async fn list_channels_all(State(db): State<Arc<ConfigDatabase>>) -> Json<Vec<Channel>> {
    Json(db.get_channels("default").unwrap_or_default())
}

async fn get_channel(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<Channel> {
    let channel = db.get_channel(&id).ok().flatten();
    Json(channel.unwrap_or(Channel {
        id: String::new(),
        profile_id: String::new(),
        channel_type: String::new(),
        config: String::new(),
        is_enabled: true,
        created_at: String::new(),
        updated_at: String::new(),
    }))
}

async fn create_channel(
    State(db): State<Arc<ConfigDatabase>>,
    Json(input): Json<ChannelInput>,
) -> Json<Channel> {
    let channel = Channel {
        id: uuid::Uuid::new_v4().to_string(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        channel_type: input.channel_type,
        config: input.config,
        is_enabled: input.is_enabled.unwrap_or(true),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.create_channel(&channel);
    Json(channel)
}

async fn update_channel(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
    Json(input): Json<ChannelInput>,
) -> Json<MessageResponse> {
    let channel = Channel {
        id: id.clone(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        channel_type: input.channel_type,
        config: input.config,
        is_enabled: input.is_enabled.unwrap_or(true),
        created_at: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let result = db.update_channel(&channel);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Channel updated".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

async fn delete_channel(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<MessageResponse> {
    let result = db.delete_channel(&id);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Channel deleted".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

// ==================== Agents ====================

async fn list_agents_all(State(db): State<Arc<ConfigDatabase>>) -> Json<Vec<Agent>> {
    Json(db.get_agents("default").unwrap_or_default())
}

async fn get_agent(State(db): State<Arc<ConfigDatabase>>, Path(id): Path<String>) -> Json<Agent> {
    let agent = db.get_agent(&id).ok().flatten();
    Json(agent.unwrap_or(Agent {
        id: String::new(),
        profile_id: String::new(),
        name: String::new(),
        provider: String::new(),
        model: None,
        api_key: None,
        api_url: None,
        system_prompt: None,
        temperature: None,
        max_depth: None,
        agentic: false,
        allowed_tools: None,
        max_iterations: None,
        metadata: None,
        created_at: String::new(),
        updated_at: String::new(),
    }))
}

async fn create_agent(
    State(db): State<Arc<ConfigDatabase>>,
    Json(input): Json<AgentInput>,
) -> Json<Agent> {
    let agent = Agent {
        id: uuid::Uuid::new_v4().to_string(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        name: input.name,
        provider: input.provider,
        model: input.model,
        api_key: input.api_key,
        api_url: input.api_url,
        system_prompt: input.system_prompt,
        temperature: input.temperature,
        max_depth: input.max_depth,
        agentic: input.agentic.unwrap_or(false),
        allowed_tools: input.allowed_tools,
        max_iterations: input.max_iterations,
        metadata: input.metadata,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.create_agent(&agent);
    Json(agent)
}

async fn update_agent(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
    Json(input): Json<AgentInput>,
) -> Json<MessageResponse> {
    let agent = Agent {
        id: id.clone(),
        profile_id: input.profile_id.unwrap_or_else(|| "default".to_string()),
        name: input.name,
        provider: input.provider,
        model: input.model,
        api_key: input.api_key,
        api_url: input.api_url,
        system_prompt: input.system_prompt,
        temperature: input.temperature,
        max_depth: input.max_depth,
        agentic: input.agentic.unwrap_or(false),
        allowed_tools: input.allowed_tools,
        max_iterations: input.max_iterations,
        metadata: input.metadata,
        created_at: String::new(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let result = db.update_agent(&agent);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Agent updated".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

async fn delete_agent(
    State(db): State<Arc<ConfigDatabase>>,
    Path(id): Path<String>,
) -> Json<MessageResponse> {
    let result = db.delete_agent(&id);
    Json(MessageResponse {
        success: result.is_ok(),
        message: if result.is_ok() {
            "Agent deleted".to_string()
        } else {
            result.err().unwrap().to_string()
        },
    })
}

// ==================== Types ====================

#[derive(Serialize)]
struct ProfileResponse {
    id: String,
    name: String,
    description: Option<String>,
    is_active: bool,
    created_at: String,
    updated_at: String,
}

impl From<crate::config::db::Profile> for ProfileResponse {
    fn from(p: crate::config::db::Profile) -> Self {
        Self {
            id: p.id,
            name: p.name,
            description: p.description,
            is_active: p.is_active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Serialize)]
struct MessageResponse {
    success: bool,
    message: String,
}

#[derive(Deserialize)]
struct ProfileInput {
    name: String,
    description: Option<String>,
    is_active: Option<bool>,
}

#[derive(Deserialize)]
struct ProviderInput {
    #[serde(default)]
    profile_id: Option<String>,
    name: String,
    api_key: Option<String>,
    api_url: Option<String>,
    default_model: Option<String>,
    temperature: Option<f64>,
    is_enabled: Option<bool>,
    is_default: Option<bool>,
    priority: Option<i32>,
    metadata: Option<String>,
}

#[derive(Deserialize)]
struct ChannelInput {
    #[serde(default)]
    profile_id: Option<String>,
    channel_type: String,
    config: String,
    is_enabled: Option<bool>,
}

#[derive(Deserialize)]
struct AgentInput {
    #[serde(default)]
    profile_id: Option<String>,
    name: String,
    provider: String,
    model: Option<String>,
    api_key: Option<String>,
    api_url: Option<String>,
    system_prompt: Option<String>,
    temperature: Option<f64>,
    max_depth: Option<i32>,
    agentic: Option<bool>,
    allowed_tools: Option<String>,
    max_iterations: Option<i32>,
    metadata: Option<String>,
}
