use deadpool_redis::Pool as RedisPool;
use settings as config;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::watch;

pub mod errors;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod settings;
pub mod utils;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: RedisPool,
    pub config: watch::Receiver<Arc<config::Settings>>,
    pub jwt_keys: Arc<JwtKeys>,
}

pub struct JwtKeys {
    pub access_encoding: jsonwebtoken::EncodingKey,
    pub access_decoding: jsonwebtoken::DecodingKey,
    pub refresh_encoding: jsonwebtoken::EncodingKey,
    pub refresh_decoding: jsonwebtoken::DecodingKey,
}

impl JwtKeys {
    pub fn new(config: &config::JwtConfig) -> Self {
        Self {
            access_encoding: jsonwebtoken::EncodingKey::from_secret(
                config.access_token_secret.as_bytes(),
            ),
            access_decoding: jsonwebtoken::DecodingKey::from_secret(
                config.access_token_secret.as_bytes(),
            ),
            refresh_encoding: jsonwebtoken::EncodingKey::from_secret(
                config.refresh_token_secret.as_bytes(),
            ),
            refresh_decoding: jsonwebtoken::DecodingKey::from_secret(
                config.refresh_token_secret.as_bytes(),
            ),
        }
    }
}
