use argon2::{self | Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation};
use rand::Rng;
use chrono::{Utc, Duration};
use std::error::Error;
mod secrets {
    use once_cell::sync::Lazy;

    // Private static variable
    static SECRET_KEY: Lazy<String> = Lazy::new(|| {
        std::env::var("SECRET_KEY").expect("SECRET_KEY not set")
    });

    // Public getter (controlled access)
    pub fn get_api_key() -> &'static str {
        &SECRET_KEY
    }
}
pub fn hash_password(password: &str) -> Result<String, Box<dyn Error>> {
    let salt: [u8; 16] = rand::thread_rng().gen();
    let config = argon2::Config::default();
    let hash = Argon2::default().hash_password(password.as_bytes(), &salt, &config)?;
    Ok(hash.to_string())
}
pub fn verify_password(hash: &str, password: &str) -> Result<bool, Box<dyn Error>> {
    let parsed_hash = PasswordHash::new(hash)?;
    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
}
pub fn generate_jwt(username: &str) -> Result<String, Box<dyn Error>> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;
    let claims = Claims {
        sub: username.to_string(),
        exp: expiration,
    };
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET_KEY))?;
    Ok(token)
}
pub fn verify_jwt(token: &str) -> Result<Claims, Box<dyn Error>> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET_KEY),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
