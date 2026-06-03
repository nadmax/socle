use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Top-level application error type.
///
/// Every variant maps to an HTTP status code and a stable `code` string
/// that clients can match on without parsing human-readable messages.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("token has expired")]
    TokenExpired,

    #[error("token is invalid")]
    TokenInvalid,

    #[error("missing authorization header")]
    MissingAuthHeader,

    #[error("refresh token not found or revoked")]
    RefreshTokenInvalid,

    #[error("email is already taken")]
    EmailTaken,

    #[error("username is already taken")]
    UsernameTaken,

    #[error("user not found")]
    UserNotFound,

    #[error("account is disabled")]
    AccountDisabled,

    #[error("insufficient permissions")]
    Forbidden,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("password hashing error")]
    Hashing,

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials
            | Self::TokenExpired
            | Self::TokenInvalid
            | Self::MissingAuthHeader
            | Self::RefreshTokenInvalid => StatusCode::UNAUTHORIZED,

            Self::EmailTaken | Self::UsernameTaken => StatusCode::CONFLICT,

            Self::UserNotFound => StatusCode::NOT_FOUND,

            Self::AccountDisabled | Self::Forbidden => StatusCode::FORBIDDEN,

            Self::Database(_) | Self::Hashing | Self::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::TokenExpired => "TOKEN_EXPIRED",
            Self::TokenInvalid => "TOKEN_INVALID",
            Self::MissingAuthHeader => "MISSING_AUTH_HEADER",
            Self::RefreshTokenInvalid => "REFRESH_TOKEN_INVALID",
            Self::EmailTaken => "EMAIL_TAKEN",
            Self::UsernameTaken => "USERNAME_TAKEN",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::AccountDisabled => "ACCOUNT_DISABLED",
            Self::Forbidden => "FORBIDDEN",
            Self::Database(_) => "DATABASE_ERROR",
            Self::Hashing => "HASHING_ERROR",
            Self::Internal(_) => "INTERNAL_ERROR",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = json!({
            "error": {
                "code":    self.error_code(),
                "message": self.to_string(),
            }
        });

        tracing::warn!(
            status = status.as_u16(),
            code   = self.error_code(),
            error  = %self,
            "request error"
        );

        (status, Json(body)).into_response()
    }
}

/// Application result type using `AppError` as the error variant.
pub type AppResult<T> = Result<T, AppError>;
