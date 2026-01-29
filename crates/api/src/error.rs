use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("database error")]
    DatabaseError(#[from] sqlx::Error),
    #[error("unknown error")]
    AnyhowError(#[from] anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::DatabaseError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("err:{}", err))
            }
            ApiError::AnyhowError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("unexpected err: {}", err),
            ),
        };
        let body = Json(json!({"error": error_message}));

        (status, body).into_response()
    }
}
