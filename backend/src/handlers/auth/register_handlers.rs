use crate::database::users::create_user;
use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

enum RegisterPayloadError {
    InvalidEmail,
    InvalidUserName,
    WeakPassword,
}

fn validate_register_payload(
    username: &str,
    email: &str,
    password: &str,
) -> Result<(), RegisterPayloadError> {
    // Username: 3-32 chars
    if username.len() < 3 || username.len() > 32 {
        return Err(RegisterPayloadError::InvalidUserName);
    }
    // Email: basic check for '@'
    if !email.contains('@') || !email.contains('.') {
        return Err(RegisterPayloadError::InvalidEmail);
    }
    // Password: at least 8 chars
    if password.len() < 8 {
        return Err(RegisterPayloadError::WeakPassword);
    }

    Ok(())
}

#[post("/auth/register")]
pub async fn register_user(
    req_body: web::Json<RegisterRequest>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let username = &req_body.username;
    let password = &req_body.password;
    let email = &req_body.email;

    if let Err(validate_error) = validate_register_payload(username, email, password) {
        let error_message = match validate_error {
            RegisterPayloadError::InvalidEmail => "Invalid Email",
            RegisterPayloadError::InvalidUserName => "Invalid Username",
            RegisterPayloadError::WeakPassword => "Weak password",
        };

        return HttpResponse::BadRequest().json(error_message);
    }

    match create_user(&pool, username, email, password).await {
        Ok(_) => HttpResponse::Created().body("Sucess"),
        Err(msg) => HttpResponse::InternalServerError().json(msg.to_string()),
    }
}
