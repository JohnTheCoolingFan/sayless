use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

pub async fn connect_db() -> Result<Pool<MySql>, sqlx::Error> {
    log::info!("connecting to db");
    MySqlPoolOptions::new()
        .connect(
            &dotenvy::var("DATABASE_URL")
                .expect("DATABASE_URL must be set to connect to the database"),
        )
        .await
}

pub async fn create_ip_recording_tables(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    log::debug!("Creating `origins` table");
    sqlx::query_file!("db/optional/origins.sql")
        .execute(db)
        .await?;
    log::debug!("Creating `strikes` table");
    sqlx::query_file!("db/optional/strikes.sql")
        .execute(db)
        .await?;
    Ok(())
}

pub async fn create_token_tables(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    log::debug!("Creating `tokens` table");
    sqlx::query_file!("db/optional/tokens.sql")
        .execute(db)
        .await?;
    Ok(())
}
