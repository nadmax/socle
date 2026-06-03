use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Access tier assigned to every user.
///
/// Stored as the Postgres `user_role` enum; serialised as a lowercase string
/// in JWTs and API responses so clients never have to handle integer codes.
///
/// | Role    | Intended for                                          |
/// |---------|-------------------------------------------------------|
/// | `guest` | Provisional accounts or pre-verification users        |
/// | `user`  | Fully registered members (default on registration)   |
/// | `admin` | Operators with elevated privileges                    |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Guest,
    User,
    Admin,
}

impl Role {
    /// Returns `true` if this role is at least as privileged as `required`.
    ///
    /// Hierarchy (ascending): `Guest < User < Admin`.
    pub fn is_at_least(self, required: Role) -> bool {
        self.level() >= required.level()
    }

    fn level(self) -> u8 {
        match self {
            Role::Guest => 0,
            Role::User => 1,
            Role::Admin => 2,
        }
    }
}

/// Full user row as stored in the database.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Claims embedded in every access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (the user's UUID)
    pub sub: Uuid,

    /// Issued-at (Unix timestamp seconds).
    pub iat: u64,

    /// Expiration (Unix timestamp seconds).
    pub exp: u64,

    pub email: String,
    pub username: String,

    /// Role at the time the token was issued.
    ///
    /// If a user's role changes, they must obtain a new access token before
    /// the new role takes effect (i.e. after the current token expires or on
    /// the next refresh cycle).
    pub role: Role,
}

/// Payload for `POST /auth/register`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Valid e-mail address; must be unique.
    #[schema(example = "alice@example.com")]
    pub email: String,

    /// Display name; must be unique (3–32 chars).
    #[schema(example = "alice")]
    pub username: String,

    /// Plain-text password (min 8 chars). Stored only as an Argon2 hash.
    #[schema(example = "hunter2secret")]
    pub password: String,
}

/// Payload for `POST /auth/login`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// Registered e-mail address.
    #[schema(example = "alice@example.com")]
    pub email: String,

    /// Plain-text password.
    #[schema(example = "hunter2secret")]
    pub password: String,
}

/// Payload for `POST /auth/refresh`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    /// Opaque refresh token previously issued by `/auth/login`.
    pub refresh_token: String,
}

/// Payload for `POST /auth/change-password`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    /// Current password for confirmation.
    pub current_password: String,

    /// New password (min 8 chars).
    pub new_password: String,
}

/// Payload for `PUT /admin/users/:id/role`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

/// Returned by `/auth/register` and `/auth/login`.
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    /// Short-lived JWT Bearer token.
    pub access_token: String,

    /// Opaque token used to obtain a new access token.
    pub refresh_token: String,

    /// Seconds until the access token expires.
    pub expires_in: u64,

    /// Public user information.
    pub user: UserResponse,
}

/// Returned by `GET /users/me` and admin user endpoints.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub role: Role,
    pub created_at: String,
    pub updated_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            username: u.username,
            role: u.role,
            created_at: u
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            updated_at: u
                .updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

/// Generic success acknowledgement.
#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}

impl MessageResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
