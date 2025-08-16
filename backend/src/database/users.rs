use bcrypt::{DEFAULT_COST, hash};
use sqlx::{PgPool, Row};

pub async fn get_user_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query("SELECT email, password_hash FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    match row {
        Some(row) => {
            let username: String = row.get("email");
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
    password: &str,
) -> Result<(i32, String, String), sqlx::Error> {
    let password_hash = hash(password, DEFAULT_COST).expect("Failed to hash password");
    let row = sqlx::query(
        r#"
        INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)
        RETURNING id, username, email
    "#,
    )
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    Ok((row.get("id"), row.get("email"), row.get("username")))
}

#[cfg(test)]
mod users_db_test {
    use super::*;
    use dotenv::from_filename;
    use sqlx::PgPool;
    use std::env;

    // Helper function to create an in-memory SQLite database for testing
    async fn setup_test_db() -> PgPool {
        from_filename("../.env").ok();

        let database_url = env::var("DATABASE_URL").expect("Database url not configured");
        // For testing, we'll use SQLite instead of PostgreSQL for simplicity
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to create test database");

        // Create the users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create users table");

        // Clear out test data from previous runs
        sqlx::query("DELETE FROM users where email like '%test%'")
            .execute(&pool)
            .await
            .expect("Error deleting data from 'users' table");

        pool
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
    async fn test_get_user_by_email_success() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "testuser2";
        let test_email = "test2@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm"; // bcrypt hash for "password123"

        // Insert test user
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test
        let result = get_user_by_email(&pool, test_username).await;

        // Assert
        assert!(result.is_ok(), "Expected successful result");
        let user_data = result.unwrap();
        assert!(user_data.is_some(), "Expected user to be found");

        let (username, password_hash) = user_data.unwrap();
        assert_eq!(username, test_username);
        assert_eq!(password_hash, test_password_hash);
    }

    #[tokio::test]
    async fn test_get_user_by_email_not_found() {
        // Setup
        let pool = setup_test_db().await;
        let nonexistent_username = "nonexistent_user";

        // Test
        let result = get_user_by_email(&pool, nonexistent_username).await;

        // Assert
        assert!(
            result.is_ok(),
            "Expected successful result even when user not found"
        );
        let user_data = result.unwrap();
        assert!(user_data.is_none(), "Expected no user to be found");
    }

    #[tokio::test]
    async fn test_get_user_by_email_case_sensitive() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "TestUser";
        let test_email = "test@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Insert test user with specific case
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test with exact case
        let result_exact = get_user_by_email(&pool, "TestUser").await;
        assert!(result_exact.is_ok());
        assert!(result_exact.unwrap().is_some());

        // Test with different case
        let result_different_case = get_user_by_email(&pool, "testuser").await;
        assert!(result_different_case.is_ok());
        assert!(
            result_different_case.unwrap().is_none(),
            "Username search should be case-sensitive"
        );
    }

    #[tokio::test]
    async fn test_get_user_by_email_with_special_characters() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "user@domain.com";
        let test_email = "test4@domain.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Insert test user with special characters
        insert_test_user(&pool, test_username, test_email, test_password_hash).await;

        // Test
        let result = get_user_by_email(&pool, test_username).await;

        // Assert
        assert!(result.is_ok());
        let user_data = result.unwrap();
        assert!(user_data.is_some());

        let (username, _) = user_data.unwrap();
        assert_eq!(username, test_username);
    }

    #[tokio::test]
    async fn test_create_user_and_retrieve() {
        // Setup
        let pool = setup_test_db().await;
        let test_username = "newuser";
        let test_email = "test5@example.com";
        let test_password_hash = "$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj/RK.s5uDfm";

        // Create user using the create_user function
        let create_result = create_user(&pool, test_username, test_email, test_password_hash).await;
        assert!(create_result.is_ok(), "Failed to create user");

        // Retrieve the user
        let result = get_user_by_email(&pool, test_username).await;

        // Assert
        assert!(result.is_ok());
        let user_data = result.unwrap();
        assert!(user_data.is_some());

        let (username, password_hash) = user_data.unwrap();
        assert_eq!(username, test_username);
        assert_eq!(password_hash, test_password_hash);
    }
}
