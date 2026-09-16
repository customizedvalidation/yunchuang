//! User model, DB row and API request/response shapes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// Database row for the `users` table. `password_hash` is never serialized
/// to clients.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub tenant_id: i64,
}

/// Public-facing user view (no password hash).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            role: u.role,
            tenant_id: u.tenant_id,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

/// Body of `POST /api/v1/users`.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 64, message = "username must be 3..=64 chars"))]
    pub username: String,
    #[validate(email(message = "email must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "password must be at least 8 chars"))]
    pub password: String,
    #[serde(default = "default_role")]
    pub role: Option<String>,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: Option<i64>,
}

/// Body of `PUT /api/v1/users/:id`. All fields optional; password is optional.
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateUserRequest {
    #[validate(length(min = 3, max = 64, message = "username must be 3..=64 chars"))]
    pub username: Option<String>,
    #[validate(email(message = "email must be a valid email"))]
    pub email: Option<String>,
    #[validate(length(min = 8, message = "password must be at least 8 chars"))]
    pub password: Option<String>,
    pub role: Option<String>,
    pub tenant_id: Option<i64>,
}

fn default_role() -> Option<String> {
    Some("user".to_string())
}
fn default_tenant_id() -> Option<i64> {
    Some(1)
}
