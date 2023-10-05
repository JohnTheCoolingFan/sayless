use crate::{
    base58::Base58Chars,
    json_schemas::create_token_params::CreateTokenParams,
    responses::token_created::TokenCreated,
    tokens::{check_permission, TokenPermissions},
    ServiceState,
};
use axum::{extract::State, http::StatusCode, Json, TypedHeader};
use headers::{authorization::Bearer, Authorization};
use rand::prelude::*;

pub async fn create_token_route(
    State(ServiceState { db, config }): State<ServiceState>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    Json(params): Json<CreateTokenParams>,
) -> Result<TokenCreated, StatusCode> {
    let auth_token_str = auth_header.token();
    if !check_permission(
        db.as_ref(),
        &config.tokens.unwrap().master_token,
        auth_token_str,
        TokenPermissions::new().create_token(),
    )
    .await?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let rng = StdRng::from_entropy();
    let new_token: String = rng.sample_iter(Base58Chars).take(44).collect();

    match params.expires_at {
        None => {
            sqlx::query!("INSERT INTO tokens (token, admin_perm, create_link_perm, create_token_perm, view_ips_perm) values (?, ?, ?, ?, ?)", &new_token, params.admin_perm, params.create_link_perm, params.create_token_perm, params.view_ips_perm)
        .execute(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Failed to insert new token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        }
        Some(expiration_date) => {
            sqlx::query!("INSERT INTO tokens (token, expires_at, admin_perm, create_link_perm, create_token_perm, view_ips_perm) values (?, ?, ?, ?, ?, ?)", &new_token, expiration_date, params.admin_perm, params.create_link_perm, params.create_token_perm, params.view_ips_perm)
                .execute(db.as_ref())
                .await
                .map_err(|e| {
                    log::error!("Failed to insert new token: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    Ok(TokenCreated { token: new_token })
}
