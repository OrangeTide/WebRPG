use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub displayName: String,
    pub passcrypt: Nullable<String>,
    pub email: String,
    pub accessLevel: i32,
    pub locked: bool,
}
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
}
#[derive(Debug, Serialize)]
pub struct Claims {
    pub sub: String, // Subject (username)
    pub exp: usize,  // Expiration timestamp
}
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
}
