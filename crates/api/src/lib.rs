pub mod config;
pub mod error;
pub mod middleware;
pub mod routes;

use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct ApiState {
    pub db: PgPool,
}
