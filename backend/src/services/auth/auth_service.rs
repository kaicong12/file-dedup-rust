use actix_web::get;

// Normal JWT login flow
// 1. User logs in with credentials, client sends user credentials to the backend, encrypted via https
// 2. The password is compared against a hashed version stored in DB
// 3. If valid, returns a JWT token in the response header, and set this token into local storage
// 4. Client sends this JWT token as Bearer <auth_token> using the Authorization header in future requests
pub fn authenticate_user(username: String, password: String) -> bool {
    false
}
