use crate::{
    AppState,
    errors::{AppError, AppResult},
    models::ApiKey,
};
use deadpool_redis::Pool as RedisPool;
use redis::AsyncCommands;
use std::time::Duration;
use uuid::Uuid;

pub struct RateLimitService {
    redis: RedisPool,
}

impl RateLimitService {
    pub fn new(state: &AppState) -> Self {
        Self {
            redis: state.redis.clone(),
        }
    }

    pub async fn check_rate_limit(&self, key: &ApiKey) -> AppResult<RateLimitInfo> {
        let mut conn = self.redis.get().await.map_err(|e| {
            AppError::RedisError(redis::RedisError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        let minute_key = format!("rate:{}:minute:{}", key.id, current_minute());
        let day_key = format!("rate:{}:day:{}", key.id, current_day());

        // Get current counts
        let (minute_count, day_count): (Option<i64>, Option<i64>) = redis::pipe()
            .get(&minute_key)
            .get(&day_key)
            .query_async(&mut *conn)
            .await?;

        let minute_count = minute_count.unwrap_or(0);
        let day_count = day_count.unwrap_or(0);

        // Check limits
        if minute_count >= key.rate_limit_per_minute as i64 {
            return Err(AppError::RateLimitExceeded {
                retry_after: 60 - (chrono::Utc::now().timestamp() % 60) as u64,
            });
        }

        if day_count >= key.rate_limit_per_day as i64 {
            return Err(AppError::RateLimitExceeded {
                retry_after: seconds_until_midnight(),
            });
        }

        Ok(RateLimitInfo {
            minute_remaining: key.rate_limit_per_minute - minute_count as i32,
            day_remaining: key.rate_limit_per_day - day_count as i32,
            minute_limit: key.rate_limit_per_minute,
            day_limit: key.rate_limit_per_day,
        })
    }

    pub async fn increment(&self, key_id: Uuid) -> AppResult<()> {
        let mut conn = self.redis.get().await.map_err(|e| {
            AppError::RedisError(redis::RedisError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )))
        })?;

        let minute_key = format!("rate:{}:minute:{}", key_id, current_minute());
        let day_key = format!("rate:{}:day:{}", key_id, current_day());

        // Increment with TTL
        redis::pipe()
            .incr(&minute_key, 1i64)
            .expire(&minute_key, 60)
            .incr(&day_key, 1i64)
            .expire(&day_key, 86400)
            .query_async::<()>(&mut *conn)
            .await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct RateLimitInfo {
    pub minute_remaining: i32,
    pub day_remaining: i32,
    pub minute_limit: i32,
    pub day_limit: i32,
}

fn current_minute() -> i64 {
    chrono::Utc::now().timestamp() / 60
}

fn current_day() -> i64 {
    chrono::Utc::now().timestamp() / 86400
}

fn seconds_until_midnight() -> u64 {
    let now = chrono::Utc::now();
    let tomorrow = (now + chrono::Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    (tomorrow.and_utc().timestamp() - now.timestamp()) as u64
}
