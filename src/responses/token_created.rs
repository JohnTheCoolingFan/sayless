use axum::{
    body::HttpBody,
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
            .body(
                self.token
                    .boxed_unsync()
                    .map_err(|_| unreachable!())
                    .boxed_unsync(),
            )
            .unwrap()
    }
}
