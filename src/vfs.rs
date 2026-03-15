//! # Virtual File System (VFS)
//!
//! A drive-letter based virtual file system inspired by DOS and CP/M,
//! with Unix-style `/` path separators.
//!
//! ## Drive Letters
//!
//! | Drive | Scope | Storage | Quota | Lifetime |
//! |-------|-------|---------|-------|----------|
//! | A:, B: | Per-tab | Browser IndexedDB | 10 MB each | Disappears when tab closes |
//! | C: | Per-game session | Server DB + CAS | 100 MB | Persists with game; archived 30 days |
//! | U: | Per-user account | Server DB + CAS | 10/20 MB | Persists with account |
//!
//! D: through T: are reserved for future use.
//!
//! Scratch drives (A:, B:) are implemented client-side and do not use the
//! server-side database operations in this module. The server-side
//! `connection_id` support is deprecated and will be removed in a future
//! migration. Path parsing, pattern matching, and permission logic compile
//! to both server and WASM targets.
//!
//! ## Path Syntax
//!
//! Paths use `/` as the separator. Backslashes are illegal.
//!
//! ```text
//! C:/maps/dungeon.png    — absolute path with drive
//! /maps/dungeon.png      — absolute path on current drive (via resolve())
//! maps/dungeon.png       — relative to working directory (via resolve())
//! ../dungeon.png         — parent traversal (via resolve())
//! ```
//!
//! [`VfsPath::parse`] handles absolute paths with a drive letter.
//! [`VfsPath::resolve`] handles all forms by resolving against a working directory.
//!
//! ## Storage Architecture
//!
//! Files are stored in the `vfs_files` database table. The storage strategy
//! depends on file size:
//!
//! - **≤ [`INLINE_THRESHOLD`]** (8 KB): stored as a BLOB in the `inline_data`
//!   column for fast single-query access.
//! - **> [`INLINE_THRESHOLD`]**: stored in the existing content-addressable
//!   storage (CAS) system, with `media_hash` referencing the `media` table.
//!
//! The inline threshold may need tuning with benchmarks in the future.
//!
//! ## Scope and Ownership
//!
//! Each drive type uses a different scope column in `vfs_files`:
//!
//! - **A:/B:**: `connection_id` (UUID per WebSocket connection)
//! - **C:**: `session_id` (game session)
//! - **U:**: `user_id` (user account)
//!
//! A CHECK constraint enforces that exactly one scope column is set per row,
//! matching the drive letter. Partial unique indexes enforce path uniqueness
//! within each scope.
//!
//! ## Permissions
//!
//! Files use Unix-style `rwx` permission bits. The owner is always the
//! GM; all other users are "other". The group bits (0o070) are reserved
//! but unused.
//!
//! - New files get mode `0o666` (rw-rw-rw-) minus the umask.
//! - New directories get mode `0o777` (rwxrwxrwx) minus the umask.
//! - The GM always has full access regardless of permission bits.
//! - `r` (read) controls [`vfs_read`].
//! - `w` (write) controls [`vfs_write`] overwrites and [`vfs_delete`].
//! - `x` (execute) on directories controls [`vfs_list`] traversal.
//! - [`vfs_chmod`] is GM-only.
//!
//! ## Filename Rules
//!
//! The following characters are illegal in filenames:
//! `: / \ * ? " < > |` and control characters (0x00–0x1F).
//!
//! ## Path Equivalence
//!
//! Path strings are normalized for storage, but equivalence checks should
//! resolve paths to their `vfs_files.id` (inode) rather than comparing
//! strings.
//!
//! ## Consistency Model
//!
//! Multi-step operations (rename with descendants, overwrite, delete)
//! do not use explicit transactions. Each individual SQL statement is
//! atomic under SQLite autocommit, but a sequence of statements is not.
//! If the process crashes mid-operation (e.g. during a directory rename
//! with many descendants), the directory tree may be left in a partially
//! renamed state. This is similar to the behavior of early DOS and CP/M
//! file systems, where interrupted operations could leave inconsistent
//! directory entries.
//!
//! A single-query approach for directory renames would improve atomicity:
//!
//! ```sql
//! UPDATE vfs_files
//! SET path = :new_prefix || substr(path, length(:old_prefix) + 1),
//!     modified_by = :user_id,
//!     updated_at = unixepoch()
//! WHERE drive = :drive AND user_id = :uid
//!   AND (path = :old_path OR path LIKE :old_prefix || '%')
//! ```
//!
//! This updates the directory and all its descendants in one atomic
//! statement. This optimization is planned but not yet implemented.

#[cfg(feature = "ssr")]
use diesel::prelude::*;

/// Inline storage threshold: files at or below this size are stored
/// directly in the database. Larger files go to content-addressable storage.
/// This value may need tuning with benchmarks in the future.
pub const INLINE_THRESHOLD: usize = 8 * 1024; // 8 KB

// ===== Unix-style permissions =====

/// Permission bits (Unix-style octal). Owner = GM, other = everyone else.
/// The "group" bits (070) are unused but reserved.
pub const MODE_OWNER_R: i32 = 0o400;
pub const MODE_OWNER_W: i32 = 0o200;
pub const MODE_OWNER_X: i32 = 0o100;
pub const MODE_OTHER_R: i32 = 0o004;
pub const MODE_OTHER_W: i32 = 0o002;
pub const MODE_OTHER_X: i32 = 0o001;

/// Default mode for new files: `rw-rw-rw-` (0o666).
pub const DEFAULT_FILE_MODE: i32 = 0o666;
/// Default mode for new directories: `rwxrwxrwx` (0o777).
pub const DEFAULT_DIR_MODE: i32 = 0o777;
/// Default umask: no bits masked (everything permitted).
pub const DEFAULT_UMASK: i32 = 0o000;

/// Apply a umask to a default mode.
pub fn apply_umask(default_mode: i32, umask: i32) -> i32 {
    default_mode & !umask
}

/// Check if a permission is allowed for a given user.
///
/// The GM (owner) is checked against the owner bits (0o700).
/// All other users are checked against the "other" bits (0o007).
/// The GM always has access regardless of permission bits — the owner
/// bits are checked for consistency but never deny the GM.
pub fn check_permission(mode: i32, is_gm: bool, perm: i32) -> bool {
    if is_gm {
        return true; // GM always has full access
    }
    // Check "other" bits
    (mode & perm) != 0
}

/// Drive letter definitions with their scope and quota rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    A,
    B,
    C,
    U,
}

/// How a drive is scoped — determines which ID column is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveScope {
    Connection,
    Session,
    User,
}

impl Drive {
    pub fn from_letter(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'A' => Some(Drive::A),
            'B' => Some(Drive::B),
            'C' => Some(Drive::C),
            'U' => Some(Drive::U),
            _ => None,
        }
    }

    pub fn letter(&self) -> char {
        match self {
            Drive::A => 'A',
            Drive::B => 'B',
            Drive::C => 'C',
            Drive::U => 'U',
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Drive::A => "A",
            Drive::B => "B",
            Drive::C => "C",
            Drive::U => "U",
        }
    }

    pub fn scope(&self) -> DriveScope {
        match self {
            Drive::A | Drive::B => DriveScope::Connection,
            Drive::C => DriveScope::Session,
            Drive::U => DriveScope::User,
        }
    }

    /// Maximum size in bytes for this drive.
    pub fn quota_bytes(&self, is_gm: bool) -> u64 {
        match self {
            Drive::A | Drive::B => 10 * 1024 * 1024, // 10 MB
            Drive::C => 100 * 1024 * 1024,           // 100 MB
            Drive::U => {
                if is_gm {
                    20 * 1024 * 1024 // 20 MB
                } else {
                    10 * 1024 * 1024 // 10 MB
                }
            }
        }
    }
}

/// A parsed VFS path consisting of a drive letter and a normalized path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsPath {
    pub drive: Drive,
    pub path: String,
}

impl std::fmt::Display for VfsPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.drive.letter(), self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsPathError {
    Empty,
    InvalidDriveLetter(char),
    InvalidPath(String),
    IllegalCharacter(char),
}

impl std::fmt::Display for VfsPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsPathError::Empty => write!(f, "path is empty"),
            VfsPathError::InvalidDriveLetter(c) => write!(f, "invalid drive letter: {}", c),
            VfsPathError::InvalidPath(reason) => write!(f, "invalid path: {}", reason),
            VfsPathError::IllegalCharacter(c) => {
                write!(f, "illegal character in filename: {:?}", c)
            }
        }
    }
}

/// Characters that are illegal in filenames.
const ILLEGAL_FILENAME_CHARS: &[char] = &[':', '/', '\\', '*', '?', '"', '<', '>', '|', '\0'];

/// Check if a filename component contains only legal characters.
fn validate_filename(name: &str) -> Result<(), VfsPathError> {
    for c in name.chars() {
        if c.is_control() || ILLEGAL_FILENAME_CHARS.contains(&c) {
            return Err(VfsPathError::IllegalCharacter(c));
        }
    }
    Ok(())
}

