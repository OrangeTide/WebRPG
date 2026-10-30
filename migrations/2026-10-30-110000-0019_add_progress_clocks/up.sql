-- Progress clocks: session-persistent segmented clocks, each shown in its own
-- window. The GM creates and advances them; players see them update live.
CREATE TABLE progress_clocks (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    label VARCHAR(255) NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    segments INTEGER NOT NULL,
    filled INTEGER NOT NULL DEFAULT 0,
    filled_color VARCHAR(32) NOT NULL DEFAULT '#cc4444',
    unfilled_color VARCHAR(32) NOT NULL DEFAULT '#2a2a2a',
    locked BOOLEAN NOT NULL DEFAULT 0,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
