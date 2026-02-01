use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub access_token_secret: String,
    pub refresh_token_secret: String,
    pub access_token_expiry_secs: i64,
    pub refresh_token_expiry_secs: i64,
    pub issuer: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    pub default_requests_per_minute: u32,
    pub default_requests_per_day: u32,
    pub burst_size: u32,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name("config/local").required(false))
            .add_source(config::Environment::with_prefix("APP").separator("__"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

pub fn create_config_watcher() -> (watch::Sender<Arc<Settings>>, watch::Receiver<Arc<Settings>>) {
    let settings = Settings::load().expect("Failed to load config");
    watch::channel(Arc::new(settings))
}
