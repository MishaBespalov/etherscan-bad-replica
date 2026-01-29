use crate::{AppState, errors::AppError, models::token::Claims, services::auth::AuthService};
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};

#[derive(Clone)]
pub struct AuthUser {
    pub claims: Claims,
}

pub async fn jwt_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::AuthenticationFailed(
            "Missing authorization header".into(),
        ))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::AuthenticationFailed(
            "Invalid authorization format".into(),
        ))?;

    let auth_service = AuthService::new(&state);
    let claims = auth_service.validate_access_token(token)?;

    request
        .extensions_mut()
        .insert(AuthUser { claims });

    Ok(next.run(request).await)
}

// For API key authentication
pub async fn api_key_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::AuthenticationFailed("Missing API key".into()))?;

    let api_key_service = crate::services::api_key::ApiKeyService::new(&state);
    let key = api_key_service
        .validate_key(api_key)
        .await?;

    // Check rate limit
    let rate_limit_service = crate::services::rate_limit::RateLimitService::new(&state);
    let rate_info = rate_limit_service
        .check_rate_limit(&key)
        .await?;

    // Store key info for handlers
    request.extensions_mut().insert(key);
    request
        .extensions_mut()
        .insert(rate_info);

    let response = next.run(request).await;

    Ok(response)
}
