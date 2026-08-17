//! Serving files a module ships in its `assets/` directory.
//!
//! Module art is read straight off disk rather than copied into media storage,
//! so a pack stays self-contained and editing a card's PNG shows up on reload.
//! Path safety lives in [`crate::modules::loader::asset_path`], which refuses
//! anything that could climb out of the module directory.

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;

use crate::modules::loader;

/// Content type for the image formats a module may ship.
fn content_type_for(name: &str) -> Option<&'static str> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// `GET /api/modules/{module_id}/assets/{*path}`
pub async fn serve_module_asset(
    Path((module_id, asset)): Path<(String, String)>,
) -> Result<impl IntoResponse, StatusCode> {
    let content_type = content_type_for(&asset).ok_or(StatusCode::BAD_REQUEST)?;
    let path = loader::asset_path(&module_id, &asset).ok_or(StatusCode::NOT_FOUND)?;

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        content_type
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    // Modules are edited in place during authoring, so this is deliberately
    // not the immutable caching that content-addressed media gets.
    headers.insert(
        header::CACHE_CONTROL,
        "public, max-age=300"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok((headers, bytes))
}

#[cfg(test)]
mod tests {
    use super::content_type_for;

    #[test]
    fn only_image_types_are_served() {
        assert_eq!(content_type_for("cards/torch.png"), Some("image/png"));
        assert_eq!(content_type_for("MAP.JPG"), Some("image/jpeg"));
        assert_eq!(content_type_for("module.json"), None);
        assert_eq!(content_type_for("notes"), None);
    }
}
