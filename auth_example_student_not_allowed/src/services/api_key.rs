use crate::{
    AppState,
    errors::{AppError, AppResult},
    models::{ApiKey, ApiKeyResponse, CreateApiKeyRequest, CreateApiKeyResponse},
    utils::{generate_api_key, hash_api_key},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct ApiKeyService {
    db: PgPool,
}

impl ApiKeyService {
    pub fn new(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    pub async fn create_key(
        &self,
        user_id: Uuid,
        req: CreateApiKeyRequest,
    ) -> AppResult<CreateApiKeyResponse> {
        let config = crate::config::Settings::load()?;

        // Generate API key (format: prefix_randompart)
        let (api_key, prefix) = generate_api_key();
        let key_hash = hash_api_key(&api_key);

        let expires_at = req
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days));

        let rate_limit_per_minute = req.rate_limit_per_minute.unwrap_or(
            config
                .rate_limit
                .default_requests_per_minute as i32,
        );
        let rate_limit_per_day = req.rate_limit_per_day.unwrap_or(
            config
                .rate_limit
                .default_requests_per_day as i32,
        );

        let scopes = req.scopes.unwrap_or_default();

        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            INSERT INTO api_keys (
                user_id, name, key_hash, key_prefix, scopes,
                rate_limit_per_minute, rate_limit_per_day, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&req.name)
        .bind(&key_hash)
        .bind(&prefix)
        .bind(&scopes)
        .bind(rate_limit_per_minute)
        .bind(rate_limit_per_day)
        .bind(expires_at)
        .fetch_one(&self.db)
        .await?;

        Ok(CreateApiKeyResponse {
            api_key, // Only returned once!
            details: ApiKeyResponse::from(key),
        })
    }

    pub async fn list_keys(&self, user_id: Uuid) -> AppResult<Vec<ApiKeyResponse>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(keys
            .into_iter()
            .map(ApiKeyResponse::from)
            .collect())
    }

    pub async fn delete_key(&self, user_id: Uuid, key_id: Uuid) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
            .bind(key_id)
            .bind(user_id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::ApiKeyNotFound);
        }

        Ok(())
    }

    pub async fn validate_key(&self, api_key: &str) -> AppResult<ApiKey> {
        let prefix = &api_key[..8.min(api_key.len())];
        let key_hash = hash_api_key(api_key);

        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys 
            WHERE key_prefix = $1 
            AND key_hash = $2 
            AND is_active = true
            "#,
        )
        .bind(prefix)
        .bind(&key_hash)
        .fetch_optional(&self.db)
        .await?
        .ok_or(AppError::ApiKeyNotFound)?;

        // Check expiration
        if let Some(expires_at) = key.expires_at
            && expires_at < Utc::now()
        {
            return Err(AppError::ApiKeyExpired);
        }

        // Update last used
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(key.id)
            .execute(&self.db)
            .await?;

        Ok(key)
    }
}
