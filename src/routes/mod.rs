use crate::{
    routes::{
        create_link::create_link_route, create_token::create_token_route, get_link::get_link_route,
        get_link_info::get_link_info_route, status::status_route,
    },
    service_config::ServiceConfig,
    ServiceState,
};
use axum::{
    routing::{get, post},
    Router,
};

pub mod create_link;
pub mod create_token;
pub mod get_link;
pub mod get_link_info;
pub mod status;

pub fn create_router(config: &ServiceConfig) -> Router<ServiceState> {
    log::info!("Building router");
    let mut router = Router::new()
        .route("/l/create", post(create_link_route))
        .route("/l/status", get(status_route))
        .route("/l/:id", get(get_link_route))
        .route("/l/:id/info", get(get_link_info_route));

    if config.tokens.is_some() {
        router = router.route("/l/tokens/create", post(create_token_route));
    }

    router
}
