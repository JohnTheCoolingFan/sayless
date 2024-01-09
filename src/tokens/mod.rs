use axum::http::StatusCode;
use sqlx::{MySql, Pool};

use super::json_schemas::token_permissions::TokenPermissions;

#[derive(Debug)]
struct TokenExistenceCheck {
    tok_exists: bool,
}

#[derive(Debug)]
struct Token {
    #[allow(dead_code)]
    token: String,
}

pub async fn check_permission(
    db: &Pool<MySql>,
    master_token: &str,
    token: &str,
    TokenPermissions {
        admin_perm: _,
        create_link_perm,
        view_ips_perm,
    }: TokenPermissions,
) -> Result<bool, StatusCode> {
    if token == master_token {
        return Ok(true);
    }
    let tok_exists = sqlx::query_as!(
        TokenExistenceCheck,
        r#"
        SELECT CASE WHEN EXISTS (
            SELECT *
            FROM tokens
            WHERE token = ? AND expires_at > CURRENT_TIMESTAMP
        )
        THEN TRUE
        ELSE FALSE
        END AS `tok_exists: _`
        "#,
        token,
    )
    .fetch_one(db)
    .await
    .map_err(|e| {
        log::error!("Error checking if token `{token}` exists: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .tok_exists;

    if !tok_exists {
        return Err(StatusCode::UNAUTHORIZED);
    }

    match sqlx::query_as!(
        Token,
        r#"SELECT
            token
        FROM tokens
        WHERE token = ?
        OR admin_perm = 1
        AND (create_link_perm = 1 OR create_link_perm = ?)
        AND (view_ips_perm = 1 OR view_ips_perm = ?);
        "#,
        token,
        create_link_perm,
        view_ips_perm
    )
    .fetch_optional(db)
    .await
    .map_err(|e| {
        log::error!("Error fetching permissions for token `{}`: {}", token, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        None => Ok(false),
        Some(_) => Ok(true),
    }
}
