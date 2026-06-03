use sqlx::PgPool;
use uuid::Uuid;

use crate::{
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
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Fetch a user by their UUID. Returns [`AppError::UserNotFound`] if absent.
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
                // Postgres unique-violation code: 23505
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
    pub async fn deactivate(&self, user_id: Uuid) -> AppResult<()> {
        sqlx::query!(
            "UPDATE users SET is_active = FALSE, updated_at = NOW() WHERE id = $1",
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
