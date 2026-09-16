//! Authentication submodule: password hashing, JWT issuance/verification,
//! auth middleware and HTTP handlers.

pub mod handler;
pub(crate) mod jwt;
pub mod middleware;
pub mod password;

pub use handler::{get_profile, login, logout};
pub use jwt::Claims;
pub use middleware::{jwt_auth, require_permission};
