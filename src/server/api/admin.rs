//! Server administration endpoints. Every function here requires the caller to
//! be an administrator (see [`require_admin`]). Admin status is carried by the
//! user's `access_level` column; accounts listed in the `ADMIN_LIST` environment
//! variable are promoted to admin at startup (see [`promote_admins_from_env`]).

use leptos::prelude::*;

use crate::models::{AdminSessionInfo, AdminUserInfo};

/// Resolve the current request's user and require administrator access. Returns
/// the full user row on success, or an error for unauthenticated or non-admin
/// callers. Used as the first line of every admin endpoint.
#[cfg(feature = "ssr")]
async fn require_admin() -> Result<crate::models::db_models::User, ServerFnError> {
    match super::auth::current_user_full().await? {
        Some(u) if crate::models::is_admin_level(u.access_level) => Ok(u),
        Some(_) => Err(ServerFnError::new(
            "Forbidden: administrator access required",
        )),
        None => Err(ServerFnError::new("Not authenticated")),
    }
}

/// Promote every account named in the `ADMIN_LIST` environment variable to
/// administrator access. `ADMIN_LIST` is a comma-separated list of usernames.
/// Promotion is additive: it never demotes accounts, so administrators granted
/// in-app survive a restart, and dropping a name from `ADMIN_LIST` does not
/// revoke that account (revoke it in-app instead). Run once at startup.
#[cfg(feature = "ssr")]
pub fn promote_admins_from_env() {
    use crate::db;
    use crate::models::ACCESS_LEVEL_ADMIN;
    use crate::schema::users;
    use diesel::prelude::*;

    let raw = match std::env::var("ADMIN_LIST") {
        Ok(v) => v,
        Err(_) => return,
    };

    let names: Vec<&str> = raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if names.is_empty() {
        return;
    }

    let conn = &mut db::get_conn();
    let mut promoted = 0usize;
    for name in &names {
        match diesel::update(users::table.filter(users::username.eq(name)))
            .filter(users::access_level.lt(ACCESS_LEVEL_ADMIN))
            .set(users::access_level.eq(ACCESS_LEVEL_ADMIN))
            .execute(conn)
        {
            Ok(n) => promoted += n,
            Err(e) => eprintln!("ADMIN_LIST: failed to promote '{name}': {e}"),
        }
    }
    println!(
        "ADMIN_LIST: {} name(s) configured, {promoted} account(s) promoted to admin",
        names.len()
    );
}

/// Whether the current request is from an administrator. Returns `Ok(false)`
/// (not an error) when the caller is not logged in or not an admin, so the
/// client can quietly gate the admin UI without surfacing an error.
#[server]
pub async fn current_user_is_admin() -> Result<bool, ServerFnError> {
    match super::auth::current_user_full().await? {
        Some(u) => Ok(crate::models::is_admin_level(u.access_level)),
        None => Ok(false),
    }
}

// ===== User accounts =====

/// List every user account. Admin only.
#[server]
pub async fn admin_list_users() -> Result<Vec<AdminUserInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::User;
    use crate::schema::users;
    use diesel::prelude::*;

    require_admin().await?;

    let conn = &mut db::get_conn();
    let rows: Vec<User> = users::table
        .order(users::id.asc())
        .select(User::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|u| AdminUserInfo {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            access_level: u.access_level,
            locked: u.locked,
        })
        .collect())
}

/// Create a new user account. Admin only.
#[server]
pub async fn admin_add_user(
    username: String,
    display_name: String,
    email: String,
    password: String,
) -> Result<AdminUserInfo, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::{NewUser, User};
    use crate::schema::users;
    use diesel::prelude::*;

    require_admin().await?;

    let username = username.trim();
    let display_name = display_name.trim();
    let email = email.trim();
    if username.len() < 3 || username.len() > 50 {
        return Err(ServerFnError::new(
            "Username must be between 3 and 50 characters",
        ));
    }
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }
    if display_name.is_empty() {
        return Err(ServerFnError::new("Display name is required"));
    }

    let conn = &mut db::get_conn();

    let exists: bool = users::table
        .filter(users::username.eq(username))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;
    if exists {
        return Err(ServerFnError::new("Username already taken"));
    }

    let hash = auth::hash_password(&password).map_err(|e| ServerFnError::new(e.to_string()))?;
    let new_user = NewUser {
        username,
        display_name,
        email,
        passcrypt: &hash,
    };
    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {e}")))?;

    let u: User = users::table
        .filter(users::username.eq(username))
        .select(User::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(AdminUserInfo {
        id: u.id,
        username: u.username,
        display_name: u.display_name,
        email: u.email,
        access_level: u.access_level,
        locked: u.locked,
    })
}

