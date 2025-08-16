use crate::database::users::{create_user, get_user_by_username};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::PgPool;

// Normal JWT login flow
// 1. User logs in with credentials, client sends user credentials to the backend, encrypted via https
// 2. The password is compared against a hashed version stored in DB
// 3. If valid, returns a JWT token in the response header, and set this token into local storage
// 4. Client sends this JWT token as Bearer <auth_token> using the Authorization header in future requests
#[derive(Serialize, Deserialize)]
struct Claims {
    username: String,
    issued_at: DateTime<Utc>,
    expiration: u64, // minutes since created at before token expiration
}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    UserNotFound,
    TokenGeneration,
    InvalidToken,
}

pub async fn authenticate_user(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<bool, AuthError> {
    // Get user from database
    match get_user_by_username(pool, username).await {
        Ok(Some((_, password_hash))) => {
            // Verify password using bcrypt
            match bcrypt::verify(password, &password_hash) {
                Ok(is_valid) => Ok(is_valid),
                Err(_) => Err(AuthError::InvalidCredentials),
            }
        }
        Ok(None) => Err(AuthError::UserNotFound),
        Err(_) => Err(AuthError::InvalidCredentials),
    }
}

pub async fn create_user_account(
    pool: &PgPool,
    username: &str,
    email: &str,
    password: &str,
) -> Result<(), AuthError> {
    // Hash the password
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|_| AuthError::InvalidCredentials)?;

    // Create user in database
    create_user(pool, username, email, &password_hash)
        .await
        .map_err(|_| AuthError::InvalidCredentials)?;

    Ok(())
}

fn verify_jwt_token(token: &str) -> Result<Claims, AuthError> {
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT Secret must be specified");
    let key: Hmac<Sha256> =
        Hmac::new_from_slice(jwt_secret.as_bytes()).map_err(|_| AuthError::InvalidToken)?;

    let claims: Claims = token
        .verify_with_key(&key)
        .map_err(|_| AuthError::InvalidToken)?;

    Ok(claims)
}

pub fn generate_jwt_token(username: &str) -> Result<String, AuthError> {
    let claims = Claims {
        username: username.to_string(),
        issued_at: Utc::now(),
        expiration: 180,
    };

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT Secret must be specified");
    let key: Hmac<Sha256> =
        Hmac::new_from_slice(jwt_secret.as_bytes()).map_err(|_| AuthError::TokenGeneration)?;

    let token_str = claims
        .sign_with_key(&key)
        .map_err(|_| AuthError::TokenGeneration)?;

    Ok(token_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;

    #[test]
    fn test_generate_jwt_token_success() {
        dotenv().ok();

        let username = String::from("KaiCong");
        let token = generate_jwt_token(&username);
        assert!(token.is_ok());

        let token = token.unwrap();
        let token_str = token.as_str();
        let claim_result = verify_jwt_token(token_str);
        assert!(claim_result.is_ok());

        let verified_claim = claim_result.unwrap();
        assert_eq!(verified_claim.username, String::from("KaiCong"));
    }
}
