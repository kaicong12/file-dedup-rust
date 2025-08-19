use crate::services::auth_service::{AuthError, authenticate_user, generate_jwt_token};
use actix_web::{HttpResponse, Responder, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
struct LoginRequestBody {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
    success: bool,
}

#[derive(Serialize)]
struct SuccessResponse {
    token: String,
    success: bool,
    message: String,
    username: String,
}

#[post("/auth/login")]
pub async fn login(
    req_body: web::Json<LoginRequestBody>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let email = &req_body.email;
    let password = &req_body.password;

    // 1. check if user credentials are valid
    // 2. Generate a JWT token and return the token
    // 3. This endpoint will only be hit if JWT is expired
    match authenticate_user(&pool, email, password).await {
        Ok(true) => {
            let token_result = generate_jwt_token(email);
            match token_result {
                Ok(token) => {
                    let success_response = SuccessResponse {
                        token,
                        success: true,
                        message: format!("Welcome: {}", email),
                        username: email.to_owned(),
                    };

                    HttpResponse::Ok().json(success_response)
                }
                Err(auth_error) => {
                    let error_response = ErrorResponse {
                        message: match auth_error {
                            AuthError::TokenGeneration => {
                                "Failed to generate authentication token".to_string()
                            }
                            AuthError::InvalidCredentials => "Invalid credentials".to_string(),
                            AuthError::UserNotFound => "User not found".to_string(),
                            AuthError::InvalidToken => "Invalid token".to_string(),
                        },
                        success: false,
                    };

                    HttpResponse::InternalServerError().json(error_response)
                }
            }
        }
        Ok(false) | Err(_) => {
            let error_response = ErrorResponse {
                message: String::from("Invalid username or password"),
                success: false,
            };

            HttpResponse::Unauthorized().json(error_response)
        }
    }
}
