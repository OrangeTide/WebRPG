use leptos::prelude::*;

use crate::models::{VfsEntryInfo, VfsFileData};

// These server functions call get_current_user from the auth submodule.
#[cfg(feature = "ssr")]
use super::get_current_user;

/// Build a VfsScope for C: drive operations (session-scoped, shared).
#[cfg(feature = "ssr")]
fn vfs_session_scope(
    session_id: i32,
    user_id: i32,
    conn: &mut diesel::SqliteConnection,
) -> Result<crate::vfs::VfsScope, ServerFnError> {
    use crate::models::db_models::Session;
    use crate::schema::{session_players, sessions};
    use diesel::prelude::*;

    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let is_player: bool = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user_id))
        .count()
        .get_result::<i64>(conn)
        .unwrap_or(0)
        > 0;

    Ok(crate::vfs::VfsScope {
        session_id: Some(session_id),
        user_id: Some(user_id),
        is_gm: session.gm_user_id == user_id,
        is_player,
        umask: crate::vfs::DEFAULT_UMASK,
    })
}

/// Build a VfsScope for U: drive operations (user-scoped, private).
#[cfg(feature = "ssr")]
fn vfs_user_scope(user_id: i32) -> crate::vfs::VfsScope {
    crate::vfs::VfsScope {
        session_id: None,
        user_id: Some(user_id),
        is_gm: false,
        is_player: false,
        umask: crate::vfs::DEFAULT_UMASK,
    }
}

/// Parse a drive letter string ("C" or "U") into a Drive enum.
#[cfg(feature = "ssr")]
fn parse_drive(drive: &str) -> Result<crate::vfs::Drive, ServerFnError> {
    let c = drive
        .chars()
        .next()
        .ok_or_else(|| ServerFnError::new("Empty drive letter"))?;
    match c.to_ascii_uppercase() {
        'C' | 'U' => {}
        'A' | 'B' => {
            return Err(ServerFnError::new(
                "Scratch drives (A:, B:) are client-side only",
            ));
        }
        _ => return Err(ServerFnError::new("Invalid drive letter")),
    }
    crate::vfs::Drive::from_letter(c).ok_or_else(|| ServerFnError::new("Invalid drive letter"))
}

/// Build a VfsScope for a given drive, validating auth context.
#[cfg(feature = "ssr")]
fn vfs_scope_for(
    drive: crate::vfs::Drive,
    session_id: Option<i32>,
    user_id: i32,
    conn: &mut diesel::SqliteConnection,
) -> Result<crate::vfs::VfsScope, ServerFnError> {
    match drive {
        crate::vfs::Drive::C => {
            let sid =
                session_id.ok_or_else(|| ServerFnError::new("session_id required for C: drive"))?;
            vfs_session_scope(sid, user_id, conn)
        }
        crate::vfs::Drive::U => Ok(vfs_user_scope(user_id)),
        _ => Err(ServerFnError::new(
            "Only C: and U: drives are supported server-side",
        )),
    }
}

/// Common VFS preamble: authenticate, get DB connection, parse drive, build scope.
/// Returns (connection, drive, scope, user_id) or an error.
#[cfg(feature = "ssr")]
async fn vfs_auth_scope(
    drive_str: &str,
    session_id: Option<i32>,
) -> Result<
    (
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        crate::vfs::Drive,
        crate::vfs::VfsScope,
        i32,
    ),
    ServerFnError,
> {
    use crate::db;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let mut conn = db::get_conn();
    let drive = parse_drive(drive_str)?;
    let scope = vfs_scope_for(drive, session_id, user.id, &mut conn)?;
    Ok((conn, drive, scope, user.id))
}

/// Broadcast a VfsChanged notification for C: drive modifications.
#[cfg(feature = "ssr")]
fn vfs_broadcast(session_id: Option<i32>, path: &str, action: &str) {
    if let Some(sid) = session_id {
        use crate::ws::messages::ServerMessage;
        use crate::ws::session::SESSION_MANAGER;
        SESSION_MANAGER.broadcast(
            sid,
            &ServerMessage::VfsChanged {
                path: path.to_string(),
                action: action.to_string(),
            },
            None,
        );
    }
}

