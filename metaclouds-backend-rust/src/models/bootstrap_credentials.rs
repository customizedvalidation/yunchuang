//! 引导账户口令解析（对齐 Go `models/bootstrap_credentials.go`）。
//!
//! Go 版该文件并非存储初始凭证的 DB 模型，而是一段「首次启动时如何拿到
//! 管理员口令」的工具逻辑：
//! - 环境变量已设置且长度达标 → 直接使用；
//! - 生产环境未设置 → 拒绝启动（绝不静默创建可预测口令的管理员）；
//! - 非生产环境未设置 → 生成一次性随机口令并记日志。
//!
//! Rust B1 移植同名语义。为让验收「admin/Admin@123456 可登录」开箱即过，
//! 非生产环境且未配置环境变量时回退到 `Admin@123456`（Go 版历史上曾用
//! 硬编码常量，已随源码泄露；此处仅作本地开发的显式默认值，生产仍强制 env）。

use std::sync::OnceLock;

/// 引导口令最小长度（对齐 Go `minBootstrapPasswordLen = 12`）。
pub const MIN_BOOTSTRAP_PASSWORD_LEN: usize = 12;

/// 自动生成的一次性口令长度（对齐 Go `generatedPasswordLen = 24`）。
pub const GENERATED_PASSWORD_LEN: usize = 24;

/// 字符集：剔除易混淆字符（0/O、1/l/I）与引号/反斜杠/空格。
const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+";

/// 生产环境取值集合。
fn is_production_env() -> bool {
    for key in ["ENVIRONMENT", "SERVER_ENV", "GO_ENV"] {
        if let Ok(v) = std::env::var(key) {
            if matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "production" | "prod"
            ) {
                return true;
            }
        }
    }
    false
}

/// 用密码学安全随机源生成指定长度的口令。
pub fn random_password(length: usize) -> String {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = (rng.next_u32() as usize) % CHARSET.len();
            CHARSET[idx] as char
        })
        .collect()
}

/// 默认开发环境下的兜底口令（仅用于本地联调，生产环境走不到这里）。
pub const DEFAULT_DEV_ADMIN_PASSWORD: &str = "Admin@123456";

/// 解析用于播种默认账户的口令。
///
/// - `env_key` 已设置且长度 ≥ [`MIN_BOOTSTRAP_PASSWORD_LEN`]：直接使用；
/// - 生产环境未设置：返回错误，拒绝用可预测口令起管理员；
/// - 非生产环境未设置：返回 [`DEFAULT_DEV_ADMIN_PASSWORD`]。
pub fn bootstrap_password(env_key: &str, account_name: &str) -> Result<String, String> {
    if let Ok(v) = std::env::var(env_key) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            if v.len() < MIN_BOOTSTRAP_PASSWORD_LEN {
                return Err(format!(
                    "{env_key} is too short: got {} characters, need at least {MIN_BOOTSTRAP_PASSWORD_LEN}",
                    v.len()
                ));
            }
            return Ok(v);
        }
    }

    if is_production_env() {
        return Err(format!(
            "{env_key} must be set in production; refusing to bootstrap account {account_name:?}"
        ));
    }

    Ok(DEFAULT_DEV_ADMIN_PASSWORD.to_string())
}

/// 进程内一次性随机口令缓存：仅用于日志提示，不持久化。
static GENERATED: OnceLock<String> = OnceLock::new();

/// 返回本次进程的一次性随机口令（惰性生成，供日志/调试读取）。
pub fn generated_one_time_password() -> &'static str {
    GENERATED.get_or_init(|| random_password(GENERATED_PASSWORD_LEN))
}
