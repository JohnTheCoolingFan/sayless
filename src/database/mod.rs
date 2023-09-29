use crate::{
    database::tables::{
        links::init_links_table, origins::init_origins_table, strikes::init_strikes_table,
        tokens::init_tokens_table,
    },
    service_config::ServiceConfig,
};
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};
mod tables;

pub async fn connect_db() -> Result<Pool<MySql>, sqlx::Error> {
    log::info!("connecting to db");
    MySqlPoolOptions::new()
        .connect(
            &dotenvy::var("DATABASE_URL")
                .expect("DATABASE_URL must be set to connect to the database"),
        )
        .await
}

pub async fn init_tables(db: &Pool<MySql>, config: &ServiceConfig) -> Result<(), sqlx::Error> {
    log::info!("Creating tables");

    init_links_table(db).await?;

    if config.ip_recording.is_some() {
        init_origins_table(db).await?;
        init_strikes_table(db).await?;
    }

    if config.tokens.is_some() {
        init_tokens_table(db).await?;
    }

    Ok(())
}
