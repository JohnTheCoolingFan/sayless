use std::{net::SocketAddr, str::FromStr};

use axum::{
    extract::{ConnectInfo, State},
    http::{StatusCode, Uri},
};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use rand::prelude::*;

use crate::{
    base58::Base58Chars, json_schemas::token_permissions::TokenPermissions,
    responses::created_link::CreatedLink, tokens::check_permission, ServiceState,
};

pub async fn create_link_route(
    State(ServiceState { db, config }): State<ServiceState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    url: String,
) -> Result<CreatedLink, StatusCode> {
    if let Some(tok_config) = &config.token_config {
        if tok_config.creation_requires_auth {
            match auth_header {
                Some(auth) => {
                    if !check_permission(
                        db.as_ref(),
                        &tok_config.master_token,
                        auth.token(),
                        TokenPermissions::new().create_link(),
                    )
                    .await?
                    {
                        return Err(StatusCode::FORBIDDEN);
                    }
                }
                None => return Err(StatusCode::UNAUTHORIZED),
            }
        }
    }

    if config.ip_recording.is_some() {
        #[derive(Debug)]
        struct Strikes {
            pub amount: u16,
        }
        let created_by =
            bincode::serialize(&addr.ip()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(Strikes { amount }) = sqlx::query_as!(
            Strikes,
            "SELECT amount FROM strikes WHERE origin = ?",
            created_by
        )
        .fetch_optional(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Error looking up strikes for {}: {}", addr.ip(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
            if amount >= config.max_strikes {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    let uri = Uri::from_str(&url).map_err(|_| StatusCode::BAD_REQUEST)?;
    let uri_hash = blake3::hash(uri.to_string().as_ref());
    let uri_hash_bytes: [u8; 32] = uri_hash.into();

    if let Some(link) = sqlx::query_as!(
        CreatedLink,
        "SELECT id FROM links WHERE hash = ?",
        uri_hash_bytes.as_ref()
    )
    .fetch_optional(db.as_ref())
    .await
    .map_err(|e| {
        log::error!("Error when looking for existing link: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        Ok(link)
    } else {
        let created_by =
            bincode::serialize(&addr.ip()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let rng = StdRng::from_os_rng();
        let new_link_id: String = rng.sample_iter(Base58Chars).take(7).collect();

        sqlx::query!(
            "INSERT INTO links (id, hash, link) values (?, ?, ?)",
            &new_link_id,
            uri_hash_bytes.as_ref(),
            uri.to_string()
        )
        .execute(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Error when inserting new link: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if config.ip_recording.is_some() {
            sqlx::query!(
                "INSERT INTO origins (id, created_by) values (?, ?)",
                &new_link_id,
                created_by
            )
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
