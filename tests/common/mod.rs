#![allow(dead_code)]
use axum::Router;
use axum_test::TestServer;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

use yaima::{
    config::Config,
    routes,
    services::{auth::AuthService, token::TokenService, user::UserService},
    state::AppState,
};


/// Connect to the test database and run all pending migrations.
pub async fn test_pool() -> PgPool {
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

    pool
}

/// Minimal config suitable for tests.
pub fn test_config() -> Config {
    Config {
        database_url: String::new(),
        jwt_secret: "test-secret-that-is-long-enough-32+".to_owned(),
        access_token_expiry_secs: 3600,
        refresh_token_expiry_secs: 86_400,
        bind_addr: "0.0.0.0:0".to_owned(),
    }
}


/// Build the full Axum app wired to a real test database.
pub async fn test_app() -> (Router, PgPool) {
    let pool = test_pool().await;
    let config = test_config();

    let user_svc = UserService::new(pool.clone());
    let token_svc = TokenService::new(pool.clone(), config.clone());
    let auth_svc = AuthService::new(user_svc.clone(), token_svc.clone(), config.clone());
    let state = AppState::new(auth_svc, user_svc, token_svc);

    let app = Router::new()
        .merge(routes::auth::router())
        .merge(routes::users::router())
        .merge(routes::admin::router())
        .with_state(state);

    (app, pool)
}

/// Spin up a `TestServer` backed by a real test database.
pub async fn test_server() -> (TestServer, PgPool) {
    let (app, pool) = test_app().await;
    (TestServer::new(app), pool)
}

/// Generate a unique email so parallel tests never collide on the unique constraint.
pub fn unique_email(prefix: &str) -> String {
    format!("{}+{}@test.com", prefix, uuid::Uuid::now_v7())
}

/// Generate a unique username.
pub fn unique_username(prefix: &str) -> String {
    // Keep within the 3-32 char limit; take 8 hex chars from the UUID.
    let suffix = &uuid::Uuid::now_v7().simple().to_string()[..8];
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
