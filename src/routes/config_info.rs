use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json, TypedHeader,
};
use chrono::Duration;
use serde::{Serialize, Serializer};

use crate::{custom_headers::accept::Accept, service_config::ServiceConfig, ServiceState};

#[derive(Debug, Clone)]
pub enum ConfigInfoResponse {
    Json(Json<ConfigInfo>),
    String(String),
}

impl IntoResponse for ConfigInfoResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Json(json) => json.into_response(),
            Self::String(string) => string.into_response(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInfo {
    pub service_version: &'static str,
    pub max_strikes: u16,
    pub log_level: log::Level,
    pub ip_recording: Option<IpRecordingConfigInfo>,
    pub tokens: Option<TokenConfigInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IpRecordingConfigInfo {
    #[serde(serialize_with = "duration_to_str_serialize")]
    pub retention_period: Duration,
    pub retention_check_period: Arc<str>,
}

fn duration_to_str_serialize<S: Serializer>(dur: &Duration, ser: S) -> Result<S::Ok, S::Error> {
    dur.to_string().serialize(ser)
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenConfigInfo {
    link_creation_requires_auth: bool,
}

pub async fn config_info_route(
    State(ServiceState { db: _, config }): State<ServiceState>,
    accept: Option<TypedHeader<Accept>>,
) -> Result<ConfigInfoResponse, (StatusCode, String)> {
    let accept = accept.unwrap_or(TypedHeader(Accept(mime::TEXT_PLAIN)));

    if accept.0 .0 == mime::APPLICATION_JSON {
        Ok(ConfigInfoResponse::Json(config_info_json_handler(config)))
    } else if accept.0 .0 == mime::TEXT_PLAIN {
        Ok(ConfigInfoResponse::String(config_info_text_handler(config)))
    } else {
        Err((
            StatusCode::BAD_REQUEST,
            "Unsupported format requested".to_string(),
        ))
    }
}

pub fn config_info_json_handler(config: ServiceConfig) -> Json<ConfigInfo> {
    Json(ConfigInfo {
        service_version: env!("CARGO_PKG_VERSION"),
        max_strikes: config.max_strikes,
        log_level: log::max_level()
            .to_level()
            .expect("Logging shouldn't be turned off"),
        ip_recording: config.ip_recording.map(|iprc| IpRecordingConfigInfo {
            retention_period: iprc.retention_period,
            retention_check_period: iprc.retention_check_period,
        }),
        tokens: config.token_config.map(|tkc| TokenConfigInfo {
            link_creation_requires_auth: tkc.creation_requires_auth,
        }),
    })
}

pub fn config_info_text_handler(config: ServiceConfig) -> String {
    let ServiceConfig {
        max_strikes,
        ip_recording,
        token_config: tokens,
        log_level: _,
    } = config;
    let ip_recording_status = if ip_recording.is_some() {
        "Enabled"
    } else {
        "Disabled"
    };
    let tokens_status = if tokens.is_some() {
        "Enabled"
    } else {
        "Disabled"
    };
    let creation_requires_auth = if let Some(toks) = tokens {
        toks.creation_requires_auth
    } else {
        false
    };
    format!(
        r#"
        Sayless v{} configuration info

        IP recording: {};
        Max amount of strikes: {};
        Token authentication: {};
        Link creation requires authentication: {};

        Log level: {}
        "#,
        env!("CARGO_PKG_VERSION"),
        ip_recording_status,
        max_strikes,
        tokens_status,
        creation_requires_auth,
        log::max_level(),
    )
}
