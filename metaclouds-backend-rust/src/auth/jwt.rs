//! JWT issuance and verification (HS256).

use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Claims embedded in the access token. Field names are kept as plain
/// snake-case to match the Go v1 contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: u64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub tenant_id: u64,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
}

/// Issue a signed HS256 token for the given user. Returns `(token, expires_at_unix)`.
pub fn issue_token(
    secret: &[u8],
    user_id: i64,
    username: &str,
    email: &str,
    role: &str,
    tenant_id: i64,
    expires_secs: u64,
) -> AppResult<(String, i64)> {
    let now = Utc::now();
    let exp = (now + chrono::Duration::seconds(expires_secs as i64)).timestamp();
    let claims = Claims {
        user_id: user_id as u64,
        username: username.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        tenant_id: tenant_id as u64,
        exp,
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )?;
    Ok((token, exp))
}

/// Verify a signed token and return its claims on success.
pub fn verify_token(secret: &[u8], token: &str) -> AppResult<Claims> {
    let validation = Validation::default(); // HS256 by default
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|_| AppError::unauthorized("invalid or expired token"))?;
    Ok(data.claims)
}
