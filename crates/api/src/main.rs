use anyhow::Result;
use axum::routing::get;
use axum::{Router, serve};
use routes::addresses::get_addresses_txs;
use routes::blocks::get_block;
use routes::healthcheck::healthcheck;
use routes::transactions::get_tx;
use sqlx::postgres::{PgPool, PgPoolOptions};
mod error;
mod routes;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(tokio::time::Duration::from_secs(5))
        .connect("postgres://user:password@127.0.0.1:5432/etherscan?sslmode=disable")
        .await
        .expect("failed to connect to postgres");
    let app_state = AppState { db };

    let app = Router::new()
        .route("/health", get(healthcheck))
        .route("/api/v1/block/{number}", get(get_block))
        .route("/api/v1/tx/{hash}", get(get_tx))
        .route("/api/v1/address/{address}/txs", get(get_addresses_txs))
        .with_state(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:42069").await?;

    serve(listener, app).await?;

    Ok(())
}
