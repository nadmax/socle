use std::sync::Arc;

use serde_json::json;

use yaima::services::oauth::StateStore;

use crate::common::{register_user, test_config, test_server, unique_email, unique_username};

#[tokio::test]
async fn register_returns_201_and_user_role() {
    let (s, _) = test_server().await;
    let res = s
        .post("/auth/register")
        .json(&json!({
            "email":    unique_email("r"),
            "username": unique_username("r"),
            "password": "password123",
        }))
        .await;

    assert_eq!(res.status_code().as_u16(), 201);
    let body: serde_json::Value = res.json();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert_eq!(body["user"]["role"], "user");
}

#[tokio::test]
async fn register_duplicate_email_returns_409_email_taken() {
    let (s, _) = test_server().await;
    let email = unique_email("de");
    s.post("/auth/register")
        .json(&json!({ "email": email, "username": unique_username("de1"), "password": "password123" }))
        .await;

    let res = s
        .post("/auth/register")
        .json(&json!({ "email": email, "username": unique_username("de2"), "password": "password123" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 409);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "EMAIL_TAKEN"
    );
}

#[tokio::test]
async fn register_duplicate_username_returns_409_username_taken() {
    let (s, _) = test_server().await;
    let username = unique_username("du");
    s.post("/auth/register")
        .json(&json!({ "email": unique_email("du1"), "username": username, "password": "password123" }))
        .await;

    let res = s
        .post("/auth/register")
        .json(&json!({ "email": unique_email("du2"), "username": username, "password": "password123" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 409);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "USERNAME_TAKEN"
    );
}

#[tokio::test]
async fn login_correct_credentials_returns_200() {
    let (s, _) = test_server().await;
    let email = unique_email("li");
    let username = unique_username("li");
    s.post("/auth/register")
        .json(&json!({ "email": email, "username": username, "password": "password123" }))
        .await;

    let res = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await;

    res.assert_status_success();
    assert!(res.json::<serde_json::Value>()["access_token"].is_string());
}

#[tokio::test]
async fn login_wrong_password_returns_401_invalid_credentials() {
    let (s, _) = test_server().await;
    let email = unique_email("lw");
    s.post("/auth/register")
        .json(&json!({ "email": email, "username": unique_username("lw"), "password": "password123" }))
        .await;

    let res = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "wrong" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "INVALID_CREDENTIALS"
    );
}

#[tokio::test]
async fn login_unknown_email_returns_401_same_code_as_wrong_password() {
    let (s, _) = test_server().await;
    let res = s
        .post("/auth/login")
        .json(&json!({ "email": "nobody@nowhere.com", "password": "password123" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "INVALID_CREDENTIALS"
    );
}

#[tokio::test]
async fn refresh_returns_new_token_pair() {
    let (s, _) = test_server().await;
    let email = unique_email("rf");
    let body: serde_json::Value = s
        .post("/auth/register")
        .json(&json!({ "email": email, "username": unique_username("rf"), "password": "password123" }))
        .await
        .json();
    let refresh_token = body["refresh_token"].as_str().unwrap().to_owned();

    let res = s
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": refresh_token }))
        .await;

    res.assert_status_success();
    let new_body: serde_json::Value = res.json();
    assert_ne!(new_body["refresh_token"].as_str().unwrap(), refresh_token);
}

#[tokio::test]
async fn refresh_token_single_use_enforced() {
    let (s, _) = test_server().await;
    let body: serde_json::Value = s
        .post("/auth/register")
        .json(&json!({ "email": unique_email("su"), "username": unique_username("su"), "password": "password123" }))
        .await
        .json();
    let rt = body["refresh_token"].as_str().unwrap();

    s.post("/auth/refresh")
        .json(&json!({ "refresh_token": rt }))
        .await
        .assert_status_success();

    let res = s
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": rt }))
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "REFRESH_TOKEN_INVALID"
    );
}

#[tokio::test]
async fn logout_without_token_returns_401_missing_auth_header() {
    let (s, _) = test_server().await;
    let res = s.post("/auth/logout").await;
    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "MISSING_AUTH_HEADER"
    );
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let (s, _) = test_server().await;
    let email = unique_email("lo");
    let body: serde_json::Value = s
        .post("/auth/register")
        .json(&json!({ "email": email, "username": unique_username("lo"), "password": "password123" }))
        .await
        .json();
    let access_token = body["access_token"].as_str().unwrap();
    let refresh_token = body["refresh_token"].as_str().unwrap().to_owned();

    s.post("/auth/logout")
        .authorization_bearer(access_token)
        .await
        .assert_status_success();

    let res = s
        .post("/auth/refresh")
        .json(&json!({ "refresh_token": refresh_token }))
        .await;
    assert_eq!(res.status_code().as_u16(), 401);
}

#[tokio::test]
async fn authorize_unknown_provider_returns_404() {
    let (s, _) = test_server().await;

    let res = s.get("/auth/unknown").await;

    assert_eq!(res.status_code().as_u16(), 404);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_UNKNOWN_PROVIDER"
    );
}

#[tokio::test]
async fn authorize_unconfigured_provider_returns_503() {
    let (s, _) = test_server().await;

    let res = s.get("/auth/google").await;

    assert_eq!(res.status_code().as_u16(), 503);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_PROVIDER_NOT_CONFIGURED"
    );
}

#[tokio::test]
async fn callback_unknown_provider_returns_404() {
    let (s, _) = test_server().await;

    let res = s.get("/auth/unknown/callback?code=dummy&state=dummy").await;

    assert_eq!(res.status_code().as_u16(), 404);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_UNKNOWN_PROVIDER"
    );
}

