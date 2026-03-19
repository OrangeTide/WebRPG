-- Remove scratch drive support (connection_id) and vfs_archive table.
-- Scratch drives (A:, B:) are now client-side only (browser IndexedDB).
-- Archive was replaced by standard database backups.

-- Recreate vfs_files without connection_id column and scratch drive support.
CREATE TABLE vfs_files_new (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    drive CHAR(1) NOT NULL,
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
        (drive = 'C' AND session_id IS NOT NULL AND user_id IS NULL) OR
        (drive = 'U' AND user_id IS NOT NULL AND session_id IS NULL)
    )
);

-- Copy only C: and U: drive data (discard any scratch drive rows)
INSERT INTO vfs_files_new (id, drive, session_id, user_id, path, is_directory,
    size_bytes, content_type, inline_data, media_hash, modified_by,
    created_at, updated_at, mode)
SELECT id, drive, session_id, user_id, path, is_directory,
    size_bytes, content_type, inline_data, media_hash, modified_by,
    created_at, updated_at, mode
FROM vfs_files
WHERE drive IN ('C', 'U');

DROP TABLE vfs_files;
ALTER TABLE vfs_files_new RENAME TO vfs_files;

CREATE UNIQUE INDEX idx_vfs_files_c ON vfs_files(session_id, path) WHERE drive = 'C';
CREATE UNIQUE INDEX idx_vfs_files_u ON vfs_files(user_id, path) WHERE drive = 'U';
CREATE INDEX idx_vfs_files_media ON vfs_files(media_hash) WHERE media_hash IS NOT NULL;

-- Drop archive table
DROP TABLE IF EXISTS vfs_archive;
