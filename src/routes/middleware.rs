use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::TypedHeader;
use futures_util::future::BoxFuture;
use headers::{Authorization, authorization::Bearer};

use crate::{
    ServiceState, json_schemas::token_permissions::TokenPermissions, tokens::check_permission,
};

#[allow(clippy::type_complexity)]
pub fn create_permission_check_layer(
    master_token: Arc<str>,
    permissions: TokenPermissions,
) -> impl FnMut(
    State<ServiceState>,
    Option<TypedHeader<Authorization<Bearer>>>,
    Request,
    Next,
) -> BoxFuture<'static, Result<Response, StatusCode>>
+ Clone {
    move |State(ServiceState { db, config: _ }), auth_header, req, next| {
        let db = db.clone();
        let master_token = master_token.clone();
        Box::pin(async move {
            if let Some(auth_header) = auth_header {
                if !check_permission(&db, &master_token, auth_header.token(), permissions).await? {
                    Err(StatusCode::FORBIDDEN)
                } else {
                    Ok(next.run(req).await)
                }
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        })
    }
}
