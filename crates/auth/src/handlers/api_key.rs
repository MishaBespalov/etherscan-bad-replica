use crate::{
    AppState,
    errors::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{ApiKeyResponse, CreateApiKeyRequest, CreateApiKeyResponse},
    services::api_key::ApiKeyService,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use uuid::Uuid;
use validator::Validate;

pub async fn create_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(req): Json<CreateApiKeyRequest>,
) -> AppResult<Json<CreateApiKeyResponse>> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = ApiKeyService::new(&state);
    let response = service
        .create_key(auth.claims.sub, req)
        .await?;

    Ok(Json(response))
}

pub async fn list_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
) -> AppResult<Json<Vec<ApiKeyResponse>>> {
    let service = ApiKeyService::new(&state);
    let keys = service
        .list_keys(auth.claims.sub)
        .await?;

    Ok(Json(keys))
}

pub async fn delete_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(key_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let service = ApiKeyService::new(&state);
    service
        .delete_key(auth.claims.sub, key_id)
        .await?;

    Ok(Json(serde_json::json!({"status": "deleted"})))
}
