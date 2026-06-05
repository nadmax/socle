use yaima::{
    errors::AppError,
    models::Role,
    services::{admin::AdminService, user::UserService},
};

use crate::common::{test_pool, unique_email, unique_username};

#[tokio::test]
async fn update_user_role_promotes_to_admin() {
    let pool = test_pool().await;
    let user = UserService::new(pool);
    let user = user
        .create(&unique_email("pr"), &unique_username("pr"), "pass123")
        .await
        .unwrap();

    let admin_id = uuid::Uuid::now_v7();
    let svc = AdminService::new(&user);
    let updated = svc
        .update_user_role(admin_id, user.id, Role::Admin)
        .await
        .unwrap();

    assert_eq!(updated.role, Role::Admin);
}

#[tokio::test]
async fn update_user_role_demotes_to_guest() {
    let pool = test_pool().await;
    let user = UserService::new(pool);
    let user = user
        .create(&unique_email("dm"), &unique_username("dm"), "pass123")
        .await
        .unwrap();

    let admin_id = uuid::Uuid::now_v7();
    let svc = AdminService::new(&user);
    let updated = svc
        .update_user_role(admin_id, user.id, Role::Guest)
        .await
        .unwrap();

    assert_eq!(updated.role, Role::Guest);
}

#[tokio::test]
async fn self_role_change_returns_forbidden() {
    let pool = test_pool().await;
    let user = UserService::new(pool);
    let user = user
        .create(&unique_email("self"), &unique_username("self"), "pass123")
        .await
        .unwrap();

    let svc = AdminService::new(&user);
    let err = svc
        .update_user_role(user.id, user.id, Role::Guest)
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Forbidden));
}

#[tokio::test]
async fn unknown_target_returns_not_found() {
    let pool = test_pool().await;
    let user = UserService::new(pool);
    let svc = AdminService::new(&user);
    let err = svc
        .update_user_role(uuid::Uuid::now_v7(), uuid::Uuid::now_v7(), Role::Admin)
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::UserNotFound));
}
