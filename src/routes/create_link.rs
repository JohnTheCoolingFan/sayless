use crate::{base58::Base58Chars, responses::created_link::CreatedLink, ServiceState};
use axum::{
    extract::{ConnectInfo, State},
    http::{StatusCode, Uri},
    TypedHeader,
};
use chrono::{DateTime, Utc};
use headers::{authorization::Bearer, Authorization};
use rand::prelude::*;
use std::{net::SocketAddr, str::FromStr};

#[derive(Debug)]
struct CreateLinkPermQuery {
    admin_perm: bool,
    create_link_perm: bool,
    expires_at: DateTime<Utc>,
}

pub async fn create_link_route(
    State(ServiceState { db, config }): State<ServiceState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    url: String,
) -> Result<CreatedLink, StatusCode> {
    if let Some(tok_config) = &config.tokens {
        if tok_config.creation_requires_auth {
            match auth_header {
                Some(auth) => {
                    if auth.token() != tok_config.master_token.as_ref() {
                        let CreateLinkPermQuery { admin_perm, create_link_perm, expires_at } = 
                            sqlx::query_as!(
                                CreateLinkPermQuery,
                                r#"SELECT admin_perm as `admin_perm: bool`, create_link_perm as `create_link_perm: bool`, expires_at FROM tokens WHERE token = ?"#,
                                auth.token()
                                )
                            .fetch_one(db.as_ref())
                            .await
                            .map_err(|e| {
                                match e {
                                    sqlx::Error::RowNotFound => {
                                        log::warn!("Attempt to use invalid credentials from {}: `{}`", addr.ip(), auth.token());
                                        StatusCode::UNAUTHORIZED},
                                    _ => {
                                        log::error!("Error fetching token for permission check: {e}");
                                        StatusCode::INTERNAL_SERVER_ERROR},
                                }
                            }
                        )?;

                        if Utc::now() > expires_at {
                            log::warn!(
                                "Attempt to use expired token `{}` from {}: expired at {}",
                                auth.token(),
                                addr.ip(),
                                expires_at
                            );
                            return Err(StatusCode::UNAUTHORIZED);
                        }

                        if !(admin_perm || create_link_perm) {
                            return Err(StatusCode::FORBIDDEN);
                        }
                    }
                }
                None => return Err(StatusCode::UNAUTHORIZED),
            }
        }
    }

    if config.ip_recording.is_some() {
        let created_by =
            bincode::serialize(&addr.ip()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some((strikes,)) =
            sqlx::query_as::<_, (u16,)>("SELECT amount FROM strikes WHERE origin = ?")
                .bind(created_by)
                .fetch_optional(db.as_ref())
                .await
                .map_err(|e| {
                    log::error!("Error looking up strikes for {}: {}", addr.ip(), e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?
        {
            if strikes >= config.max_strikes {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let uri = Uri::from_str(&url).map_err(|_| StatusCode::BAD_REQUEST)?;
    let uri_hash = blake3::hash(uri.to_string().as_ref());
    let uri_hash_bytes: [u8; 32] = uri_hash.into();

    if let Some((id,)) = sqlx::query_as("SELECT id FROM links WHERE hash = ?")
        .bind(uri_hash_bytes.as_ref())
        .fetch_optional(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Error when looking for existing link: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    {
        Ok(CreatedLink { id })
    } else {
        let created_by =
            bincode::serialize(&addr.ip()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let rng = StdRng::from_entropy();
        let new_link_id: String = rng.sample_iter(Base58Chars).take(7).collect();

        sqlx::query("INSERT INTO links (id, hash, link) values (?, ?, ?)")
            .bind(&new_link_id)
            .bind(uri_hash_bytes.as_ref())
            .bind(uri.to_string())
            .execute(db.as_ref())
            .await
            .map_err(|e| {
                log::error!("Error when inserting new link: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if config.ip_recording.is_some() {
            sqlx::query("INSERT INTO origins (id, created_by) values (?, ?)")
                .bind(&new_link_id)
                .bind(created_by)
                .execute(db.as_ref())
                .await
                .map_err(|e| {
                    log::error!("Error when inserting link origin: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }

        Ok(CreatedLink { id: new_link_id })
    }
}
