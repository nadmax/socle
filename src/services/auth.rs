use crate::{
    config::Config,
    errors::{AppError, AppResult},
    models::{AuthResponse, UserResponse},
    services::{
        token::{TokenService, verify_password},
        user::UserService,
    },
};

/// Orchestrates the high-level authentication flows.
///
/// Delegates persistence to [`UserService`] and token operations to
/// [`TokenService`], keeping each concern in a single place.
#[derive(Clone)]
pub struct AuthService {
    user: UserService,
    token: TokenService,
    config: Config,
}

impl AuthService {
    #[must_use]
    pub fn new(user: UserService, token: TokenService, config: Config) -> Self {
        Self {
            user,
            token,
            config,
        }
    }

    /// Register a new account and immediately issue tokens.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if:
    /// - `password` fails the strength validation rules
    /// - `username` fails the format validation rules
    /// - the email or username is already taken
    /// - hashing the password or generating tokens fails
    /// - the underlying database call fails
    pub async fn register(
        &self,
        email: &str,
        username: &str,
        password: &str,
    ) -> AppResult<AuthResponse> {
        validate_password(password)?;
        validate_username(username)?;

        let user = self.user.create(email, username, password).await?;
        let role = user.role;
        self.issue_tokens(
            user.id,
            &user.email.clone(),
            &user.username.clone(),
            role,
            user.into(),
        )
        .await
    }

    /// Validate credentials and issue tokens.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if:
    /// - no account exists for the given email, or the password is wrong ([`AppError::InvalidCredentials`])
    /// - the account has been deactivated ([`AppError::AccountDisabled`])
    /// - verifying the password hash or generating tokens fails
    /// - the underlying database call fails
    pub async fn login(&self, email: &str, password: &str) -> AppResult<AuthResponse> {
        let user = self
            .user
            .find_by_email(email)
            .await?
            // Use a generic error to avoid leaking whether the email exists.
            .ok_or(AppError::InvalidCredentials)?;

        if !user.is_active {
            return Err(AppError::AccountDisabled);
        }

        if !verify_password(password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        let user_response = UserResponse::from(user.clone());
        self.issue_tokens(
            user.id,
            &user.email,
            &user.username,
            user.role,
            user_response,
        )
        .await
    }

    /// Exchange a valid refresh token for a fresh token pair.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if:
    /// - the refresh token is invalid, expired, or has already been rotated
    /// - the associated account has been deactivated ([`AppError::AccountDisabled`])
    /// - generating the new access token fails
    /// - the underlying database call fails
    pub async fn refresh(&self, raw_refresh_token: &str) -> AppResult<AuthResponse> {
        let (new_refresh_token, user_id) =
            self.token.rotate_refresh_token(raw_refresh_token).await?;

        let user = self.user.find_by_id(user_id).await?;

        if !user.is_active {
            return Err(AppError::AccountDisabled);
        }

        let access_token =
            self.token
                .generate_access_token(user.id, &user.email, &user.username, user.role)?;

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            expires_in: self.config.access_token_expiry_secs,
            user: user.into(),
        })
    }

    /// Revoke all refresh tokens for the authenticated user.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the underlying database call fails.
    pub async fn logout(&self, user_id: uuid::Uuid) -> AppResult<()> {
        self.token.revoke_all_user_tokens(user_id).await
    }

    async fn issue_tokens(
        &self,
        user_id: uuid::Uuid,
        email: &str,
        username: &str,
        role: crate::models::Role,
        user: UserResponse,
    ) -> AppResult<AuthResponse> {
        let access_token = self
            .token
            .generate_access_token(user_id, email, username, role)?;
        let refresh_token = self.token.create_refresh_token(user_id).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            expires_in: self.config.access_token_expiry_secs,
            user,
        })
    }
}

fn validate_password(password: &str) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::InvalidCredentials);
    }
    Ok(())
}

fn validate_username(username: &str) -> AppResult<()> {
    let len = username.len();
    if !(3..=32).contains(&len) {
        return Err(AppError::InvalidCredentials);
    }
    Ok(())
}