#[tokio::test]
async fn callback_with_error_returns_400() {
    let (s, _) = test_server().await;

    let res = s
        .get("/auth/google/callback?error=access_denied&error_description=user+cancelled")
        .await;

    assert_eq!(res.status_code().as_u16(), 400);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_PROVIDER_DENIED"
    );
}

#[tokio::test]
async fn callback_missing_params_returns_401() {
    let (s, _) = test_server().await;

    let res = s.get("/auth/google/callback").await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_INVALID_STATE"
    );
}

#[tokio::test]
async fn list_connections_without_auth_returns_401() {
    let (s, _) = test_server().await;

    let res = s.get("/auth/connections").await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "MISSING_AUTH_HEADER"
    );
}

#[tokio::test]
async fn list_connections_as_guest_returns_403() {
    let (s, pool) = test_server().await;
    let email = unique_email("lcg");
    let username = unique_username("lcg");
    let (_, _) = register_user(&s, &email, &username, "password123").await;

    sqlx::query!("UPDATE users SET role = 'guest' WHERE email = $1", email)
        .execute(&pool)
        .await
        .unwrap();

    let body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let token = body["access_token"].as_str().unwrap();

    let res = s.get("/auth/connections").authorization_bearer(token).await;

    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}

#[tokio::test]
async fn list_connections_as_user_returns_empty_list() {
    let (s, _) = test_server().await;
    let email = unique_email("lcu");
    let username = unique_username("lcu");
    let (token, _) = register_user(&s, &email, &username, "password123").await;

    let res = s
        .get("/auth/connections")
        .authorization_bearer(&token)
        .await;

    res.assert_status_success();
    let body: serde_json::Value = res.json();
    assert_eq!(body, serde_json::json!([]));
}

#[tokio::test]
async fn unlink_without_auth_returns_401() {
    let (s, _) = test_server().await;

    let res = s.delete("/auth/connections/google").await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "MISSING_AUTH_HEADER"
    );
}

#[tokio::test]
async fn unlink_as_guest_returns_403() {
    let (s, pool) = test_server().await;
    let email = unique_email("ulg");
    let (_, _) = register_user(&s, &email, &unique_username("ulg"), "password123").await;

    sqlx::query!("UPDATE users SET role = 'guest' WHERE email = $1", email)
        .execute(&pool)
        .await
        .unwrap();

    let body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let token = body["access_token"].as_str().unwrap();

    let res = s
        .delete("/auth/connections/google")
        .authorization_bearer(token)
        .await;

    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}

#[tokio::test]
async fn unlink_unknown_provider_slug_returns_404() {
    let (s, _) = test_server().await;
    let email = unique_email("uup");
    let (_, _) = register_user(&s, &email, &unique_username("uup"), "password123").await;
    let body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let token = body["access_token"].as_str().unwrap();

    let res = s
        .delete("/auth/connections/unknown")
        .authorization_bearer(token)
        .await;

    assert_eq!(res.status_code().as_u16(), 404);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "OAUTH_UNKNOWN_PROVIDER"
    );
}

#[tokio::test]
async fn unlink_non_existent_connection_returns_404() {
    let (s, _) = test_server().await;
    let email = unique_email("unc");
    let (_, _) = register_user(&s, &email, &unique_username("unc"), "password123").await;
    let body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let token = body["access_token"].as_str().unwrap();

    let res = s
        .delete("/auth/connections/google")
        .authorization_bearer(token)
        .await;

    assert_eq!(res.status_code().as_u16(), 404);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "USER_NOT_FOUND"
    );
}

#[tokio::test]
async fn session_invalid_code_returns_401() {
    let (s, _) = test_server().await;

    let res = s
        .post("/auth/session")
        .json(&json!({ "code": "non-existent-code" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 401);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "EXCHANGE_CODE_INVALID"
    );
}

#[tokio::test]
async fn session_valid_code_returns_auth_response() {
    let (s, _) = test_server().await;
    let email = unique_email("svc");
    let username = unique_username("svc");
    let register_body: serde_json::Value = s
        .post("/auth/register")
        .json(&json!({ "email": email, "username": username, "password": "password123" }))
        .await
        .json();

    let config = test_config();
    let store = Arc::new(StateStore::new(&config.valkey_url).unwrap());
    let code = "test-valid-exchange-code";
    store
        .store_exchange(code, &register_body.to_string())
        .await
        .unwrap();

    let res = s.post("/auth/session").json(&json!({ "code": code })).await;

    res.assert_status_success();
    let body: serde_json::Value = res.json();
    assert!(body["access_token"].is_string());
    assert!(body["refresh_token"].is_string());
    assert!(body["user"]["email"].is_string());
}

#[tokio::test]
async fn session_code_is_single_use() {
    let (s, _) = test_server().await;

    let email = unique_email("ssu");
    let username = unique_username("ssu");
    let register_body: serde_json::Value = s
        .post("/auth/register")
        .json(&json!({ "email": email, "username": username, "password": "password123" }))
        .await
        .json();

    let config = test_config();
    let store = Arc::new(StateStore::new(&config.valkey_url).unwrap());
    let code = "test-single-use-code";
    store
        .store_exchange(code, &register_body.to_string())
        .await
        .unwrap();

    let res1 = s.post("/auth/session").json(&json!({ "code": code })).await;
    res1.assert_status_success();

    let res2 = s.post("/auth/session").json(&json!({ "code": code })).await;
    assert_eq!(res2.status_code().as_u16(), 401);
    assert_eq!(
        res2.json::<serde_json::Value>()["error"]["code"],
        "EXCHANGE_CODE_INVALID"
    );
}
