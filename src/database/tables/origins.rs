use sqlx::{MySql, Pool};

pub async fn init_origins_table(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    log::debug!("Creating `origins` table");

    sqlx::query!(
        r#"CREATE TABLE IF NOT EXISTS
            origins (
                id TEXT NOT NULL,
                created_by TINYBLOB NOT NULL
            )"#,
    )
    .execute(db)
    .await?;

    Ok(())
}
