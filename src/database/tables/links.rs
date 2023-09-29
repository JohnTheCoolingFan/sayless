use sqlx::{MySql, Pool};

pub async fn init_links_table(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
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
    .execute(db)
    .await?;

    Ok(())
}
