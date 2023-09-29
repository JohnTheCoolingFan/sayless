use self::{ip_recording::IpRecordingConfig, token::TokenConfig};
use serde::Deserialize;
use std::{error::Error, sync::Arc};

pub mod ip_recording;
pub mod token;

#[derive(Deserialize, Clone)]
pub struct ServiceConfig {
    pub port: u16,
    #[serde(default = "default_max_strikes")]
    pub max_strikes: u16,
    #[serde(default)]
    pub ip_recording: Option<IpRecordingConfig>,
    #[serde(default)]
    pub tokens: Option<TokenConfig>,
    #[serde(default = "default_log_level")]
    pub log_level: log::Level,
}

const fn default_max_strikes() -> u16 {
    30
}

const fn default_log_level() -> log::Level {
    log::Level::Info
}

pub async fn get_config() -> Result<ServiceConfig, Box<dyn Error + Send + Sync>> {
    let config_str = tokio::fs::read_to_string("config.toml").await?;
    let mut config: ServiceConfig = toml::from_str(&config_str)?;
    if let Some(ref mut tok_config) = &mut config.tokens {
        tok_config.master_token = Arc::from(
            dotenvy::var("MASTER_TOKEN")
                .expect("Master token is required if token system is enabled")
                .into_boxed_str(),
        );
    }
    Ok(config)
}