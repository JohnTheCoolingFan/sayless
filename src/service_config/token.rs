use std::sync::Arc;

use serde::Deserialize;

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

impl std::fmt::Debug for TokenConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenConfig")
            .field("creation_requires_auth", &self.creation_requires_auth)
            .finish_non_exhaustive()
    }
}
