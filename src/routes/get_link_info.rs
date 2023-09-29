use crate::{responses::link_info::LinkInfo, service_config::token::TokenConfig, ServiceState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, TypedHeader,
};
use blake3::Hash;
use chrono::{DateTime, Utc};
use headers::{authorization::Bearer, Authorization};
use sqlx::{MySql, Pool};

#[derive(Debug)]
struct ViewIpPermQuery {
    admin_perm: bool,
    view_ips_perm: bool,
    expires_at: DateTime<Utc>,
}

async fn check_ip_view_perm(
    tok_config: &TokenConfig,
    auth_header: TypedHeader<Authorization<Bearer>>,
    db: &Pool<MySql>,
) -> Result<bool, StatusCode> {
    let auth_header_str = auth_header.token();
    if tok_config.master_token.as_ref() == auth_header_str {
        return Ok(true);
    }
    let tok_response: Result<ViewIpPermQuery, sqlx::Error> = sqlx::query_as!(
        ViewIpPermQuery,
        "SELECT admin_perm as `admin_perm: bool`, view_ips_perm as `view_ips_perm: bool`, expires_at FROM tokens WHERE token = ?",
        auth_header_str
    )
    .fetch_one(db)
    .await;
    match tok_response {
        Err(sqlx::Error::RowNotFound) => Ok(false),
        Err(err) => {
            log::error!("Failed to fetch token permissions: {}", err);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Ok(ViewIpPermQuery {
            admin_perm,
            view_ips_perm,
            expires_at,
        }) => {
            if !admin_perm || !view_ips_perm {
                return Ok(false);
            }
            if Utc::now() > expires_at {
                return Ok(false);
            }
            Ok(true)
        }
    }
}

#[derive(Debug)]
struct LinkInfoQuery {
    id: String,
    hash: Vec<u8>,
    link: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug)]
struct CreatedByQuery {
    created_by: Vec<u8>,
}

pub async fn get_link_info_route(
    State(ServiceState { db, config }): State<ServiceState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    Path(id): Path<String>,
) -> Result<Json<LinkInfo>, StatusCode> {
    let LinkInfoQuery {
        id,
        hash,
        link,
        created_at,
    } = sqlx::query_as!(LinkInfoQuery, "SELECT * FROM links WHERE id = ?", id)
        .fetch_one(db.as_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                log::error!("Error looking up link: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    log::debug!("Received link info: id {id}, hash {hash:?}, link {link}, created_at {created_at}");

    let has_ip_view_perm = if let Some(auth_header) = auth_header {
        if let Some(tok_config) = &config.tokens {
            check_ip_view_perm(tok_config, auth_header, &db).await?
        } else {
            false
        }
    } else {
        false
    };

    let created_by = if has_ip_view_perm {
        let created_by: Option<CreatedByQuery> = sqlx::query_as!(
            CreatedByQuery,
            "SELECT created_by FROM origins WHERE id = ?",
            &id
        )
        .fetch_optional(db.as_ref())
        .await
        .map_err(|e| {
            log::error!("Error looking up link `{id}` origin: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if let Some(CreatedByQuery { created_by: bytes }) = created_by {
            Some(bincode::deserialize(&bytes).map_err(|e| {
                log::error!("Error deserializing origin ip: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?)
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(LinkInfo {
        id,
        hash: <Hash as From<[u8; 32]>>::from(hash.try_into().map_err(|e: Vec<u8>| {
            log::error!(
                "Error converting hash from blob: blob length is {}",
                e.len()
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?),
        link,
        created_at,
        created_by,
    }))
}
