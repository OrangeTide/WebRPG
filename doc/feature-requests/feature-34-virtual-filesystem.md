# Feature 34: Virtual File System

Add a virtual file system with global shared storage and private per-user storage. Use a drive-letter system inspired by DOS and CP/M with Unix-style path separators (e.g., `C:/maps/dungeon.png`).

### Drive Letters

- **A:, B:** — Temporary scratch drives (10 MB each). Per-tab, stored client-side in browser IndexedDB. Contents disappear when the tab closes. Useful for intermediate computation, clipboard-style scratch space, etc.
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

### User Interface

See Feature 36 (VFS Terminal Shell) and Feature 37 (VFS File Browser) for UI access.

## Dependencies

(none)

## Status: In Progress

## Plan

### Drive Lifecycle & Limits

| Drive | Scope | Storage | Limit | Lifetime |
|-------|-------|---------|-------|----------|
| A:, B: | Per-tab | Browser IndexedDB | 10 MB each | Disappears when tab closes |
| C: | Per-game session | Server DB + CAS | 100 MB | Persists until game deleted; archived 30 days |
| U: | Per-user account | Server DB + CAS | 10/20 MB (GM) | Persists with account |

### Storage Architecture

- **A:, B: scratch drives** — stored entirely client-side in browser IndexedDB. Scoped per-tab using a random session key. No server involvement. Quota enforced client-side.
- **C:, U: persistent drives** — metadata in `vfs_files` table, with small files inline and large files in content-addressable storage.
- **Inline threshold** — files ≤ 8 KB stored as BLOBs in `vfs_files.inline_data`. Larger files stored via CAS with `media_hash` reference. This threshold may need tuning with a benchmark in the future.
- **Archive** — when a game is deleted, C: drive contents are copied to `vfs_archive` with a 30-day expiry. A server admin can recover them.

### Database Schema

```sql
-- Persistent VFS files (C: and U: drives only — scratch drives are client-side)
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
    mode INTEGER NOT NULL DEFAULT 438,      -- Unix permissions (438 = 0o666)
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    CHECK (
        (drive = 'C' AND session_id IS NOT NULL AND user_id IS NULL) OR
        (drive = 'U' AND user_id IS NOT NULL AND session_id IS NULL)
    )
);

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

Note: The actual migration still includes `connection_id` and scratch drive support. The next implementation step (step 5) is a cleanup migration to remove these columns.

### Permissions

- Unix-style `rwx` permission bits stored as `mode` column (integer bitmask)
- Owner = GM, other = everyone else in scope. Group bits (070) reserved.
- Default file mode: `0o666` (rw-rw-rw-), default dir mode: `0o777` (rwxrwxrwx)
- Umask applied on creation (default `0o000` — no bits masked)
- GM always has full access regardless of permissions
- `chmod` is GM-only
- `r` controls read, `w` controls write/delete, `x` on directories controls listing

### Implementation Steps

1. ~~Create Diesel migration with the schema above~~ ✓
2. ~~Add Diesel models and schema module for `vfs_files` and `vfs_archive`~~ ✓
3. ~~Build a `vfs` Rust module with:~~ ✓
   - ~~Path parser (drive letter + Unix path normalization)~~ ✓
   - ~~Drive enum with scope rules (connection/session/user)~~ ✓
   - ~~CRUD operations (create, read, update, delete, list, stat, copy, rename)~~ ✓
   - ~~Size quota enforcement per drive~~ ✓
   - ~~Inline vs CAS threshold logic~~ ✓
   - ~~Parent directory validation (with optional auto-create)~~ ✓
   - ~~Unix-style permissions (rwx, umask, chmod)~~ ✓
   - ~~fnmatch pattern matching~~ ✓
4. ~~Scratch drive server-side support~~ → **Replaced**: scratch drives are now client-side (IndexedDB)
5. Remove `connection_id` column and scratch drive DB support (cleanup migration)
6. Archive logic: on game deletion, copy C: files to `vfs_archive` with 30-day expiry
7. Cleanup job: periodically purge expired `vfs_archive` rows
8. Leptos server functions for C: and U: drives (single-file CRUD)
9. Client-side scratch drive implementation (IndexedDB, `#[cfg(feature = "hydrate")]`)

### Protocol Design

