use crate::{
    base58::Base58Chars, json_schemas::create_token_params::CreateTokenParams,
    responses::token_created::TokenCreated, ServiceState,
};
use axum::{extract::State, http::StatusCode, Json, TypedHeader};
use chrono::{DateTime, Utc};
use headers::{authorization::Bearer, Authorization};
use rand::prelude::*;

#[derive(Debug)]
struct CreateTokenPermQuery {
    admin_perm: bool,
    create_token_perm: bool,
    expires_at: DateTime<Utc>,
}

pub async fn create_token_route(
    State(ServiceState { db, config }): State<ServiceState>,
    auth_header: TypedHeader<Authorization<Bearer>>,
    Json(params): Json<CreateTokenParams>,
) -> Result<TokenCreated, StatusCode> {
    let auth_token_str = auth_header.token();
    if auth_token_str != config.tokens.as_ref().unwrap().master_token.as_ref() {
        let CreateTokenPermQuery {
            admin_perm,
            create_token_perm,
            expires_at,
        } = sqlx::query_as!(
            CreateTokenPermQuery,
            r#"SELECT admin_perm as `admin_perm: bool`, create_token_perm as `create_token_perm: bool`, expires_at FROM tokens WHERE token = ?"#,
            auth_token_str
        )
        .fetch_one(db.as_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::FORBIDDEN,
            _ => {
                log::error!("Failed to fetch token: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

        if expires_at < Utc::now() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        if !create_token_perm && !admin_perm {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let rng = StdRng::from_entropy();
    let new_token: String = rng.sample_iter(Base58Chars).take(44).collect();

    sqlx::query("INSERT INTO tokens (token, admin_perm, create_link_perm, create_token_perm, view_ips_perm) values (?, ?, ?, ?, ?)")
        .bind(&new_token)
        .bind(params.admin_perm)
        .bind(params.create_link_perm)
        .bind(params.create_token_perm)
        .bind(params.view_ips_perm)
        .execute(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Failed to insert new token: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(TokenCreated { token: new_token })
}
