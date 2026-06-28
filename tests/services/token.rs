use socle::{
    config::{Config, OAuthConfig},
    errors::AppError,
    models::{AuthMethod, Role},
    services::token::{TokenService, hash_password, hash_refresh_token, verify_password},
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

use crate::common::{test_config, test_pool, unique_email, unique_username};

fn svc() -> TokenService {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://localhost/unused")
        .unwrap();

    TokenService::new(
        pool,
        Config {
            database_url: String::new(),
            jwt_secret: "unit-test-secret-long-enough-32xx".to_owned(),
            valkey_url: String::new(),
            access_token_expiry_secs: 3600,
            refresh_token_expiry_secs: 86_400,
            bind_addr: String::new(),
            shutdown_timeout_secs: 30,
            oauth: OAuthConfig::default(),
        },
    )
}

#[tokio::test]
async fn generated_token_validates_successfully() {
    let svc = svc();
    let id = Uuid::now_v7();
    let tok = svc
        .generate_access_token(id, "a@test.com", "alice", Role::User, AuthMethod::Password)
        .unwrap();
    let claims = svc.validate_access_token(&tok).unwrap();
    assert_eq!(claims.sub, id);
    assert_eq!(claims.email, "a@test.com");
    assert_eq!(claims.display_name, "alice");
    assert_eq!(claims.role, Role::User);
}

#[tokio::test]
async fn tampered_signature_is_rejected() {
    let svc = svc();
    let mut tok = svc
        .generate_access_token(
            Uuid::now_v7(),
            "a@test.com",
            "alice",
            Role::User,
            AuthMethod::Password,
        )
        .unwrap();
    let last = tok.pop().unwrap();
    tok.push(if last == 'a' { 'b' } else { 'a' });

    assert!(matches!(
        svc.validate_access_token(&tok),
        Err(AppError::TokenInvalid)
    ));
}

#[tokio::test]
async fn token_signed_with_different_secret_is_rejected() {
    let svc = svc();
    let tok = svc
        .generate_access_token(
            Uuid::now_v7(),
            "a@test.com",
            "alice",
            Role::User,
            AuthMethod::Password,
        )
        .unwrap();

    let other = TokenService::new(
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        Config {
            jwt_secret: "completely-different-secret-32xxx".to_owned(),
            database_url: String::new(),
            valkey_url: String::new(),
            access_token_expiry_secs: 3600,
            refresh_token_expiry_secs: 86_400,
            bind_addr: String::new(),
            shutdown_timeout_secs: 30,
            oauth: OAuthConfig::default(),
        },
    );

    assert!(matches!(
        other.validate_access_token(&tok),
        Err(AppError::TokenInvalid)
    ));
}

#[tokio::test]
async fn role_is_round_tripped_through_token() {
    let svc = svc();
    for role in [Role::Guest, Role::User, Role::Admin] {
        let tok = svc
            .generate_access_token(
                Uuid::now_v7(),
                "x@test.com",
                "x",
                role,
                AuthMethod::Password,
            )
            .unwrap();
        let claims = svc.validate_access_token(&tok).unwrap();
        assert_eq!(claims.role, role);
    }
}

#[test]
fn correct_password_verifies() {
    let hash = hash_password("correct-horse").unwrap();
    assert!(verify_password("correct-horse", &hash).unwrap());
}

#[test]
fn wrong_password_does_not_verify() {
    let hash = hash_password("correct-horse").unwrap();
    assert!(!verify_password("wrong-horse", &hash).unwrap());
}

#[test]
fn identical_passwords_produce_different_hashes() {
    let h1 = hash_password("same").unwrap();
    let h2 = hash_password("same").unwrap();
    assert_ne!(h1, h2, "Argon2 must salt each hash independently");
}

#[test]
fn identical_tokens_produce_different_hashes() {
    let token = "some-raw-token-value-123";
    let h1 = hash_refresh_token(token);
    let h2 = hash_refresh_token(token);
    assert_ne!(
        h1, h2,
        "each refresh token hash must use a fresh random salt"
    );
}

use socle::services::user::UserService;

async fn create_user_for_token_tests() -> uuid::Uuid {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("tt");
    let (user, _) = svc
        .create(
            &email,
            email.split('@').next().unwrap(),
            &unique_username("tt"),
            "password123",
        )
        .await
        .unwrap();
    user.id
}

async fn db_svc() -> (TokenService, socle::config::Config) {
    let pool = test_pool().await;
    let config = test_config();
    (TokenService::new(pool, config.clone()), config)
}

#[tokio::test]
async fn create_refresh_token_returns_token_string() {
    let user_id = create_user_for_token_tests().await;
    let (svc, _) = db_svc().await;

    let token = svc.create_refresh_token(user_id).await.unwrap();

    assert!(!token.is_empty());
    assert_eq!(token.len(), 64);
}

#[tokio::test]
async fn rotate_refresh_token_returns_new_pair() {
    let user_id = create_user_for_token_tests().await;
    let (svc, _) = db_svc().await;

    let old_token = svc.create_refresh_token(user_id).await.unwrap();
    let (new_token, returned_id) = svc.rotate_refresh_token(&old_token).await.unwrap();

    assert_ne!(new_token, old_token);
    assert_eq!(returned_id, user_id);
}

#[tokio::test]
async fn rotate_refresh_token_cannot_be_reused() {
    let user_id = create_user_for_token_tests().await;
    let (svc, _) = db_svc().await;

    let token = svc.create_refresh_token(user_id).await.unwrap();
    svc.rotate_refresh_token(&token).await.unwrap();

    let err = svc.rotate_refresh_token(&token).await.unwrap_err();
    assert!(matches!(err, AppError::RefreshTokenInvalid));
}

#[tokio::test]
async fn revoke_all_user_tokens_prevents_refresh() {
    let user_id = create_user_for_token_tests().await;
    let (svc, _) = db_svc().await;

    let token = svc.create_refresh_token(user_id).await.unwrap();
    svc.revoke_all_user_tokens(user_id).await.unwrap();

    let err = svc.rotate_refresh_token(&token).await.unwrap_err();
    assert!(matches!(err, AppError::RefreshTokenInvalid));
}

#[tokio::test]
async fn rotate_refresh_invalid_token_returns_error() {
    let _user_id = create_user_for_token_tests().await;
    let (svc, _) = db_svc().await;

    let err = svc
        .rotate_refresh_token("this-is-not-a-valid-token")
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::RefreshTokenInvalid));
}
