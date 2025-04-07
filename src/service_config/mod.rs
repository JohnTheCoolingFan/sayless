use std::{error::Error, path::PathBuf, sync::Arc};

use serde::Deserialize;

use self::{ip_recording::IpRecordingConfig, token::TokenConfig};

pub mod ip_recording;
pub mod token;

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    #[serde(default = "default_max_strikes")]
    pub max_strikes: u16,
    #[serde(default)]
    pub ip_recording: Option<IpRecordingConfig>,
    #[serde(default)]
    pub token_config: Option<TokenConfig>,
    #[serde(default)]
    pub log_level: Option<log::Level>,
}

const fn default_max_strikes() -> u16 {
    30
}

pub async fn get_config() -> Result<ServiceConfig, Box<dyn Error + Send + Sync>> {
    let config_path: PathBuf = dotenvy::var("CONFIG_FILE")
        .ok()
        .unwrap_or_else(|| "config.toml".into())
        .parse()?;
    log::info!("Loading config from {}", config_path.to_str().unwrap());
    let config_str = tokio::fs::read_to_string(config_path.as_path()).await?;
    let mut config: ServiceConfig = toml::from_str(&config_str)?;
    if let Some(tok_config) = &mut config.token_config {
        tok_config.master_token = Arc::from(
            dotenvy::var("MASTER_TOKEN")
                .expect("Master token is required if token system is enabled")
                .into_boxed_str(),
        );
    }
    Ok(config)
}
