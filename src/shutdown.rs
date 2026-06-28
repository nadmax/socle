use tokio::sync::oneshot;

/// Return a future that resolves when a shutdown signal is received.
///
/// On Unix, listens for `SIGTERM` and `SIGINT`. On non-Unix platforms,
/// listens for `SIGINT` only via `tokio::signal::ctrl_c`.
///
/// When `rx` is `Some`, the future also resolves when the oneshot sender
/// is dropped or sent — this allows integration tests to trigger shutdown
/// without signalling the test runner process.
///
/// # Panics
///
/// Panics on Unix if the OS refuses to install the `SIGTERM` signal handler
/// (e.g. out of memory for the signal data structure). This is extremely
/// unlikely in practice and indicates a terminal system-level failure.
pub async fn shutdown_signal(rx: Option<oneshot::Receiver<()>>) {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };

    let signal = async {
        #[cfg(unix)]
        {
            let mut sig = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
            tokio::select! {
                _ = ctrl_c => tracing::info!("received SIGINT"),
                _ = sig.recv() => tracing::info!("received SIGTERM"),
            }
        }

        #[cfg(not(unix))]
        {
            ctrl_c.await;
            tracing::info!("received SIGINT");
        }
    };

    match rx {
        Some(rx) => {
            tokio::select! {
                () = signal => {},
                _ = rx => tracing::info!("shutdown signal received from test harness"),
            }
        }
        None => signal.await,
    }
}