- **C: and U: drives** — Leptos server functions (stateless, authenticated via session cookie). The server handles path validation, quota, permissions, and storage.
- **A: and B: scratch drives** — entirely client-side. Stored in browser IndexedDB, scoped per-tab. No server involvement. Quota enforced client-side. The same `VfsPath`, `vfs_fnmatch`, and permission logic compiles to WASM and runs in the browser.
- **File upload** (C:/U:): Uses existing multipart upload mechanism (same as media upload). Server computes SHA-256 hash server-side, stores in CAS, then writes the `vfs_files` row with `media_hash`. Single round-trip.
- **File download** (C:/U:): Server function returns inline data directly for small files, or a CAS URL for large files.
- **Multi-file upload, ZIP download/extraction**: See Feature 39 (VFS Upload & ZIP).
- **COMMAND.COM shell** (Feature 36): Client-side WASM. Parses commands locally, calls server functions for C:/U: and IndexedDB for A:/B:. The server never sees command strings — only VFS API calls.
- **File Browser** (Feature 37): Client-side Leptos component. Same server function API as the shell.
- **Change notifications**: `VfsChanged` broadcast via WebSocket for C: drive modifications (shared drive), so other connected clients can refresh their view.

### File Type Icons

A shared Unicode icon mapping used by both COMMAND.COM (`DIR` output) and the Finder (panel icons). Icons are resolved by file extension first, then by `content_type` fallback. This is a shared Rust module that compiles to both server and WASM targets.

| Icon | File Type | Extensions / Content-Type |
|------|-----------|---------------------------|
| 📁 | Directory | (is_directory) |
| 🖼️ | Image | `.png`, `.jpg`, `.jpeg`, `.gif`, `.bmp`, `.svg`, `.webp`, `image/*` |
| 📜 | Text / Document | `.txt`, `.md`, `.csv`, `.log`, `text/*` |
| 📊 | Data / Config | `.json`, `.xml`, `.yaml`, `.yml`, `.toml` |
| 🎵 | Audio | `.mp3`, `.ogg`, `.wav`, `.flac`, `audio/*` |
| 🎬 | Video | `.mp4`, `.webm`, `.avi`, `video/*` |
| 📦 | Archive / ZIP | `.zip`, `.tar`, `.gz`, `.7z`, `application/zip` |
| 📝 | Script / Code | `.pas`, `.js`, `.rs`, `.py`, `.sh` |
| 🗺️ | Map | `.vtt` (VTT media pack) |
| 📄 | Unknown / Generic | (fallback) |

The mapping is a simple function `fn vfs_file_icon(extension: Option<&str>, content_type: Option<&str>) -> char` returning a single Unicode character. The Finder renders these as large icons; `DIR` prefixes each filename with the icon character.

### Upload & Download Architecture

See Feature 39 (VFS Upload & ZIP) for multi-file upload, ZIP download/extraction, drag-and-drop, and progress indicators.

## Findings

- Existing `media` table uses content-addressable storage with `hash` as the unique key. VFS large files will reference `media.hash`.
- The `media` table tracks `uploaded_by`, `content_type`, and `size_bytes` — VFS can reuse these for large file metadata.
- SQLite partial unique indexes (`WHERE drive = 'C'`) are supported and will enforce path uniqueness per drive scope.
- CAS hash is computed server-side after receiving the full upload — no client-side hashing needed.
- `vfs_list` currently loads all descendants via `LIKE` prefix and filters in Rust. Could be optimized with a `NOT LIKE` clause to exclude deeper descendants at the SQL level.
- Multi-step operations (directory rename, overwrite) don't use explicit transactions. Similar to early DOS/CP/M behavior. A single-query `UPDATE ... SET path = :new || substr(path, length(:old)+1)` approach would make directory renames atomic.
- Open file tracking and unlinking of referenced files is a future work item to explore.
- `mode` column added via migration 0006: `ALTER TABLE vfs_files ADD COLUMN mode INTEGER NOT NULL DEFAULT 438` (438 = 0o666).
- 79 tests covering path parsing, DB operations, permissions, fnmatch, and copy.
- **Scratch drives moved to client-side**: Originally designed as server-side DB storage with `connection_id`. Moved to browser IndexedDB because: (1) scratch drives are inherently per-tab, (2) the COMMAND.COM shell is client-side so it can access IndexedDB directly, (3) eliminates the need for access tokens bridging stateless server functions to stateful connections, (4) reduces server load and DB storage.
- **COMMAND.COM shell is client-side**: The shell parser runs in WASM. Server functions already enforce security on C:/U: operations, so a server-side shell adds no security benefit. The Finder (Feature 37) faces the same architecture — both are client-side UIs calling server functions. Shell state (cwd, env vars, history) is naturally per-tab.
- Browser storage options considered: `sessionStorage` (5-10 MB, string-only), `IndexedDB` (large capacity, binary-friendly — chosen), `OPFS` (newer, less portable).
