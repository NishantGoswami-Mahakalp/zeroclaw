use crate::channels::traits::Channel;
use crate::config::db::ConfigDatabase;
use crate::config::ChannelsConfig;
use anyhow::Result;
use std::sync::Arc;

pub trait ChannelLoader: Send + Sync {
    fn load_channels(&self) -> Result<Vec<Arc<dyn Channel>>>;
}

pub struct ConfigTomlLoader {
    config: crate::config::Config,
}

impl ConfigTomlLoader {
    pub fn new(config: crate::config::Config) -> Self {
        Self { config }
    }
}

impl ChannelLoader for ConfigTomlLoader {
    fn load_channels(&self) -> Result<Vec<Arc<dyn Channel>>> {
        let configured =
            crate::channels::collect_configured_channels(&self.config, "ConfigTomlLoader");
        Ok(configured.into_iter().map(|c| c.channel).collect())
    }
}

pub struct DatabaseLoader {
    db: ConfigDatabase,
    profile_id: String,
}

impl DatabaseLoader {
    pub fn new(db: ConfigDatabase, profile_id: String) -> Self {
        Self { db, profile_id }
    }

    pub fn with_active_profile(db: ConfigDatabase) -> Result<Self> {
        let profile = db
            .get_active_profile()?
            .ok_or_else(|| anyhow::anyhow!("No active profile found in database"))?;
        Ok(Self::new(db, profile.id))
    }
}

impl ChannelLoader for DatabaseLoader {
    fn load_channels(&self) -> Result<Vec<Arc<dyn Channel>>> {
        let db_channels = self.db.get_channels(&self.profile_id)?;
        let mut channels = Vec::new();

        for db_channel in db_channels {
            if !db_channel.is_enabled {
                continue;
            }

            let channel = self.instantiate_channel(&db_channel.channel_type, &db_channel.config)?;
            channels.push(channel);
        }

        Ok(channels)
    }
}

impl DatabaseLoader {
    fn instantiate_channel(
        &self,
        channel_type: &str,
        config_json: &str,
    ) -> Result<Arc<dyn Channel>> {
        match channel_type.to_lowercase().as_str() {
            "telegram" => self.build_telegram_channel(config_json),
            "discord" => self.build_discord_channel(config_json),
            "slack" => self.build_slack_channel(config_json),
            "mattermost" => self.build_mattermost_channel(config_json),
            "signal" => self.build_signal_channel(config_json),
            "whatsapp" => self.build_whatsapp_channel(config_json),
            "cli" => self.build_cli_channel(config_json),
            _ => Err(anyhow::anyhow!("Unknown channel type: {}", channel_type)),
        }
    }

    fn build_telegram_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct TelegramConfig {
            bot_token: String,
            allowed_users: Option<Vec<String>>,
            group_reply_mode: Option<String>,
            ack_enabled: Option<bool>,
            base_url: Option<String>,
            stream_mode: Option<bool>,
            draft_update_interval_ms: Option<u64>,
        }

        let cfg: TelegramConfig = serde_json::from_str(config_json)?;
        let mut channel = crate::channels::TelegramChannel::new(
            cfg.bot_token,
            cfg.allowed_users.unwrap_or_default(),
            cfg.group_reply_mode
                .as_ref()
                .map(|m| m == "mention")
                .unwrap_or(false),
            cfg.ack_enabled.unwrap_or(false),
        );

        if let Some(base_url) = cfg.base_url {
            channel = channel.with_api_base(base_url);
        }

