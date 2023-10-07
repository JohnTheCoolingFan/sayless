use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct ShortLinkRedirect {
    location: String,
}

impl IntoResponse for ShortLinkRedirect {
    fn into_response(self) -> Response {
        Response::builder()
            .status(StatusCode::FOUND)
            .header(axum::http::header::LOCATION, self.location)
            .body(Default::default())
            .unwrap()
    }
}
