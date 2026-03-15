# Feature 34: Virtual File System

Add a virtual file system with global shared storage and private per-user storage. Use a drive-letter system inspired by DOS/CP/M with Unix-style path separators (e.g., `C:/maps/dungeon.png`).

### Drive Letters

- **A:, B:** — Temporary scratch drives (10 MB each). Per-connection — contents disappear when the connection drops. Useful for intermediate computation, clipboard-style scratch space, etc.
- **C:** — Shared read/write drive for the game session (100 MB). All players and the GM can read and write. Persists for the lifetime of the game. When a game is deleted, the C: contents are archived and held for 30 days, accessible by a server administrator for recovery.
- **U:** — Private per-user storage (10 MB for non-GM users, 20 MB for GMs). Tied to the user's account, not a specific session. Accessible from any game session and (in the future) from an out-of-game utility page.
- **D: through T:** — Reserved for future use (e.g., GM-only drives, read-only asset libraries).

### Storage Strategy

- **Small files** (up to a few KB) are stored inline in the database for simplicity and fast access.
- **Large files** are stored in the existing content-addressable storage, with the VFS holding a metadata reference (hash, size, MIME type).

### Path Syntax

Paths use `/` as the separator (Unix-style), not `\`. Examples:
- `U:/macros/fireball.txt`
- `C:/maps/level1.png`
- `A:/scratch/temp.dat`

This provides an in-game file abstraction for organizing assets, scripts, character data, and other content within sessions.

## Dependencies

(none)

## Status: Not Started

## Plan

### Drive Lifecycle & Limits

| Drive | Scope | Limit | Lifetime |
|-------|-------|-------|----------|
| A:, B: | Per-connection | 10 MB each | Disappears when connection drops |
| C: | Per-game session | 100 MB | Persists until game deleted; archived 30 days for admin recovery |
| U: | Per-user account | 10 MB (non-GM), 20 MB (GM) | Persists with account; accessible from any session or future utility page |

### Storage Architecture

- **A:, B: scratch drives** — in-memory only, no database tables. Managed by the WebSocket connection handler. Cleaned up automatically on disconnect.
- **C:, U: persistent drives** — metadata in `vfs_files` table, with small files inline and large files in content-addressable storage.
- **Inline threshold** — files ≤ 8 KB stored as BLOBs in `vfs_files.inline_data`. Larger files stored via CAS with `media_hash` reference. This threshold may need tuning with a benchmark in the future.
- **Archive** — when a game is deleted, C: drive contents are copied to `vfs_archive` with a 30-day expiry. A server admin can recover them.

### Database Schema

```sql
-- Persistent VFS files (C: and U: drives)
CREATE TABLE vfs_files (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    drive CHAR(1) NOT NULL,                -- 'C' or 'U'
    session_id INTEGER REFERENCES sessions(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id) ON DELETE CASCADE,
    path TEXT NOT NULL,                     -- e.g. '/maps/dungeon.png'
    is_directory BOOLEAN NOT NULL DEFAULT FALSE,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    content_type VARCHAR(100),
    inline_data BLOB,                       -- small files (≤ 8 KB, tunable)
    media_hash VARCHAR(64),                 -- large files → content-addressable storage
    modified_by INTEGER REFERENCES users(id),  -- last user to modify (for shared drives)
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    -- C: requires session_id; U: requires user_id
    CHECK (
        (drive = 'C' AND session_id IS NOT NULL AND user_id IS NULL) OR
        (drive = 'U' AND user_id IS NOT NULL AND session_id IS NULL)
    )
);

-- Each path must be unique within its drive scope
CREATE UNIQUE INDEX idx_vfs_files_c ON vfs_files(session_id, path) WHERE drive = 'C';
CREATE UNIQUE INDEX idx_vfs_files_u ON vfs_files(user_id, path) WHERE drive = 'U';
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
    expires_at INTEGER NOT NULL              -- archived_at + 30 days
);

CREATE INDEX idx_vfs_archive_expires ON vfs_archive(expires_at);
CREATE INDEX idx_vfs_archive_session ON vfs_archive(original_session_id);
```

### Implementation Steps

1. Create Diesel migration with the schema above
2. Add Diesel models and schema module for `vfs_files` and `vfs_archive`
3. Build a `vfs` Rust module with:
   - Path parser (drive letter + Unix path normalization)
   - Drive trait/enum for abstracting in-memory vs DB-backed drives
   - CRUD operations (create, read, update, delete, list, stat)
   - Size quota enforcement per drive
   - Inline vs CAS threshold logic
4. In-memory scratch drive implementation (A:, B:) tied to WS connection state
5. Archive logic: on game deletion, copy C: files to `vfs_archive` with 30-day expiry
6. Cleanup job: periodically purge expired `vfs_archive` rows
7. Server functions or WebSocket messages for file operations
8. UI file browser (future — can defer to a later iteration)

## Findings

- Existing `media` table uses content-addressable storage with `hash` as the unique key. VFS large files will reference `media.hash`.
- The `media` table tracks `uploaded_by`, `content_type`, and `size_bytes` — VFS can reuse these for large file metadata.
- SQLite partial unique indexes (`WHERE drive = 'C'`) are supported and will enforce path uniqueness per drive scope.
- A:/B: scratch drives don't need database backing — in-memory `HashMap<String, Vec<u8>>` per connection is sufficient given the 10 MB limit.