/// Update an existing user's display name, email, lock state, and access level.
/// Admin only. An administrator cannot lock their own account or lower their own
/// access level, which guards against accidentally locking every admin out.
#[server]
pub async fn admin_update_user(
    id: i32,
    display_name: String,
    email: String,
    locked: bool,
    access_level: i32,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::users;
    use diesel::prelude::*;

    let me = require_admin().await?;

    let display_name = display_name.trim();
    let email = email.trim();
    if display_name.is_empty() {
        return Err(ServerFnError::new("Display name is required"));
    }

    if id == me.id && (locked || !crate::models::is_admin_level(access_level)) {
        return Err(ServerFnError::new(
            "You cannot lock your own account or remove your own administrator access",
        ));
    }

    let affected = diesel::update(users::table.find(id))
        .set((
            users::display_name.eq(display_name),
            users::email.eq(email),
            users::locked.eq(locked),
            users::access_level.eq(access_level),
        ))
        .execute(&mut db::get_conn())
        .map_err(|e| ServerFnError::new(format!("Failed to update user: {e}")))?;
    if affected == 0 {
        return Err(ServerFnError::new("User not found"));
    }
    Ok(())
}

/// Remove a user account. Admin only. Refuses to remove an account that is still
/// the GM of one or more sessions (archive or remove those sessions first) and
/// refuses self-removal. Deletes the user's personal rows (characters and their
/// resources, session memberships, chat messages, and user-scoped VFS files) in
/// a transaction. Shared content-addressable media is left intact.
#[server]
pub async fn admin_remove_user(id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{
        character_resources, characters, chat_messages, session_players, sessions, users, vfs_files,
    };
    use diesel::prelude::*;

    let me = require_admin().await?;
    if id == me.id {
        return Err(ServerFnError::new("You cannot remove your own account"));
    }

    let conn = &mut db::get_conn();

    let exists: bool = users::table
        .find(id)
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;
    if !exists {
        return Err(ServerFnError::new("User not found"));
    }

    let gm_count: i64 = sessions::table
        .filter(sessions::gm_user_id.eq(id))
        .count()
        .get_result(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
    if gm_count > 0 {
        return Err(ServerFnError::new(format!(
            "This account is the GM of {gm_count} session(s). Remove or reassign those sessions first."
        )));
    }

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let char_ids: Vec<i32> = characters::table
            .filter(characters::user_id.eq(id))
            .select(characters::id)
            .load(conn)?;
        diesel::delete(
            character_resources::table.filter(character_resources::character_id.eq_any(&char_ids)),
        )
        .execute(conn)?;
        diesel::delete(characters::table.filter(characters::user_id.eq(id))).execute(conn)?;
        diesel::delete(session_players::table.filter(session_players::user_id.eq(id)))
            .execute(conn)?;
        diesel::delete(chat_messages::table.filter(chat_messages::user_id.eq(id))).execute(conn)?;
        diesel::delete(vfs_files::table.filter(vfs_files::user_id.eq(Some(id)))).execute(conn)?;
        diesel::delete(users::table.find(id)).execute(conn)?;
        Ok(())
    })
    .map_err(|e| ServerFnError::new(format!("Failed to remove user: {e}")))?;

    Ok(())
}

// ===== Sessions =====

/// List every game session, including archived (inactive) ones. Admin only.
#[server]
pub async fn admin_list_sessions() -> Result<Vec<AdminSessionInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::Session;
    use crate::schema::{session_players, sessions, users};
    use diesel::prelude::*;

    require_admin().await?;

    let conn = &mut db::get_conn();
    let rows: Vec<Session> = sessions::table
        .order(sessions::id.asc())
        .select(Session::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let mut out = Vec::with_capacity(rows.len());
    for s in rows {
        let gm_username: String = users::table
            .find(s.gm_user_id)
            .select(users::username)
            .first(conn)
            .optional()
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
            .unwrap_or_else(|| format!("(missing user {})", s.gm_user_id));
        let player_count: i64 = session_players::table
            .filter(session_players::session_id.eq(s.id))
            .count()
            .get_result(conn)
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
        out.push(AdminSessionInfo {
            id: s.id,
            name: s.name,
            gm_username,
            active: s.active,
            created_at: s.created_at,
            player_count,
        });
    }
    Ok(out)
}

/// Rename a session. Admin only.
#[server]
pub async fn admin_rename_session(id: i32, name: String) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::sessions;
    use diesel::prelude::*;

    require_admin().await?;

    let name = name.trim();
    if name.is_empty() {
        return Err(ServerFnError::new("Session name is required"));
    }

    let affected = diesel::update(sessions::table.find(id))
        .set(sessions::name.eq(name))
        .execute(&mut db::get_conn())
        .map_err(|e| ServerFnError::new(format!("Failed to rename session: {e}")))?;
    if affected == 0 {
        return Err(ServerFnError::new("Session not found"));
    }
    Ok(())
}

