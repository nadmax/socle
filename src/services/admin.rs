use uuid::Uuid;

use crate::{
    errors::AppResult,
    models::{Role, User},
    services::user::UserService,
};

/// Handles all admin-level operations.
///
/// Constructed per-request from the caller's [`UserService`] handle — no
/// separate field on [`AppState`] is needed since [`UserService`] holds an
/// `Arc<PgPool>` internally and is cheap to clone.
pub struct AdminService<'a> {
    user: &'a UserService,
}

impl<'a> AdminService<'a> {
    #[must_use]
    pub fn new(user: &'a UserService) -> Self {
        Self { user }
    }

    /// Assign a new role to a user.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Forbidden`] if the admin attempts to change their
    /// own role, preventing accidental self-lockout.
    pub async fn update_user_role(
        &self,
        admin_id: Uuid,
        target_id: Uuid,
        new_role: Role,
    ) -> AppResult<User> {
        if admin_id == target_id {
            return Err(crate::errors::AppError::Forbidden);
        }

        let user = self.user.update_role(target_id, new_role).await?;

        tracing::info!(
            admin_id  = %admin_id,
            target_id = %target_id,
            new_role  = ?new_role,
            "user role updated"
        );

        Ok(user)
    }
}
