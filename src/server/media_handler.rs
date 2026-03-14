use axum::extract::{Multipart, Path};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use diesel::prelude::*;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::auth;
use crate::db;
use crate::models::db_models::*;
use crate::schema::*;

const MAX_FILE_SIZE: usize = 20 * 1024 * 1024; // 20 MB

fn media_dir() -> PathBuf {
    PathBuf::from(std::env::var("MEDIA_DIR").unwrap_or_else(|_| "uploads/media".to_string()))
}

fn allowed_content_type(ct: &str) -> Option<&'static str> {
    match ct {
        "image/png" => Some("image"),
        "image/jpeg" => Some("image"),
        "image/gif" => Some("image"),
        "image/webp" => Some("image"),
        "audio/wav" => Some("audio"),
        "audio/mpeg" => Some("audio"),
        "audio/mp3" => Some("audio"),
        _ => None,
    }
}

fn extract_jwt_from_cookies(headers: &HeaderMap) -> Result<(i32, String), StatusCode> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = cookie_header
        .split(';')
        .find_map(|cookie| cookie.trim().strip_prefix("token="))
        .filter(|t| !t.is_empty())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = auth::verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    auth::parse_claims_sub(&claims.sub).ok_or(StatusCode::UNAUTHORIZED)
}

pub async fn upload_media(
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let (user_id, _username) = extract_jwt_from_cookies(&headers)?;

    let mut file_data: Option<(String, Vec<u8>)> = None; // (content_type, bytes)
    let mut tags_str: Option<String> = None;
    let mut original_filename: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" => {
                let ct = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                original_filename = field.file_name().map(|s| s.to_string());
                let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

                if bytes.len() > MAX_FILE_SIZE {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }

                file_data = Some((ct, bytes.to_vec()));
            }
            "tags" => {
                tags_str = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            _ => {}
        }
    }

    let (content_type, bytes) = file_data.ok_or(StatusCode::BAD_REQUEST)?;
    let media_type =
        allowed_content_type(&content_type).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    // Compute SHA-256
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    // Write file to disk (dedup: skip if already exists)
    let dir = media_dir().join(&hash[..2]);
    let file_path = dir.join(&hash);

    if !file_path.exists() {
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        tokio::fs::write(&file_path, &bytes)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Insert into DB (or get existing)
    let conn = &mut db::get_conn();

    let existing: Option<Media> = media::table
        .filter(media::hash.eq(&hash))
        .select(Media::as_select())
        .first(conn)
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let media_id = if let Some(existing) = existing {
        existing.id
    } else {
        let new_media = NewMedia {
            hash: &hash,
            content_type: &content_type,
            media_type,
            size_bytes: bytes.len() as i32,
            uploaded_by: user_id,
        };

        diesel::insert_into(media::table)
            .values(&new_media)
            .execute(conn)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
            "last_insert_rowid()",
        ))
        .get_result::<i32>(conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Add tags
    let mut tag_list: Vec<String> = tags_str
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Add original filename as tag
    if let Some(ref fname) = original_filename {
        if !fname.is_empty() && !tag_list.iter().any(|t| t == fname) {
            tag_list.push(fname.clone());
        }
    }

    for tag in &tag_list {
        let new_tag = NewMediaTag { media_id, tag };
        // Ignore duplicate tag errors
        let _ = diesel::insert_or_ignore_into(media_tags::table)
            .values(&new_tag)
            .execute(conn);
    }

    let url = format!("/api/media/{hash}");

    let response = serde_json::json!({
        "id": media_id,
        "hash": hash,
        "url": url,
        "content_type": content_type,
    });

    Ok((StatusCode::OK, axum::Json(response)))
}

pub async fn serve_media(Path(hash): Path<String>) -> Result<impl IntoResponse, StatusCode> {
    // Validate hash format
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let conn = &mut db::get_conn();

    let media_row: Media = media::table
        .filter(media::hash.eq(&hash))
        .select(Media::as_select())
        .first(conn)
        .optional()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let file_path = media_dir().join(&hash[..2]).join(&hash);
    let bytes = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        media_row
            .content_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=31536000, immutable"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, bytes))
}
