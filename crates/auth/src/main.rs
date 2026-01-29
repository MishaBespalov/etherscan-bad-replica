use axum::{
    Router, middleware,
    routing::{delete, get, post},
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::signal;
use tower_http::{
    ServiceBuilderExt,
    cors::{Any, CorsLayer},
    request_id::MakeRequestUuid,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use auth::config::settings as config;
use auth_service::{
    AppState, JwtKeys,
    config::{self, Settings},
    handlers::{api_key, auth, usage},
    middleware::auth::{api_key_auth, jwt_auth},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().init();

    // Load configuration
    dotenvy::dotenv().ok();
    let settings = Settings::load()?;

    // Create config watch channel
    let (config_tx, config_rx) = config::create_config_watcher();

    // Setup database pool
    let db_pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .connect(&settings.database.url)
        .await?;

    // Run migrations
    // sqlx::migrate!("../../migrations/auth/")
    //     .run(&db_pool)
    //     .await?;

    // Setup Redis pool
    let redis_cfg = deadpool_redis::Config::from_url(&settings.redis.url);
    let redis_pool = redis_cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

    // Create JWT keys
    let jwt_keys = Arc::new(JwtKeys::new(&settings.jwt));

    // Create app state
    let state = AppState {
        db: db_pool,
        redis: redis_pool,
        config: config_rx,
        jwt_keys,
    };

    // Build router
    let app = create_router(state.clone());

    // Spawn config watcher task
    spawn_config_watcher(config_tx);

    // Start server
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("🚀 Server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/refresh", post(auth::refresh));

    // Protected routes (JWT auth required)
    let protected_routes = Router::new()
        .route("/auth/api-keys", post(api_key::create_key))
        .route("/auth/api-keys", get(api_key::list_keys))
        .route("/auth/api-keys/:id", delete(api_key::delete_key))
        .route("/auth/usage", get(usage::get_usage))
        .layer(middleware::from_fn_with_state(state.clone(), jwt_auth));

    // Health check
    let health_route = Router::new().route("/health", get(|| async { "OK" }));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(health_route)
        .layer(
            tower::ServiceBuilder::new()
                .set_x_request_id(MakeRequestUuid)
                .layer(TraceLayer::new_for_http())
                .propagate_x_request_id()
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        )
        .with_state(state)
}

fn spawn_config_watcher(tx: tokio::sync::watch::Sender<Arc<Settings>>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            match Settings::load() {
                Ok(new_settings) => {
                    if let Err(e) = tx.send(Arc::new(new_settings)) {
                        tracing::error!("Failed to broadcast config update: {}", e);
                    } else {
                        tracing::debug!("Config reloaded successfully");
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to reload config: {}", e);
                }
            }
        }
    });
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
