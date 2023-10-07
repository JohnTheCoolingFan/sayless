use axum::extract::State;

use crate::{service_config::ServiceConfig, ServiceState};

pub async fn status_route(
    State(ServiceState {
        db: _,
        config:
            ServiceConfig {
                max_strikes,
                ip_recording,
                token_config: tokens,
                log_level: _,
            },
    }): State<ServiceState>,
) -> String {
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
        Sayless v{}

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
