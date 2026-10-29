use leptos::prelude::*;

use crate::models::SessionInfo;

#[cfg(feature = "ssr")]
use super::get_current_user;

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
    use crate::schema::{session_players, sessions};
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
pub async fn create_map(
    session_id: i32,
    name: String,
    width: i32,
    height: i32,
    cell_size: Option<i32>,
    background_url: Option<String>,
) -> Result<crate::models::MapInfo, ServerFnError> {
    use crate::db;
    use crate::schema::{maps, sessions};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    let gm_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if gm_id != user.id {
        return Err(ServerFnError::new("Only the GM can create maps"));
    }

    let cell = cell_size.unwrap_or(40).clamp(10, 200);

    let new_map = crate::models::db_models::NewMap {
        session_id,
        name: &name,
        width,
        height,
    };

    diesel::insert_into(maps::table)
        .values(&new_map)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create map: {e}")))?;

    let map_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Failed to get map id: {e}")))?;

    // Update cell_size and background if non-default
    let _ = diesel::update(maps::table.find(map_id))
        .set((
            maps::cell_size.eq(cell),
            maps::background_url.eq(&background_url),
        ))
        .execute(conn);

    let map = maps::table
        .find(map_id)
        .select(crate::models::db_models::Map::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(crate::models::MapInfo {
        id: map.id,
        name: map.name,
        width: map.width,
        height: map.height,
        cell_size: map.cell_size,
        background_url: map.background_url,
        default_token_color: map.default_token_color,
    })
}

#[server]
pub async fn list_maps(session_id: i32) -> Result<Vec<crate::models::MapInfo>, ServerFnError> {
    use crate::db;
    use crate::schema::maps;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    let rows = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.asc())
        .select(crate::models::db_models::Map::as_select())
        .load::<crate::models::db_models::Map>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|m| crate::models::MapInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            cell_size: m.cell_size,
            background_url: m.background_url,
            default_token_color: m.default_token_color,
        })
        .collect())
}

#[server]
pub async fn delete_map(map_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{fog_of_war, maps, sessions, token_instances, tokens};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    // Verify GM
    let session_id: i32 = maps::table
        .find(map_id)
        .select(maps::session_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Map not found"))?;

    let gm_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if gm_id != user.id {
        return Err(ServerFnError::new("Only the GM can delete maps"));
    }

    // Delete token instances for tokens on this map
    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::map_id.eq(map_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();

    for tid in &token_ids {
        let _ = diesel::delete(token_instances::table.filter(token_instances::token_id.eq(tid)))
            .execute(conn);
    }

    // Delete tokens, fog, and the map
    let _ = diesel::delete(tokens::table.filter(tokens::map_id.eq(map_id))).execute(conn);
    let _ = diesel::delete(fog_of_war::table.filter(fog_of_war::map_id.eq(map_id))).execute(conn);
    let _ = diesel::delete(maps::table.find(map_id)).execute(conn);

    Ok(())
}

#[server]
pub async fn get_ws_token() -> Result<String, ServerFnError> {
    let req_parts: axum::http::request::Parts = leptos_axum::extract().await?;

    let cookie_header = match req_parts.headers.get(axum::http::header::COOKIE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Err(ServerFnError::new("Not logged in")),
    };

    let token = cookie_header.split(';').find_map(|cookie| {
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