impl VfsPath {
    /// Parse a VFS path string.
    ///
    /// If the second character is `:`, the first character is treated as a
    /// drive letter and the remainder is the path. Otherwise the entire input
    /// is treated as a relative path (no drive).
    ///
    /// The path is normalized:
    /// - Duplicate slashes are collapsed
    /// - `.` components are removed
    /// - `..` components go up one level (clamped at root)
    /// - Trailing slashes are removed (except for root `/`)
    /// - Result always starts with `/`
    ///
    /// Note: The normalized string is used for storage. For path equivalence
    /// checks, resolve paths to their `vfs_files.id` (inode) rather than
    /// comparing strings.
    pub fn parse(input: &str) -> Result<Self, VfsPathError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(VfsPathError::Empty);
        }

        let chars: Vec<char> = input.chars().collect();

        // If second character is ':', first character is a drive letter
        if chars.len() >= 2 && chars[1] == ':' {
            let drive =
                Drive::from_letter(chars[0]).ok_or(VfsPathError::InvalidDriveLetter(chars[0]))?;
            let rest: String = chars[2..].iter().collect();
            let path = normalize_path(&rest)?;
            Ok(VfsPath { drive, path })
        } else {
            // No drive letter — relative path, caller must resolve against
            // a working directory. For now, return an error since we need
            // a drive to form a complete VfsPath.
            Err(VfsPathError::InvalidPath(
                "no drive letter — use resolve() with a working directory".to_string(),
            ))
        }
    }

    /// Resolve a user-typed path against a working directory.
    ///
    /// - `C:/maps/test.png` — absolute path (second char is `:`)
    /// - `/maps/test.png` — absolute path on the working directory's drive
    /// - `maps/test.png` — relative to working directory
    /// - `../test.png` — relative with parent traversal
    pub fn resolve(input: &str, cwd: &VfsPath) -> Result<Self, VfsPathError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(VfsPathError::Empty);
        }

        let chars: Vec<char> = input.chars().collect();

        // Absolute path with drive letter
        if chars.len() >= 2 && chars[1] == ':' {
            return VfsPath::parse(input);
        }

        // Absolute path on current drive (starts with /)
        if chars[0] == '/' {
            let path = normalize_path(input)?;
            return Ok(VfsPath {
                drive: cwd.drive,
                path,
            });
        }

        // Relative path — prepend working directory
        let combined = if cwd.path == "/" {
            format!("/{}", input)
        } else {
            format!("{}/{}", cwd.path, input)
        };
        let path = normalize_path(&combined)?;
        Ok(VfsPath {
            drive: cwd.drive,
            path,
        })
    }

    /// Create a VfsPath from a drive and a raw path string, normalizing the path.
    pub fn new(drive: Drive, raw_path: &str) -> Result<Self, VfsPathError> {
        let path = normalize_path(raw_path)?;
        Ok(VfsPath { drive, path })
    }

    /// Return the parent directory path, or None if this is the root.
    pub fn parent(&self) -> Option<String> {
        if self.path == "/" {
            return None;
        }
        match self.path.rfind('/') {
            Some(0) => Some("/".to_string()),
            Some(pos) => Some(self.path[..pos].to_string()),
            None => Some("/".to_string()),
        }
    }

    /// Return the filename component, or None if this is the root.
    pub fn filename(&self) -> Option<&str> {
        if self.path == "/" {
            return None;
        }
        self.path.rsplit('/').next()
    }
}

/// Normalize a path string.
fn normalize_path(raw: &str) -> Result<String, VfsPathError> {
    // If empty or doesn't start with /, prepend /
    let raw = if raw.is_empty() || !raw.starts_with('/') {
        format!("/{}", raw)
    } else {
        raw.to_string()
    };

    // Split into components and normalize
    let mut components: Vec<&str> = Vec::new();
    for component in raw.split('/') {
        match component {
            "" | "." => continue,
            ".." => {
                components.pop(); // go up one level, clamped at root
            }
            c => {
                validate_filename(c)?;
                components.push(c);
            }
        }
    }

    if components.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", components.join("/")))
    }
}

/// Match a filename against a pattern, roughly like POSIX `fnmatch(3)`.
///
/// Supports:
/// - `*` — matches zero or more characters (except `/`)
/// - `?` — matches exactly one character (except `/`)
/// - `[abc]` — matches any one character in the set
/// - `[a-z]` — matches any character in the range
/// - `[!abc]` or `[^abc]` — matches any character NOT in the set
///
/// Matching is case-insensitive (DOS convention). The pattern is matched
/// against the filename component only, not the full path. Slashes in
/// the pattern or string are not given special treatment beyond the
/// `*`/`?` exclusion above.
pub fn vfs_fnmatch(pattern: &str, name: &str) -> bool {
    fn matches(pat: &[u8], name: &[u8]) -> bool {
        let (mut pi, mut ni) = (0, 0);
        let (mut star_pi, mut star_ni) = (usize::MAX, usize::MAX);

        while ni < name.len() {
            if pi < pat.len() && pat[pi] == b'[' {
                // Bracket expression
                if let Some((matched, end)) = match_bracket(&pat[pi..], name[ni]) {
                    if matched {
                        pi += end;
                        ni += 1;
                        continue;
                    }
                }
                // No match in bracket — try star backtrack
                if star_pi != usize::MAX {
                    pi = star_pi + 1;
                    star_ni += 1;
                    ni = star_ni;
                    continue;
                }
                return false;
            } else if pi < pat.len() && pat[pi] == b'?' && name[ni] != b'/' {
                pi += 1;
                ni += 1;
            } else if pi < pat.len() && pat[pi] == b'*' {
                star_pi = pi;
                star_ni = ni;
                pi += 1;
            } else if pi < pat.len()
                && name[ni].to_ascii_lowercase() == pat[pi].to_ascii_lowercase()
            {
                pi += 1;
                ni += 1;
            } else if star_pi != usize::MAX {
                pi = star_pi + 1;
                star_ni += 1;
                ni = star_ni;
            } else {
                return false;
            }
        }

        // Consume trailing stars
        while pi < pat.len() && pat[pi] == b'*' {
            pi += 1;
        }
        pi == pat.len()
    }

    /// Match a bracket expression `[...]` against a single byte.
    /// Returns `Some((matched, end_offset))` where `end_offset` is
    /// the index past the closing `]`, or `None` if malformed.
    fn match_bracket(pat: &[u8], ch: u8) -> Option<(bool, usize)> {
        if pat.is_empty() || pat[0] != b'[' {
            return None;
        }
        let mut i = 1;
        let negate = if i < pat.len() && (pat[i] == b'!' || pat[i] == b'^') {
            i += 1;
            true
        } else {
            false
        };

        let ch_lower = ch.to_ascii_lowercase();
        let mut matched = false;

        // Allow ] as first char in set
        if i < pat.len() && pat[i] == b']' {
            if ch_lower == b']' {
                matched = true;
            }
            i += 1;
        }

        while i < pat.len() && pat[i] != b']' {
            if i + 2 < pat.len() && pat[i + 1] == b'-' && pat[i + 2] != b']' {
                // Range
                let lo = pat[i].to_ascii_lowercase();
                let hi = pat[i + 2].to_ascii_lowercase();
                if ch_lower >= lo && ch_lower <= hi {
                    matched = true;
                }
                i += 3;
            } else {
                if ch_lower == pat[i].to_ascii_lowercase() {
                    matched = true;
                }
                i += 1;
            }
        }

        if i < pat.len() && pat[i] == b']' {
            Some((matched != negate, i + 1))
        } else {
            None // malformed — no closing ]
        }
    }

    matches(pattern.as_bytes(), name.as_bytes())
}

/// File metadata returned by stat/list operations.
#[derive(Debug, Clone)]
pub struct VfsEntry {
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: i32,
    pub content_type: Option<String>,
    pub modified_by: Option<i32>,
    pub created_at: i32,
    pub updated_at: i32,
    pub mode: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    NotFound(String),
    AlreadyExists(String),
    PermissionDenied(String),
    QuotaExceeded { drive: char, used: u64, limit: u64 },
    NotADirectory(String),
    IsADirectory(String),
    DirectoryNotEmpty(String),
    InvalidPath(VfsPathError),
    DatabaseError(String),
    StorageError(String),
}

impl std::fmt::Display for VfsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VfsError::NotFound(p) => write!(f, "not found: {}", p),
            VfsError::AlreadyExists(p) => write!(f, "already exists: {}", p),
            VfsError::PermissionDenied(p) => write!(f, "permission denied: {}", p),
            VfsError::QuotaExceeded { drive, used, limit } => {
                write!(
                    f,
                    "quota exceeded on {}: drive — {} of {} bytes used",
                    drive, used, limit
                )
            }
            VfsError::NotADirectory(p) => write!(f, "not a directory: {}", p),
            VfsError::IsADirectory(p) => write!(f, "is a directory: {}", p),
            VfsError::DirectoryNotEmpty(p) => write!(f, "directory not empty: {}", p),
            VfsError::InvalidPath(e) => write!(f, "{}", e),
            VfsError::DatabaseError(e) => write!(f, "database error: {}", e),
            VfsError::StorageError(e) => write!(f, "storage error: {}", e),
        }
    }
}

impl From<VfsPathError> for VfsError {
    fn from(e: VfsPathError) -> Self {
        VfsError::InvalidPath(e)
    }
}

