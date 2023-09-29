use sqlx::{MySql, Pool};

pub async fn init_strikes_table(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    log::debug!("Creating `strikes` table");

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS
            strikes (
                origin TINYBLOB NOT NULL,
                amount SMALLINT UNSIGNED NOT NULL
            )"#,
    )
    .execute(db)
    .await?;

    Ok(())
}
