use crate::{
    AppState, errors::AppError, models::api_key::ApiKey, services::rate_limit::RateLimitService,
};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Get API key from extensions (set by api_key_auth middleware)
    let api_key = request
        .extensions()
        .get::<ApiKey>()
        .cloned();

    let response = next.run(request).await;

    // Increment rate limit counter after successful request
    if let Some(key) = api_key {
        let service = RateLimitService::new(&state);
        // Fire and forget - don't block response
        tokio::spawn(async move {
            if let Err(e) = service.increment(key.id).await {
                tracing::error!("Failed to increment rate limit: {:?}", e);
            }
        });
    }

    Ok(response)
}
