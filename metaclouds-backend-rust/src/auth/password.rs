//! 密码哈希与校验。
//!
//! 新密码一律使用 Argon2id 哈希；为了过渡期兼容 Go 版遗留的 bcrypt 密码，
//! `verify_password` 采用双读策略：
//!
//! - 若存储的哈希是 bcrypt 格式（以 `$2a$` / `$2b$` / `$2y$` 开头），用 bcrypt 校验；
//! - 否则按 Argon2 PHC 字符串解析并校验。
//!
//! 这样老用户（Go 侧 bcrypt）与新用户（Rust 侧 argon2）都能登录，
//! 风险登记见 R2。`hash_password` 始终产出 Argon2id，不回退到 bcrypt。

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

use crate::error::AppResult;

/// 判断存储的哈希是否为 bcrypt 格式。
fn is_bcrypt_hash(hash: &str) -> bool {
    hash.starts_with("$2a$") || hash.starts_with("$2b$") || hash.starts_with("$2y$")
}

/// Hash a plaintext password with Argon2id (default params, random salt).
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against a stored hash.
///
/// 自动识别 bcrypt（遗留）与 Argon2（新）两种格式；任一格式解析/校验失败都返回
/// false，不向调用方泄漏具体原因。
pub fn verify_password(password: &str, hash: &str) -> bool {
    if is_bcrypt_hash(hash) {
        return bcrypt::verify(password, hash).unwrap_or(false);
    }
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argon2_roundtrip() {
        let hash = hash_password("hunter2!pass").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password("hunter2!pass", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn bcrypt_legacy_verify() {
        // 一个 bcrypt cost=4 的已知测试向量（"password"），验证老哈希可读。
        let legacy = bcrypt::hash("password", 4).unwrap();
        assert!(is_bcrypt_hash(&legacy));
        assert!(verify_password("password", &legacy));
        assert!(!verify_password("not-password", &legacy));
    }

    #[test]
    fn garbage_hash_does_not_panic() {
        assert!(!verify_password("anything", "not-a-real-hash"));
    }
}