/// List directory contents.
#[server]
pub async fn vfs_list_dir(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<Vec<VfsEntryInfo>, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let entries = vfs::vfs_list(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(entries
        .into_iter()
        .map(|e| VfsEntryInfo {
            path: e.path,
            is_directory: e.is_directory,
            size_bytes: e.size_bytes,
            content_type: e.content_type,
            modified_by: e.modified_by,
            created_at: e.created_at,
            updated_at: e.updated_at,
            mode: e.mode,
        })
        .collect())
}

/// Get file/directory metadata.
#[server]
pub async fn vfs_stat_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<VfsEntryInfo, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let entry = vfs::vfs_stat(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(VfsEntryInfo {
        path: entry.path,
        is_directory: entry.is_directory,
        size_bytes: entry.size_bytes,
        content_type: entry.content_type,
        modified_by: entry.modified_by,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        mode: entry.mode,
    })
}

/// Read file contents. Returns inline data for small files, or a CAS URL
/// for large files that the client can fetch directly.
#[server]
pub async fn vfs_read_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<VfsFileData, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let content = vfs::vfs_read(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    match content {
        vfs::VfsFileContent::Inline { data, content_type } => {
            Ok(VfsFileData::Inline { data, content_type })
        }
        vfs::VfsFileContent::CasReference {
            hash,
            content_type,
            size_bytes,
        } => Ok(VfsFileData::CasUrl {
            url: format!("/api/media/{}", hash),
            content_type,
            size_bytes,
        }),
    }
}

/// Write a small file (inline, up to 8 KB). For larger files, use the
/// media upload endpoint and then call `vfs_write_cas` with the hash.
#[server]
pub async fn vfs_write_file(
    drive: String,
    path: String,
    data: Vec<u8>,
    content_type: Option<String>,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_write(
        conn,
        &scope,
        drive,
        &parsed.path,
        &data,
        content_type.as_deref(),
        None,
        user_id,
        true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "write");
    Ok(())
}

/// Write a CAS-referenced file (large file already uploaded via media endpoint).
#[server]
pub async fn vfs_write_cas(
    drive: String,
    path: String,
    media_hash: String,
    size_bytes: i64,
    content_type: Option<String>,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Verify the media hash exists in CAS
    use crate::schema::media;
    use diesel::prelude::*;
    let exists: bool = media::table
        .filter(media::hash.eq(&media_hash))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;
    if !exists {
        return Err(ServerFnError::new("Media hash not found in storage"));
    }

    // Use vfs_write with the media_hash for CAS reference
    let dummy = vec![0u8; size_bytes as usize];
    vfs::vfs_write(
        conn,
        &scope,
        drive,
        &parsed.path,
        &dummy,
        content_type.as_deref(),
        Some(&media_hash),
        user_id,
        true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "write");
    Ok(())
}

/// Create a directory.
#[server]
pub async fn vfs_mkdir_dir(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_mkdir(conn, &scope, drive, &parsed.path, user_id, true)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "mkdir");
    Ok(())
}

/// Delete a file or empty directory.
#[server]
pub async fn vfs_delete_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_delete(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "delete");
    Ok(())
}

/// Delete a file, or a directory and everything beneath it.
///
/// The file browser deletes folders this way, the way a file manager does.
/// COMMAND.COM's RMDIR uses [`vfs_delete_file`] instead, which refuses a
/// non-empty directory.
#[server]
pub async fn vfs_delete_tree(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_delete_recursive(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "delete");
    Ok(())
}

/// Rename or move a file/directory within the same drive.
#[server]
pub async fn vfs_rename_file(
    drive: String,
    old_path: String,
    new_path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let old = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), old_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let new = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), new_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_rename(conn, &scope, drive, &old.path, &new.path, user_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &old.path, "rename");
    Ok(())
}

/// Copy a file (within same drive or across C:/U: drives).
#[server]
pub async fn vfs_copy_file(
    src_drive: String,
    src_path: String,
    dst_drive: String,
    dst_path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, src_drv, src_scope, user_id) = vfs_auth_scope(&src_drive, session_id).await?;
    let conn = &mut conn;
    let dst_drv = parse_drive(&dst_drive)?;
    let dst_scope = vfs_scope_for(dst_drv, session_id, user_id, conn)?;
    let src = vfs::VfsPath::parse(&format!("{}:{}", src_drv.letter(), src_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let dst = vfs::VfsPath::parse(&format!("{}:{}", dst_drv.letter(), dst_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_copy(
        conn, &src_scope, src_drv, &src.path, &dst_scope, dst_drv, &dst.path, user_id, true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Broadcast if destination is on C: drive
    if matches!(dst_drv, vfs::Drive::C) {
        vfs_broadcast(session_id, &dst.path, "write");
    }
    Ok(())
}

/// Change file permissions (GM-only).
#[server]
pub async fn vfs_chmod_file(
    drive: String,
    path: String,
    mode: i32,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_chmod(conn, &scope, drive, &parsed.path, mode, user_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "chmod");
    Ok(())
}

/// Get drive usage and quota information.
#[server]
pub async fn vfs_get_drive_info(
    drive: String,
    session_id: Option<i32>,
) -> Result<crate::models::VfsDriveInfo, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;

    let used_bytes =
        vfs::vfs_drive_usage(conn, &scope, drive).map_err(|e| ServerFnError::new(e.to_string()))?;
    let quota_bytes = drive.quota_bytes(scope.is_gm);

    Ok(crate::models::VfsDriveInfo {
        drive: drive.letter(),
        used_bytes,
        quota_bytes,
    })
}