/// Scope identifier for VFS operations — determines which files are visible.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct VfsScope {
    pub connection_id: Option<String>,
    pub session_id: Option<i32>,
    pub user_id: Option<i32>,
    pub is_gm: bool,
    /// Umask applied to newly created files and directories.
    /// Default: `0o000` (no bits masked — full default permissions).
    pub umask: i32,
}

#[cfg(feature = "ssr")]
impl VfsScope {
    fn scope_for_drive(&self, drive: Drive) -> Result<DriveFilter, VfsError> {
        match drive.scope() {
            DriveScope::Connection => {
                let cid = self.connection_id.as_ref().ok_or_else(|| {
                    VfsError::StorageError("no connection_id for scratch drive".to_string())
                })?;
                Ok(DriveFilter::Connection(cid.clone()))
            }
            DriveScope::Session => {
                let sid = self.session_id.ok_or_else(|| {
                    VfsError::StorageError("no session_id for session drive".to_string())
                })?;
                Ok(DriveFilter::Session(sid))
            }
            DriveScope::User => {
                let uid = self.user_id.ok_or_else(|| {
                    VfsError::StorageError("no user_id for user drive".to_string())
                })?;
                Ok(DriveFilter::User(uid))
            }
        }
    }
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
enum DriveFilter {
    Connection(String),
    Session(i32),
    User(i32),
}

// ===== Database operations =====

#[cfg(feature = "ssr")]
use crate::models::db_models::{NewVfsFile, VfsFile};
#[cfg(feature = "ssr")]
use crate::schema::vfs_files;

#[cfg(feature = "ssr")]
impl From<VfsFile> for VfsEntry {
    fn from(f: VfsFile) -> Self {
        VfsEntry {
            path: f.path,
            is_directory: f.is_directory,
            size_bytes: f.size_bytes,
            content_type: f.content_type,
            modified_by: f.modified_by,
            created_at: f.created_at,
            updated_at: f.updated_at,
            mode: f.mode,
        }
    }
}

// ----- Scope helpers -----

/// Build a boxed SELECT query filtered by drive letter and scope.
#[cfg(feature = "ssr")]
fn scoped_query<'a>(
    drive_str: &'a str,
    filter: &'a DriveFilter,
) -> vfs_files::BoxedQuery<'a, diesel::sqlite::Sqlite> {
    let query = vfs_files::table
        .filter(vfs_files::drive.eq(drive_str))
        .into_boxed();
    match filter {
        DriveFilter::Connection(cid) => query.filter(vfs_files::connection_id.eq(cid.clone())),
        DriveFilter::Session(sid) => query.filter(vfs_files::session_id.eq(*sid)),
        DriveFilter::User(uid) => query.filter(vfs_files::user_id.eq(*uid)),
    }
}

/// Extract scope IDs from a [`DriveFilter`] for building [`NewVfsFile`] structs.
#[cfg(feature = "ssr")]
fn scope_ids(filter: &DriveFilter) -> (Option<&str>, Option<i32>, Option<i32>) {
    match filter {
        DriveFilter::Connection(cid) => (Some(cid.as_str()), None, None),
        DriveFilter::Session(sid) => (None, Some(*sid), None),
        DriveFilter::User(uid) => (None, None, Some(*uid)),
    }
}

/// Delete rows matching a drive scope and path.
#[cfg(feature = "ssr")]
fn scoped_delete_path(
    conn: &mut diesel::SqliteConnection,
    drive_str: &str,
    filter: &DriveFilter,
    path: &str,
) -> Result<usize, diesel::result::Error> {
    match filter {
        DriveFilter::Connection(cid) => diesel::delete(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::connection_id.eq(cid.as_str()))
                .filter(vfs_files::path.eq(path)),
        )
        .execute(conn),
        DriveFilter::Session(sid) => diesel::delete(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::session_id.eq(*sid))
                .filter(vfs_files::path.eq(path)),
        )
        .execute(conn),
        DriveFilter::User(uid) => diesel::delete(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::user_id.eq(*uid))
                .filter(vfs_files::path.eq(path)),
        )
        .execute(conn),
    }
}

/// Update file content for a scoped path. Used by [`vfs_write`] for overwrites.
#[cfg(feature = "ssr")]
fn scoped_update_file(
    conn: &mut diesel::SqliteConnection,
    drive_str: &str,
    filter: &DriveFilter,
    path: &str,
    size: i32,
    content_type: Option<&str>,
    inline_data: Option<&[u8]>,
    media_hash: Option<&str>,
    user_id: i32,
) -> Result<usize, diesel::result::Error> {
    macro_rules! do_update {
        ($target:expr) => {
            diesel::update($target)
                .set((
                    vfs_files::size_bytes.eq(size),
                    vfs_files::content_type.eq(content_type),
                    vfs_files::inline_data.eq(inline_data),
                    vfs_files::media_hash.eq(media_hash),
                    vfs_files::modified_by.eq(user_id),
                    vfs_files::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Integer>(
                        "unixepoch()",
                    )),
                ))
                .execute(conn)
        };
    }
    match filter {
        DriveFilter::Connection(cid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::connection_id.eq(cid.as_str()))
                .filter(vfs_files::path.eq(path))
        ),
        DriveFilter::Session(sid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::session_id.eq(*sid))
                .filter(vfs_files::path.eq(path))
        ),
        DriveFilter::User(uid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::user_id.eq(*uid))
                .filter(vfs_files::path.eq(path))
        ),
    }
}

/// Update the path column for a scoped file (used by [`vfs_rename`]).
#[cfg(feature = "ssr")]
fn scoped_update_path(
    conn: &mut diesel::SqliteConnection,
    drive_str: &str,
    filter: &DriveFilter,
    old_path: &str,
    new_path: &str,
    user_id: i32,
) -> Result<usize, diesel::result::Error> {
    macro_rules! do_update {
        ($target:expr) => {
            diesel::update($target)
                .set((
                    vfs_files::path.eq(new_path),
                    vfs_files::modified_by.eq(user_id),
                    vfs_files::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Integer>(
                        "unixepoch()",
                    )),
                ))
                .execute(conn)
        };
    }
    match filter {
        DriveFilter::Connection(cid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::connection_id.eq(cid.as_str()))
                .filter(vfs_files::path.eq(old_path))
        ),
        DriveFilter::Session(sid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::session_id.eq(*sid))
                .filter(vfs_files::path.eq(old_path))
        ),
        DriveFilter::User(uid) => do_update!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::user_id.eq(*uid))
                .filter(vfs_files::path.eq(old_path))
        ),
    }
}

// ----- Public VFS operations -----

/// Get file or directory metadata.
#[cfg(feature = "ssr")]
pub fn vfs_stat(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
) -> Result<VfsEntry, VfsError> {
    let filter = scope.scope_for_drive(drive)?;
    let file: VfsFile = scoped_query(drive.as_str(), &filter)
        .filter(vfs_files::path.eq(path))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                VfsError::NotFound(format!("{}:{}", drive.letter(), path))
            }
            other => VfsError::DatabaseError(other.to_string()),
        })?;
    Ok(file.into())
}

/// List direct children of a directory.
///
/// Returns entries immediately under `dir_path`, excluding deeper
/// descendants. Uses a single atomic SELECT for consistency — all
/// results reflect the directory state at one point in time.
///
/// # Performance note
///
/// Currently fetches all descendants matching the prefix via `LIKE`
/// and filters in Rust to find direct children. For directories with
/// many deeply nested files this is wasteful. A more efficient SQL
/// approach would be:
///
/// ```sql
/// SELECT * FROM vfs_files
/// WHERE drive = ? AND user_id = ?
///   AND path LIKE ? || '%'
///   AND path NOT LIKE ? || '%/%'
/// ORDER BY path
/// ```
///
/// This second `NOT LIKE` clause excludes deeper descendants at the
/// SQL level. Worth switching if directory trees grow large.
#[cfg(feature = "ssr")]
pub fn vfs_list(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    dir_path: &str,
) -> Result<Vec<VfsEntry>, VfsError> {
    // Check execute permission on the directory (root is always accessible)
    if dir_path != "/" {
        let dir_entry = vfs_stat(conn, scope, drive, dir_path)?;
        if !dir_entry.is_directory {
            return Err(VfsError::NotADirectory(format!(
                "{}:{}",
                drive.letter(),
                dir_path
            )));
        }
        if !check_permission(dir_entry.mode, scope.is_gm, MODE_OTHER_X) {
            return Err(VfsError::PermissionDenied(format!(
                "{}:{}",
                drive.letter(),
                dir_path
            )));
        }
    }

    let filter = scope.scope_for_drive(drive)?;
    let prefix = if dir_path == "/" {
        "/".to_string()
    } else {
        format!("{}/", dir_path)
    };

    let files: Vec<VfsFile> = scoped_query(drive.as_str(), &filter)
        .filter(vfs_files::path.like(format!("{}%", prefix)))
        .order(vfs_files::path.asc())
        .load(conn)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;

    let entries = files
        .into_iter()
        .filter(|f| !f.path[prefix.len()..].contains('/'))
        .map(VfsEntry::from)
        .collect();
    Ok(entries)
}

