mod common;

use std::sync::Arc;

use yaima::{
    config::Config,
    services::{auth::AuthService, oauth::StateStore, token::TokenService, user::UserService},
    state::AppState,
};

use crate::common::{test_config, test_pool};

#[tokio::test]
async fn app_state_new_constructs_all_services() {
    let pool = test_pool().await;
    let config = test_config();
    let oauth_store = Arc::new(StateStore::new(&config.redis_url).unwrap());
    let user = UserService::new(pool.clone());
    let token = TokenService::new(pool.clone(), config.clone());
    let auth = AuthService::new(user.clone(), token.clone(), config.clone());
    let state = AppState::new(auth, user, token, config, oauth_store);

    let _: &AuthService = &state.auth;
    let _: &UserService = &state.user;
    let _: &TokenService = &state.token;
    let _: &Config = &state.config;
    let _: &Arc<StateStore> = &state.oauth_store;
}
