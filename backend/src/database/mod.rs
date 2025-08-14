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

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use sqlx::PgPool;
    use std::env;

    // Helper function to create an in-memory SQLite database for testing
    async fn setup_test_db() -> PgPool {
        let database_url = env::var("DATABASE_URL").expect("Database url not configured");
        // For testing, we'll use SQLite instead of PostgreSQL for simplicity
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to create test database");

        // Create the users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create users table");

        pool
    }

    async fn cleanup_test_data(pool: &PgPool) {
        sqlx::query("DELETE from users")
            .execute(pool)
            .await
            .expect("Error cleaning up test data");
    }

    // Helper function to insert a test user
    async fn insert_test_user(pool: &PgPool, username: &str, email: &str, password_hash: &str) {
        sqlx::query("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)")
            .bind(username)
            .bind(email)
            .bind(password_hash)
            .execute(pool)
            .await
            .expect("Failed to insert test user");
    }

    #[tokio::test]
    async fn test_get_user_by_username_success() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "testuser2";
        let test_email = "test2@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm"; // bcrypt hash for "password123"

        // Insert test user
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test
        let result = get_user_by_username(&pool, test_username).await;

        // Assert
        assert!(result.is_ok(), "Expected successful result");
        let user_data = result.unwrap();
        assert!(user_data.is_some(), "Expected user to be found");

        let (username, password_hash) = user_data.unwrap();
        assert_eq!(username, test_username);
        assert_eq!(password_hash, test_password_hash);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_get_user_by_username_not_found() {
        // Setup
        let pool = setup_test_db().await;
        let nonexistent_username = "nonexistent_user";

        // Test
        let result = get_user_by_username(&pool, nonexistent_username).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected successful result even when user not found"
        );
        let user_data = result.unwrap();
        assert!(user_data.is_none(), "Expected no user to be found");

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_get_user_by_username_case_sensitive() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "TestUser";
        let test_email = "test@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Insert test user with specific case
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test with exact case
        let result_exact = get_user_by_username(&pool, "TestUser").await;
        assert!(result_exact.is_ok());
        assert!(result_exact.unwrap().is_some());

        // Test with different case
        let result_different_case = get_user_by_username(&pool, "testuser").await;
        assert!(result_different_case.is_ok());
        assert!(
            result_different_case.unwrap().is_none(),
            "Username search should be case-sensitive"
        );

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_get_user_by_username_with_special_characters() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "user@domain.com";
        let test_email = "user@domain.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Insert test user with special characters
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test
        let result = get_user_by_username(&pool, test_username).await;

        // Assert
        assert!(result.is_ok());
        let user_data = result.unwrap();
        assert!(user_data.is_some());

        let (username, _) = user_data.unwrap();
        assert_eq!(username, test_username);

        cleanup_test_data(&pool).await;
    }

    #[tokio::test]
    async fn test_get_user_by_username_empty_string() {
        // Setup
        let pool = setup_test_db().await;

        // Test with empty string
        let result = get_user_by_username(&pool, "").await;

        // Assert
        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "Empty username should not match any user"
        );
    }

    #[tokio::test]
    async fn test_create_user_and_retrieve() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "newuser";
        let test_email = "newuser@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Create user using the create_user function
        let create_result = create_user(&pool, test_username, test_email, test_password_hash).await;
        assert!(create_result.is_ok(), "Failed to create user");

        // Retrieve the user
        let result = get_user_by_username(&pool, test_username).await;

        // Assert
        assert!(result.is_ok());
        let user_data = result.unwrap();
        assert!(user_data.is_some());

        let (username, password_hash) = user_data.unwrap();
        assert_eq!(username, test_username);
        assert_eq!(password_hash, test_password_hash);
    }
}
