use sqlx::{PgPool, Row};
use std::env;

pub async fn create_connection_pool() -> Result<PgPool, sqlx::Error> {
    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment variables");

    PgPool::connect(&database_url).await
}

pub async fn init_database(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create users table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(255) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query("SELECT username, password_hash FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => {
            let username: String = row.get("username");
            let password_hash: String = row.get("password_hash");
            Ok(Some((username, password_hash)))
        }
        None => Ok(None),
    }
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)")
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .execute(pool)
        .await?;

    Ok(())
}
