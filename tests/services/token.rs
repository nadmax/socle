use ama::{
    config::Config,
    errors::AppError,
    models::Role,
    services::token::{TokenService, hash_password, verify_password},
};
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

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
            access_token_expiry_secs: 3600,
            refresh_token_expiry_secs: 86_400,
            bind_addr: String::new(),
        },
    )
}

#[test]
fn generated_token_validates_successfully() {
    let svc = svc();
    let id = Uuid::now_v7();
    let tok = svc
        .generate_access_token(id, "a@test.com", "alice", Role::User)
        .unwrap();
    let claims = svc.validate_access_token(&tok).unwrap();
    assert_eq!(claims.sub, id);
    assert_eq!(claims.email, "a@test.com");
    assert_eq!(claims.username, "alice");
    assert_eq!(claims.role, Role::User);
}

#[test]
fn tampered_signature_is_rejected() {
    let svc = svc();
    let mut tok = svc
        .generate_access_token(Uuid::now_v7(), "a@test.com", "alice", Role::User)
        .unwrap();
    let last = tok.pop().unwrap();
    tok.push(if last == 'a' { 'b' } else { 'a' });

    assert!(matches!(
        svc.validate_access_token(&tok),
        Err(AppError::TokenInvalid)
    ));
}

#[test]
fn token_signed_with_different_secret_is_rejected() {
    let svc = svc();
    let tok = svc
        .generate_access_token(Uuid::now_v7(), "a@test.com", "alice", Role::User)
        .unwrap();

    let other = TokenService::new(
        PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        Config {
            jwt_secret: "completely-different-secret-32xxx".to_owned(),
            database_url: String::new(),
            access_token_expiry_secs: 3600,
            refresh_token_expiry_secs: 86_400,
            bind_addr: String::new(),
        },
    );

    assert!(matches!(
        other.validate_access_token(&tok),
        Err(AppError::TokenInvalid)
    ));
}

#[test]
fn role_is_round_tripped_through_token() {
    let svc = svc();
    for role in [Role::Guest, Role::User, Role::Admin] {
        let tok = svc
            .generate_access_token(Uuid::now_v7(), "x@test.com", "x", role)
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

