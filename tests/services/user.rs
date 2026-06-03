use ama::{errors::AppError, models::Role, services::user::UserService};

use crate::common::{test_pool, unique_email, unique_username};

#[tokio::test]
async fn create_returns_user_with_default_role() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let user = svc
        .create(&unique_email("cu"), &unique_username("cu"), "password123")
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
    svc.create(&email, &unique_username("a"), "password123")
        .await
        .unwrap();
    let err = svc
        .create(&email, &unique_username("b"), "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::EmailTaken));
}

#[tokio::test]
async fn create_duplicate_username_returns_username_taken() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let username = unique_username("dup");
    svc.create(&unique_email("a"), &username, "password123")
        .await
        .unwrap();
    let err = svc
        .create(&unique_email("b"), &username, "password123")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::UsernameTaken));
}

#[tokio::test]
async fn find_by_id_returns_user() {
    let pool = test_pool().await;
    let svc = UserService::new(pool);
    let user = svc
        .create(&unique_email("fbi"), &unique_username("fbi"), "password123")
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
    svc.create(&email, &unique_username("fbe"), "password123")
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
    let user = svc
        .create(&unique_email("cp"), &unique_username("cp"), "old-pass")
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
    let user = svc
        .create(&unique_email("cpw"), &unique_username("cpw"), "correct")
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
    let user = svc
        .create(&unique_email("ur"), &unique_username("ur"), "password123")
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
    let user = svc
        .create(&unique_email("da"), &unique_username("da"), "password123")
        .await
        .unwrap();
    svc.deactivate(user.id).await.unwrap();
    let found = svc.find_by_id(user.id).await.unwrap();
    assert!(!found.is_active);
}

