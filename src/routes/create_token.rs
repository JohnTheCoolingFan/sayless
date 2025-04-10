use axum::{extract::State, http::StatusCode, Json};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use rand::prelude::*;

use crate::{
    base58::Base58Chars,
    json_schemas::{create_token_params::CreateTokenParams, token_permissions::TokenPermissions},
    responses::token_created::TokenCreated,
    tokens::check_permission,
    ServiceState,
};

pub async fn create_token_route(
    State(ServiceState { db, config }): State<ServiceState>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    Json(params): Json<CreateTokenParams>,
) -> Result<TokenCreated, StatusCode> {
    let auth_token_str = auth_header.token();
    if !check_permission(
        db.as_ref(),
        &config.token_config.unwrap().master_token,
        auth_token_str,
        TokenPermissions::new().admin(),
    )
    .await?
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let rng = StdRng::from_os_rng();
    let new_token: String = rng.sample_iter(Base58Chars).take(44).collect();

    match params.expires_at {
        None => {
            sqlx::query!(
                r#"
                INSERT INTO tokens (
                    token,
                    admin_perm,
                    create_link_perm,
                    view_ips_perm
                ) values (?, ?, ?, ?)
                "#,
                &new_token,
                params.perms.admin_perm,
                params.perms.create_link_perm,
                params.perms.view_ips_perm
            )
            .execute(db.as_ref())
            .await
            .map_err(|e| {
                log::error!("Failed to insert new token: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
        Some(expiration_date) => {
            sqlx::query!(
                r#"
                INSERT INTO tokens (
                    token,
                    expires_at,
                    admin_perm,
                    create_link_perm,
                    view_ips_perm
                ) values (?, ?, ?, ?, ?)
                "#,
                &new_token,
                expiration_date,
                params.perms.admin_perm,
                params.perms.create_link_perm,
                params.perms.view_ips_perm
            )
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
