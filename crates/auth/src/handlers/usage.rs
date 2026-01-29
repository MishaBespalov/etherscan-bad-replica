use crate::{AppState, errors::AppResult, middleware::auth::AuthUser};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub api_key_id: Option<uuid::Uuid>,
}

#[derive(Debug, Serialize)]
pub struct UsageStats {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub avg_response_time_ms: f64,
    pub total_request_bytes: i64,
    pub total_response_bytes: i64,
    pub by_endpoint: Vec<EndpointStats>,
    pub by_day: Vec<DailyStats>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EndpointStats {
    pub endpoint: String,
    pub method: String,
    pub count: i64,
    pub avg_response_time_ms: f64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyStats {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

pub async fn get_usage(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Query(query): Query<UsageQuery>,
) -> AppResult<Json<UsageStats>> {
    let from = query
        .from
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let to = query.to.unwrap_or_else(Utc::now);

    // Get aggregated stats
    let stats = sqlx::query_as::<_, (i64, i64, i64, f64, i64, i64)>(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE status_code < 400) as successful,
            COUNT(*) FILTER (WHERE status_code >= 400) as failed,
            COALESCE(AVG(response_time_ms), 0) as avg_time,
            COALESCE(SUM(request_size_bytes), 0) as req_bytes,
            COALESCE(SUM(response_size_bytes), 0) as res_bytes
        FROM usage_logs ul
        JOIN api_keys ak ON ul.api_key_id = ak.id
        WHERE ak.user_id = $1
        AND ul.created_at BETWEEN $2 AND $3
        AND ($4::uuid IS NULL OR ak.id = $4)
        "#,
    )
    .bind(auth.claims.sub)
    .bind(from)
    .bind(to)
    .bind(query.api_key_id)
    .fetch_one(&state.db)
    .await?;

    // Get stats by endpoint
    let by_endpoint = sqlx::query_as::<_, EndpointStats>(
        r#"
        SELECT 
            endpoint,
            method,
            COUNT(*) as count,
            AVG(response_time_ms) as avg_response_time_ms
        FROM usage_logs ul
        JOIN api_keys ak ON ul.api_key_id = ak.id
        WHERE ak.user_id = $1
        AND ul.created_at BETWEEN $2 AND $3
        GROUP BY endpoint, method
        ORDER BY count DESC
        LIMIT 20
        "#,
    )
    .bind(auth.claims.sub)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?;

    // Get daily stats
    let by_day = sqlx::query_as::<_, DailyStats>(
        r#"
        SELECT 
            DATE(ul.created_at) as date,
            COUNT(*) as count
        FROM usage_logs ul
        JOIN api_keys ak ON ul.api_key_id = ak.id
        WHERE ak.user_id = $1
        AND ul.created_at BETWEEN $2 AND $3
        GROUP BY DATE(ul.created_at)
        ORDER BY date
        "#,
    )
    .bind(auth.claims.sub)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(UsageStats {
        total_requests: stats.0,
        successful_requests: stats.1,
        failed_requests: stats.2,
        avg_response_time_ms: stats.3,
        total_request_bytes: stats.4,
        total_response_bytes: stats.5,
        by_endpoint,
        by_day,
    }))
}