/// Get total bytes used on a drive for quota enforcement.
#[cfg(feature = "ssr")]
pub fn vfs_drive_usage(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
) -> Result<u64, VfsError> {
    use diesel::dsl::sum;

    let filter = scope.scope_for_drive(drive)?;
    let total: Option<i64> = scoped_query(drive.as_str(), &filter)
        .select(sum(vfs_files::size_bytes))
        .first(conn)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    Ok(total.unwrap_or(0) as u64)
}

/// Ensure the parent directory of `path` exists.
///
/// If `create_parents` is true, creates all missing ancestor directories
/// (like `mkdir -p`). If false, returns [`VfsError::NotFound`] when the
/// parent doesn't exist. The root `/` is always implicit.
#[cfg(feature = "ssr")]
fn ensure_parent_exists(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    user_id: i32,
    create_parents: bool,
) -> Result<(), VfsError> {
    let vp = VfsPath::new(drive, path)?;
    let parent = match vp.parent() {
        Some(p) => p,
        None => return Ok(()), // path is root, no parent needed
    };
    if parent == "/" {
        return Ok(()); // root is implicit
    }

    match vfs_stat(conn, scope, drive, &parent) {
        Ok(entry) => {
            if entry.is_directory {
                Ok(())
            } else {
                Err(VfsError::NotADirectory(format!(
                    "{}:{}",
                    drive.letter(),
                    parent
                )))
            }
        }
        Err(VfsError::NotFound(_)) => {
            if create_parents {
                vfs_mkdir_p(conn, scope, drive, &parent, user_id)
            } else {
                Err(VfsError::NotFound(format!(
                    "parent directory {}:{}",
                    drive.letter(),
                    parent
                )))
            }
        }
        Err(e) => Err(e),
    }
}

/// Create a directory, optionally creating parent directories.
///
/// If `create_parents` is true, behaves like `mkdir -p` — creates all
/// missing ancestor directories and does not error if the target already
/// exists as a directory. If false, fails if the path already exists or
/// the parent directory is missing.
#[cfg(feature = "ssr")]
pub fn vfs_mkdir(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    user_id: i32,
    create_parents: bool,
) -> Result<(), VfsError> {
    if create_parents {
        vfs_mkdir_p(conn, scope, drive, path, user_id)
    } else {
        // Verify parent exists
        ensure_parent_exists(conn, scope, drive, path, user_id, false)?;
        vfs_mkdir_one(conn, scope, drive, path, user_id)
    }
}

/// Create a single directory — fails if it already exists.
#[cfg(feature = "ssr")]
fn vfs_mkdir_one(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    user_id: i32,
) -> Result<(), VfsError> {
    let filter = scope.scope_for_drive(drive)?;

    if vfs_stat(conn, scope, drive, path).is_ok() {
        return Err(VfsError::AlreadyExists(format!(
            "{}:{}",
            drive.letter(),
            path
        )));
    }

    let (connection_id, session_id, user_id_col) = scope_ids(&filter);
    let new_file = NewVfsFile {
        drive: drive.as_str(),
        connection_id,
        session_id,
        user_id: user_id_col,
        path,
        is_directory: true,
        size_bytes: 0,
        content_type: None,
        inline_data: None,
        media_hash: None,
        modified_by: Some(user_id),
        mode: apply_umask(DEFAULT_DIR_MODE, scope.umask),
    };

    diesel::insert_into(vfs_files::table)
        .values(&new_file)
        .execute(conn)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    Ok(())
}

/// Recursive mkdir — creates all missing path components.
/// Does not error if the directory already exists.
#[cfg(feature = "ssr")]
fn vfs_mkdir_p(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    user_id: i32,
) -> Result<(), VfsError> {
    if path == "/" {
        return Ok(());
    }
    match vfs_stat(conn, scope, drive, path) {
        Ok(entry) => {
            if entry.is_directory {
                return Ok(()); // already exists as directory
            }
            return Err(VfsError::NotADirectory(format!(
                "{}:{}",
                drive.letter(),
                path
            )));
        }
        Err(VfsError::NotFound(_)) => {} // need to create it
        Err(e) => return Err(e),
    }

    // Ensure parent exists first (recursive)
    let vp = VfsPath::new(drive, path)?;
    if let Some(parent) = vp.parent() {
        if parent != "/" {
            vfs_mkdir_p(conn, scope, drive, &parent, user_id)?;
        }
    }

    // Create this directory
    vfs_mkdir_one(conn, scope, drive, path, user_id)
}

/// Write a file to the VFS.
///
/// If the file already exists it is overwritten. If the path is a
/// directory, returns [`VfsError::IsADirectory`].
///
/// Files at or below [`INLINE_THRESHOLD`] are stored as BLOBs in the
/// database. Larger files must be stored in CAS by the caller, with
/// `media_hash` providing the reference. If the file exceeds the inline
/// threshold and no `media_hash` is provided, returns
/// [`VfsError::StorageError`].
///
/// Quota is checked before writing. When overwriting, the old file's
/// size is subtracted from usage first so that replacing a file with
/// one of equal size doesn't falsely exceed the quota.
///
/// If `create_parents` is true, missing parent directories are
/// automatically created (like `mkdir -p`). If false, the parent
/// directory must already exist or [`VfsError::NotFound`] is returned.
/// The root directory `/` is always implicit and never needs to exist
/// as a row.
#[cfg(feature = "ssr")]
pub fn vfs_write(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    data: &[u8],
    content_type: Option<&str>,
    media_hash: Option<&str>,
    user_id: i32,
    create_parents: bool,
) -> Result<(), VfsError> {
    let filter = scope.scope_for_drive(drive)?;
    let size = data.len() as i32;

    // Reject large files without a CAS hash
    if data.len() > INLINE_THRESHOLD && media_hash.is_none() {
        return Err(VfsError::StorageError(format!(
            "file exceeds inline threshold ({} bytes > {}) but no media_hash provided",
            data.len(),
            INLINE_THRESHOLD
        )));
    }

    // Check for existing entry (single stat call)
    let existing = match vfs_stat(conn, scope, drive, path) {
        Ok(entry) => {
            if entry.is_directory {
                return Err(VfsError::IsADirectory(format!(
                    "{}:{}",
                    drive.letter(),
                    path
                )));
            }
            Some(entry)
        }
        Err(VfsError::NotFound(_)) => None,
        Err(e) => return Err(e),
    };

    // Permission check on overwrite
    if let Some(ref entry) = existing {
        if !check_permission(entry.mode, scope.is_gm, MODE_OTHER_W) {
            return Err(VfsError::PermissionDenied(format!(
                "{}:{}",
                drive.letter(),
                path
            )));
        }
    }

    // Quota check — subtract old size when overwriting
    let usage = vfs_drive_usage(conn, scope, drive)?;
    let old_size = existing.as_ref().map_or(0u64, |e| e.size_bytes as u64);
    let effective_usage = usage - old_size;
    let quota = drive.quota_bytes(scope.is_gm);
    if effective_usage + data.len() as u64 > quota {
        return Err(VfsError::QuotaExceeded {
            drive: drive.letter(),
            used: effective_usage,
            limit: quota,
        });
    }

    // Ensure parent directory exists
    if existing.is_none() {
        ensure_parent_exists(conn, scope, drive, path, user_id, create_parents)?;
    }

    // Decide inline vs CAS
    let (inline, hash) = if data.len() <= INLINE_THRESHOLD {
        (Some(data), None)
    } else {
        (None, media_hash)
    };

    // Overwrite or create
    if existing.is_some() {
        scoped_update_file(
            conn,
            drive.as_str(),
            &filter,
            path,
            size,
            content_type,
            inline,
            hash,
            user_id,
        )
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    } else {
        let (connection_id, session_id, user_id_col) = scope_ids(&filter);
        let new_file = NewVfsFile {
            drive: drive.as_str(),
            connection_id,
            session_id,
            user_id: user_id_col,
            path,
            is_directory: false,
            size_bytes: size,
            content_type,
            inline_data: inline,
            media_hash: hash,
            modified_by: Some(user_id),
            mode: apply_umask(DEFAULT_FILE_MODE, scope.umask),
        };
        diesel::insert_into(vfs_files::table)
            .values(&new_file)
            .execute(conn)
            .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}

/// Read file contents.
///
/// Returns [`VfsFileContent::Inline`] for small files or
/// [`VfsFileContent::CasReference`] for files stored in CAS.
/// The caller is responsible for fetching CAS content via the
/// media hash.
#[cfg(feature = "ssr")]
pub fn vfs_read(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
) -> Result<VfsFileContent, VfsError> {
    let filter = scope.scope_for_drive(drive)?;
    let file: VfsFile = scoped_query(drive.as_str(), &filter)
        .filter(vfs_files::path.eq(path))
        .first(conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                VfsError::NotFound(format!("{}:{}", drive.letter(), path))
            }
            other => VfsError::DatabaseError(other.to_string()),
        })?;

    if file.is_directory {
        return Err(VfsError::IsADirectory(format!(
            "{}:{}",
            drive.letter(),
            path
        )));
    }

    if !check_permission(file.mode, scope.is_gm, MODE_OTHER_R) {
        return Err(VfsError::PermissionDenied(format!(
            "{}:{}",
            drive.letter(),
            path
        )));
    }

    if let Some(data) = file.inline_data {
        Ok(VfsFileContent::Inline {
            data,
            content_type: file.content_type,
        })
    } else if let Some(hash) = file.media_hash {
        Ok(VfsFileContent::CasReference {
            hash,
            content_type: file.content_type,
            size_bytes: file.size_bytes,
        })
    } else {
        Err(VfsError::StorageError(
            "file has neither inline data nor media hash".to_string(),
        ))
    }
}

