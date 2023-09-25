use axum::{
    body::HttpBody,
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, HeaderName, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use blake3::Hash;
use chrono::{DateTime, Utc};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

#[derive(Deserialize, Clone)]
struct ServiceConfig {
    port: u16,
    #[serde(default = "default_max_strikes")]
    max_strikes: u16,
    #[serde(default)]
    record_ips: bool,
    #[serde(default)]
    tokens: Option<TokenConfig>,
    #[serde(default = "default_log_level")]
    log_level: log::Level,
    #[serde(skip_deserializing)]
    master_token: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename = "snake_case")]
struct TokenConfig {
    #[serde(default)]
    creation_requires_auth: bool,
}

const fn default_max_strikes() -> u16 {
    30
}

const fn default_log_level() -> log::Level {
    log::Level::Info
}

async fn get_config() -> Result<ServiceConfig, Box<dyn Error + Send + Sync>> {
    let config_str = tokio::fs::read_to_string("config.toml").await?;
    let config = toml::from_str(&config_str)?;
    Ok(config)
}

type DbPool = Arc<Pool<MySql>>;

#[derive(Clone)]
struct ServiceState {
    db: DbPool,
    config: ServiceConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut config = get_config().await.expect("Reading config failed");

    simple_logger::init_with_level(config.log_level)?;

    dotenvy::dotenv()?;

    log::info!("connecting to db");
    let db = MySqlPoolOptions::new()
        .connect(&dotenvy::var("DATABASE_URL")?)
        .await?;

    log::info!("Creating tables");

    log::debug!("Creating `links` table");
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS links (
            id TEXT NOT NULL,
            hash BLOB NOT NULL,
            link TEXT NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#,
    )
    .execute(&db)
    .await?;

    if config.record_ips {
        log::debug!("Creating `origins` table");
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS
            origins (
                id TEXT NOT NULL,
                created_by TINYBLOB NOT NULL
            )"#,
        )
        .execute(&db)
        .await?;

        log::debug!("Creating `strikes` table");
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS 
            strikes (
                origin TINYBLOB NOT NULL,
                amount SMALLINT UNSIGNED NOT NULL
            )"#,
        )
        .execute(&db)
        .await?;
    }

    if config.tokens.is_some() {
        let master_token =
            dotenvy::var("MASTER_TOKEN").expect("master token needs to be set if tokens are used");
        config.master_token = Some(master_token);
        log::debug!("Creating `tokens` table");
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS
            tokens (
                token TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
                expires_at TIMESTAMP DEFAULT (CURRENT_TIMESTAMP + INTERVAL 1 YEAR) NOT NULL,
                admin_perm BOOL NOT NULL,
                create_link_perm BOOL NOT NULL,
                create_token_perm BOOL NOT NULL,
                view_ips_perm BOOL NOT NULL
            )"#,
        )
        .execute(&db)
        .await?;
    }

    let db = Arc::new(db);

    log::info!("Building router");
    let mut router = Router::new()
        .route("/l/create", post(create_link_route))
        .route("/l/:id", get(get_link_route))
        .route("/l/:id/info", get(get_link_info_route));

    if config.tokens.is_some() {
        router = router.route("/l/tokens/create", post(create_token_route));
    }

    let server_port = config.port;

    let state = ServiceState { db, config };

    let router = router.with_state(state);

    log::info!("Starting server");
    axum::Server::bind(&SocketAddr::from((
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        server_port,
    )))
    .serve(router.into_make_service_with_connect_info::<SocketAddr>())
    .await
    .unwrap();

    Ok(())
}

const LOCATION_HEADER: HeaderName = HeaderName::from_static("location");

const AUTH_HEADER: HeaderName = HeaderName::from_static("authorization");

#[derive(Debug, Clone)]
struct CreatedLink {
    id: String,
}

impl IntoResponse for CreatedLink {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::CREATED)
            .header(LOCATION_HEADER, format!("/l/{}", self.id))
            .body(Default::default())
            .unwrap()
    }
}

#[derive(Serialize, Deserialize)]
struct LinkInfo {
    id: String,
    #[serde(with = "serde_hash")]
    hash: Hash,
    link: String,
    created_at: DateTime<Utc>,
}

