use serde::Deserialize;

/// Application configuration loaded from environment variables.
///
/// Uses `dotenvy` to load a `.env` file and `envy` to deserialize
/// into this struct. All fields are required unless a `default` is given.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Full `PostgreSQL` connection string.
    pub database_url: String,

    /// Secret used to sign JWT access tokens. Must be at least 32 bytes.
    pub jwt_secret: String,

    /// Access token lifetime in seconds (default: 15 minutes).
    #[serde(default = "default_access_token_expiry")]
    pub access_token_expiry_secs: u64,

    /// Refresh token lifetime in seconds (default: 7 days).
    #[serde(default = "default_refresh_token_expiry")]
    pub refresh_token_expiry_secs: u64,

    /// TCP address the server binds to (default: 0.0.0.0:8080).
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
}

impl Config {
    /// Load configuration from the environment, optionally reading a `.env` file first.
    ///
    /// # Errors
    ///
    /// Returns an [`envy::Error`] if any required environment variables are missing or
    /// cannot be deserialized into the expected types.
    pub fn from_env() -> Result<Self, envy::Error> {
        let _ = dotenvy::dotenv();
        envy::from_env()
    }
}

fn default_access_token_expiry() -> u64 {
    900 // 15 minutes
}

fn default_refresh_token_expiry() -> u64 {
    604_800 // 7 days
}

fn default_bind_addr() -> String {
    "0.0.0.0:8080".to_owned()
}
