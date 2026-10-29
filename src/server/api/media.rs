use leptos::prelude::*;

use crate::models::MediaInfo;

#[cfg(feature = "ssr")]
use super::get_current_user;

#[server]
pub async fn list_media(
    media_type: Option<String>,
    search: Option<String>,
    tag: Option<String>,
) -> Result<Vec<MediaInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let mut query = media::table.into_boxed();

    if let Some(ref mt) = media_type {
        query = query.filter(media::media_type.eq(mt));
    }

    let media_rows: Vec<Media> = query
        .order(media::id.desc())
        .select(Media::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let mut results = Vec::new();
    for m in media_rows {
        let tags: Vec<String> = media_tags::table
            .filter(media_tags::media_id.eq(m.id))
            .select(media_tags::tag)
            .load(conn)
            .unwrap_or_default();

        // Filter by tag if specified
        if let Some(ref filter_tag) = tag
            && !tags.iter().any(|t| t == filter_tag)
        {
            continue;
        }

        // Filter by search term (matches against tags)
        if let Some(ref search_term) = search {
            let term = search_term.to_lowercase();
            if !tags.iter().any(|t| t.to_lowercase().contains(&term)) {
                continue;
            }
        }

        results.push(MediaInfo {
            id: m.id,
            hash: m.hash.clone(),
            url: format!("/api/media/{}", m.hash),
            content_type: m.content_type,
            media_type: m.media_type,
            size_bytes: m.size_bytes,
            tags,
        });
    }

    Ok(results)
}

#[server]
pub async fn list_media_tags(prefix: Option<String>) -> Result<Vec<String>, ServerFnError> {
    use crate::db;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let mut query = media_tags::table
        .select(media_tags::tag)
        .distinct()
        .into_boxed();

    if let Some(ref p) = prefix {
        // The prefix is typed by a user, so `%` and `_` in it must match
        // themselves rather than act as wildcards.
        query = query.filter(
            media_tags::tag
                .like(format!("{}%", crate::vfs::like_escape(p)))
                .escape('\\'),
        );
    }

    let tags: Vec<String> = query
        .order(media_tags::tag.asc())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(tags)
}

#[server]
pub async fn add_media_tag(media_id: i32, tag: String) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::NewMediaTag;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let _ = diesel::insert_or_ignore_into(media_tags::table)
        .values(&NewMediaTag {
            media_id,
            tag: &tag,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to add tag: {e}")))?;

    Ok(())
}

#[server]
pub async fn remove_media_tag(media_id: i32, tag: String) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    diesel::delete(
        media_tags::table
            .filter(media_tags::media_id.eq(media_id))
            .filter(media_tags::tag.eq(&tag)),
    )
    .execute(conn)
    .map_err(|e| ServerFnError::new(format!("Failed to remove tag: {e}")))?;

    Ok(())
}

#[server]
pub async fn delete_media(media_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::Media;
    use crate::schema::*;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let m: Media = media::table
        .find(media_id)
        .select(Media::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Media not found"))?;

    // Delete tags first, then media record
    diesel::delete(media_tags::table.filter(media_tags::media_id.eq(media_id)))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete tags: {e}")))?;

    diesel::delete(media::table.find(media_id))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete media: {e}")))?;

    // Delete file from disk
    let media_dir = std::path::PathBuf::from(
        std::env::var("MEDIA_DIR").unwrap_or_else(|_| "uploads/media".to_string()),
    );
    let file_path = media_dir.join(&m.hash[..2]).join(&m.hash);
    let _ = std::fs::remove_file(&file_path);

    Ok(())
}

#[server]
pub async fn update_character_portrait(
    character_id: i32,
    portrait_url: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::Character;
    use crate::schema::characters;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let character: Character = characters::table
        .find(character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Character not found"))?;

    if character.user_id != user.id {
        return Err(ServerFnError::new("Not your character"));
    }

    diesel::update(characters::table.find(character_id))
        .set(characters::portrait_url.eq(&portrait_url))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to update portrait: {e}")))?;

    // Update image on any tokens linked to this character
    use crate::schema::tokens;
    let updated_token_ids: Vec<i32> = tokens::table
        .filter(tokens::character_id.eq(character_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();
    if !updated_token_ids.is_empty() {
        diesel::update(tokens::table.filter(tokens::character_id.eq(character_id)))
            .set(tokens::image_url.eq(&portrait_url))
            .execute(conn)
            .ok();
    }

    // Broadcast character change to all clients in the session
    use crate::ws::messages::ServerMessage;
    use crate::ws::session::SESSION_MANAGER;
    SESSION_MANAGER.broadcast(
        character.session_id,
        &ServerMessage::CharacterUpdated {
            character_id,
            field_path: "portrait_url".to_string(),
            value: serde_json::json!(portrait_url),
        },
        None,
    );

    // Broadcast token image updates
    for tid in updated_token_ids {
        SESSION_MANAGER.broadcast(
            character.session_id,
            &ServerMessage::TokenImageUpdated {
                token_id: tid,
                image_url: portrait_url.clone(),
            },
            None,
        );
    }

    Ok(())
}
