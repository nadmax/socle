use crate::services::{auth::AuthService, token::TokenService, user::UserService};

/// Shared application state cloned into every request handler.
///
/// All contained services are cheap to clone (they hold an `Arc` internally
/// via `PgPool` / `Config`).
#[derive(Clone)]
pub struct AppState {
    pub auth: AuthService,
    pub user: UserService,
    pub token: TokenService,
}

impl AppState {
    #[must_use]
    pub fn new(auth: AuthService, user: UserService, token: TokenService) -> Self {
        Self { auth, user, token }
    }
}
