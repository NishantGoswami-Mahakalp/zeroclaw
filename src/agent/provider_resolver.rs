use crate::config::db::ConfigDatabase;
use crate::config::schema::Config;
use anyhow::{Context, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProviderResolverConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub default_model: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AgentResolverConfig {
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f64>,
    pub max_depth: Option<i32>,
    pub agentic: bool,
    pub allowed_tools: Option<String>,
    pub max_iterations: Option<i32>,
}

pub trait ProviderResolver: Send + Sync {
    fn get_default_provider(&self) -> Result<ProviderResolverConfig>;
    fn get_provider(&self, name: &str) -> Result<Option<ProviderResolverConfig>>;
}

pub trait AgentResolver: Send + Sync {
    fn get_agent(&self, name: &str) -> Result<Option<AgentResolverConfig>>;
    fn get_all_agents(&self) -> Result<Vec<AgentResolverConfig>>;
}

pub struct ConfigTomlProvider {
    config: Config,
}

impl ConfigTomlProvider {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl ProviderResolver for ConfigTomlProvider {
    fn get_default_provider(&self) -> Result<ProviderResolverConfig> {
        let provider_name = self
            .config
            .default_provider
            .clone()
            .unwrap_or_else(|| "openrouter".to_string());

        let model_provider = self.config.model_providers.get(&provider_name).cloned();

        if let Some(mp) = model_provider {
            Ok(ProviderResolverConfig {
                name: mp.name.unwrap_or(provider_name),
                api_key: mp.api_key.or_else(|| self.config.api_key.clone()),
                api_url: mp.base_url.or_else(|| self.config.api_url.clone()),
                default_model: mp
                    .default_model
                    .or_else(|| self.config.default_model.clone()),
                temperature: Some(self.config.default_temperature),
            })
        } else {
            Ok(ProviderResolverConfig {
                name: provider_name,
                api_key: self.config.api_key.clone(),
                api_url: self.config.api_url.clone(),
                default_model: self.config.default_model.clone(),
                temperature: Some(self.config.default_temperature),
            })
        }
    }

    fn get_provider(&self, name: &str) -> Result<Option<ProviderResolverConfig>> {
        let model_provider = self.config.model_providers.get(name).cloned();

        Ok(model_provider.map(|mp| ProviderResolverConfig {
            name: mp.name.unwrap_or_else(|| name.to_string()),
            api_key: mp.api_key.or_else(|| self.config.api_key.clone()),
            api_url: mp.base_url.or_else(|| self.config.api_url.clone()),
            default_model: mp
                .default_model
                .or_else(|| self.config.default_model.clone()),
            temperature: Some(self.config.default_temperature),
        }))
    }
}

pub struct DatabaseProvider {
    db: ConfigDatabase,
    profile_id: String,
}

impl DatabaseProvider {
    pub fn new(db: ConfigDatabase, profile_id: String) -> Self {
        Self { db, profile_id }
    }
}

impl ProviderResolver for DatabaseProvider {
    fn get_default_provider(&self) -> Result<ProviderResolverConfig> {
        let provider = self
            .db
            .get_default_provider(&self.profile_id)
            .context("Failed to get default provider from database")?
            .ok_or_else(|| anyhow::anyhow!("No default provider found in database"))?;

        Ok(ProviderResolverConfig {
            name: provider.name,
            api_key: provider.api_key,
            api_url: provider.api_url,
            default_model: provider.default_model,
            temperature: provider.temperature,
        })
    }

    fn get_provider(&self, name: &str) -> Result<Option<ProviderResolverConfig>> {
        let providers = self
            .db
            .get_providers(&self.profile_id)
            .context("Failed to get providers from database")?;

        let provider = providers
            .into_iter()
            .find(|p| p.name == name && p.is_enabled);

        Ok(provider.map(|p| ProviderResolverConfig {
            name: p.name,
            api_key: p.api_key,
            api_url: p.api_url,
            default_model: p.default_model,
            temperature: p.temperature,
        }))
    }
}

pub struct ChainedProviderResolver {
    db_resolver: Option<DatabaseProvider>,
    config_resolver: ConfigTomlProvider,
}

impl ChainedProviderResolver {
    pub fn new(config: Config, db: Option<ConfigDatabase>, profile_id: Option<String>) -> Self {
        let db_resolver = db
            .zip(profile_id)
            .map(|(db, profile_id)| DatabaseProvider::new(db, profile_id));
        let config_resolver = ConfigTomlProvider::new(config);

        Self {
            db_resolver,
            config_resolver,
        }
    }
}

impl ProviderResolver for ChainedProviderResolver {
    fn get_default_provider(&self) -> Result<ProviderResolverConfig> {
        if let Some(ref db_resolver) = self.db_resolver {
            match db_resolver.get_default_provider() {
                Ok(config) => return Ok(config),
                Err(e) => {
                    tracing::warn!(
                        "Failed to get default provider from DB, falling back to config.toml: {}",
                        e
                    );
                }
            }
        }
        self.config_resolver.get_default_provider()
    }

    fn get_provider(&self, name: &str) -> Result<Option<ProviderResolverConfig>> {
        if let Some(ref db_resolver) = self.db_resolver {
            match db_resolver.get_provider(name) {
                Ok(Some(config)) => return Ok(Some(config)),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Failed to get provider {} from DB, falling back to config.toml: {}",
                        name,
                        e
                    );
                }
            }
        }
        self.config_resolver.get_provider(name)
    }
}

