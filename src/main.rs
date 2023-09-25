use axum::{
    debug_handler,
    extract::{Path, State},
    http::{HeaderName, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use blake3::Hash;
use chrono::{DateTime, NaiveDateTime, Utc};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlPoolOptions},
    MySql, Pool,
};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

#[derive(Deserialize)]
struct ServiceConfig {
    port: u16,
    #[serde(default)]
    record_ips: bool,
    #[serde(default)]
    log_level: Option<log::Level>,
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = get_config().await.expect("Reading config failed");

    simple_logger::init_with_level(config.log_level.unwrap_or(log::Level::Info))?;

    log::info!("connecting to db");
    //let db = SqlitePoolOptions::new().connect("sqlite::memory:").await?;
    let db = MySqlPoolOptions::new()
        .connect(&dotenvy::var("DATABASE_URL")?)
        .await?;

    log::info!("Creating tables");

    log::debug!("Creating `links` table");
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS links (id TEXT NOT NULL, hash BLOB NOT NULL, link TEXT NOT NULL, created_at TEXT NOT NULL)",
    )
    .execute(&db)
    .await?;

    if config.record_ips {
        log::debug!("Creating `origins` table");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS origins (id TEXT NOT NULL, created_by TINYBLOB NOT NULL)",
        )
        .execute(&db)
        .await?;

        log::debug!("Creating `strikes` table");
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS strikes (origin TINYBLOB NOT NULL, amount SMALLINT UNSIGNED NOT NULL)",
        )
        .execute(&db)
        .await?;
    }

    let db = Arc::new(db);

    let state = ServiceState { db };

    log::info!("Building router");
    let router = Router::new()
        .route("/l/create", post(create_link_route))
        .route("/l/:id", get(get_link_route))
        .route("/l/:id/info", get(get_link_info_route));

    let router = router.with_state(state);

    log::info!("Starting server");
    axum::Server::bind(&SocketAddr::from((
        IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        config.port,
    )))
    //.serve(router.into_make_service_with_connect_info::<SocketAddr>())
    .serve(router.into_make_service())
    .await
    .unwrap();

    Ok(())
}

const LOCATION_HEADER: HeaderName = HeaderName::from_static("location");

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
    //created_by: IpAddr,
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

#[debug_handler]
async fn create_link_route(
    State(ServiceState { db }): State<ServiceState>,
    //ConnectInfo(addr): ConnectInfo<SocketAddr>,
    url: String,
) -> Result<CreatedLink, StatusCode> {
    let uri = Uri::from_str(&url).map_err(|_| StatusCode::BAD_REQUEST)?;
    let uri_hash = blake3::hash(uri.to_string().as_ref());
    let uri_hash_bytes: [u8; 32] = uri_hash.into();
    if let Some((id,)) = sqlx::query_as("SELECT id FROM links WHERE hash = $1")
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
        /*
        let created_by = match addr.ip() {
            IpAddr::V4(ip4) => Some(ip4),
            _ => None,
        };
        */
        let rng = StdRng::from_entropy();
        let new_link_id: String = rng.sample_iter(Base58Chars).take(7).collect();
        sqlx::query("INSERT INTO links (id, hash, link, created_at, created_by) values ($1, $2, $3, DATETIME('now'), $4)")
            .bind(&new_link_id)
            .bind(uri_hash_bytes.as_ref())
            .bind(uri.to_string())
            //.bind(created_by.map(|ipv4| ipv4.octets()).unwrap_or([0u8; 4]).as_ref())
            .bind([0; 4].as_ref())
            .execute(db.as_ref())
            .await
            .map_err(|e| {
                log::error!("Error when inserting new link: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Ok(CreatedLink { id: new_link_id })
    }
}

async fn get_link_route(
    State(ServiceState { db }): State<ServiceState>,
    Path(id): Path<String>,
) -> Result<Redirect, StatusCode> {
    let link_row: (String,) = sqlx::query_as("SELECT link FROM links WHERE id = $1")
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
    State(ServiceState { db }): State<ServiceState>,
    Path(id): Path<String>,
) -> Result<Json<LinkInfo>, StatusCode> {
    let (id, hash, link, created_at, _created_by): (String, Vec<u8>, String, String, Vec<u8>) =
        sqlx::query_as("SELECT * FROM links WHERE id = $1")
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
    let created_at =
        NaiveDateTime::parse_from_str(&created_at, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            log::error!("Error parsing created_at DateTime: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
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
                "Error converting hash from blob: bloc length is {}",
                e.len()
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?),
        link,
        created_at: created_at.and_utc(),
        //created_by,
    }))
}
