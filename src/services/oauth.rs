use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse, basic::BasicClient, reqwest::async_http_client,
};
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::config::{OAuthProvider, OAuthProviderConfig};

/// Errors that can occur during any stage of the OAuth 2.0 flow.
#[derive(Debug, Error)]
pub enum OAuthError {
    /// The requested provider is not enabled in the server configuration.
    #[error("OAuth provider '{0}' is not configured")]
    ProviderNotConfigured(OAuthProvider),

    /// The `state` parameter returned by the provider did not match any pending
    /// authorisation. Either it expired, was already consumed, or is forged.
    #[error("invalid or expired OAuth state token")]
    InvalidState,

    /// The `state` in the callback belongs to a different provider than the
    /// route that received it — likely a misconfigured redirect URI.
    #[error("OAuth state provider mismatch: expected {expected}, got {actual}")]
    ProviderMismatch {
        expected: OAuthProvider,
        actual: OAuthProvider,
    },

    /// The authorisation code could not be exchanged for tokens.
    #[error("token exchange failed: {0}")]
    TokenExchange(String),

    /// A network request to the provider's API failed.
    #[error("provider unreachable: {0}")]
    ProviderUnreachable(#[from] reqwest::Error),

    /// The provider's user-info response was missing a required field.
    #[error("incomplete provider profile: missing field '{0}'")]
    IncompleteProfile(&'static str),

    /// The provider returned a redirect URI that could not be parsed.
    #[error("invalid redirect URI: {0}")]
    InvalidRedirectUri(#[from] oauth2::url::ParseError),
}

/// Data stored server-side while the user is on the provider consent screen.
struct PendingAuth {
    /// PKCE verifier — must be presented on the callback to prove the
    /// authorisation request originated here.
    verifier: PkceCodeVerifier,
    /// Which provider initiated this flow; validated in the callback to prevent
    /// cross-provider state reuse.
    provider: OAuthProvider,
    /// Wall-clock instant the entry was inserted; used for expiry sweeps.
    created_at: Instant,
}

/// Thread-safe, in-memory store for short-lived PKCE/CSRF state tokens.
///
/// Each entry is keyed by the opaque `state` string that travels to the
/// provider and back.  Entries older than [`STATE_TTL`] are lazily removed on
/// the next [`StateStore::take`] call for that key.
///
/// For multi-instance deployments replace this with a shared Redis store
/// (SETNX + EXPIRE) while keeping the same `insert`/`take` interface.
pub struct StateStore {
    inner: DashMap<String, PendingAuth>,
}

/// How long a pending authorisation state is considered valid.
const STATE_TTL: Duration = Duration::from_secs(10 * 60); // 10 minutes

impl StateStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
        }
    }

    /// Wrap in an [`Arc`] for sharing across Axum handler clones.
    #[must_use]
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Insert a new pending auth entry, keyed by `state`.
    fn insert(&self, state: String, provider: OAuthProvider, verifier: PkceCodeVerifier) {
        self.inner.insert(
            state,
            PendingAuth {
                verifier,
                provider,
                created_at: Instant::now(),
            },
        );
    }

