use std::time::Duration;

use tokio::sync::oneshot;
use yaima::shutdown::shutdown_signal;

use crate::common::test_app;

#[tokio::test]
async fn server_should_shutdown_gracefully_when_signal_received() {
    let (app, _pool) = test_app().await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind test listener");
    let addr = listener.local_addr().unwrap();

    let (tx, rx) = oneshot::channel();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(Some(rx)))
            .await
            .ok();
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    let client = reqwest::Client::new();

    client
        .post(format!("http://{addr}/auth/register"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .expect("request to running server should not fail");

    tx.send(()).unwrap();

    tokio::time::timeout(Duration::from_secs(10), server_handle)
        .await
        .expect("server did not shut down in time")
        .unwrap();
}
