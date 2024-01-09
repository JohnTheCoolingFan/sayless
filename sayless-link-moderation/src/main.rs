use std::sync::Arc;

use base64::Engine;
use sqlx::mysql::MySqlPoolOptions;

pub fn url_identifier_from_string(url: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(url)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    simple_logger::init_with_env().unwrap();

    let virustotal_api_key: Arc<str> = Arc::from(
        dotenvy::var("VIRUSTOTAL_API_KEY")
            .expect("$VIRUSTOTAL_API_KEY must be set")
            .as_str(),
    );

    let db = Arc::new(
        MySqlPoolOptions::new()
            .connect(&dotenvy::var("DATABASE_URL").expect("$DATABASE_URL must be set"))
            .await
            .unwrap(),
    );

    let client = reqwest::Client::new();
}
