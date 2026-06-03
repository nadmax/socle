use crate::services::{auth::AuthService, token::TokenService, user::UserService};

/// Shared application state cloned into every request handler.
///
/// All contained services are cheap to clone (they hold an `Arc` internally
/// via `PgPool` / `Config`).
#[derive(Clone)]
pub struct AppState {
    pub auth_svc: AuthService,
    pub user_svc: UserService,
    pub token_svc: TokenService,
}

impl AppState {
    pub fn new(auth_svc: AuthService, user_svc: UserService, token_svc: TokenService) -> Self {
        Self {
            auth_svc,
            user_svc,
            token_svc,
        }
    }
}
