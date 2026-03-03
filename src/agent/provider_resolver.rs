use crate::config::db::ConfigDatabase;
use crate::config::schema::Config;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ProviderResolverConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub default_model: Option<String>,
    pub temperature: Option<f64>,
}

pub trait ProviderResolver: Send + Sync {
    fn get_default_provider(&self) -> Result<ProviderResolverConfig>;
    fn get_provider(&self, name: &str) -> Result<Option<ProviderResolverConfig>>;
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