        Ok(Arc::new(channel))
    }

    fn build_discord_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct DiscordConfig {
            bot_token: String,
            guild_id: Option<String>,
            allowed_users: Option<Vec<String>>,
            listen_to_bots: Option<bool>,
            group_reply_mode: Option<String>,
        }

        let cfg: DiscordConfig = serde_json::from_str(config_json)?;
        let channel = crate::channels::DiscordChannel::new(
            cfg.bot_token,
            cfg.guild_id.unwrap_or_default(),
            cfg.allowed_users.unwrap_or_default(),
            cfg.listen_to_bots.unwrap_or(false),
            cfg.group_reply_mode
                .as_ref()
                .map(|m| m == "mention")
                .unwrap_or(false),
        );

        Ok(Arc::new(channel))
    }

    fn build_slack_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct SlackConfig {
            bot_token: String,
            app_token: Option<String>,
            channel_id: String,
            channel_ids: Option<Vec<String>>,
            allowed_users: Option<Vec<String>>,
            group_reply_mode: Option<String>,
        }

        let cfg: SlackConfig = serde_json::from_str(config_json)?;
        let channel = crate::channels::SlackChannel::new(
            cfg.bot_token,
            cfg.app_token.unwrap_or_default(),
            cfg.channel_id,
            cfg.channel_ids.unwrap_or_default(),
            cfg.allowed_users.unwrap_or_default(),
        );

        Ok(Arc::new(channel))
    }

    fn build_mattermost_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct MattermostConfig {
            url: String,
            bot_token: String,
            channel_id: String,
            allowed_users: Option<Vec<String>>,
            thread_replies: Option<bool>,
            group_reply_mode: Option<String>,
        }

        let cfg: MattermostConfig = serde_json::from_str(config_json)?;
        let channel = crate::channels::MattermostChannel::new(
            cfg.url,
            cfg.bot_token,
            cfg.channel_id,
            cfg.allowed_users.unwrap_or_default(),
            cfg.thread_replies.unwrap_or(true),
            cfg.group_reply_mode
                .as_ref()
                .map(|m| m == "mention")
                .unwrap_or(false),
        );

        Ok(Arc::new(channel))
    }

    fn build_signal_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct SignalConfig {
            http_url: String,
            account: String,
            group_id: Option<String>,
            allowed_from: Option<Vec<String>>,
            ignore_attachments: Option<bool>,
            ignore_stories: Option<bool>,
        }

        let cfg: SignalConfig = serde_json::from_str(config_json)?;
        let channel = crate::channels::SignalChannel::new(
            cfg.http_url,
            cfg.account,
            cfg.group_id.unwrap_or_default(),
            cfg.allowed_from.unwrap_or_default(),
            cfg.ignore_attachments.unwrap_or(false),
            cfg.ignore_stories.unwrap_or(false),
        );

        Ok(Arc::new(channel))
    }

    fn build_whatsapp_channel(&self, config_json: &str) -> Result<Arc<dyn Channel>> {
        #[derive(serde::Deserialize)]
        struct WhatsAppConfig {
            phone_number_id: Option<String>,
            access_token: Option<String>,
            verify_token: Option<String>,
            session_path: Option<String>,
            pair_phone: Option<String>,
            pair_code: Option<String>,
            allowed_numbers: Option<Vec<String>>,
        }

        let cfg: WhatsAppConfig = serde_json::from_str(config_json)?;

        if cfg.phone_number_id.is_some() && cfg.access_token.is_some() && cfg.verify_token.is_some()
        {
            let channel = crate::channels::WhatsAppChannel::new(
                cfg.access_token.unwrap_or_default(),
                cfg.phone_number_id.unwrap_or_default(),
                cfg.verify_token.unwrap_or_default(),
                cfg.allowed_numbers.unwrap_or_default(),
            );
            Ok(Arc::new(channel))
        } else if cfg.session_path.is_some() {
            #[cfg(feature = "whatsapp-web")]
            {
                let channel = crate::channels::WhatsAppWebChannel::new(
                    cfg.session_path.unwrap_or_default(),
                    cfg.pair_phone.unwrap_or_default(),
                    cfg.pair_code.unwrap_or_default(),
                    cfg.allowed_numbers.unwrap_or_default(),
                );
                Ok(Arc::new(channel))
            }
            #[cfg(not(feature = "whatsapp-web"))]
            {
                Err(anyhow::anyhow!(
                    "WhatsApp Web requires 'whatsapp-web' feature"
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "WhatsApp config must have either phone_number_id (Cloud API) or session_path (Web)"
            ))
        }
    }

    fn build_cli_channel(&self, _config_json: &str) -> Result<Arc<dyn Channel>> {
        Ok(Arc::new(crate::channels::CliChannel::new()))
    }
}

pub struct UnifiedChannelLoader {
    db: Option<ConfigDatabase>,
    config: Option<crate::config::Config>,
}

impl UnifiedChannelLoader {
    pub fn new(db: Option<ConfigDatabase>, config: Option<crate::config::Config>) -> Self {
        Self { db, config }
    }
}

impl ChannelLoader for UnifiedChannelLoader {
    fn load_channels(&self) -> Result<Vec<Arc<dyn Channel>>> {
        if let Some(ref db) = self.db {
            let loader = DatabaseLoader::new(db.clone(), "default".to_string());
            match loader.load_channels() {
                Ok(channels) if !channels.is_empty() => return Ok(channels),
                Ok(_) => {
                    tracing::debug!("Database has no channels, falling back to config.toml");
                }
                Err(e) => {
                    tracing::debug!(
                        "Failed to load from database: {}, falling back to config.toml",
                        e
                    );
                }
            }
        }

        if let Some(ref config) = self.config {
            let loader = ConfigTomlLoader::new(config.clone());
            return loader.load_channels();
        }

        Err(anyhow::anyhow!("No channel loader available"))
    }
}
