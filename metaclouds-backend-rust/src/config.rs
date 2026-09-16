//! Application configuration loaded from environment variables.
//!
//! Loads `.env` via `dotenvy` (if present) and validates required values.

use std::time::Duration;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expires: Duration,
    pub log_level: String,
}

impl Config {
    /// Load and validate configuration from the process environment.
    ///
    /// Panics on misconfiguration at startup (fail fast).
    pub fn from_env() -> AppResult<Self> {
        // Best-effort `.env` load; missing file is fine in production.
        let _ = dotenvy::dotenv();

        let server_host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let server_port: u16 = std::env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8001);

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| AppError::bad_request("DATABASE_URL must be set"))?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| AppError::bad_request("JWT_SECRET must be set"))?;
        if jwt_secret.len() < 32 {
            return Err(AppError::bad_request(
                "JWT_SECRET must be at least 32 characters",
            ));
        }

        let jwt_expires_secs: u64 = std::env::var("JWT_EXPIRES_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(86_400);

        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());

        Ok(Self {
            server_host,
            server_port,
            database_url,
            jwt_secret,
            jwt_expires: Duration::from_secs(jwt_expires_secs),
            log_level,
        })
    }

    /// Socket address for axum to bind to.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
