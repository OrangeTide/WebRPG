//! Shared game-master determination. A session has one owner
//! (`sessions.gm_user_id`) plus any number of additional GMs recorded in
//! `session_gms`. A user has GM privileges if they are the owner or an
//! additional GM. Every GM check in the server routes through here so the rule
//! lives in one place.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

/// The owner (primary GM) of a session, or `None` if the session is missing.
pub fn session_owner_id(conn: &mut SqliteConnection, session_id: i32) -> Option<i32> {
    use crate::schema::sessions;
    sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first::<i32>(conn)
        .optional()
        .ok()
        .flatten()
}

/// The user ids of the additional (non-owner) GMs of a session.
pub fn additional_gm_ids(conn: &mut SqliteConnection, session_id: i32) -> Vec<i32> {
    use crate::schema::session_gms;
    session_gms::table
        .filter(session_gms::session_id.eq(session_id))
        .select(session_gms::user_id)
        .load::<i32>(conn)
        .unwrap_or_default()
}

/// Whether a user has GM privileges in a session: the owner, or an additional
/// GM. Returns `false` on any database error or missing session.
pub fn is_session_gm(conn: &mut SqliteConnection, session_id: i32, user_id: i32) -> bool {
    if session_owner_id(conn, session_id) == Some(user_id) {
        return true;
    }
    use crate::schema::session_gms;
    diesel::select(diesel::dsl::exists(
        session_gms::table
            .filter(session_gms::session_id.eq(session_id))
            .filter(session_gms::user_id.eq(user_id)),
    ))
    .get_result::<bool>(conn)
    .unwrap_or(false)
}

/// Whether a user is the owner (primary GM) of a session. Only the owner may
/// manage the additional GM list.
pub fn is_session_owner(conn: &mut SqliteConnection, session_id: i32, user_id: i32) -> bool {
    session_owner_id(conn, session_id) == Some(user_id)
}
