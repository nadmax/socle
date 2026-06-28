#![allow(dead_code)]
use axum::Router;
use axum_test::TestServer;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use socle::{
    config::{Config, OAuthConfig},
    rate_limit::RateLimiter,
    routes,
    services::{auth::AuthService, oauth::StateStore, token::TokenService, user::UserService},
    state::AppState,
};

static DB_CLEANED: AtomicBool = AtomicBool::new(false);

/// Connect to the test database and run all pending migrations.
///
/// Stale data is cleaned exactly once per process (not once per test) to allow
/// multiple test binaries to share the same database without colliding on
/// leftover rows from a previous run.
pub async fn test_pool() -> PgPool {
    let _ = dotenvy::dotenv();
    let url = env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set for integration tests");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    if DB_CLEANED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_ok()
    {
        sqlx::query!(
            "TRUNCATE TABLE users, refresh_tokens, oauth_credentials, local_credentials CASCADE"
        )
        .execute(&pool)
        .await
        .expect("failed to clean test tables");
    }

    pool
}

/// Minimal config suitable for tests.
///
/// Rate‑limit parameters use generous defaults so existing tests are never
/// accidentally throttled.
pub fn test_config() -> Config {
    Config {
        database_url: String::new(),
        jwt_secret: "test-secret-that-is-long-enough-32+".to_owned(),
        valkey_url: "redis://127.0.0.1:6379".to_owned(),
        access_token_expiry_secs: 3600,
        refresh_token_expiry_secs: 86_400,
        bind_addr: "0.0.0.0:0".to_owned(),
        shutdown_timeout_secs: 5,
        auth_rate_limit_max: 10_000,
        auth_rate_limit_window_secs: 3600,
        oauth: OAuthConfig::default(),
    }
}

/// Build the full Axum app wired to a real test database.
pub async fn test_app() -> (Router, PgPool) {
    let pool = test_pool().await;
    let config = test_config();

    let oauth_store = Arc::new(
        StateStore::new(&config.valkey_url).expect("failed to create oauth state store for tests"),
    );
    let rate_limiter = RateLimiter::new(
        &config.valkey_url,
        config.auth_rate_limit_max,
        config.auth_rate_limit_window_secs,
    )
    .expect("failed to create rate limiter for tests");
    let user = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    let auth = AuthService::new(user.clone(), token.clone(), config.clone());
    let state = AppState::new(auth, user, token, config, oauth_store, rate_limiter);

    let app = Router::new()
        .merge(routes::auth::router(state.rate_limiter.clone()))
        .merge(routes::users::router())
        .merge(routes::admin::router())
        .with_state(state);

    (app, pool)
}

/// Build an Axum app with custom rate‑limit parameters, wrapped in a
/// [`TestServer`].
///
/// Useful for testing that a low limit correctly triggers `429` responses.
pub async fn test_app_with_limits(
    auth_rate_limit_max: u64,
    auth_rate_limit_window_secs: u64,
) -> (TestServer, PgPool) {
    let pool = test_pool().await;
    let config = Config {
        auth_rate_limit_max,
        auth_rate_limit_window_secs,
        ..test_config()
    };

    let oauth_store = Arc::new(
        StateStore::new(&config.valkey_url).expect("failed to create oauth state store for tests"),
    );
    let rate_limiter = RateLimiter::new(
        &config.valkey_url,
        config.auth_rate_limit_max,
        config.auth_rate_limit_window_secs,
    )
    .expect("failed to create rate limiter for tests");
    let user = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    let auth = AuthService::new(user.clone(), token.clone(), config.clone());
    let state = AppState::new(auth, user, token, config, oauth_store, rate_limiter);

    let app = Router::new()
        .merge(routes::auth::router(state.rate_limiter.clone()))
        .merge(routes::users::router())
        .merge(routes::admin::router())
        .with_state(state);

    (TestServer::new(app), pool)
}

/// Spin up a `TestServer` backed by a real test database.
pub async fn test_server() -> (TestServer, PgPool) {
    let (app, pool) = test_app().await;
    (TestServer::new(app), pool)
}

use std::sync::atomic::AtomicU16;

static IP_COUNTER: AtomicU16 = AtomicU16::new(0);

/// Generate a unique IP address so parallel rate‑limit tests never collide.
pub fn unique_ip() -> String {
    let n = IP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("10.0.{}.{}", n >> 8, n & 0xff)
}

/// Generate a unique email so parallel tests never collide on the unique constraint.
pub fn unique_email(prefix: &str) -> String {
    format!("{}+{}@test.com", prefix, uuid::Uuid::now_v7())
}

/// Generate a unique username (3-32 chars).
pub fn unique_username(prefix: &str) -> String {
    // Use 16 hex chars from the UUID (64 bits of randomness) — enough to avoid
    // within-millisecond collisions across concurrent tests.
    let suffix = &uuid::Uuid::now_v7().simple().to_string()[..16];
    format!("{prefix}{suffix}")
}

/// Register a user through the API and return `(access_token, refresh_token)`.
pub async fn register_user(
    server: &TestServer,
    email: &str,
    username: &str,
    password: &str,
) -> (String, String) {
    let body: serde_json::Value = server
        .post("/auth/register")
        .json(&serde_json::json!({
            "email":    email,
            "username": username,
            "password": password,
        }))
        .await
        .json();

    (
        body["access_token"].as_str().unwrap().to_owned(),
        body["refresh_token"].as_str().unwrap().to_owned(),
    )
}

/// Promote a user to admin directly in the DB, then return a fresh token
/// that reflects the new role.
pub async fn make_admin(server: &TestServer, pool: &PgPool, email: &str, password: &str) -> String {
    sqlx::query!("UPDATE users SET role = 'admin' WHERE email = $1", email)
        .execute(pool)
        .await
        .unwrap();

    server
        .post("/auth/login")
        .json(&serde_json::json!({ "email": email, "password": password }))
        .await
        .json::<serde_json::Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_owned()
}