mod serde_hash {
    use blake3::Hash;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(hash: &Hash, ser: S) -> Result<S::Ok, S::Error> {
        hash.to_hex().serialize(ser)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(des: D) -> Result<Hash, D::Error> {
        Hash::from_hex(String::deserialize(des)?).map_err(serde::de::Error::custom)
    }
}

struct Base58Chars;

static BASE_58_CHARS: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

impl Distribution<char> for Base58Chars {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> char {
        let idx = rng.gen_range(0..58);
        char::from(BASE_58_CHARS[idx])
    }
}

async fn create_link_route(
    State(ServiceState { db, config }): State<ServiceState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    url: String,
) -> Result<CreatedLink, StatusCode> {
    if let Some(tok_config) = &config.tokens {
        if tok_config.creation_requires_auth {
            let auth_header = headers.get(AUTH_HEADER).ok_or(StatusCode::FORBIDDEN)?;
            let auth_header_str = auth_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
            let (admin_perm, link_creation_perm, expiry_date): (bool, bool, DateTime<Utc>) =
                sqlx::query_as(
                    "SELECT admin_perm, create_link_perm, expires_at FROM tokens WHERE token = ?",
                )
                .bind(auth_header_str)
                .fetch_one(db.as_ref())
                .await
                .map_err(|e| match e {
                    sqlx::Error::RowNotFound => StatusCode::FORBIDDEN,
                    _ => {
                        log::error!("Failed to fetch tokens for perm check: {e}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                })?;
            if !link_creation_perm && !admin_perm {
                return Err(StatusCode::FORBIDDEN);
            }
            let utc_now = Utc::now();
            if utc_now > expiry_date {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    if config.record_ips {
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

        if config.record_ips {
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

async fn get_link_route(
    State(ServiceState { db, config: _ }): State<ServiceState>,
    Path(id): Path<String>,
) -> Result<Redirect, StatusCode> {
    let link_row: (String,) = sqlx::query_as("SELECT link FROM links WHERE id = ?")
        .bind(id)
        .fetch_one(db.as_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Redirect::to(&link_row.0))
}

async fn get_link_info_route(
    State(ServiceState { db, config: _ }): State<ServiceState>,
    Path(id): Path<String>,
) -> Result<Json<LinkInfo>, StatusCode> {
    let (id, hash, link, created_at): (String, Vec<u8>, String, DateTime<Utc>) =
        sqlx::query_as("SELECT * FROM links WHERE id = ?")
            .bind(id)
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
    // TODO: data access levels, only admins should be able to see created_by ip address
    /*
    let created_by = IpAddr::V4(<Ipv4Addr as From<[u8; 4]>>::from(
        created_by.try_into().map_err(|e: Vec<u8>| {
            log::error!("Ip address contains invalid amount of bytes: {}", e.len());
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
    ));
    */
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
    }))
}

struct TokenCreated {
    token: String,
}

impl IntoResponse for TokenCreated {
    fn into_response(self) -> Response {
        Response::<()>::builder()
            .status(StatusCode::CREATED)
            .body(
                self.token
                    .boxed_unsync()
                    .map_err(|_| unreachable!())
                    .boxed_unsync(),
            )
            .unwrap()
    }
}

#[derive(Deserialize)]
struct CreateTokenParams {
    #[serde(default)]
    admin_perm: bool,
    #[serde(default)]
    create_link_perm: bool,
    #[serde(default)]
    create_token_perm: bool,
    #[serde(default)]
    view_ips_perm: bool,
}

async fn create_token_route(
    State(ServiceState { db, config }): State<ServiceState>,
    headers: HeaderMap,
    Json(params): Json<CreateTokenParams>,
) -> Result<TokenCreated, StatusCode> {
    let auth_token = headers.get(AUTH_HEADER).ok_or(StatusCode::FORBIDDEN)?;
    let auth_token_str = auth_token.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    if auth_token_str != config.master_token.unwrap() {
        let (admin_perm, tok_create_perm, expiry_date): (bool, bool, DateTime<Utc>) =
            sqlx::query_as(
                "SELECT admin_perm, create_token_perm, expires_at FROM tokens WHERE token = ?",
            )
            .bind(auth_token_str)
            .fetch_one(db.as_ref())
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => StatusCode::FORBIDDEN,
                _ => {
                    log::error!("Failed to fetch token: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            })?;
        if !tok_create_perm && !admin_perm {
            return Err(StatusCode::FORBIDDEN);
        }
        let utc_now = Utc::now();
        if expiry_date < utc_now {
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
