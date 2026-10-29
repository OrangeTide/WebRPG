use leptos::prelude::*;

use crate::models::UserInfo;

/// Set a JWT authentication cookie on the current response.
#[cfg(feature = "ssr")]
fn set_jwt_cookie(token: &str) -> Result<(), ServerFnError> {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&format!(
            "token={token}; HttpOnly; Path=/; Max-Age=86400; SameSite=Strict"
        ))
        .map_err(|e| ServerFnError::new(e.to_string()))?,
    );
    Ok(())
}

/// Clear the JWT authentication cookie (logout).
#[cfg(feature = "ssr")]
fn clear_jwt_cookie() {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_static(
            "token=; HttpOnly; Path=/; Max-Age=0; SameSite=Strict",
        ),
    );
}

/// Resolve the full `User` row for the request's auth cookie, if any. Returns
/// `Ok(None)` when the request is unauthenticated or the token is invalid, so
/// callers can distinguish "not logged in" from a database error. Server-side
/// helper shared by `get_current_user` and the admin guard.
#[cfg(feature = "ssr")]
pub(crate) async fn current_user_full()
-> Result<Option<crate::models::db_models::User>, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::User;
    use crate::schema::users;
    use diesel::prelude::*;

    let req_parts: axum::http::request::Parts = leptos_axum::extract().await?;

    let cookie_header = match req_parts.headers.get(axum::http::header::COOKIE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Ok(None),
    };

    let token = cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie.strip_prefix("token=")
    });

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    let claims = match auth::verify_jwt(token) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let (user_id, _username) = match auth::parse_claims_sub(&claims.sub) {
        Some(parsed) => parsed,
        None => return Ok(None),
    };

    let conn = &mut db::get_conn();
    users::table
        .find(user_id)
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))
}

#[server]
pub async fn login(username: String, password: String) -> Result<UserInfo, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::User;
    use crate::schema::users;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let user = users::table
        .filter(users::username.eq(&username))
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Invalid username or password"))?;

    if user.locked {
        return Err(ServerFnError::new("Account is locked"));
    }

    let passcrypt = user
        .passcrypt
        .as_deref()
        .ok_or_else(|| ServerFnError::new("Account has no password set"))?;

    let valid = auth::verify_password(passcrypt, &password)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !valid {
        return Err(ServerFnError::new("Invalid username or password"));
    }

    let token = auth::generate_jwt(user.id, &user.username)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    set_jwt_cookie(&token)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
    })
}

#[server]
pub async fn signup(
    username: String,
    password: String,
    display_name: String,
    email: String,
) -> Result<UserInfo, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::{NewUser, User};
    use crate::schema::users;
    use diesel::prelude::*;

    if username.len() < 3 || username.len() > 50 {
        return Err(ServerFnError::new(
            "Username must be between 3 and 50 characters",
        ));
    }
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }
    let email = crate::models::validate_email(&email).map_err(ServerFnError::new)?;

    let conn = &mut db::get_conn();

    // Check if username already exists
    let exists: bool = users::table
        .filter(users::username.eq(&username))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if exists {
        return Err(ServerFnError::new("Username already taken"));
    }

    let hash = auth::hash_password(&password).map_err(|e| ServerFnError::new(e.to_string()))?;

    let new_user = NewUser {
        username: &username,
        display_name: &display_name,
        email: &email,
        passcrypt: &hash,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {e}")))?;

    let user = users::table
        .filter(users::username.eq(&username))
        .select(User::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let token = auth::generate_jwt(user.id, &user.username)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    set_jwt_cookie(&token)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
    })
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    clear_jwt_cookie();
    Ok(())
}

#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    let user = current_user_full().await?;
    Ok(user.map(|u| UserInfo {
        id: u.id,
        username: u.username,
        display_name: u.display_name,
    }))
}
