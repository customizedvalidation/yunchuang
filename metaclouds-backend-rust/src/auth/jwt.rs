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

// 测试模块位于 verify_token 定义之前，保留现有组织顺序即可。
#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    /// 模拟 Go 侧 `jwttool.GenerateToken` 用 `jwt.MapClaims` 手动签发的令牌：
    /// 字段名与取值类型与 Go 完全一致（user_id/tenant_id 为数字，jti 为字符串），
    /// 验证 Rust 端能跨语言校验通过（互操作 R 项）。
    fn build_go_style_token(secret: &str, exp_offset: i64) -> String {
        let now = Utc::now().timestamp();
        let mut claims = serde_json::Map::new();
        claims.insert("user_id".into(), serde_json::json!(42));
        claims.insert("username".into(), serde_json::json!("alice"));
        claims.insert("email".into(), serde_json::json!("alice@example.com"));
        claims.insert("role".into(), serde_json::json!("user"));
        claims.insert("tenant_id".into(), serde_json::json!(1));
        claims.insert("exp".into(), serde_json::json!(now + exp_offset));
        claims.insert("iat".into(), serde_json::json!(now));
        claims.insert("jti".into(), serde_json::json!("1700000000000000000"));
        encode(
            &Header::new(jsonwebtoken::Algorithm::HS256),
            &serde_json::Value::Object(claims),
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[test]
    fn go_issued_token_is_accepted() {
        let secret = "interop-secret-at-least-32-characters-long";
        let token = build_go_style_token(secret, 3600);
        let claims = verify_token(secret.as_bytes(), &token).expect("Go 格式令牌应被 Rust 接受");
        assert_eq!(claims.user_id, 42);
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.role, "user");
        assert_eq!(claims.tenant_id, 1);
        assert_eq!(claims.jti, "1700000000000000000");
    }

    #[test]
    fn expired_go_token_is_rejected() {
        let secret = "interop-secret-at-least-32-characters-long";
        // 已超过默认 leeway=60s 的容忍窗口，确保必被判过期
        let token = build_go_style_token(secret, -120);
        assert!(verify_token(secret.as_bytes(), &token).is_err());
    }
}

/// Verify a signed token and return its claims on success.
pub fn verify_token(secret: &[u8], token: &str) -> AppResult<Claims> {
    // `Validation::default()` 固定 HS256 并校验 exp，等价于 Go 侧
    // `jwt.Parse` 中「仅接受 HMAC 签名方法 + 校验过期」的行为。
    let validation = Validation::default();
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map_err(|_| AppError::unauthorized("invalid or expired token"))?;
    Ok(data.claims)
}
