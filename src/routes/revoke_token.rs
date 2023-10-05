use axum::{debug_handler, extract::State, http::StatusCode, TypedHeader};
use headers::{authorization::Bearer, Authorization};

use crate::{
    tokens::{check_permission, TokenPermissions},
    ServiceState,
};

#[debug_handler]
pub async fn revoke_token_route(
    State(ServiceState { db, config }): State<ServiceState>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    token: String,
) -> Result<(), StatusCode> {
    if check_permission(
        db.as_ref(),
        &config.tokens.as_ref().unwrap().master_token,
        auth_header.token(),
        TokenPermissions::new().admin(),
    )
    .await?
    {
        sqlx::query!(
            "UPDATE tokens SET expires_at = CURRENT_TIMESTAMP WHERE token = ?",
            token
        )
        .execute(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Error setting expiration date: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
