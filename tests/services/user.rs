use yaima::{
    config::OAuthProvider,
    errors::AppError,
    models::Role,
    services::{oauth::OAuthProfile, user::UserService},
};

use crate::common::{test_pool, unique_email, unique_username};

fn email_display(email: &str) -> &str {
    email.split('@').next().unwrap_or("user")
}

#[tokio::test]
async fn create_returns_user_with_default_role() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cu");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("cu"), "password123")
        .await
        .unwrap();
    assert_eq!(user.role, Role::User);
    assert!(user.is_active);
}

#[tokio::test]
async fn create_duplicate_email_returns_email_taken() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("dup");
    let a_username = unique_username("a");
    svc.create(&email, email_display(&email), &a_username, "password123")
        .await
        .unwrap();
    let err = svc
        .create(&email, email_display(&email), &unique_username("b"), "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::EmailTaken));
}

#[tokio::test]
async fn create_duplicate_username_returns_username_taken() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let username = unique_username("dup");
    let email_a = unique_email("a");
    svc.create(&email_a, email_display(&email_a), &username, "password123")
        .await
        .unwrap();
    let err = svc
        .create(&unique_email("b"), email_display(&unique_email("b")), &username, "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::UsernameTaken));
}

#[tokio::test]
async fn find_by_id_returns_user() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("fbi");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("fbi"), "password123")
        .await
        .unwrap();
    let found = svc.find_by_id(user.id).await.unwrap();
    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn find_by_id_unknown_returns_not_found() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let err = svc.find_by_id(uuid::Uuid::now_v7()).await.unwrap_err();
    assert!(matches!(err, AppError::UserNotFound));
}

#[tokio::test]
async fn find_by_email_returns_some_for_existing_user() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("fbe");
    svc.create(&email, email_display(&email), &unique_username("fbe"), "password123")
        .await
        .unwrap();
    assert!(svc.find_by_email(&email).await.unwrap().is_some());
}

#[tokio::test]
async fn find_by_email_returns_none_for_unknown_email() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    assert!(
        svc.find_by_email("nobody@unknown.com")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn change_password_succeeds_with_correct_current() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cp");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("cp"), "old-pass")
        .await
        .unwrap();
    svc.change_password(user.id, "old-pass", "new-pass123")
        .await
        .unwrap();
}

#[tokio::test]
async fn change_password_rejects_wrong_current() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cpw");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("cpw"), "correct")
        .await
        .unwrap();
    let err = svc
        .change_password(user.id, "wrong", "new-pass123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::InvalidCredentials));
}

#[tokio::test]
async fn update_role_persists_new_role() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("ur");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("ur"), "password123")
        .await
        .unwrap();
    let updated = svc.update_role(user.id, Role::Admin).await.unwrap();
    assert_eq!(updated.role, Role::Admin);
}

#[tokio::test]
async fn update_role_unknown_user_returns_not_found() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let err = svc
        .update_role(uuid::Uuid::now_v7(), Role::Admin)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::UserNotFound));
}

#[tokio::test]
async fn deactivate_sets_is_active_false() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("da");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("da"), "password123")
        .await
        .unwrap();
    svc.deactivate(user.id).await.unwrap();
    let found = svc.find_by_id(user.id).await.unwrap();
    assert!(!found.is_active);
}

#[tokio::test]
async fn find_local_credential_returns_some_for_local_user() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("flc");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("flc"), "password123")
        .await
        .unwrap();

    let credential = svc
        .find_local_credential(user.id)
        .await
        .unwrap()
        .expect("local user should have a credential");

    assert_eq!(credential.user_id, user.id);
}

#[tokio::test]
async fn find_local_credential_returns_none_for_nonexistent_user() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);

    let credential = svc
        .find_local_credential(uuid::Uuid::now_v7())
        .await
        .unwrap();

    assert!(credential.is_none());
}

#[tokio::test]
async fn create_from_oauth_creates_user_without_local_credential() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cfo");

    let user = svc
        .create_from_oauth(&email, Some("OAuth Display Name"))
        .await
        .unwrap();

    assert_eq!(user.email, email);
    assert_eq!(user.display_name, "OAuth Display Name");
    assert_eq!(user.role, Role::User);
    assert!(user.is_active);

    let credential = svc.find_local_credential(user.id).await.unwrap();
    assert!(credential.is_none());
}

#[tokio::test]
async fn create_from_oauth_uses_email_prefix_when_display_name_not_provided() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cfo-nn");

    let user = svc.create_from_oauth(&email, None).await.unwrap();

    let expected_prefix = email.split('@').next().unwrap();
    assert_eq!(user.display_name, expected_prefix);
}

