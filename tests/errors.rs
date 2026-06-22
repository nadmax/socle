mod common;

use axum::response::IntoResponse;
use yaima::errors::AppError;

#[test]
fn error_http_mapping_is_stable() {
    let cases: Vec<(&str, AppError, u16)> = vec![
        ("INVALID_CREDENTIALS", AppError::InvalidCredentials, 401),
        ("TOKEN_EXPIRED", AppError::TokenExpired, 401),
        ("TOKEN_INVALID", AppError::TokenInvalid, 401),
        ("MISSING_AUTH_HEADER", AppError::MissingAuthHeader, 401),
        ("REFRESH_TOKEN_INVALID", AppError::RefreshTokenInvalid, 401),
        ("EMAIL_TAKEN", AppError::EmailTaken, 409),
        ("USERNAME_TAKEN", AppError::UsernameTaken, 409),
        ("USER_NOT_FOUND", AppError::UserNotFound, 404),
        ("ACCOUNT_DISABLED", AppError::AccountDisabled, 403),
        ("FORBIDDEN", AppError::Forbidden, 403),
        ("HASHING_ERROR", AppError::Hashing, 500),
    ];

    for (code, err, expected_status) in cases {
        let response = err.into_response();
        assert_eq!(
            response.status().as_u16(),
            expected_status,
            "wrong HTTP status for error code {code}"
        );
    }
}
