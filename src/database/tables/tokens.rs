use sqlx::{MySql, Pool};

pub async fn init_tokens_table(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    log::debug!("Creating `tokens` table");

    sqlx::query!(
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
    .execute(db)
    .await?;

    Ok(())
}