/// Content of a VFS file — either inline data or a CAS reference.
#[derive(Debug, Clone)]
pub enum VfsFileContent {
    Inline {
        data: Vec<u8>,
        content_type: Option<String>,
    },
    CasReference {
        hash: String,
        content_type: Option<String>,
        size_bytes: i32,
    },
}

/// Delete a file or empty directory.
///
/// Non-empty directories cannot be deleted — remove their contents first.
#[cfg(feature = "ssr")]
pub fn vfs_delete(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
) -> Result<(), VfsError> {
    let entry = vfs_stat(conn, scope, drive, path)?;

    // Write permission required to delete
    if !check_permission(entry.mode, scope.is_gm, MODE_OTHER_W) {
        return Err(VfsError::PermissionDenied(format!(
            "{}:{}",
            drive.letter(),
            path
        )));
    }

    if entry.is_directory {
        let children = vfs_list(conn, scope, drive, path)?;
        if !children.is_empty() {
            return Err(VfsError::DirectoryNotEmpty(format!(
                "{}:{}",
                drive.letter(),
                path
            )));
        }
    }

    let filter = scope.scope_for_drive(drive)?;
    let deleted = scoped_delete_path(conn, drive.as_str(), &filter, path)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;

    if deleted == 0 {
        return Err(VfsError::NotFound(format!("{}:{}", drive.letter(), path)));
    }
    Ok(())
}

/// Rename or move a file/directory within the same drive.
///
/// Works like Unix `rename(2)`: atomically changes the path. If the
/// source is a directory, all descendant paths are updated as well.
/// If the destination exists and is a file, it is replaced. If the
/// destination exists and is a non-empty directory, the operation fails.
///
/// Moving across drives is not supported — use copy + delete instead.
///
/// # Quota note
///
/// When the destination is replaced, its space is freed but quota is
/// not re-checked — quota is always computed from `SUM(size_bytes)`,
/// so the final state is correct regardless of intermediate states.
#[cfg(feature = "ssr")]
pub fn vfs_rename(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    old_path: &str,
    new_path: &str,
    user_id: i32,
) -> Result<(), VfsError> {
    if old_path == new_path {
        return Ok(());
    }

    let source = vfs_stat(conn, scope, drive, old_path)?;
    let filter = scope.scope_for_drive(drive)?;
    let drive_str = drive.as_str();

    // If destination exists, handle replacement
    if let Ok(dest) = vfs_stat(conn, scope, drive, new_path) {
        if dest.is_directory {
            let children = vfs_list(conn, scope, drive, new_path)?;
            if !children.is_empty() {
                return Err(VfsError::DirectoryNotEmpty(format!(
                    "{}:{}",
                    drive.letter(),
                    new_path
                )));
            }
        }
        // Remove the target before renaming
        scoped_delete_path(conn, drive_str, &filter, new_path)
            .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    }

    // Rename the entry itself
    scoped_update_path(conn, drive_str, &filter, old_path, new_path, user_id)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;

    // If source is a directory, update all descendant paths
    if source.is_directory {
        let old_prefix = format!("{}/", old_path);
        let new_prefix = format!("{}/", new_path);

        // Fetch all descendants
        let descendants: Vec<VfsFile> = scoped_query(drive_str, &filter)
            .filter(vfs_files::path.like(format!("{}%", old_prefix)))
            .load(conn)
            .map_err(|e| VfsError::DatabaseError(e.to_string()))?;

        for desc in descendants {
            let updated_path = format!("{}{}", new_prefix, &desc.path[old_prefix.len()..]);
            scoped_update_path(conn, drive_str, &filter, &desc.path, &updated_path, user_id)
                .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
        }
    }

    Ok(())
}

/// Copy a file within or across drives.
///
/// Reads the source file and writes a copy to the destination. If the
/// source is stored inline, the data is duplicated. If stored via CAS,
/// the `media_hash` reference is shared (deduplication — no data is
/// copied, only the hash reference). Directories cannot be copied —
/// use recursive copy at a higher level.
///
/// If `create_parents` is true, missing parent directories at the
/// destination are automatically created.
#[cfg(feature = "ssr")]
pub fn vfs_copy(
    conn: &mut diesel::SqliteConnection,
    src_scope: &VfsScope,
    src_drive: Drive,
    src_path: &str,
    dst_scope: &VfsScope,
    dst_drive: Drive,
    dst_path: &str,
    user_id: i32,
    create_parents: bool,
) -> Result<(), VfsError> {
    let content = vfs_read(conn, src_scope, src_drive, src_path)?;
    match content {
        VfsFileContent::Inline { data, content_type } => vfs_write(
            conn,
            dst_scope,
            dst_drive,
            dst_path,
            &data,
            content_type.as_deref(),
            None,
            user_id,
            create_parents,
        ),
        VfsFileContent::CasReference {
            hash,
            content_type,
            size_bytes,
        } => vfs_write_cas_ref(
            conn,
            dst_scope,
            dst_drive,
            dst_path,
            size_bytes,
            content_type.as_deref(),
            &hash,
            user_id,
            create_parents,
        ),
    }
}

/// Write a CAS-referenced file to the VFS (no inline data).
///
/// Used internally by [`vfs_copy`] for large files. Creates a VFS entry
/// that references existing CAS content by hash, without allocating or
/// copying the file data.
#[cfg(feature = "ssr")]
fn vfs_write_cas_ref(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    size_bytes: i32,
    content_type: Option<&str>,
    media_hash: &str,
    user_id: i32,
    create_parents: bool,
) -> Result<(), VfsError> {
    let filter = scope.scope_for_drive(drive)?;

    // Check for existing entry
    let existing = match vfs_stat(conn, scope, drive, path) {
        Ok(entry) => {
            if entry.is_directory {
                return Err(VfsError::IsADirectory(format!(
                    "{}:{}",
                    drive.letter(),
                    path
                )));
            }
            Some(entry)
        }
        Err(VfsError::NotFound(_)) => None,
        Err(e) => return Err(e),
    };

    // Quota check
    let usage = vfs_drive_usage(conn, scope, drive)?;
    let old_size = existing.as_ref().map_or(0u64, |e| e.size_bytes as u64);
    let effective_usage = usage - old_size;
    let quota = drive.quota_bytes(scope.is_gm);
    if effective_usage + size_bytes as u64 > quota {
        return Err(VfsError::QuotaExceeded {
            drive: drive.letter(),
            used: effective_usage,
            limit: quota,
        });
    }

    if existing.is_none() {
        ensure_parent_exists(conn, scope, drive, path, user_id, create_parents)?;
    }

    if existing.is_some() {
        scoped_update_file(
            conn,
            drive.as_str(),
            &filter,
            path,
            size_bytes,
            content_type,
            None,
            Some(media_hash),
            user_id,
        )
        .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    } else {
        let (connection_id, session_id, user_id_col) = scope_ids(&filter);
        let new_file = NewVfsFile {
            drive: drive.as_str(),
            connection_id,
            session_id,
            user_id: user_id_col,
            path,
            is_directory: false,
            size_bytes,
            content_type,
            inline_data: None,
            media_hash: Some(media_hash),
            modified_by: Some(user_id),
            mode: apply_umask(DEFAULT_FILE_MODE, scope.umask),
        };
        diesel::insert_into(vfs_files::table)
            .values(&new_file)
            .execute(conn)
            .map_err(|e| VfsError::DatabaseError(e.to_string()))?;
    }

    Ok(())
}

/// Change the permission mode of a file or directory.
#[cfg(feature = "ssr")]
pub fn vfs_chmod(
    conn: &mut diesel::SqliteConnection,
    scope: &VfsScope,
    drive: Drive,
    path: &str,
    mode: i32,
    user_id: i32,
) -> Result<(), VfsError> {
    // Only GM can change permissions
    if !scope.is_gm {
        return Err(VfsError::PermissionDenied(format!(
            "{}:{}",
            drive.letter(),
            path
        )));
    }
    let filter = scope.scope_for_drive(drive)?;
    let drive_str = drive.as_str();
    macro_rules! do_chmod {
        ($target:expr) => {
            diesel::update($target)
                .set((
                    vfs_files::mode.eq(mode & 0o777),
                    vfs_files::modified_by.eq(user_id),
                    vfs_files::updated_at.eq(diesel::dsl::sql::<diesel::sql_types::Integer>(
                        "unixepoch()",
                    )),
                ))
                .execute(conn)
        };
    }
    let updated = match &filter {
        DriveFilter::Connection(cid) => do_chmod!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::connection_id.eq(cid.as_str()))
                .filter(vfs_files::path.eq(path))
        ),
        DriveFilter::Session(sid) => do_chmod!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::session_id.eq(*sid))
                .filter(vfs_files::path.eq(path))
        ),
        DriveFilter::User(uid) => do_chmod!(
            vfs_files::table
                .filter(vfs_files::drive.eq(drive_str))
                .filter(vfs_files::user_id.eq(*uid))
                .filter(vfs_files::path.eq(path))
        ),
    }
    .map_err(|e| VfsError::DatabaseError(e.to_string()))?;

    if updated == 0 {
        return Err(VfsError::NotFound(format!("{}:{}", drive.letter(), path)));
    }
    Ok(())
}

