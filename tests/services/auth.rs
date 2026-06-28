use socle::{
    config::OAuthProvider,
    errors::AppError,
    services::{auth::AuthService, oauth::OAuthProfile, token::TokenService, user::UserService},
};

use crate::common::{test_config, test_pool, unique_email, unique_username};

async fn auth() -> AuthService {
    let pool = test_pool().await;
    let config = test_config();
    let user = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    AuthService::new(user, token, config)
}

#[tokio::test]
async fn register_creates_user_with_tokens() {
    let svc = auth().await;
    let resp = svc
        .register(&unique_email("reg"), &unique_username("reg"), "password123")
        .await
        .unwrap();
    assert!(!resp.access_token.is_empty());
    assert!(!resp.refresh_token.is_empty());
}

#[tokio::test]
async fn register_short_password_is_rejected() {
    let svc = auth().await;
    let err = svc
        .register(&unique_email("sp"), &unique_username("sp"), "short")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidCredentials));
}

#[tokio::test]
async fn register_short_username_is_rejected() {
    let svc = auth().await;
    let err = svc
        .register(&unique_email("su"), "ab", "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidCredentials));
}

#[tokio::test]
async fn login_with_correct_credentials_returns_tokens() {
    let svc = auth().await;
    let email = unique_email("li");
    svc.register(&email, &unique_username("li"), "password123")
        .await
        .unwrap();
    let resp = svc.login(&email, "password123").await.unwrap();
    assert!(!resp.access_token.is_empty());
}

#[tokio::test]
async fn login_wrong_password_returns_invalid_credentials() {
    let svc = auth().await;
    let email = unique_email("lw");
    svc.register(&email, &unique_username("lw"), "password123")
        .await
        .unwrap();
    let err = svc.login(&email, "wrong").await.unwrap_err();
    assert!(matches!(err, AppError::InvalidCredentials));
}

#[tokio::test]
async fn login_unknown_email_returns_invalid_credentials() {
    let svc = auth().await;
    let err = svc
        .login("nobody@unknown.com", "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidCredentials));
}

#[tokio::test]
async fn login_disabled_account_returns_account_disabled() {
    let pool = test_pool().await;
    let config = test_config();
    let user_svc = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    let svc = AuthService::new(user_svc.clone(), token, config);

    let email = unique_email("dis");
    let username = unique_username("dis");
    let password = "password123";
    let (user, _) = user_svc
        .create(
            &email,
            email.split('@').next().unwrap(),
            &username,
            password,
        )
        .await
        .unwrap();
    user_svc.deactivate(user.id).await.unwrap();

    let err = svc.login(&email, "password123").await.unwrap_err();
    assert!(matches!(err, AppError::AccountDisabled));
}

#[tokio::test]
async fn refresh_returns_new_token_pair() {
    let svc = auth().await;
    let email = unique_email("rf");
    let resp1 = svc
        .register(&email, &unique_username("rf"), "password123")
        .await
        .unwrap();
    let resp2 = svc.refresh(&resp1.refresh_token).await.unwrap();
    assert_ne!(resp1.refresh_token, resp2.refresh_token);
    assert!(!resp2.access_token.is_empty());
}

#[tokio::test]
async fn refresh_token_cannot_be_reused() {
    let svc = auth().await;
    let email = unique_email("rr");
    let resp = svc
        .register(&email, &unique_username("rr"), "password123")
        .await
        .unwrap();

    svc.refresh(&resp.refresh_token).await.unwrap();
    let err = svc.refresh(&resp.refresh_token).await.unwrap_err();
    assert!(matches!(err, AppError::RefreshTokenInvalid));
}

#[tokio::test]
async fn logout_revokes_all_tokens() {
    let svc = auth().await;
    let email = unique_email("lo");
    let resp = svc
        .register(&email, &unique_username("lo"), "password123")
        .await
        .unwrap();

    svc.logout(resp.user.id).await.unwrap();

    let err = svc.refresh(&resp.refresh_token).await.unwrap_err();
    assert!(matches!(err, AppError::RefreshTokenInvalid));
}

#[tokio::test]
async fn login_or_register_oauth_creates_user_and_returns_tokens() {
    let svc = auth().await;
    let email = unique_email("oa");
    let profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-new-1".into(),
        email: email.clone(),
        display_name: Some("OAuth User".into()),
        avatar_url: None,
    };

    let resp = svc.login_or_register_oauth(&profile).await.unwrap();

    assert!(!resp.access_token.is_empty());
    assert!(!resp.refresh_token.is_empty());
    assert_eq!(resp.user.email, email);
    assert!(!resp.user.has_local_credential);
}

#[tokio::test]
async fn login_or_register_oauth_links_existing_user_by_email() {
    let svc = auth().await;
    let email = unique_email("link");
    svc.register(&email, &unique_username("link"), "password123")
        .await
        .unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-link-1".into(),
        email: email.clone(),
        display_name: None,
        avatar_url: None,
    };

    let resp = svc.login_or_register_oauth(&profile).await.unwrap();
    assert_eq!(resp.user.email, email);
    assert!(resp.user.has_local_credential);
}

#[tokio::test]
async fn login_or_register_oauth_returns_tokens_for_existing_link() {
    let svc = auth().await;
    let email = unique_email("existing");
    let profile = OAuthProfile {
        provider: OAuthProvider::GitHub,
        provider_user_id: "github-existing-1".into(),
        email: email.clone(),
        display_name: Some("Existing".into()),
        avatar_url: None,
    };

    svc.login_or_register_oauth(&profile).await.unwrap();

    let resp = svc.login_or_register_oauth(&profile).await.unwrap();
    assert_eq!(resp.user.email, email);
    assert!(!resp.access_token.is_empty());
}

#[tokio::test]
async fn login_or_register_oauth_rejects_disabled_account() {
    let pool = test_pool().await;
    let config = test_config();
    let user_svc = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    let svc = AuthService::new(user_svc.clone(), token, config);
    let email = unique_email("dis-oa");

    let username = unique_username("dis-oa");
    let (created, _) = user_svc
        .create(
            &email,
            email.split('@').next().unwrap(),
            &username,
            "password123",
        )
        .await
        .unwrap();
    user_svc.deactivate(created.id).await.unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::GitHub,
        provider_user_id: "github-disabled-1".into(),
        email: email.clone(),
        display_name: None,
        avatar_url: None,
    };

    let err = svc.login_or_register_oauth(&profile).await.unwrap_err();
    assert!(matches!(err, AppError::AccountDisabled));
}
