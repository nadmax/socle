use std::time::{SystemTime, UNIX_EPOCH};

use deadpool_redis::{Config as RedisConfig, Pool as RedisPool, Runtime};
use redis::AsyncCommands;

/// Valkey‑backed fixed‑window rate limiter for auth endpoints.
///
/// Each bucket is keyed by `rl:{endpoint}:{ip}:{window}` so that every
/// combination of endpoint, client IP and time window gets its own counter.
/// Keys automatically expire after `2 * window_secs` of inactivity.
#[derive(Clone)]
pub struct RateLimiter {
    pool: RedisPool,
    max_requests: u64,
    window_secs: u64,
}

impl RateLimiter {
    /// Construct a [`RateLimiter`] from a Valkey URL + rate‑limit parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL is invalid or the pool cannot be created.
    pub fn new(
        valkey_url: &str,
        max_requests: u64,
        window_secs: u64,
    ) -> Result<Self, deadpool_redis::CreatePoolError> {
        let cfg = RedisConfig::from_url(valkey_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self {
            pool,
            max_requests,
            window_secs,
        })
    }

    /// Close the underlying Valkey connection pool.
    pub fn close(&self) {
        self.pool.close();
    }

    /// Check whether a request from `ip` for `endpoint` is within limits.
    ///
    /// Returns `Ok(remaining)` when allowed, `Err(retry_after_secs)` when the
    /// limit has been exceeded. On Valkey errors the limiter **fails open**
    /// (allows the request) so infrastructure issues never permanently lock
    /// users out.
    ///
    /// # Errors
    ///
    /// This function itself never returns an error; Valkey errors are silently
    /// treated as "within limits" (fail‑open). The `Result` type is used purely
    /// to distinguish `Ok(remaining)` from `Err(retry_after_secs)`.
    pub async fn check(&self, ip: &str, endpoint: &str) -> Result<u64, u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window = now / self.window_secs;
        let key = format!("rl:{endpoint}:{ip}:{window}");

        let count: u64 = match self.incr(&key).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "rate‑limiter Valkey error, failing open");
                return Ok(self.max_requests);
            }
        };

        if count > self.max_requests {
            let retry_after = (window + 1) * self.window_secs - now;
            Err(retry_after.max(1))
        } else {
            Ok(self.max_requests - count)
        }
    }

    /// Increment the counter for the given key atomically.
    ///
    /// Sets expiry on the first increment within a window so the key
    /// eventually cleans itself up.
    #[expect(clippy::cast_possible_wrap)]
    async fn incr(&self, key: &str) -> redis::RedisResult<u64> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| redis::RedisError::from(std::io::Error::other(e.to_string())))?;
        let count: u64 = conn.incr(key, 1).await?;
        if count == 1 {
            let _: () = conn.expire(key, (self.window_secs * 2) as i64).await?;
        }
        Ok(count)
    }
}
