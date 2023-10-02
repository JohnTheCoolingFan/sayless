use crate::{database::connect_db, routes::create_router};
use chrono::Utc;
use service_config::ServiceConfig;
use sqlx::{MySql, Pool};
use std::{
    error::Error,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio_cron_scheduler::{Job, JobScheduler};

mod base58;
mod database;
mod json_schemas;
mod responses;
mod routes;
mod service_config;
mod tokens;

pub type DbPool = Arc<Pool<MySql>>;

#[derive(Clone)]
pub struct ServiceState {
    pub db: DbPool,
    pub config: ServiceConfig,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = service_config::get_config()
        .await
        .expect("Reading config failed");

    let ip_record_config = config.ip_recording.clone();

    simple_logger::init_with_level(config.log_level)?;

    dotenvy::dotenv()?;

    let server_port = dotenvy::var("PORT")
        .expect("PORT environment variable must be set")
        .parse()
        .expect("Parsing port number failed");

    let db = Arc::new(connect_db().await?);

    log::info!("Applying migrations");
    sqlx::migrate!().run(db.as_ref()).await?;

    let router = create_router(&config);

    let state = ServiceState {
        db: Arc::clone(&db),
        config,
    };

    let router = router.with_state(state);

    log::info!("Starting server");
    let server_handle = tokio::spawn(
        axum::Server::bind(&SocketAddr::from((
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            server_port,
        )))
        .serve(router.into_make_service_with_connect_info::<SocketAddr>()),
    );

    if let Some(ip_recoding_config) = ip_record_config {
        let sched = JobScheduler::new().await?;

        sched
            .add(Job::new_async(
                ip_recoding_config.retention_check_period.as_ref(),
                move |_, _| {
                    let db_cloned = Arc::clone(&db);
                    Box::pin(async move {
                        log::debug!("IP address retention check");
                        let expired_date = Utc::now() - ip_recoding_config.retention_period;
                        if let Err(why) = sqlx::query(
                            r#"
                            DELETE FROM origins orgs
                            WHERE orgs.id in (
                                SELECT linkst.id
                                FROM links linkst
                                WHERE created_at < ?
                            )"#,
                        )
                        .bind(expired_date)
                        .execute(db_cloned.as_ref())
                        .await
                        {
                            log::error!("Error in IP retention check query: {}", why);
                        }
                    })
                },
            )?)
            .await?;

        sched.shutdown_on_ctrl_c();

        sched.start().await?;
    }
    server_handle.await??;

    Ok(())
}
