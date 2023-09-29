use crate::ServiceState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
};

#[derive(Debug)]
struct LinkQuery {
    link: String,
}

pub async fn get_link_route(
    State(ServiceState { db, config: _ }): State<ServiceState>,
    Path(id): Path<String>,
) -> Result<Redirect, StatusCode> {
    let LinkQuery { link } = sqlx::query_as!(LinkQuery, "SELECT link FROM links WHERE id = ?", id)
        .fetch_one(db.as_ref())
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        })?;
    Ok(Redirect::to(&link))
}
