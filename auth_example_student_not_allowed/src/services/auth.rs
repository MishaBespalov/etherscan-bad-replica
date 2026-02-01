use crate::{
    AppState, JwtKeys,
    config::JwtConfig,
    errors::{AppError, AppResult},
    models::{AuthResponse, Claims, LoginRequest, RegisterRequest, TokenType, User, UserResponse},
    utils::{hash_password, verify_password},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, Header, Validation, decode, encode};
use sqlx::PgPool;
use std::sync::Arc;

pub struct AuthService {
    db: PgPool,
    jwt_keys: Arc<JwtKeys>,
    jwt_config: JwtConfig,
}

impl AuthService {
    pub fn new(state: &AppState) -> Self {
        let config = state.config.borrow().clone();
        Self {
            db: state.db.clone(),
            jwt_keys: state.jwt_keys.clone(),
            jwt_config: config.jwt.clone(),
        }
    }

    pub async fn register(&self, req: RegisterRequest) -> AppResult<AuthResponse> {
        // Check if user exists
        let existing =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
                .bind(&req.email)
                .fetch_one(&self.db)
                .await?;

        if existing {
            return Err(AppError::UserAlreadyExists);
        }

        // Hash password
        let password_hash = hash_password(&req.password)?;

        // Create user
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(&req.email)
        .bind(&password_hash)
        .fetch_one(&self.db)
        .await?;

        // Generate tokens
        self.create_auth_response(&user).await
    }

    pub async fn login(&self, req: LoginRequest) -> AppResult<AuthResponse> {
        let user =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND is_active = true")
                .bind(&req.email)
                .fetch_optional(&self.db)
                .await?
                .ok_or(AppError::InvalidCredentials)?;

        // Verify password
        if !verify_password(&req.password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        self.create_auth_response(&user).await
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<AuthResponse> {
        // Validate refresh token
        let claims = self.validate_refresh_token(refresh_token)?;

        // Check if token is revoked
        let token_hash = crate::utils::hash_token(refresh_token);
        let is_valid = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM refresh_tokens 
                WHERE token_hash = $1 
                AND user_id = $2 
                AND revoked_at IS NULL 
                AND expires_at > NOW()
            )
            "#,
        )
        .bind(&token_hash)
        .bind(claims.sub)
        .fetch_one(&self.db)
        .await?;

        if !is_valid {
            return Err(AppError::InvalidToken);
        }

        // Get user
        let user =
            sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 AND is_active = true")
                .bind(claims.sub)
                .fetch_optional(&self.db)
                .await?
                .ok_or(AppError::UserNotFound)?;

        // Revoke old refresh token
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NOW() WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(&self.db)
            .await?;

        // Create new tokens
        self.create_auth_response(&user).await
    }

    async fn create_auth_response(&self, user: &User) -> AppResult<AuthResponse> {
        let now = Utc::now();

        // Access token
        let access_claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.jwt_config.access_token_expiry_secs)).timestamp(),
            iss: self.jwt_config.issuer.clone(),
            token_type: TokenType::Access,
        };

        let access_token = encode(
            &Header::default(),
            &access_claims,
            &self.jwt_keys.access_encoding,
        )
        .map_err(|e| AppError::InternalError(e.into()))?;

        // Refresh token
        let refresh_claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            iat: now.timestamp(),
            exp: (now
                + Duration::seconds(
                    self.jwt_config
                        .refresh_token_expiry_secs,
                ))
            .timestamp(),
            iss: self.jwt_config.issuer.clone(),
            token_type: TokenType::Refresh,
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &self.jwt_keys.refresh_encoding,
        )
        .map_err(|e| AppError::InternalError(e.into()))?;

        // Store refresh token hash
        let token_hash = crate::utils::hash_token(&refresh_token);
        let expires_at = now
            + Duration::seconds(
                self.jwt_config
                    .refresh_token_expiry_secs,
            );

        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user.id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_config.access_token_expiry_secs,
            user: UserResponse::from(user.clone()),
        })
    }

    pub fn validate_access_token(&self, token: &str) -> AppResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.jwt_config.issuer]);

        let token_data = decode::<Claims>(token, &self.jwt_keys.access_decoding, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        if token_data.claims.token_type != TokenType::Access {
            return Err(AppError::InvalidToken);
        }

        Ok(token_data.claims)
    }

    fn validate_refresh_token(&self, token: &str) -> AppResult<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.jwt_config.issuer]);

        let token_data = decode::<Claims>(token, &self.jwt_keys.refresh_decoding, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::InvalidToken,
            })?;

        if token_data.claims.token_type != TokenType::Refresh {
            return Err(AppError::InvalidToken);
        }

        Ok(token_data.claims)
    }
}
