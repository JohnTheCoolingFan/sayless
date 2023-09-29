use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use headers::HeaderName;

const LOCATION_HEADER: HeaderName = HeaderName::from_static("location");

#[derive(Debug, Clone)]
pub struct CreatedLink {
    pub id: String,
}

impl IntoResponse for CreatedLink {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::CREATED)
            .header(LOCATION_HEADER, format!("/l/{}", self.id))
            .body(Default::default())
            .unwrap()
    }
}
