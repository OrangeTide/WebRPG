-- Virtual File System tables (Feature 34)

CREATE TABLE vfs_files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    drive CHAR(1) NOT NULL,                -- 'A', 'B', 'C', or 'U'
    connection_id VARCHAR(36),             -- for A:/B: scratch drives (UUID)
    session_id INTEGER REFERENCES sessions(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,                     -- e.g. '/maps/dungeon.png'
    is_directory BOOLEAN NOT NULL DEFAULT FALSE,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_type VARCHAR(100),
    inline_data BLOB,                       -- small files (≤ 8 KB)
    media_hash VARCHAR(64),                 -- large files → content-addressable storage
    modified_by INTEGER REFERENCES users(id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    CHECK (
        (drive IN ('A','B') AND connection_id IS NOT NULL AND session_id IS NULL AND user_id IS NULL) OR
        (drive = 'C' AND session_id IS NOT NULL AND connection_id IS NULL AND user_id IS NULL) OR
        (drive = 'U' AND user_id IS NOT NULL AND connection_id IS NULL AND session_id IS NULL)
    )
);

-- Each path must be unique within its drive scope
CREATE UNIQUE INDEX idx_vfs_files_scratch ON vfs_files(connection_id, drive, path) WHERE drive IN ('A','B');
CREATE UNIQUE INDEX idx_vfs_files_c ON vfs_files(session_id, path) WHERE drive = 'C';
CREATE UNIQUE INDEX idx_vfs_files_u ON vfs_files(user_id, path) WHERE drive = 'U';
CREATE INDEX idx_vfs_files_connection ON vfs_files(connection_id) WHERE connection_id IS NOT NULL;
CREATE INDEX idx_vfs_files_media ON vfs_files(media_hash) WHERE media_hash IS NOT NULL;

-- Archived C: drive contents (retained 30 days after game deletion)
CREATE TABLE vfs_archive (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    original_session_id INTEGER NOT NULL,
    session_name TEXT NOT NULL,             -- snapshot of session name at deletion time
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_type VARCHAR(100),
    inline_data BLOB,
    media_hash VARCHAR(64),
    archived_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL             -- archived_at + 30 days
);

CREATE INDEX idx_vfs_archive_expires ON vfs_archive(expires_at);
CREATE INDEX idx_vfs_archive_session ON vfs_archive(original_session_id);
