use serde_json::json;

use crate::common::{register_user, test_server, unique_email, unique_username};

#[tokio::test]
async fn get_me_returns_own_profile() {
    let (s, _) = test_server().await;
    let email = unique_email("gm");
    let username = unique_username("gm");
    let (token, _) = register_user(&s, &email, &username, "password123").await;

    let body: serde_json::Value = s.get("/users/me").authorization_bearer(&token).await.json();
    assert_eq!(body["email"], email);
    assert_eq!(body["username"], username);
    assert_eq!(body["role"], "user");
    assert!(body["created_at"].is_string());
}

#[tokio::test]
async fn get_me_without_token_returns_401_missing_auth_header() {
    let (s, _) = test_server().await;
    let res = s.get("/users/me").await;
    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "MISSING_AUTH_HEADER"
    );
}

#[tokio::test]
async fn get_me_with_invalid_token_returns_401_token_invalid() {
    let (s, _) = test_server().await;
    let res = s.get("/users/me").authorization_bearer("not.a.token").await;
    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "TOKEN_INVALID"
    );
}

#[tokio::test]
async fn change_password_succeeds_and_new_password_works() {
    let (s, _) = test_server().await;
    let email = unique_email("cp");
    let (token, _) = register_user(&s, &email, &unique_username("cp"), "old-pass").await;

    s.put("/users/me/password")
        .authorization_bearer(&token)
        .json(&json!({ "current_password": "old-pass", "new_password": "new-pass123" }))
        .await
        .assert_status_success();

    s.post("/auth/login")
        .json(&json!({ "email": email, "password": "new-pass123" }))
        .await
        .assert_status_success();
}

#[tokio::test]
async fn change_password_wrong_current_returns_401() {
    let (s, _) = test_server().await;
    let (token, _) =
        register_user(&s, &unique_email("cpw"), &unique_username("cpw"), "correct1").await;

    let res = s
        .put("/users/me/password")
        .authorization_bearer(&token)
        .json(&json!({ "current_password": "wrong", "new_password": "new-pass123" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "INVALID_CREDENTIALS"
    );
}

#[tokio::test]
async fn change_password_invalidates_existing_refresh_token() {
    let (s, _) = test_server().await;
    let (token, refresh_token) = register_user(
        &s,
        &unique_email("cpi"),
        &unique_username("cpi"),
        "old-pass",
    )
    .await;

    s.put("/users/me/password")
        .authorization_bearer(&token)
        .json(&json!({ "current_password": "old-pass", "new_password": "new-pass123" }))
        .await
        .assert_status_success();

    let res = s
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": refresh_token }))
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}

#[tokio::test]
async fn deactivate_prevents_login_with_account_disabled_code() {
    let (s, _) = test_server().await;
    let email = unique_email("da");
    let (token, _) = register_user(&s, &email, &unique_username("da"), "password123").await;

    s.delete("/users/me")
        .authorization_bearer(&token)
        .await
        .assert_status_success();

    let res = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await;
    assert_eq!(res.status_code().as_u16(), 403);
    // Must be ACCOUNT_DISABLED, not INVALID_CREDENTIALS.
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "ACCOUNT_DISABLED"
    );
}

#[tokio::test]
async fn deactivate_revokes_all_refresh_tokens() {
    let (s, _) = test_server().await;
    let (token, refresh_token) = register_user(
        &s,
        &unique_email("drt"),
        &unique_username("drt"),
        "password123",
    )
    .await;

    s.delete("/users/me")
        .authorization_bearer(&token)
        .await
        .assert_status_success();

    let res = s
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": refresh_token }))
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}

#[tokio::test]
async fn guest_role_cannot_access_user_endpoints() {
    let (s, pool) = test_server().await;
    let email = unique_email("guest");
    let (_, _) = register_user(&s, &email, &unique_username("guest"), "password123").await;

    sqlx::query!("UPDATE users SET role = 'guest' WHERE email = $1", email)
        .execute(&pool)
        .await
        .unwrap();

    let guest_token = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json::<serde_json::Value>()["access_token"]
        .as_str()
        .unwrap()
        .to_owned();

    let res = s.get("/users/me").authorization_bearer(&guest_token).await;
    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}