/// Archive (deactivate) or unarchive a session. Archived sessions are hidden
/// from the normal session list and cannot be joined, but their data is kept.
/// Admin only.
#[server]
pub async fn admin_set_session_active(id: i32, active: bool) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::sessions;
    use diesel::prelude::*;

    require_admin().await?;

    let affected = diesel::update(sessions::table.find(id))
        .set(sessions::active.eq(active))
        .execute(&mut db::get_conn())
        .map_err(|e| ServerFnError::new(format!("Failed to update session: {e}")))?;
    if affected == 0 {
        return Err(ServerFnError::new("Session not found"));
    }
    Ok(())
}

/// Permanently remove a session and all of its data. Admin only. Because the
/// database does not enforce foreign keys, this deletes every dependent row in a
/// single transaction: token instances, tokens, fog, maps, character resources,
/// characters, creatures, initiative, inventory, chat, memberships, and VFS
/// files scoped to the session, then the session itself.
#[server]
pub async fn admin_remove_session(id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use diesel::prelude::*;

    require_admin().await?;

    let conn = &mut db::get_conn();
    let exists: bool = {
        use crate::schema::sessions;
        sessions::table
            .find(id)
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
            > 0
    };
    if !exists {
        return Err(ServerFnError::new("Session not found"));
    }

    conn.transaction::<_, diesel::result::Error, _>(|conn| cascade_delete_session(conn, id))
        .map_err(|e| ServerFnError::new(format!("Failed to remove session: {e}")))?;

    Ok(())
}

/// Delete a session and every row that depends on it, deepest first. Runs inside
/// a transaction supplied by the caller.
#[cfg(feature = "ssr")]
fn cascade_delete_session(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
) -> Result<(), diesel::result::Error> {
    use crate::schema::*;
    use diesel::prelude::*;

    let map_ids: Vec<i32> = maps::table
        .filter(maps::session_id.eq(session_id))
        .select(maps::id)
        .load(conn)?;
    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::map_id.eq_any(&map_ids))
        .select(tokens::id)
        .load(conn)?;

    diesel::delete(token_instances::table.filter(token_instances::token_id.eq_any(&token_ids)))
        .execute(conn)?;
    diesel::delete(tokens::table.filter(tokens::map_id.eq_any(&map_ids))).execute(conn)?;
    diesel::delete(fog_of_war::table.filter(fog_of_war::map_id.eq_any(&map_ids))).execute(conn)?;
    diesel::delete(maps::table.filter(maps::session_id.eq(session_id))).execute(conn)?;

    let char_ids: Vec<i32> = characters::table
        .filter(characters::session_id.eq(session_id))
        .select(characters::id)
        .load(conn)?;
    diesel::delete(
        character_resources::table.filter(character_resources::character_id.eq_any(&char_ids)),
    )
    .execute(conn)?;
    diesel::delete(characters::table.filter(characters::session_id.eq(session_id)))
        .execute(conn)?;

    diesel::delete(creatures::table.filter(creatures::session_id.eq(session_id))).execute(conn)?;
    diesel::delete(initiative::table.filter(initiative::session_id.eq(session_id)))
        .execute(conn)?;
    diesel::delete(inventory_items::table.filter(inventory_items::session_id.eq(session_id)))
        .execute(conn)?;
    diesel::delete(chat_messages::table.filter(chat_messages::session_id.eq(session_id)))
        .execute(conn)?;
    diesel::delete(session_players::table.filter(session_players::session_id.eq(session_id)))
        .execute(conn)?;
    diesel::delete(vfs_files::table.filter(vfs_files::session_id.eq(Some(session_id))))
        .execute(conn)?;
    diesel::delete(sessions::table.find(session_id)).execute(conn)?;

    Ok(())
}

// ===== Media storage =====

/// Permanently remove a media object from storage. Admin only. Deletes the
/// database row, its tags, and the file on disk. Mirrors `delete_media` but
/// gated to administrators for use from the server administration page.
#[server]
pub async fn admin_delete_media(media_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{media, media_tags};
    use diesel::prelude::*;

    require_admin().await?;

    let conn = &mut db::get_conn();

    let hash: Option<String> = media::table
        .find(media_id)
        .select(media::hash)
        .first::<String>(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
    let hash = match hash {
        Some(h) => h,
        None => return Err(ServerFnError::new("Media not found")),
    };

    diesel::delete(media_tags::table.filter(media_tags::media_id.eq(media_id)))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete tags: {e}")))?;
    diesel::delete(media::table.find(media_id))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete media: {e}")))?;

    // Remove the file: uploads/media/<first two hex chars>/<hash>.
    let media_dir = std::env::var("MEDIA_DIR").unwrap_or_else(|_| "uploads/media".to_string());
    let shard = &hash[..2.min(hash.len())];
    let file_path = std::path::Path::new(&media_dir).join(shard).join(&hash);
    let _ = std::fs::remove_file(&file_path);

    Ok(())
}
