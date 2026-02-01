use anyhow::Result;
use api::{
    ApiState,
    routes::{
        addresses::get_addresses_txs, blocks::get_block, healthcheck::healthcheck,
        transactions::get_tx,
    },
};
use axum::{Router, routing::get, serve};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let db = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(tokio::time::Duration::from_secs(5))
        .connect("postgres://user:password@127.0.0.1:5432/etherscan?sslmode=disable")
        .await
        .expect("failed to connect to postgres");
    let app_state = ApiState { db };

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
