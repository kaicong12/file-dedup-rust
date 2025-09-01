use actix_web::{
    Error, HttpResponse,
    body::{BoxBody, EitherBody},
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::StatusCode,
};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};

use crate::services::auth::{AuthError, verify_jwt_token};

pub struct Auth {
    jwt_secret: String,
}

impl Auth {
    pub fn new(jwt_secret: String) -> Self {
        Auth { jwt_secret }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    // ⬇️ Unify the body type for the whole middleware
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware {
            service,
            jwt_secret: self.jwt_secret.clone(),
        }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
    jwt_secret: String,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    // ⬇️ Must match the Transform::Response type
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let jwt_secret = self.jwt_secret.clone();

        // Extract and verify the authorization token
        let auth_result = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .map(|token| {
                // Remove "Bearer " prefix if present
                let token = if token.starts_with("Bearer ") {
                    &token[7..]
                } else {
                    token
                };
                verify_jwt_token(token, &jwt_secret)
            });

        match auth_result {
            Some(Ok(_claims)) => {
                // Authorized → call next service and map into Left
                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
            Some(Err(auth_error)) => {
                // Handle specific auth errors with appropriate messages
                let error_message = match auth_error {
                    AuthError::InvalidToken => "Invalid or malformed JWT token",
                    AuthError::InvalidCredentials => {
                        "JWT token verification failed - invalid signature or expired token"
                    }
                    AuthError::UserNotFound => "User not found",
                    AuthError::TokenGeneration => "Token generation error",
                };

                let res = req.into_response(HttpResponse::build(StatusCode::UNAUTHORIZED).json(
                    serde_json::json!({
                        "error": "Unauthorized",
                        "message": error_message
                    }),
                ));
                Box::pin(async move { Ok(res.map_into_right_body()) })
            }
            None => {
                // No Authorization header provided
                let res = req.into_response(HttpResponse::build(StatusCode::UNAUTHORIZED).json(
                    serde_json::json!({
                        "error": "Unauthorized",
                        "message": "Authorization header missing"
                    }),
                ));
                Box::pin(async move { Ok(res.map_into_right_body()) })
            }
        }
    }
}
