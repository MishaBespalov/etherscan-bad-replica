use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub email_verified: bool,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub email_verified: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
            email_verified: user.email_verified,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid, // User ID
    pub email: String,
    pub iat: i64,    // Issued at
    pub exp: i64,    // Expiration
    pub iss: String, // Issuer
    pub token_type: TokenType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    Access,
    Refresh,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_day: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateApiKeyRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_in_days: Option<i64>,
    pub rate_limit_per_minute: Option<i32>,
    pub rate_limit_per_day: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub rate_limit_per_minute: i32,
    pub rate_limit_per_day: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String, // Only shown once at creation
    pub details: ApiKeyResponse,
}

impl From<ApiKey> for ApiKeyResponse {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            scopes: key.scopes,
            rate_limit_per_minute: key.rate_limit_per_minute,
            rate_limit_per_day: key.rate_limit_per_day,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
            created_at: key.created_at,
            is_active: key.is_active,
        }
    }
}
