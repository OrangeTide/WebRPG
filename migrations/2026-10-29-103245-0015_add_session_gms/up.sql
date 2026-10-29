-- Additional GMs for a session, beyond the owner in sessions.gm_user_id.
-- A user is a GM of a session if they are the owner or appear here.
CREATE TABLE session_gms (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    UNIQUE(session_id, user_id)
);
