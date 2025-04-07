use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct TokenCreated {
    pub token: String,
}

impl IntoResponse for TokenCreated {
    fn into_response(self) -> Response {
        Response::<()>::builder()
            .status(StatusCode::CREATED)
            .body(self.token.into())
            .unwrap()
    }
}
