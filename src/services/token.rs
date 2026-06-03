use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng as ArgonOsRng,
    },
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::{
    config::Config,
    errors::{AppError, AppResult},
    models::{Claims, Role},
};

/// Minimal projection of a `refresh_tokens` row.
/// Only the columns we actually read after the query are selected.
#[derive(sqlx::FromRow)]
struct RefreshTokenRow {
    id: Uuid,
    user_id: Uuid,
    expires_at: time::OffsetDateTime,
    revoked: bool,
}

/// Manages JWT creation/validation and refresh token persistence.
#[derive(Clone)]
pub struct TokenService {
    pool: PgPool,
    config: Config,
}

impl TokenService {
    pub fn new(pool: PgPool, config: Config) -> Self {
        Self { pool, config }
    }

    /// Encode a new signed JWT access token for the given user.
    pub fn generate_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        username: &str,
        role: Role,
    ) -> AppResult<String> {
        let now = unix_now();
        let claims = Claims {
            sub: user_id,
            iat: now,
            exp: now + self.config.access_token_expiry_secs,
            email: email.to_owned(),
            username: username.to_owned(),
            role,
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(|_| AppError::TokenInvalid)
    }

    /// Validate a JWT access token and return its claims.
    pub fn validate_access_token(&self, token: &str) -> AppResult<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|err| {
            use jsonwebtoken::errors::ErrorKind;
            match err.kind() {
                ErrorKind::ExpiredSignature => AppError::TokenExpired,
                _ => AppError::TokenInvalid,
            }
        })
    }

    /// Create a new opaque refresh token, persist its hash, and return the
    /// raw token string (never stored in plaintext).
    pub async fn create_refresh_token(&self, user_id: Uuid) -> AppResult<String> {
        let raw = generate_opaque_token();
        let token_hash = hash_refresh_token(&raw)?;

        let expires_at = OffsetDateTime::now_utc()
            + Duration::seconds(self.config.refresh_token_expiry_secs as i64);

        sqlx::query!(
            r#"
            INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::now_v7(),
            user_id,
            token_hash,
            expires_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(raw)
    }

    /// Validate a raw refresh token: look up by hash, check expiry/revocation,
    /// then immediately rotate it (old token revoked, new one issued).
    ///
    /// Returns `(new_raw_token, user_id)` on success.
    pub async fn rotate_refresh_token(&self, raw_token: &str) -> AppResult<(String, Uuid)> {
        let hash = hash_refresh_token(raw_token)?;

        let record: RefreshTokenRow = sqlx::query_as!(
            RefreshTokenRow,
            r#"
            SELECT id, user_id, expires_at, revoked
            FROM refresh_tokens
            WHERE token_hash = $1
            "#,
            hash,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::RefreshTokenInvalid)?;

        if record.revoked || record.expires_at < OffsetDateTime::now_utc() {
            self.revoke_all_user_tokens(record.user_id).await?;
            return Err(AppError::RefreshTokenInvalid);
        }

        sqlx::query!(
            "UPDATE refresh_tokens SET revoked = TRUE WHERE id = $1",
            record.id,
        )
        .execute(&self.pool)
        .await?;

        let new_raw = self.create_refresh_token(record.user_id).await?;
        Ok((new_raw, record.user_id))
    }

    /// Revoke all refresh tokens belonging to a user (used on logout).
    pub async fn revoke_all_user_tokens(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query!(
            "UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1 AND revoked = FALSE",
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// Generate a cryptographically-random 256-bit token encoded as hex.
fn generate_opaque_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS RNG failed");
    hex::encode(bytes)
}

/// Argon2id hash of the raw refresh token value.
///
/// Using Argon2 ensures that a DB leak cannot be trivially converted into
/// working tokens without substantial compute.
fn hash_refresh_token(raw: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut ArgonOsRng);
    Argon2::default()
        .hash_password(raw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AppError::Hashing)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_secs()
}

/// Hash a plaintext password with Argon2id.
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut ArgonOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| AppError::Hashing)
}

/// Verify a plaintext password against an Argon2 hash.
pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|_| AppError::Hashing)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
