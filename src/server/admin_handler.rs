//! Axum handlers for the operational admin actions that move raw files or
//! restart the process: database backup download and restore from upload. These
//! are plain Axum routes rather than Leptos server functions because they stream
//! file bytes and set download/upload headers directly. Each one authenticates
//! the request's cookie and requires administrator access.

use axum::extract::Multipart;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth;
use crate::db;
use crate::models::is_admin_level;
use crate::schema::users;

/// Uploaded restore files are capped to keep a single request from buffering an
/// unbounded amount into memory. SQLite databases for this app are small.
const MAX_RESTORE_SIZE: usize = 256 * 1024 * 1024; // 256 MB

/// The leading bytes of every SQLite 3 database file.
const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";

/// Require the request's auth cookie to belong to an administrator. Returns the
/// matching HTTP status on failure so handlers can short-circuit with `?`.
fn require_admin_headers(headers: &HeaderMap) -> Result<(), StatusCode> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let token = cookie_header
        .split(';')
        .find_map(|c| c.trim().strip_prefix("token="))
        .filter(|t| !t.is_empty())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = auth::verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let (user_id, _username) =
        auth::parse_claims_sub(&claims.sub).ok_or(StatusCode::UNAUTHORIZED)?;

    let conn = &mut db::get_conn();
    let level: i32 = users::table
        .find(user_id)
        .select(users::access_level)
        .first(conn)
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if is_admin_level(level) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// The configured database file path, rejecting in-memory databases that cannot
/// be backed up or restored as a file.
fn database_file_path() -> Result<String, StatusCode> {
    let url = std::env::var("DATABASE_URL").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if url == ":memory:" || url.starts_with("file::memory:") || url.contains("mode=memory") {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(url)
}

/// Schedule a hard process exit shortly after the current response is sent. The
/// delay lets the response flush; the exit relies on a process supervisor
/// (systemd, docker restart policy) to start the server again. `process::exit`
/// is deliberate: it skips destructors, so the connection pool cannot checkpoint
/// stale WAL pages over a database file a restore has just replaced.
pub fn schedule_restart() {
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        println!("Admin requested restart; exiting for the supervisor to restart.");
        std::process::exit(0);
    });
}

/// Download a consistent snapshot of the database. Admin only. Uses SQLite
/// `VACUUM INTO` to produce a clean copy (safe even with a live WAL), then
/// streams it as a file attachment.
pub async fn download_backup(headers: HeaderMap) -> Result<impl IntoResponse, StatusCode> {
    require_admin_headers(&headers)?;
    database_file_path()?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tmp = std::env::temp_dir().join(format!(
        "webrpg-backup-{}-{}-{}.db",
        std::process::id(),
        now.as_secs(),
        now.subsec_nanos()
    ));

    {
        let conn = &mut db::get_conn();
        let target = tmp.to_string_lossy().replace('\'', "''");
        conn.batch_execute(&format!("VACUUM INTO '{target}'"))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let bytes = tokio::fs::read(&tmp)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = tokio::fs::remove_file(&tmp).await;

    let filename = format!("webrpg-backup-{}.db", now.as_secs());
    let mut out = HeaderMap::new();
    out.insert(
        header::CONTENT_TYPE,
        "application/x-sqlite3"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    out.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{filename}\"")
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok((out, bytes))
}

/// Restore the database from an uploaded SQLite file. Admin only. Validates the
/// upload, copies the current database aside as a `.bak`, writes the uploaded
/// file over the configured database path, clears any stale WAL sidecars, and
/// restarts the server so connections reopen on the restored database.
pub async fn restore_backup(
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    require_admin_headers(&headers)?;
    let db_path = database_file_path()?;

    let mut data: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        if field.name() == Some("file") {
            let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if bytes.len() > MAX_RESTORE_SIZE {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            data = Some(bytes.to_vec());
        }
    }

    let bytes = data.ok_or(StatusCode::BAD_REQUEST)?;
    if bytes.len() < SQLITE_MAGIC.len() || &bytes[..SQLITE_MAGIC.len()] != SQLITE_MAGIC {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    // Keep the previous database as a timestamped backup before overwriting.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let bak = format!("{db_path}.{}.bak", now.as_secs());
    let _ = tokio::fs::copy(&db_path, &bak).await;

    tokio::fs::write(&db_path, &bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // The uploaded file is self-contained; stale WAL sidecars from the old
    // database would otherwise be replayed over it on restart.
    let _ = tokio::fs::remove_file(format!("{db_path}-wal")).await;
    let _ = tokio::fs::remove_file(format!("{db_path}-shm")).await;

    schedule_restart();

    Ok(Html(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Restore complete</title>\
         </head><body style=\"font-family:sans-serif;max-width:40rem;margin:4rem auto\">\
         <h1>Database restored</h1>\
         <p>The uploaded database has been installed and the previous one saved alongside it \
         as a <code>.bak</code> file. The server is restarting.</p>\
         <p>Once it is back up, <a href=\"/admin\">return to administration</a>.</p>\
         </body></html>",
    ))
}
