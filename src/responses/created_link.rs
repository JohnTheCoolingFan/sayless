use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, Clone)]
pub struct CreatedLink {
    pub id: String,
}

impl IntoResponse for CreatedLink {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::CREATED)
            .header(axum::http::header::LOCATION, format!("/l/{}", self.id))
            .body(Default::default())
            .unwrap()
    }
}
