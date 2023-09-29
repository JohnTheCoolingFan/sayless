use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Clone)]
#[serde(rename = "snake_case")]
pub struct TokenConfig {
    #[serde(default)]
    pub creation_requires_auth: bool,
    #[serde(skip_deserializing, default = "default_arc_str")]
    pub master_token: Arc<str>,
}

fn default_arc_str() -> Arc<str> {
    "".into()
}