/// Delete all files on scratch drives for a given connection.
///
/// Called on WebSocket disconnect to clean up ephemeral A: and B: drives.
#[cfg(feature = "ssr")]
pub fn vfs_cleanup_connection(
    conn: &mut diesel::SqliteConnection,
    connection_id: &str,
) -> Result<usize, VfsError> {
    diesel::delete(vfs_files::table.filter(vfs_files::connection_id.eq(connection_id)))
        .execute(conn)
        .map_err(|e| VfsError::DatabaseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_path() {
        let p = VfsPath::parse("C:/maps/dungeon.png").unwrap();
        assert_eq!(p.drive, Drive::C);
        assert_eq!(p.path, "/maps/dungeon.png");
    }

    #[test]
    fn parse_root() {
        let p = VfsPath::parse("U:/").unwrap();
        assert_eq!(p.drive, Drive::U);
        assert_eq!(p.path, "/");
    }

    #[test]
    fn parse_no_slash() {
        let p = VfsPath::parse("A:file.txt").unwrap();
        assert_eq!(p.drive, Drive::A);
        assert_eq!(p.path, "/file.txt");
    }

    #[test]
    fn parse_collapse_slashes() {
        let p = VfsPath::parse("C:///maps///test.png").unwrap();
        assert_eq!(p.path, "/maps/test.png");
    }

    #[test]
    fn parse_dot_removal() {
        let p = VfsPath::parse("C:/./maps/./test.png").unwrap();
        assert_eq!(p.path, "/maps/test.png");
    }

    #[test]
    fn parse_dotdot_resolves() {
        let p = VfsPath::parse("C:/maps/sub/../dungeon.png").unwrap();
        assert_eq!(p.path, "/maps/dungeon.png");
    }

    #[test]
    fn parse_dotdot_at_root_clamps() {
        let p = VfsPath::parse("C:/../../../etc/passwd").unwrap();
        assert_eq!(p.path, "/etc/passwd");
    }

    #[test]
    fn parse_dotdot_to_root() {
        let p = VfsPath::parse("C:/maps/..").unwrap();
        assert_eq!(p.path, "/");
    }

    #[test]
    fn parse_backslash_rejected() {
        assert!(matches!(
            VfsPath::parse("C:/maps\\test.png"),
            Err(VfsPathError::IllegalCharacter('\\'))
        ));
    }

    #[test]
    fn parse_trailing_slash_removed() {
        let p = VfsPath::parse("C:/maps/").unwrap();
        assert_eq!(p.path, "/maps");
    }

    #[test]
    fn parse_invalid_drive() {
        assert!(matches!(
            VfsPath::parse("Z:/test"),
            Err(VfsPathError::InvalidDriveLetter('Z'))
        ));
    }

    #[test]
    fn parse_empty() {
        assert!(matches!(VfsPath::parse(""), Err(VfsPathError::Empty)));
    }

    #[test]
    fn parse_no_drive_is_error() {
        // "C/test" has no colon, so it's a relative path — parse() requires a drive
        assert!(VfsPath::parse("C/test").is_err());
    }

    #[test]
    fn illegal_chars_rejected() {
        assert!(matches!(
            VfsPath::parse("C:/te*st"),
            Err(VfsPathError::IllegalCharacter('*'))
        ));
        assert!(matches!(
            VfsPath::parse("C:/te?st"),
            Err(VfsPathError::IllegalCharacter('?'))
        ));
        assert!(matches!(
            VfsPath::parse("C:/te\"st"),
            Err(VfsPathError::IllegalCharacter('"'))
        ));
        assert!(matches!(
            VfsPath::parse("C:/te<st"),
            Err(VfsPathError::IllegalCharacter('<'))
        ));
        assert!(matches!(
            VfsPath::parse("C:/te>st"),
            Err(VfsPathError::IllegalCharacter('>'))
        ));
        assert!(matches!(
            VfsPath::parse("C:/te|st"),
            Err(VfsPathError::IllegalCharacter('|'))
        ));
    }

    #[test]
    fn resolve_absolute_with_drive() {
        let cwd = VfsPath::parse("A:/scratch").unwrap();
        let p = VfsPath::resolve("C:/maps/test.png", &cwd).unwrap();
        assert_eq!(p.drive, Drive::C);
        assert_eq!(p.path, "/maps/test.png");
    }

    #[test]
    fn resolve_absolute_no_drive() {
        let cwd = VfsPath::parse("C:/maps").unwrap();
        let p = VfsPath::resolve("/other/file.txt", &cwd).unwrap();
        assert_eq!(p.drive, Drive::C);
        assert_eq!(p.path, "/other/file.txt");
    }

    #[test]
    fn resolve_relative() {
        let cwd = VfsPath::parse("C:/maps").unwrap();
        let p = VfsPath::resolve("dungeon.png", &cwd).unwrap();
        assert_eq!(p.drive, Drive::C);
        assert_eq!(p.path, "/maps/dungeon.png");
    }

    #[test]
    fn resolve_relative_dotdot() {
        let cwd = VfsPath::parse("C:/maps/sub").unwrap();
        let p = VfsPath::resolve("../dungeon.png", &cwd).unwrap();
        assert_eq!(p.drive, Drive::C);
        assert_eq!(p.path, "/maps/dungeon.png");
    }

    #[test]
    fn resolve_relative_from_root() {
        let cwd = VfsPath::parse("C:/").unwrap();
        let p = VfsPath::resolve("file.txt", &cwd).unwrap();
        assert_eq!(p.path, "/file.txt");
    }

    #[test]
    fn resolve_single_letter_dir() {
        // "C/test" from cwd should resolve as directory "C" containing "test"
        let cwd = VfsPath::parse("U:/").unwrap();
        let p = VfsPath::resolve("C/test", &cwd).unwrap();
        assert_eq!(p.drive, Drive::U);
        assert_eq!(p.path, "/C/test");
    }

    #[test]
    fn parse_case_insensitive_drive() {
        let p = VfsPath::parse("c:/test").unwrap();
        assert_eq!(p.drive, Drive::C);
    }

    #[test]
    fn display_path() {
        let p = VfsPath::parse("C:/maps/dungeon.png").unwrap();
        assert_eq!(format!("{}", p), "C:/maps/dungeon.png");
    }

    #[test]
    fn parent_path() {
        let p = VfsPath::parse("C:/maps/dungeon.png").unwrap();
        assert_eq!(p.parent(), Some("/maps".to_string()));
    }

    #[test]
    fn parent_root() {
        let p = VfsPath::parse("C:/").unwrap();
        assert_eq!(p.parent(), None);
    }

    #[test]
    fn parent_top_level() {
        let p = VfsPath::parse("C:/file.txt").unwrap();
        assert_eq!(p.parent(), Some("/".to_string()));
    }

    #[test]
    fn filename() {
        let p = VfsPath::parse("C:/maps/dungeon.png").unwrap();
        assert_eq!(p.filename(), Some("dungeon.png"));
    }

    #[test]
    fn filename_root() {
        let p = VfsPath::parse("C:/").unwrap();
        assert_eq!(p.filename(), None);
    }

    #[test]
    fn drive_quotas() {
        assert_eq!(Drive::A.quota_bytes(false), 10 * 1024 * 1024);
        assert_eq!(Drive::C.quota_bytes(false), 100 * 1024 * 1024);
        assert_eq!(Drive::U.quota_bytes(false), 10 * 1024 * 1024);
        assert_eq!(Drive::U.quota_bytes(true), 20 * 1024 * 1024);
    }

    // ===== Database integration tests =====

    #[cfg(feature = "ssr")]
    mod db_tests {
        use super::super::*;
        use diesel::SqliteConnection;
        use diesel::prelude::*;

        const VFS_SCHEMA: &str = r#"
            CREATE TABLE vfs_files (
                id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                drive CHAR(1) NOT NULL,
                connection_id VARCHAR(36),
                session_id INTEGER,
                user_id INTEGER,
                path TEXT NOT NULL,
                is_directory BOOLEAN NOT NULL DEFAULT FALSE,
                size_bytes INTEGER NOT NULL DEFAULT 0,
                content_type VARCHAR(100),
                inline_data BLOB,
                media_hash VARCHAR(64),
                modified_by INTEGER,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
                mode INTEGER NOT NULL DEFAULT 438,
                CHECK (
                    (drive IN ('A','B') AND connection_id IS NOT NULL AND session_id IS NULL AND user_id IS NULL) OR
                    (drive = 'C' AND session_id IS NOT NULL AND connection_id IS NULL AND user_id IS NULL) OR
                    (drive = 'U' AND user_id IS NOT NULL AND connection_id IS NULL AND session_id IS NULL)
                )
            );
        "#;

        fn test_db() -> SqliteConnection {
            let mut conn = SqliteConnection::establish(":memory:").unwrap();
            diesel::sql_query(VFS_SCHEMA).execute(&mut conn).unwrap();
            conn
        }

        fn user_scope() -> VfsScope {
            VfsScope {
                connection_id: None,
                session_id: None,
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            }
        }

        fn session_scope() -> VfsScope {
            VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            }
        }

        fn scratch_scope() -> VfsScope {
            VfsScope {
                connection_id: Some("test-conn-001".to_string()),
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            }
        }

        #[test]
        fn write_and_read_inline() {
            let mut conn = test_db();
            let scope = user_scope();
            let data = b"hello world";
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/test.txt",
                data,
                Some("text/plain"),
                None,
                1,
                false,
            )
            .unwrap();

            match vfs_read(&mut conn, &scope, Drive::U, "/test.txt").unwrap() {
                VfsFileContent::Inline { data, content_type } => {
                    assert_eq!(data, b"hello world");
                    assert_eq!(content_type, Some("text/plain".to_string()));
                }
                VfsFileContent::CasReference { .. } => panic!("expected inline"),
            }
        }

        #[test]
        fn write_and_stat() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/doc.txt",
                b"abc",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            let entry = vfs_stat(&mut conn, &scope, Drive::U, "/doc.txt").unwrap();
            assert!(!entry.is_directory);
            assert_eq!(entry.size_bytes, 3);
        }

        #[test]
        fn mkdir_and_list() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/maps", 1, false).unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/maps/a.png",
                b"img",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/maps/b.png",
                b"img2",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            let entries = vfs_list(&mut conn, &scope, Drive::U, "/maps").unwrap();
            assert_eq!(entries.len(), 2);
            assert_eq!(entries[0].path, "/maps/a.png");
            assert_eq!(entries[1].path, "/maps/b.png");
        }

        #[test]
        fn list_excludes_nested() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/a", 1, false).unwrap();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/a/b", 1, false).unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/a/file.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/a/b/deep.txt",
                b"y",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            let entries = vfs_list(&mut conn, &scope, Drive::U, "/a").unwrap();
            let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
            assert!(paths.contains(&"/a/b"));
            assert!(paths.contains(&"/a/file.txt"));
            assert!(!paths.contains(&"/a/b/deep.txt"));
        }

        #[test]
        fn delete_file() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/tmp.txt",
                b"data",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_delete(&mut conn, &scope, Drive::U, "/tmp.txt").unwrap();
            assert!(matches!(
                vfs_stat(&mut conn, &scope, Drive::U, "/tmp.txt"),
                Err(VfsError::NotFound(_))
            ));
        }

        #[test]
        fn delete_nonempty_dir_fails() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/stuff", 1, false).unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/stuff/f.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            assert!(matches!(
                vfs_delete(&mut conn, &scope, Drive::U, "/stuff"),
                Err(VfsError::DirectoryNotEmpty(_))
            ));
        }

        #[test]
        fn overwrite_preserves_quota() {
            let mut conn = test_db();
            let scope = user_scope();
            let data = vec![0u8; 1000];
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/f.bin",
                &data,
                None,
                None,
                1,
                false,
            )
            .unwrap();
            let usage1 = vfs_drive_usage(&mut conn, &scope, Drive::U).unwrap();

            // Overwrite with same size — usage should not double
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/f.bin",
                &data,
                None,
                None,
                1,
                false,
            )
            .unwrap();
            let usage2 = vfs_drive_usage(&mut conn, &scope, Drive::U).unwrap();
            assert_eq!(usage1, usage2);
        }

        #[test]
        fn quota_exceeded() {
            let mut conn = test_db();
            // Use scratch drive (10 MB limit) for a tractable test
            let scope = scratch_scope();
            // Write a 5 MB file (needs media_hash since it exceeds inline threshold)
            let data_5mb = vec![0u8; 5 * 1024 * 1024];
            vfs_write(
                &mut conn,
                &scope,
                Drive::A,
                "/big1.bin",
                &data_5mb,
                None,
                Some("hash1"),
                1,
                false,
            )
            .unwrap();
            // Write another 5 MB — should succeed (exactly at limit)
            vfs_write(
                &mut conn,
                &scope,
                Drive::A,
                "/big2.bin",
                &data_5mb,
                None,
                Some("hash2"),
                1,
                false,
            )
            .unwrap();
            // One more byte should fail
            assert!(matches!(
                vfs_write(
                    &mut conn,
                    &scope,
                    Drive::A,
                    "/overflow.bin",
                    &[0u8],
                    None,
                    None,
                    1,
                    false
                ),
                Err(VfsError::QuotaExceeded { .. })
            ));
        }

        #[test]
        fn rename_file() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/old.txt",
                b"data",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_rename(&mut conn, &scope, Drive::U, "/old.txt", "/new.txt", 1).unwrap();

            assert!(matches!(
                vfs_stat(&mut conn, &scope, Drive::U, "/old.txt"),
                Err(VfsError::NotFound(_))
            ));
            let entry = vfs_stat(&mut conn, &scope, Drive::U, "/new.txt").unwrap();
            assert_eq!(entry.size_bytes, 4);
        }

        #[test]
        fn rename_directory_updates_descendants() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/old", 1, false).unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/old/a.txt",
                b"a",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/old/sub", 1, false).unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/old/sub/b.txt",
                b"b",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            vfs_rename(&mut conn, &scope, Drive::U, "/old", "/new", 1).unwrap();

            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/new").is_ok());
            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/new/a.txt").is_ok());
            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/new/sub").is_ok());
            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/new/sub/b.txt").is_ok());
            // Old paths should be gone
            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/old").is_err());
            assert!(vfs_stat(&mut conn, &scope, Drive::U, "/old/a.txt").is_err());
        }

        #[test]
        fn rename_replaces_target_file() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/src.txt",
                b"new",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/dst.txt",
                b"old",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_rename(&mut conn, &scope, Drive::U, "/src.txt", "/dst.txt", 1).unwrap();

            match vfs_read(&mut conn, &scope, Drive::U, "/dst.txt").unwrap() {
                VfsFileContent::Inline { data, .. } => assert_eq!(data, b"new"),
                _ => panic!("expected inline"),
            }
        }

        #[test]
        fn cleanup_connection() {
            let mut conn = test_db();
            let scope = scratch_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::A,
                "/tmp1.txt",
                b"a",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_write(
                &mut conn,
                &scope,
                Drive::A,
                "/tmp2.txt",
                b"b",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            let deleted = vfs_cleanup_connection(&mut conn, "test-conn-001").unwrap();
            assert_eq!(deleted, 2);
            assert_eq!(vfs_drive_usage(&mut conn, &scope, Drive::A).unwrap(), 0);
        }

        #[test]
        fn scope_isolation() {
            let mut conn = test_db();
            let scope1 = VfsScope {
                connection_id: None,
                session_id: None,
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };
            let scope2 = VfsScope {
                connection_id: None,
                session_id: None,
                user_id: Some(2),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };
            vfs_write(
                &mut conn,
                &scope1,
                Drive::U,
                "/secret.txt",
                b"mine",
                None,
                None,
                1,
                false,
            )
            .unwrap();

            // User 2 should not see user 1's files
            assert!(matches!(
                vfs_stat(&mut conn, &scope2, Drive::U, "/secret.txt"),
                Err(VfsError::NotFound(_))
            ));
        }

        #[test]
        fn write_without_parent_fails() {
            let mut conn = test_db();
            let scope = user_scope();
            assert!(matches!(
                vfs_write(
                    &mut conn,
                    &scope,
                    Drive::U,
                    "/nonexistent/file.txt",
                    b"data",
                    None,
                    None,
                    1,
                    false
                ),
                Err(VfsError::NotFound(_))
            ));
        }

        #[test]
        fn write_create_parents() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/a/b/c/file.txt",
                b"deep",
                None,
                None,
                1,
                true,
            )
            .unwrap();

            // Verify parent directories were created
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a")
                    .unwrap()
                    .is_directory
            );
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a/b")
                    .unwrap()
                    .is_directory
            );
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a/b/c")
                    .unwrap()
                    .is_directory
            );
            assert_eq!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a/b/c/file.txt")
                    .unwrap()
                    .size_bytes,
                4
            );
        }

        #[test]
        fn mkdir_create_parents() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_mkdir(&mut conn, &scope, Drive::U, "/x/y/z", 1, true).unwrap();

            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/x")
                    .unwrap()
                    .is_directory
            );
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/x/y")
                    .unwrap()
                    .is_directory
            );
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/x/y/z")
                    .unwrap()
                    .is_directory
            );
        }

        #[test]
        fn mkdir_without_parent_fails() {
            let mut conn = test_db();
            let scope = user_scope();
            assert!(matches!(
                vfs_mkdir(&mut conn, &scope, Drive::U, "/no/parent", 1, false),
                Err(VfsError::NotFound(_))
            ));
        }

        #[test]
        fn large_file_without_hash_fails() {
            let mut conn = test_db();
            let scope = user_scope();
            let big_data = vec![0u8; INLINE_THRESHOLD + 1];
            assert!(matches!(
                vfs_write(
                    &mut conn,
                    &scope,
                    Drive::U,
                    "/big.bin",
                    &big_data,
                    None,
                    None,
                    1,
                    false
                ),
                Err(VfsError::StorageError(_))
            ));
        }

        #[test]
        fn copy_inline_file() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/orig.txt",
                b"hello",
                Some("text/plain"),
                None,
                1,
                false,
            )
            .unwrap();
            vfs_copy(
                &mut conn,
                &scope,
                Drive::U,
                "/orig.txt",
                &scope,
                Drive::U,
                "/copy.txt",
                1,
                false,
            )
            .unwrap();

            match vfs_read(&mut conn, &scope, Drive::U, "/copy.txt").unwrap() {
                VfsFileContent::Inline { data, content_type } => {
                    assert_eq!(data, b"hello");
                    assert_eq!(content_type, Some("text/plain".to_string()));
                }
                _ => panic!("expected inline"),
            }
        }

        #[test]
        fn copy_across_scopes() {
            let mut conn = test_db();
            let user_scope = VfsScope {
                connection_id: None,
                session_id: None,
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };
            let session_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };

            vfs_write(
                &mut conn,
                &user_scope,
                Drive::U,
                "/mine.txt",
                b"data",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_copy(
                &mut conn,
                &user_scope,
                Drive::U,
                "/mine.txt",
                &session_scope,
                Drive::C,
                "/shared.txt",
                1,
                false,
            )
            .unwrap();

            // Both copies should exist independently
            assert!(vfs_stat(&mut conn, &user_scope, Drive::U, "/mine.txt").is_ok());
            assert!(vfs_stat(&mut conn, &session_scope, Drive::C, "/shared.txt").is_ok());
        }

        #[test]
        fn copy_with_create_parents() {
            let mut conn = test_db();
            let scope = user_scope();
            vfs_write(
                &mut conn,
                &scope,
                Drive::U,
                "/src.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_copy(
                &mut conn,
                &scope,
                Drive::U,
                "/src.txt",
                &scope,
                Drive::U,
                "/a/b/dst.txt",
                1,
                true,
            )
            .unwrap();

            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a")
                    .unwrap()
                    .is_directory
            );
            assert!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a/b")
                    .unwrap()
                    .is_directory
            );
            assert_eq!(
                vfs_stat(&mut conn, &scope, Drive::U, "/a/b/dst.txt")
                    .unwrap()
                    .size_bytes,
                1
            );
        }

        #[test]
        fn permission_read_denied() {
            let mut conn = test_db();
            let gm_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: true,
                umask: DEFAULT_UMASK,
            };
            let player_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };

            // GM writes a file, then makes it unreadable to others
            vfs_write(
                &mut conn,
                &gm_scope,
                Drive::C,
                "/secret.txt",
                b"hidden",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_chmod(&mut conn, &gm_scope, Drive::C, "/secret.txt", 0o600, 1).unwrap();

            // Player can't read it
            assert!(matches!(
                vfs_read(&mut conn, &player_scope, Drive::C, "/secret.txt"),
                Err(VfsError::PermissionDenied(_))
            ));

            // GM can still read it
            assert!(vfs_read(&mut conn, &gm_scope, Drive::C, "/secret.txt").is_ok());
        }

        #[test]
        fn permission_write_denied() {
            let mut conn = test_db();
            let gm_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: true,
                umask: DEFAULT_UMASK,
            };
            let player_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };

            // GM writes a read-only file
            vfs_write(
                &mut conn,
                &gm_scope,
                Drive::C,
                "/readonly.txt",
                b"data",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_chmod(&mut conn, &gm_scope, Drive::C, "/readonly.txt", 0o644, 1).unwrap();

            // Player can't overwrite it
            assert!(matches!(
                vfs_write(
                    &mut conn,
                    &player_scope,
                    Drive::C,
                    "/readonly.txt",
                    b"new",
                    None,
                    None,
                    1,
                    false
                ),
                Err(VfsError::PermissionDenied(_))
            ));
        }

        #[test]
        fn permission_delete_denied() {
            let mut conn = test_db();
            let gm_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: true,
                umask: DEFAULT_UMASK,
            };
            let player_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };

            vfs_write(
                &mut conn,
                &gm_scope,
                Drive::C,
                "/protected.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            vfs_chmod(&mut conn, &gm_scope, Drive::C, "/protected.txt", 0o444, 1).unwrap();

            assert!(matches!(
                vfs_delete(&mut conn, &player_scope, Drive::C, "/protected.txt"),
                Err(VfsError::PermissionDenied(_))
            ));
        }

        #[test]
        fn permission_list_denied() {
            let mut conn = test_db();
            let gm_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: true,
                umask: DEFAULT_UMASK,
            };
            let player_scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: DEFAULT_UMASK,
            };

            vfs_mkdir(&mut conn, &gm_scope, Drive::C, "/private", 1, false).unwrap();
            vfs_chmod(&mut conn, &gm_scope, Drive::C, "/private", 0o700, 1).unwrap();

            assert!(matches!(
                vfs_list(&mut conn, &player_scope, Drive::C, "/private"),
                Err(VfsError::PermissionDenied(_))
            ));
        }

        #[test]
        fn umask_applied() {
            let mut conn = test_db();
            let scope = VfsScope {
                connection_id: None,
                session_id: Some(1),
                user_id: Some(1),
                is_gm: false,
                umask: 0o022, // remove write from group/other
            };

            vfs_write(
                &mut conn,
                &scope,
                Drive::C,
                "/masked.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            let entry = vfs_stat(&mut conn, &scope, Drive::C, "/masked.txt").unwrap();
            assert_eq!(entry.mode, 0o644); // 0o666 & !0o022

            vfs_mkdir(&mut conn, &scope, Drive::C, "/masked_dir", 1, false).unwrap();
            let dir_entry = vfs_stat(&mut conn, &scope, Drive::C, "/masked_dir").unwrap();
            assert_eq!(dir_entry.mode, 0o755); // 0o777 & !0o022
        }

        #[test]
        fn chmod_non_gm_denied() {
            let mut conn = test_db();
            let scope = session_scope(); // is_gm: false
            vfs_write(
                &mut conn,
                &scope,
                Drive::C,
                "/file.txt",
                b"x",
                None,
                None,
                1,
                false,
            )
            .unwrap();
            assert!(matches!(
                vfs_chmod(&mut conn, &scope, Drive::C, "/file.txt", 0o444, 1),
                Err(VfsError::PermissionDenied(_))
            ));
        }
    }

    // ===== fnmatch tests =====

    #[test]
    fn fnmatch_star() {
        assert!(vfs_fnmatch("*.txt", "readme.txt"));
        assert!(vfs_fnmatch("*.txt", ".txt"));
        assert!(!vfs_fnmatch("*.txt", "readme.md"));
    }

    #[test]
    fn fnmatch_question() {
        assert!(vfs_fnmatch("?.txt", "a.txt"));
        assert!(!vfs_fnmatch("?.txt", "ab.txt"));
        assert!(!vfs_fnmatch("?.txt", ".txt"));
    }

    #[test]
    fn fnmatch_star_middle() {
        assert!(vfs_fnmatch("read*me", "readme"));
        assert!(vfs_fnmatch("read*me", "read-this-me"));
        assert!(!vfs_fnmatch("read*me", "readmex"));
    }

    #[test]
    fn fnmatch_case_insensitive() {
        assert!(vfs_fnmatch("*.TXT", "readme.txt"));
        assert!(vfs_fnmatch("*.txt", "README.TXT"));
        assert!(vfs_fnmatch("Read*", "readme"));
    }

    #[test]
    fn fnmatch_bracket() {
        assert!(vfs_fnmatch("[abc].txt", "a.txt"));
        assert!(vfs_fnmatch("[abc].txt", "b.txt"));
        assert!(!vfs_fnmatch("[abc].txt", "d.txt"));
    }

    #[test]
    fn fnmatch_bracket_range() {
        assert!(vfs_fnmatch("[a-z].txt", "m.txt"));
        assert!(!vfs_fnmatch("[a-z].txt", "1.txt"));
    }

    #[test]
    fn fnmatch_bracket_negate() {
        assert!(!vfs_fnmatch("[!abc].txt", "a.txt"));
        assert!(vfs_fnmatch("[!abc].txt", "d.txt"));
        assert!(vfs_fnmatch("[^abc].txt", "x.txt"));
    }

    #[test]
    fn fnmatch_literal() {
        assert!(vfs_fnmatch("readme.txt", "readme.txt"));
        assert!(!vfs_fnmatch("readme.txt", "readme.md"));
    }

    #[test]
    fn fnmatch_star_all() {
        assert!(vfs_fnmatch("*", "anything"));
        assert!(vfs_fnmatch("*", ""));
        assert!(vfs_fnmatch("*.*", "file.txt"));
    }

    #[test]
    fn fnmatch_complex() {
        assert!(vfs_fnmatch("map_[0-9][0-9].png", "map_42.png"));
        assert!(!vfs_fnmatch("map_[0-9][0-9].png", "map_abc.png"));
    }
}
