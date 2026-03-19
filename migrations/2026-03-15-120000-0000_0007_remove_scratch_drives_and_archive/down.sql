-- Restore vfs_files with connection_id and scratch drive support.
CREATE TABLE vfs_files_backup AS SELECT * FROM vfs_files;

DROP TABLE vfs_files;

CREATE TABLE vfs_files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    drive CHAR(1) NOT NULL,
    connection_id VARCHAR(36),
    session_id INTEGER REFERENCES sessions(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    is_directory BOOLEAN NOT NULL DEFAULT FALSE,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_type VARCHAR(100),
    inline_data BLOB,
    media_hash VARCHAR(64),
    modified_by INTEGER REFERENCES users(id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    mode INTEGER NOT NULL DEFAULT 438,
    CHECK (
        (drive IN ('A','B') AND connection_id IS NOT NULL AND session_id IS NULL AND user_id IS NULL) OR
        (drive = 'C' AND session_id IS NOT NULL AND connection_id IS NULL AND user_id IS NULL) OR
        (drive = 'U' AND user_id IS NOT NULL AND connection_id IS NULL AND session_id IS NULL)
    )
);

INSERT INTO vfs_files (id, drive, session_id, user_id, path, is_directory,
    size_bytes, content_type, inline_data, media_hash, modified_by,
    created_at, updated_at, mode)
SELECT id, drive, session_id, user_id, path, is_directory,
    size_bytes, content_type, inline_data, media_hash, modified_by,
    created_at, updated_at, mode
FROM vfs_files_backup;

DROP TABLE vfs_files_backup;

CREATE UNIQUE INDEX idx_vfs_files_scratch ON vfs_files(connection_id, drive, path) WHERE drive IN ('A','B');
CREATE UNIQUE INDEX idx_vfs_files_c ON vfs_files(session_id, path) WHERE drive = 'C';
CREATE UNIQUE INDEX idx_vfs_files_u ON vfs_files(user_id, path) WHERE drive = 'U';
CREATE INDEX idx_vfs_files_connection ON vfs_files(connection_id) WHERE connection_id IS NOT NULL;
CREATE INDEX idx_vfs_files_media ON vfs_files(media_hash) WHERE media_hash IS NOT NULL;

-- Restore archive table
CREATE TABLE vfs_archive (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    original_session_id INTEGER NOT NULL,
    session_name TEXT NOT NULL,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_type VARCHAR(100),
    inline_data BLOB,
    media_hash VARCHAR(64),
    archived_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER NOT NULL
);

CREATE INDEX idx_vfs_archive_expires ON vfs_archive(expires_at);
CREATE INDEX idx_vfs_archive_session ON vfs_archive(original_session_id);
