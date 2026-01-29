use crate::{
    AppState,
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{AuthResponse, LoginRequest, RefreshRequest, RegisterRequest},
    services::auth::AuthService,
};
use axum::{Json, extract::State};
use validator::Validate;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = AuthService::new(&state);
    let response = service.register(req).await?;

    Ok(Json(response))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = AuthService::new(&state);
    let response = service.login(req).await?;

    Ok(Json(response))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    let service = AuthService::new(&state);
    let response = service
        .refresh_token(&req.refresh_token)
        .await?;

    Ok(Json(response))
}