pub struct ConfigTomlAgent {
    config: Config,
}

impl ConfigTomlAgent {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl AgentResolver for ConfigTomlAgent {
    fn get_agent(&self, name: &str) -> Result<Option<AgentResolverConfig>> {
        let agent_config = self.config.agents.get(name).cloned();

        Ok(agent_config.map(|ac| AgentResolverConfig {
            name: name.to_string(),
            provider: ac.provider,
            model: Some(ac.model),
            api_key: ac.api_key,
            api_url: None,
            system_prompt: ac.system_prompt,
            temperature: ac.temperature,
            max_depth: Some(ac.max_depth as i32),
            agentic: ac.agentic,
            allowed_tools: Some(ac.allowed_tools.join(",")),
            max_iterations: Some(ac.max_iterations as i32),
        }))
    }

    fn get_all_agents(&self) -> Result<Vec<AgentResolverConfig>> {
        let agents: Vec<AgentResolverConfig> = self
            .config
            .agents
            .iter()
            .map(|(name, ac)| AgentResolverConfig {
                name: name.clone(),
                provider: ac.provider.clone(),
                model: Some(ac.model.clone()),
                api_key: ac.api_key.clone(),
                api_url: None,
                system_prompt: ac.system_prompt.clone(),
                temperature: ac.temperature,
                max_depth: Some(ac.max_depth as i32),
                agentic: ac.agentic,
                allowed_tools: Some(ac.allowed_tools.join(",")),
                max_iterations: Some(ac.max_iterations as i32),
            })
            .collect();

        Ok(agents)
    }
}

pub struct DatabaseAgent {
    db: ConfigDatabase,
    profile_id: String,
}

impl DatabaseAgent {
    pub fn new(db: ConfigDatabase, profile_id: String) -> Self {
        Self { db, profile_id }
    }
}

impl AgentResolver for DatabaseAgent {
    fn get_agent(&self, name: &str) -> Result<Option<AgentResolverConfig>> {
        let agents = self
            .db
            .get_agents(&self.profile_id)
            .context("Failed to get agents from database")?;

        let agent = agents.into_iter().find(|a| a.name == name);

        Ok(agent.map(|a| AgentResolverConfig {
            name: a.name,
            provider: a.provider,
            model: a.model,
            api_key: a.api_key,
            api_url: a.api_url,
            system_prompt: a.system_prompt,
            temperature: a.temperature,
            max_depth: a.max_depth,
            agentic: a.agentic,
            allowed_tools: a.allowed_tools,
            max_iterations: a.max_iterations,
        }))
    }

    fn get_all_agents(&self) -> Result<Vec<AgentResolverConfig>> {
        let agents = self
            .db
            .get_agents(&self.profile_id)
            .context("Failed to get agents from database")?;

        let agents_config: Vec<AgentResolverConfig> = agents
            .into_iter()
            .map(|a| AgentResolverConfig {
                name: a.name,
                provider: a.provider,
                model: a.model,
                api_key: a.api_key,
                api_url: a.api_url,
                system_prompt: a.system_prompt,
                temperature: a.temperature,
                max_depth: a.max_depth,
                agentic: a.agentic,
                allowed_tools: a.allowed_tools,
                max_iterations: a.max_iterations,
            })
            .collect();

        Ok(agents_config)
    }
}

pub struct ChainedAgentResolver {
    db_resolver: Option<DatabaseAgent>,
    config_resolver: ConfigTomlAgent,
}

impl ChainedAgentResolver {
    pub fn new(config: Config, db: Option<ConfigDatabase>, profile_id: Option<String>) -> Self {
        let db_resolver = db
            .zip(profile_id)
            .map(|(db, profile_id)| DatabaseAgent::new(db, profile_id));
        let config_resolver = ConfigTomlAgent::new(config);

        Self {
            db_resolver,
            config_resolver,
        }
    }
}

impl AgentResolver for ChainedAgentResolver {
    fn get_agent(&self, name: &str) -> Result<Option<AgentResolverConfig>> {
        if let Some(ref db_resolver) = self.db_resolver {
            match db_resolver.get_agent(name) {
                Ok(Some(config)) => return Ok(Some(config)),
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(
                        "Failed to get agent {} from DB, falling back to config.toml: {}",
                        name,
                        e
                    );
                }
            }
        }
        self.config_resolver.get_agent(name)
    }

    fn get_all_agents(&self) -> Result<Vec<AgentResolverConfig>> {
        let mut all_agents: HashMap<String, AgentResolverConfig> = HashMap::new();

        if let Some(ref db_resolver) = self.db_resolver {
            match db_resolver.get_all_agents() {
                Ok(agents) => {
                    for agent in agents {
                        all_agents.insert(agent.name.clone(), agent);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to get agents from DB, falling back to config.toml: {}",
                        e
                    );
                }
            }
        }

        match self.config_resolver.get_all_agents() {
            Ok(agents) => {
                for agent in agents {
                    all_agents.insert(agent.name.clone(), agent);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get agents from config.toml: {}", e);
            }
        }

        Ok(all_agents.into_values().collect())
    }
}
