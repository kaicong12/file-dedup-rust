use crate::services::auth::authenticate_user;

// #[get("/auth/login")]
pub fn hello_auth(name: String) -> String {
    let username = String::from("KaiCong");
    let password = String::from("something");
    let is_authenticated = authenticate_user(username, password);
    println!("Is authenticated: {is_authenticated}");
    ["Hello".to_string(), name].concat()
}
