use axum::{Json, Router, extract::State, http::StatusCode, routing::post};

use crate::{
    errors::AppResult,
    middleware::AuthUser,
    models::{AuthResponse, LoginRequest, MessageResponse, RefreshRequest, RegisterRequest},
    state::AppState,
};

/// Mount all `/auth/*` routes onto a new [`Router`].
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
}

/// Register a new user account.
///
/// # Errors
///
/// Returns an [`AppError`] if the email or username is already taken, or if
/// the underlying service or database call fails.
#[utoipa::path(
    post,
    path = "/auth/register",
    tag  = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Account created",      body = AuthResponse),
        (status = 409, description = "Email/username taken", body = serde_json::Value),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    let response = state
        .auth
        .register(&req.email, &req.username, &req.password)
        .await?;

    tracing::info!(email = %req.email, "new user registered");
    Ok((StatusCode::CREATED, Json(response)))
}

/// Authenticate with email and password.
///
/// # Errors
///
/// Returns an [`AppError`] if the credentials are invalid, or if the
/// underlying service or database call fails.
#[utoipa::path(
    post,
    path = "/auth/login",
    tag  = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authenticated",        body = AuthResponse),
        (status = 401, description = "Invalid credentials",  body = serde_json::Value),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let response = state.auth.login(&req.email, &req.password).await?;

    tracing::info!(email = %req.email, "user logged in");
    Ok(Json(response))
}

/// Exchange a refresh token for a new token pair.
///
/// # Errors
///
/// Returns an [`AppError`] if the refresh token is invalid or expired, or if
/// the underlying service or database call fails.
#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag  = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed",       body = AuthResponse),
        (status = 401, description = "Refresh token invalid", body = serde_json::Value),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    let response = state.auth.refresh(&req.refresh_token).await?;
    Ok(Json(response))
}

/// Revoke all refresh tokens for the currently authenticated user.
///
/// # Errors
///
/// Returns an [`AppError`] if the underlying service or database call fails.
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag  = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logged out",        body = MessageResponse),
        (status = 401, description = "Not authenticated", body = serde_json::Value),
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<MessageResponse>> {
    state.auth.logout(claims.sub).await?;

    tracing::info!(user_id = %claims.sub, "user logged out");
    Ok(Json(MessageResponse::new("Logged out successfully")))
}
