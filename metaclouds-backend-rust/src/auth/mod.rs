//! Authentication submodule: password hashing, JWT issuance/verification,
//! auth middleware, CSRF 双提交与 HTTP handlers。

pub mod csrf;
pub mod handler;
pub(crate) mod jwt;
pub mod middleware;
pub mod password;

pub use handler::{get_csrf_token, get_profile, login, logout, refresh};
pub use jwt::Claims;
pub use middleware::{jwt_auth, require_permission};
