use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default)]
pub struct TokenPermissions {
    #[serde(default)]
    pub admin_perm: bool,
    #[serde(default)]
    pub create_link_perm: bool,
    #[serde(default)]
    pub create_token_perm: bool,
    #[serde(default)]
    pub view_ips_perm: bool,
}

#[allow(dead_code)]
impl TokenPermissions {
    pub const fn new() -> Self {
        Self {
            admin_perm: false,
            create_link_perm: false,
            create_token_perm: false,
            view_ips_perm: false,
        }
    }

    pub fn admin(mut self) -> Self {
        self.admin_perm = true;
        self
    }

    pub fn create_link(mut self) -> Self {
        self.create_link_perm = true;
        self
    }

    pub fn create_token(mut self) -> Self {
        self.create_token_perm = true;
        self
    }

    pub fn view_ips(mut self) -> Self {
        self.view_ips_perm = true;
        self
    }
}

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
    master_token: Option<&str>,
    token: &str,
    TokenPermissions {
        admin_perm,
        create_link_perm,
        create_token_perm,
        view_ips_perm,
    }: TokenPermissions,
) -> Result<bool, StatusCode> {
    if let Some(master_token) = master_token {
        if token == master_token {
            return Ok(true);
        }
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
        AND (admin_perm = 1 OR admin_perm = ?)
        AND (create_link_perm = 1 OR create_link_perm = ?)
        AND (create_token_perm = 1 OR create_token_perm = ?)
        AND (view_ips_perm = 1 OR view_ips_perm = ?);
        "#,
        token,
        admin_perm,
        create_link_perm,
        create_token_perm,
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
