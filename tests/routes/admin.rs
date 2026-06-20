use serde_json::json;

use crate::common::{make_admin, register_user, test_server, unique_email, unique_username};

#[tokio::test]
async fn admin_updates_user_role_via_http() {
    let (s, pool) = test_server().await;
    let email = unique_email("au");
    let username = unique_username("au");
    let (_, _) = register_user(&s, &email, &username, "password123").await;

    let target_body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let target_id = target_body["user"]["id"].as_str().unwrap().to_owned();

    let admin_email = unique_email("admin-au");
    let (_, _) = register_user(&s, &admin_email, &unique_username("admin-au"), "password123").await;
    let admin_token = make_admin(&s, &pool, &admin_email, "password123").await;

    let res = s
        .put(&format!("/admin/users/{target_id}/role"))
        .authorization_bearer(&admin_token)
        .json(&json!({ "role": "admin" }))
        .await;

    res.assert_status_success();
    let updated: serde_json::Value = res.json();
    assert_eq!(updated["role"], "admin");
    assert_eq!(updated["id"], target_id);
}

#[tokio::test]
async fn admin_demotes_user_to_guest() {
    let (s, pool) = test_server().await;
    let email = unique_email("ad");
    let (_, _) = register_user(&s, &email, &unique_username("ad"), "password123").await;

    let target_body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": email, "password": "password123" }))
        .await
        .json();
    let target_id = target_body["user"]["id"].as_str().unwrap().to_owned();

    let admin_email = unique_email("admin-ad");
    let (_, _) = register_user(
        &s,
        &admin_email,
        &unique_username("admin-ad"),
        "password123",
    )
    .await;
    let admin_token = make_admin(&s, &pool, &admin_email, "password123").await;

    let res = s
        .put(&format!("/admin/users/{target_id}/role"))
        .authorization_bearer(&admin_token)
        .json(&json!({ "role": "guest" }))
        .await;

    res.assert_status_success();
    assert_eq!(res.json::<serde_json::Value>()["role"], "guest");
}

#[tokio::test]
async fn non_admin_cannot_update_role() {
    let (s, _) = test_server().await;
    let email = unique_email("na");
    let (token, _) = register_user(&s, &email, &unique_username("na"), "password123").await;

    let res = s
        .put(&format!("/admin/users/{}/role", uuid::Uuid::now_v7()))
        .authorization_bearer(&token)
        .json(&json!({ "role": "admin" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}

#[tokio::test]
async fn guest_cannot_access_admin_endpoint() {
    let (s, pool) = test_server().await;
    let email = unique_email("gaa");
    let (_, _) = register_user(&s, &email, &unique_username("gaa"), "password123").await;

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
        .put(&format!("/admin/users/{}/role", uuid::Uuid::now_v7()))
        .authorization_bearer(token)
        .json(&json!({ "role": "admin" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}

#[tokio::test]
async fn admin_cannot_change_own_role() {
    let (s, pool) = test_server().await;
    let admin_email = unique_email("ascr");
    let (_, _) = register_user(
        &s,
        &admin_email,
        &unique_username("ascr"),
        "password123",
    )
    .await;
    let admin_token = make_admin(&s, &pool, &admin_email, "password123").await;

    let admin_body: serde_json::Value = s
        .post("/auth/login")
        .json(&json!({ "email": admin_email, "password": "password123" }))
        .await
        .json();
    let admin_id = admin_body["user"]["id"].as_str().unwrap();

    let res = s
        .put(&format!("/admin/users/{admin_id}/role"))
        .authorization_bearer(&admin_token)
        .json(&json!({ "role": "guest" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 403);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "FORBIDDEN"
    );
}

#[tokio::test]
async fn admin_update_unknown_user_returns_404() {
    let (s, pool) = test_server().await;
    let admin_email = unique_email("auu");
    let (_, _) = register_user(
        &s,
        &admin_email,
        &unique_username("auu"),
        "password123",
    )
    .await;
    let admin_token = make_admin(&s, &pool, &admin_email, "password123").await;

    let res = s
        .put(&format!("/admin/users/{}/role", uuid::Uuid::now_v7()))
        .authorization_bearer(&admin_token)
        .json(&json!({ "role": "admin" }))
        .await;

    assert_eq!(res.status_code().as_u16(), 404);
    assert_eq!(
        res.json::<serde_json::Value>()["error"]["code"],
        "USER_NOT_FOUND"
    );
}
