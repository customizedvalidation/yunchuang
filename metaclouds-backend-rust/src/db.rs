//! Database pool + migrations + default seed data.

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::error::{AppError, AppResult, ErrorCode};

/// Connect to the database described by `url`, run pending migrations and
/// seed the default admin user if the `users` table is empty.
pub async fn connect_and_migrate(url: &str) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(url)
        .map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "invalid DATABASE_URL", e)
        })?
        .create_if_missing(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorCode::InternalServerError,
                "failed to connect to database",
                e,
            )
        })?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorCode::InternalServerError,
                "database migration failed",
                e,
            )
        })?;

    Ok(pool)
}

/// Insert the default admin user if the users table is empty.
pub async fn seed_admin_if_empty(pool: &SqlitePool) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if count > 0 {
        return Ok(());
    }

    let hash = crate::auth::password::hash_password("Admin@123456")?;
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, 'admin', 'admin@example.com', ?3, 'admin', 1)",
    )
    .bind(now)
    .bind(now)
    .bind(hash)
    .execute(pool)
    .await?;
    tracing::info!("seeded default admin user");
    Ok(())
}
