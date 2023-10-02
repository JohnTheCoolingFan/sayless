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
