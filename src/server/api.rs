use leptos::prelude::*;

use crate::models::{SessionInfo, UserInfo};

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

    let valid =
        auth::verify_password(passcrypt, &password).map_err(|e| ServerFnError::new(e.to_string()))?;

    if !valid {
        return Err(ServerFnError::new("Invalid username or password"));
    }

    let token =
        auth::generate_jwt(user.id, &user.username).map_err(|e| ServerFnError::new(e.to_string()))?;

    // Set JWT as HttpOnly cookie
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&format!(
            "token={token}; HttpOnly; Path=/; Max-Age=86400; SameSite=Strict"
        ))
        .map_err(|e| ServerFnError::new(e.to_string()))?,
    );

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
        return Err(ServerFnError::new(
            "Password must be at least 8 characters",
        ));
    }

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

    let token =
        auth::generate_jwt(user.id, &user.username).map_err(|e| ServerFnError::new(e.to_string()))?;

    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&format!(
            "token={token}; HttpOnly; Path=/; Max-Age=86400; SameSite=Strict"
        ))
        .map_err(|e| ServerFnError::new(e.to_string()))?,
    );

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
    })
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_static(
            "token=; HttpOnly; Path=/; Max-Age=0; SameSite=Strict",
        ),
    );
    Ok(())
}

#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
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

    let token = cookie_header
        .split(';')
        .find_map(|cookie| {
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
    let user = users::table
        .find(user_id)
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(user.map(|u| UserInfo {
        id: u.id,
        username: u.username,
        display_name: u.display_name,
    }))
}

#[server]
pub async fn list_sessions() -> Result<Vec<SessionInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::Session;
    use crate::schema::{sessions, users};
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let results: Vec<(Session, String)> = sessions::table
        .inner_join(users::table.on(users::id.eq(sessions::gm_user_id)))
        .filter(sessions::active.eq(true))
        .select((Session::as_select(), users::username))
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(results
        .into_iter()
        .map(|(session, gm_username)| SessionInfo {
            id: session.id,
            name: session.name,
            gm_username,
            active: session.active,
        })
        .collect())
}

#[server]
pub async fn create_session(name: String) -> Result<SessionInfo, ServerFnError> {
    use crate::db;
    use crate::models::db_models::NewSession;
    use crate::schema::{sessions, session_players};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let new_session = NewSession {
        name: &name,
        gm_user_id: user.id,
        template_id: None,
    };

    diesel::insert_into(sessions::table)
        .values(&new_session)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {e}")))?;

    let session_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    // Add GM as session player
    diesel::insert_into(session_players::table)
        .values(&crate::models::db_models::NewSessionPlayer {
            session_id,
            user_id: user.id,
            role: "gm".to_string(),
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to add GM to session: {e}")))?;

    Ok(SessionInfo {
        id: session_id,
        name,
        gm_username: user.username,
        active: true,
    })
}

#[server]
pub async fn get_ws_token() -> Result<String, ServerFnError> {
    let req_parts: axum::http::request::Parts = leptos_axum::extract().await?;

    let cookie_header = match req_parts.headers.get(axum::http::header::COOKIE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Err(ServerFnError::new("Not logged in")),
    };

    let token = cookie_header
        .split(';')
        .find_map(|cookie| {
            let cookie = cookie.trim();
            cookie.strip_prefix("token=")
        });

    match token {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(ServerFnError::new("Not logged in")),
    }
}

#[server]
pub async fn join_session(session_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{session_players, sessions};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    // Check session exists and is active
    let exists = sessions::table
        .find(session_id)
        .filter(sessions::active.eq(true))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if !exists {
        return Err(ServerFnError::new("Session not found or inactive"));
    }

    // Check if already a member
    let already_member = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user.id))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if !already_member {
        diesel::insert_into(session_players::table)
            .values(&crate::models::db_models::NewSessionPlayer {
                session_id,
                user_id: user.id,
                role: "player".to_string(),
            })
            .execute(conn)
            .map_err(|e| ServerFnError::new(format!("Failed to join session: {e}")))?;
    }

    Ok(())
}