    /// Remove and return the entry for `state`, or `None` if missing or expired.
    ///
    /// Consuming the entry prevents replay attacks: a valid callback URL cannot
    /// be reused even if intercepted.
    fn take(&self, state: &str) -> Option<PendingAuth> {
        let (_, entry) = self.inner.remove(state)?;

        if entry.created_at.elapsed() > STATE_TTL {
            return None;
        }

        Some(entry)
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Provider-agnostic user identity returned after a successful OAuth flow.
///
/// `services/user.rs` uses this to find-or-create a local account and link
/// the `OAuthAccount` relation (see `models.rs`).
#[derive(Debug, Clone)]
pub struct OAuthProfile {
    /// Which provider authenticated this user.
    pub provider: OAuthProvider,
    /// The user's stable, unique identifier within that provider.
    pub provider_user_id: String,
    /// Primary email address as reported by the provider.
    ///
    /// Treat with care: not all providers verify email addresses.
    /// Google and GitHub both do, so this field is trusted for account merging
    /// only for those two providers.
    pub email: String,
    /// Human-readable display name, if the provider exposes one.
    pub display_name: Option<String>,
    /// Public avatar URL, if available.
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: u64,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
    login: String,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

/// Outcome of [`build_authorization_url`].
pub struct AuthorizationRequest {
    /// The URL to redirect the user's browser to.
    pub url: Url,
    /// The opaque `state` token that must be round-tripped through the provider
    /// and passed to [`exchange_code`].  Store it in [`StateStore`] immediately.
    pub state_key: String,
}

/// Build the provider consent-screen URL and register the PKCE state.
///
/// The caller **must** redirect the user to [`AuthorizationRequest::url`] and
/// record `state_key` in the [`StateStore`] — both happen atomically in the
/// route handler so there is no window where the state exists but the redirect
/// has not yet been issued.
///
/// # Errors
///
/// Returns [`OAuthError::ProviderNotConfigured`] if `provider_cfg` is `None`,
/// or [`OAuthError::InvalidRedirectUri`] if the stored redirect URI is malformed.
pub fn build_authorization_url(
    provider: OAuthProvider,
    provider_cfg: &OAuthProviderConfig,
    store: &StateStore,
) -> Result<AuthorizationRequest, OAuthError> {
    let client = make_basic_client(provider, provider_cfg)?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(provider_scopes(provider))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_key = csrf_token.secret().clone();
    store.insert(state_key.clone(), provider, pkce_verifier);

    Ok(AuthorizationRequest { url, state_key })
}

/// Exchange an authorisation code for an access token.
///
/// Validates the `state` against the [`StateStore`] (CSRF + PKCE) and
/// returns the raw access token on success.  The state entry is consumed so
/// it cannot be replayed.
///
/// # Errors
///
/// - [`OAuthError::InvalidState`]: unknown, expired, or already-used state
/// - [`OAuthError::ProviderMismatch`]: state belongs to a different provider
/// - [`OAuthError::TokenExchange`]: provider rejected the code
/// - [`OAuthError::InvalidRedirectUri`]: config error
pub async fn exchange_code(
    provider: OAuthProvider,
    provider_cfg: &OAuthProviderConfig,
    code: &str,
    state: &str,
    store: &StateStore,
) -> Result<String, OAuthError> {
    let pending = store.take(state).ok_or(OAuthError::InvalidState)?;

    if pending.provider != provider {
        return Err(OAuthError::ProviderMismatch {
            expected: provider,
            actual: pending.provider,
        });
    }

    let client = make_basic_client(provider, provider_cfg)?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(code.to_owned()))
        .set_pkce_verifier(pending.verifier)
        .request_async(async_http_client)
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

    Ok(token_result.access_token().secret().clone())
}

/// Fetch and normalise the authenticated user's profile from the provider.
///
/// Each provider exposes a different user-info API; this function maps them all
/// to [`OAuthProfile`] so the rest of the application stays provider-agnostic.
///
/// # Errors
///
/// - [`OAuthError::ProviderUnreachable`]: HTTP request failed
/// - [`OAuthError::IncompleteProfile`]: required field absent in response
pub async fn fetch_user_profile(
    provider: OAuthProvider,
    access_token: &str,
) -> Result<OAuthProfile, OAuthError> {
    match provider {
        OAuthProvider::Google => fetch_google_profile(access_token).await,
        OAuthProvider::GitHub => fetch_github_profile(access_token).await,
    }
}

/// Construct an [`oauth2::basic::BasicClient`] from stored config.
fn make_basic_client(
    provider: OAuthProvider,
    cfg: &OAuthProviderConfig,
) -> Result<BasicClient, OAuthError> {
    let (auth_url, token_url) = provider_endpoints(provider);

    let client = BasicClient::new(
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
        oauth2::AuthUrl::new(auth_url.to_owned()).map_err(OAuthError::InvalidRedirectUri)?,
        Some(oauth2::TokenUrl::new(token_url.to_owned()).map_err(OAuthError::InvalidRedirectUri)?),
    )
    .set_redirect_uri(
        RedirectUrl::new(cfg.redirect_uri.clone()).map_err(OAuthError::InvalidRedirectUri)?,
    );

    Ok(client)
}

/// Static provider endpoints. Extending to a third provider means adding one
/// arm here and in `provider_scopes`.
fn provider_endpoints(provider: OAuthProvider) -> (&'static str, &'static str) {
    match provider {
        OAuthProvider::Google => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
        ),
        OAuthProvider::GitHub => (
            "https://github.com/login/oauth/authorize",
            "https://github.com/login/oauth/access_token",
        ),
    }
}

/// Scopes to request from each provider.
fn provider_scopes(provider: OAuthProvider) -> Vec<Scope> {
    match provider {
        OAuthProvider::Google => vec![
            Scope::new("openid".to_owned()),
            Scope::new("email".to_owned()),
            Scope::new("profile".to_owned()),
        ],
        OAuthProvider::GitHub => vec![
            Scope::new("read:user".to_owned()),
            Scope::new("user:email".to_owned()),
        ],
    }
}

async fn fetch_google_profile(access_token: &str) -> Result<OAuthProfile, OAuthError> {
    let info: GoogleUserInfo = reqwest::Client::new()
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(OAuthProfile {
        provider: OAuthProvider::Google,
        provider_user_id: info.sub,
        email: info.email,
        display_name: info.name,
        avatar_url: info.picture,
    })
}

async fn fetch_github_profile(access_token: &str) -> Result<OAuthProfile, OAuthError> {
    let client = reqwest::Client::new();

    let info: GitHubUserInfo = client
        .get("https://api.github.com/user")
        .bearer_auth(access_token)
        .header(reqwest::header::USER_AGENT, "your-app-name")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let email = match info.email {
        Some(e) => e,
        None => fetch_github_primary_email(&client, access_token).await?,
    };

    Ok(OAuthProfile {
        provider: OAuthProvider::GitHub,
        provider_user_id: info.id.to_string(),
        email,
        display_name: info.name.or(Some(info.login)),
        avatar_url: info.avatar_url,
    })
}

/// Fetch the primary verified email from GitHub's `/user/emails` endpoint.
///
/// This is a separate request; it is only made when the public profile omits
/// the email (common for users who mark it private).
async fn fetch_github_primary_email(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<String, OAuthError> {
    let emails: Vec<GitHubEmail> = client
        .get("https://api.github.com/user/emails")
        .bearer_auth(access_token)
        .header(reqwest::header::USER_AGENT, "your-app-name")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
        .ok_or(OAuthError::IncompleteProfile("email"))
}
