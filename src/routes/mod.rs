use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    routes::{
        config_info::config_info_route, create_link::create_link_route,
        create_token::create_token_route, get_link::get_link_route,
        get_link_info::get_link_info_route, revoke_token::revoke_token_route,
    },
    service_config::ServiceConfig,
    ServiceState,
};

pub mod config_info;
pub mod create_link;
pub mod create_token;
pub mod get_link;
pub mod get_link_info;
pub mod revoke_token;

pub fn create_router(config: &ServiceConfig) -> Router<ServiceState> {
    log::info!("Building router");
    let mut router = Router::new()
        .route("/l/create", post(create_link_route))
        .route("/l/:id", get(get_link_route))
        .route("/l/:id/info", get(get_link_info_route))
        .route("/l/config_info", get(config_info_route));

    if config.token_config.is_some() {
        router = router
            .route("/l/tokens/create", post(create_token_route))
            .route("/l/tokens/revoke", post(revoke_token_route));
    }

    router
}
