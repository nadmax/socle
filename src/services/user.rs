use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    config::OAuthProvider,
    errors::{AppError, AppResult},
    models::{Role, User},
    services::token::{hash_password, verify_password},
};

/// Handles all user-related database operations.
#[derive(Clone)]
pub struct UserService {
    pool: PgPool,
}

impl UserService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Fetch a user by their UUID. Returns [`AppError::UserNotFound`] if absent.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::UserNotFound`] if no user exists with the given `id`, or
    /// an [`AppError`] if the database query fails.
    pub async fn find_by_id(&self, id: Uuid) -> AppResult<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, email, username, password_hash,
                   role AS "role: Role",
                   is_active, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)
    }

    /// Fetch a user by email address.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the database query fails. Returns `Ok(None)` if
    /// no user exists with the given email.
    pub async fn find_by_email(&self, email: &str) -> AppResult<Option<User>> {
        sqlx::query_as!(
            User,
            r#"
            SELECT id, email, username, password_hash,
                   role AS "role: Role",
                   is_active, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::from)
    }

    /// Create a new user row. Defaults to [`Role::User`].
    ///
    /// Returns [`AppError::EmailTaken`] or [`AppError::UsernameTaken`] on
    /// unique-constraint violations so callers get a precise error code.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::EmailTaken`] or [`AppError::UsernameTaken`] if the email
    /// or username is already in use, [`AppError::Hashing`] if password hashing
    /// fails, or an [`AppError`] if the database insert fails.
    pub async fn create(&self, email: &str, username: &str, password: &str) -> AppResult<User> {
        let password_hash = hash_password(password)?;
        let id = Uuid::now_v7();

        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, email, username, password_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, username, password_hash,
                      role AS "role: Role",
                      is_active, created_at, updated_at
            "#,
            id,
            email,
            username,
            password_hash,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|err| {
            if let sqlx::Error::Database(ref db_err) = err {
                if db_err.code().as_deref() == Some("23505") {
                    let msg = db_err.message();
                    if msg.contains("users_email_key") {
                        return AppError::EmailTaken;
                    }
                    if msg.contains("users_username_key") {
                        return AppError::UsernameTaken;
                    }
                }
            }
            AppError::Database(err)
        })
    }

    /// Update the stored password hash after verifying the current password.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::UserNotFound`] if the user no longer exists,
    /// [`AppError::InvalidCredentials`] if `current_password` is wrong,
    /// [`AppError::Hashing`] if hashing the new password fails, or an [`AppError`]
    /// if the database update fails.
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        let user = self.find_by_id(user_id).await?;

        if !verify_password(current_password, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        let new_hash = hash_password(new_password)?;

        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            new_hash,
            user_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Assign a new role to a user (admin operation).
    ///
    /// # Errors
    ///
    /// Returns [`AppError::UserNotFound`] if no user exists with the given
    /// `user_id`, or an [`AppError`] if the database update fails.
    pub async fn update_role(&self, user_id: Uuid, role: Role) -> AppResult<User> {
        sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET role = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, email, username, password_hash,
                      role AS "role: Role",
                      is_active, created_at, updated_at
            "#,
            role as Role,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)
    }

    /// Soft-delete a user by setting `is_active = FALSE`.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the database update fails.
    pub async fn deactivate(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query!(
            "UPDATE users SET is_active = FALSE, updated_at = NOW() WHERE id = $1",
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Look up a user via an existing OAuth link.
    ///
    /// Returns `None` when no `oauth_accounts` row exists for the given
    /// `(provider, provider_user_id)` pair — not an error, just "first login".
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the database query fails.
    pub async fn find_by_oauth_identity(
        &self,
        provider: OAuthProvider,
        provider_user_id: &str,
    ) -> AppResult<Option<User>> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT u.id, u.email, u.username, u.password_hash,
                   u.role AS "role: Role", u.is_active, u.created_at, u.updated_at
            FROM oauth_accounts oa
            JOIN users u ON u.id = oa.user_id
            WHERE oa.provider = $1 AND oa.provider_user_id = $2
            "#,
            provider.to_string(),
            provider_user_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Create a new user seeded from an OAuth profile.
    ///
    /// No password is set: the hash is a prefixed UUID that can never pass
    /// Argon2 verification, making it visually distinct in the DB and
    /// impossible to use for a password login.
    ///
    /// `display_name` is used as the initial username; falls back to the
    /// email prefix when the provider did not supply one.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the database insert fails (e.g. duplicate email).
    pub async fn create_from_oauth(
        &self,
        email: &str,
        display_name: Option<&str>,
    ) -> AppResult<User> {
        let username = display_name.unwrap_or_else(|| email.split('@').next().unwrap_or("user"));
        let unusable_hash = format!("oauth:{}", Uuid::now_v7());

        self.create(email, username, &unusable_hash).await
    }

    /// Persist an `oauth_accounts` row linking `user_id` to the external identity.
    ///
    /// `ON CONFLICT DO NOTHING` makes this idempotent: a second concurrent
    /// first-login for the same provider identity silently no-ops instead of
    /// returning a unique-constraint error.
    ///
    /// # Errors
    ///
    /// Returns an [`AppError`] if the database upsert fails.
    pub async fn link_oauth_account(&self, user_id: Uuid, profile: &OAuthProfile) -> AppResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO oauth_accounts (id, user_id, provider, provider_user_id, avatar_url)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (provider, provider_user_id) DO NOTHING
            "#,
            Uuid::now_v7(),
            user_id,
            profile.provider.to_string(),
            profile.provider_user_id,
            profile.avatar_url.as_deref(),
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
