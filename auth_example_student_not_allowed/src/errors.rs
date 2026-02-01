use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use deadpool_redis::redis;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("User not found")]
    UserNotFound,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("API key not found")]
    ApiKeyNotFound,

    #[error("API key expired")]
    ApiKeyExpired,

    #[error("Rate limit exceeded")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Redis error")]
    RedisError(#[from] redis::RedisError),

    #[error("Internal server error")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::AuthenticationFailed(msg) => {
                (StatusCode::UNAUTHORIZED, "AUTH_FAILED", msg.clone())
            }
            AppError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                self.to_string(),
            ),
            AppError::TokenExpired => (StatusCode::UNAUTHORIZED, "TOKEN_EXPIRED", self.to_string()),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "INVALID_TOKEN", self.to_string()),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "USER_NOT_FOUND", self.to_string()),
            AppError::UserAlreadyExists => (StatusCode::CONFLICT, "USER_EXISTS", self.to_string()),
            AppError::ApiKeyNotFound => {
                (StatusCode::NOT_FOUND, "API_KEY_NOT_FOUND", self.to_string())
            }
            AppError::ApiKeyExpired => (
                StatusCode::UNAUTHORIZED,
                "API_KEY_EXPIRED",
                self.to_string(),
            ),
            AppError::RateLimitExceeded { retry_after } => {
                let body = Json(json!({
                    "error": {
                        "code": "RATE_LIMIT_EXCEEDED",
                        "message": self.to_string(),
                        "retry_after": retry_after
                    }
                }));
                return (StatusCode::TOO_MANY_REQUESTS, body).into_response();
            }
            AppError::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string())
            }
            AppError::ValidationError(msg) => {
                (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone())
            }
            AppError::DatabaseError(_) | AppError::RedisError(_) | AppError::InternalError(_) => {
                tracing::error!("Internal error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
