use crate::{
    responses::link_info::LinkInfo,
    tokens::{check_permission, TokenPermissions},
    ServiceState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, TypedHeader,
};
use blake3::Hash;
use chrono::{DateTime, Utc};
use headers::{authorization::Bearer, Authorization};

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
    State(ServiceState { db, config: _ }): State<ServiceState>,
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

    let has_ip_view_perm = match auth_header {
        None => false,
        Some(tok) => {
            check_permission(db.as_ref(), tok.token(), TokenPermissions::new().view_ips()).await?
        }
    };

    let created_by = if has_ip_view_perm {
        let created_by = sqlx::query_as!(
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
