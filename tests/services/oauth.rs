use std::sync::Arc;

use yaima::{
    config::{OAuthProvider, OAuthProviderConfig},
    errors::OAuthError,
    services::oauth::{AuthorizationRequest, StateStore, build_authorization_url},
};

use crate::common::test_config;

fn test_store() -> Arc<StateStore> {
    let config = test_config();
    Arc::new(StateStore::new(&config.valkey_url).unwrap())
}

fn google_cfg() -> OAuthProviderConfig {
    OAuthProviderConfig {
        client_id: "test-client-id".into(),
        client_secret: "test-client-secret".into(),
        redirect_uri: "http://localhost:8080/auth/google/callback".into(),
    }
}

#[test]
fn state_store_new_returns_error_for_invalid_url() {
    let err = StateStore::new("!not-a-url!");
    assert!(err.is_err());
}

#[test]
fn state_store_new_succeeds_for_valid_url() {
    let config = test_config();
    let store = StateStore::new(&config.valkey_url);
    assert!(store.is_ok());
}

#[tokio::test]
async fn state_store_insert_take_round_trip() {
    let store = test_store();
    let verifier = oauth2::PkceCodeVerifier::new("test-verifier-secret-12345".into());

    store
        .insert("test-state-key-1", OAuthProvider::Google, verifier)
        .await
        .unwrap();

    let taken = store
        .take("test-state-key-1", OAuthProvider::Google)
        .await
        .unwrap();

    assert_eq!(taken.secret(), "test-verifier-secret-12345");
}

#[tokio::test]
async fn state_store_take_returns_invalid_state_for_unknown_key() {
    let store = test_store();

    let err = store
        .take("non-existent-key", OAuthProvider::Google)
        .await
        .unwrap_err();

    assert!(matches!(err, OAuthError::InvalidState));
}

#[tokio::test]
async fn state_store_take_rejects_wrong_provider() {
    let store = test_store();
    let verifier = oauth2::PkceCodeVerifier::new("test-verifier-wp".into());

    store
        .insert("test-state-key-wp", OAuthProvider::Google, verifier)
        .await
        .unwrap();

    let err = store
        .take("test-state-key-wp", OAuthProvider::GitHub)
        .await
        .unwrap_err();

    assert!(matches!(err, OAuthError::ProviderMismatch { .. }));
}

#[tokio::test]
async fn state_store_take_consumes_key_atomically() {
    let store = test_store();
    let verifier = oauth2::PkceCodeVerifier::new("test-verifier-at".into());

    store
        .insert("test-state-key-at", OAuthProvider::Google, verifier)
        .await
        .unwrap();

    store
        .take("test-state-key-at", OAuthProvider::Google)
        .await
        .unwrap();

    let err = store
        .take("test-state-key-at", OAuthProvider::Google)
        .await
        .unwrap_err();

    assert!(matches!(err, OAuthError::InvalidState));
}

#[tokio::test]
async fn build_authorization_url_returns_valid_google_url() {
    let store = test_store();
    let cfg = google_cfg();

    let req: AuthorizationRequest = build_authorization_url(OAuthProvider::Google, &cfg, &store)
        .await
        .unwrap();

    let url_str = req.url.to_string();
    assert!(url_str.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(url_str.contains("client_id=test-client-id"));
    assert!(
        url_str.contains("redirect_uri=http%3A%2F%2Flocalhost%3A8080%2Fauth%2Fgoogle%2Fcallback")
    );
    assert!(url_str.contains("code_challenge"));
    assert!(url_str.contains("state="));
}

#[tokio::test]
async fn build_authorization_url_returns_valid_github_url() {
    let store = test_store();
    let cfg = OAuthProviderConfig {
        client_id: "gh-client".into(),
        client_secret: "gh-secret".into(),
        redirect_uri: "http://localhost:8080/auth/github/callback".into(),
    };

    let req = build_authorization_url(OAuthProvider::GitHub, &cfg, &store)
        .await
        .unwrap();

    let url_str = req.url.to_string();
    assert!(url_str.starts_with("https://github.com/login/oauth/authorize"));
    assert!(url_str.contains("client_id=gh-client"));
}

#[test]
fn user_agent_is_not_placeholder() {
    let ua = yaima::services::oauth::APP_USER_AGENT;
    assert_ne!(
        ua, "your-app-name",
        "User-Agent must be set from Cargo package metadata, not a placeholder"
    );
    assert!(ua.contains('/'), "User-Agent should contain name/version");
}