#[tokio::test]
async fn create_from_oauth_duplicate_email_returns_email_taken() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("cfo-dup");

    svc.create_from_oauth(&email, Some("First"))
        .await
        .unwrap();

    let err = svc
        .create_from_oauth(&email, Some("Second"))
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::EmailTaken));
}

#[tokio::test]
async fn find_by_oauth_identity_returns_none_when_not_linked() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);

    let result = svc
        .find_by_oauth_identity(OAuthProvider::Google, "nonexistent-google-id")
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn find_by_oauth_identity_returns_user_when_linked() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("fboi");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("fboi"), "password123")
        .await
        .unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-linked-id".into(),
        email: email.clone(),
        display_name: Some("Linked".into()),
        avatar_url: None,
    };
    svc.link_oauth_account(user.id, &profile).await.unwrap();

    let found = svc
        .find_by_oauth_identity(OAuthProvider::Google, "google-linked-id")
        .await
        .unwrap()
        .expect("should find user by oauth identity");

    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn link_oauth_account_links_successfully() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("loa");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("loa"), "password123")
        .await
        .unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::GitHub,
        provider_user_id: "github-link-1".into(),
        email: unique_email("loa-p"),
        display_name: None,
        avatar_url: Some("https://avatars.example.com/1".into()),
    };

    svc.link_oauth_account(user.id, &profile).await.unwrap();

    let found = svc
        .find_by_oauth_identity(OAuthProvider::GitHub, "github-link-1")
        .await
        .unwrap()
        .expect("should find linked user");

    assert_eq!(found.id, user.id);
}

#[tokio::test]
async fn link_oauth_account_is_idempotent() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("loai");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("loai"), "password123")
        .await
        .unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-idempotent-1".into(),
        email: unique_email("loai-p"),
        display_name: None,
        avatar_url: None,
    };

    svc.link_oauth_account(user.id, &profile).await.unwrap();
    svc.link_oauth_account(user.id, &profile).await.unwrap();

    let connections = svc.list_oauth_connections(user.id).await.unwrap();
    assert_eq!(connections.len(), 1);
}

#[tokio::test]
async fn list_oauth_connections_returns_empty_list_initially() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("loce");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("loce"), "password123")
        .await
        .unwrap();

    let connections = svc.list_oauth_connections(user.id).await.unwrap();
    assert!(connections.is_empty());
}

#[tokio::test]
async fn list_oauth_connections_returns_linked_providers() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("locl");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("locl"), "password123")
        .await
        .unwrap();

    let google_profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-list-1".into(),
        email: email.clone(),
        display_name: None,
        avatar_url: None,
    };
    let github_profile = OAuthProfile {
        provider: OAuthProvider::GitHub,
        provider_user_id: "github-list-1".into(),
        email: email.clone(),
        display_name: None,
        avatar_url: None,
    };

    svc.link_oauth_account(user.id, &google_profile)
        .await
        .unwrap();
    svc.link_oauth_account(user.id, &github_profile)
        .await
        .unwrap();

    let connections = svc.list_oauth_connections(user.id).await.unwrap();
    assert_eq!(connections.len(), 2);
    let provider_names: Vec<&str> = connections
        .iter()
        .map(|c| match c.provider {
            yaima::models::Provider::Google => "google",
            yaima::models::Provider::GitHub => "github",
        })
        .collect();
    assert!(provider_names.contains(&"google"));
    assert!(provider_names.contains(&"github"));
}

#[tokio::test]
async fn unlink_oauth_account_removes_link() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("uoa");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("uoa"), "password123")
        .await
        .unwrap();

    let profile = OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: "google-unlink-1".into(),
        email: email.clone(),
        display_name: None,
        avatar_url: None,
    };
    svc.link_oauth_account(user.id, &profile).await.unwrap();

    svc.unlink_oauth_account(user.id, OAuthProvider::Google)
        .await
        .unwrap();

    let connections = svc.list_oauth_connections(user.id).await.unwrap();
    assert!(connections.is_empty());
}

#[tokio::test]
async fn unlink_oauth_account_returns_not_found_for_missing_link() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let email = unique_email("uoa-nf");
    let (user, _) = svc
        .create(&email, email_display(&email), &unique_username("uoa-nf"), "password123")
        .await
        .unwrap();

    let err = svc
        .unlink_oauth_account(user.id, OAuthProvider::GitHub)
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::UserNotFound));
}
