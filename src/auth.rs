use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::error::Error;

static SECRET_KEY: Lazy<String> =
    Lazy::new(|| std::env::var("SECRET_KEY").expect("SECRET_KEY must be set"));

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> Result<String, Box<dyn Error>> {
    use argon2::password_hash::SaltString;
    use argon2::password_hash::rand_core::OsRng;
    use argon2::{Argon2, PasswordHasher};

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hashing failed: {e}"))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(hash: &str, password: &str) -> Result<bool, Box<dyn Error>> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let parsed_hash = PasswordHash::new(hash).map_err(|e| format!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn generate_jwt(user_id: i32, username: &str) -> Result<String, Box<dyn Error>> {
    use chrono::{Duration, Utc};
    use jsonwebtoken::{EncodingKey, Header, encode};

    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: format!("{}:{}", user_id, username),
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET_KEY.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_jwt(token: &str) -> Result<Claims, Box<dyn Error>> {
    use jsonwebtoken::{DecodingKey, Validation, decode};

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET_KEY.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Parse user_id and username from JWT claims subject ("id:username")
pub fn parse_claims_sub(sub: &str) -> Option<(i32, String)> {
    let mut parts = sub.splitn(2, ':');
    let id = parts.next()?.parse::<i32>().ok()?;
    let username = parts.next()?.to_string();
    Some((id, username))
}

#[cfg(test)]
mod tests {
    use super::parse_claims_sub;

    #[test]
    fn valid_format() {
        assert_eq!(parse_claims_sub("1:alice"), Some((1, "alice".to_string())));
        assert_eq!(parse_claims_sub("42:bob"), Some((42, "bob".to_string())));
    }

    #[test]
    fn username_with_colon() {
        // splitn(2, ':') should keep everything after the first colon
        assert_eq!(
            parse_claims_sub("1:user:name"),
            Some((1, "user:name".to_string()))
        );
    }

    #[test]
    fn missing_colon() {
        assert_eq!(parse_claims_sub("123"), None);
    }

    #[test]
    fn non_numeric_id() {
        assert_eq!(parse_claims_sub("abc:alice"), None);
    }

    #[test]
    fn empty_string() {
        assert_eq!(parse_claims_sub(""), None);
    }

    #[test]
    fn empty_username() {
        // "1:" → id=1, username=""
        assert_eq!(parse_claims_sub("1:"), Some((1, "".to_string())));
    }

    #[test]
    fn negative_id() {
        assert_eq!(
            parse_claims_sub("-1:admin"),
            Some((-1, "admin".to_string()))
        );
    }
}
