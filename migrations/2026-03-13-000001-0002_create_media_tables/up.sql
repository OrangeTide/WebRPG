CREATE TABLE media (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    hash VARCHAR(64) NOT NULL UNIQUE,
    content_type VARCHAR(100) NOT NULL,
    media_type VARCHAR(10) NOT NULL,
    size_bytes INTEGER NOT NULL,
    uploaded_by INTEGER NOT NULL REFERENCES users(id),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE media_tags (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    media_id INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    tag VARCHAR(255) NOT NULL,
    UNIQUE(media_id, tag)
);
CREATE INDEX idx_media_tags_tag ON media_tags(tag);
CREATE INDEX idx_media_tags_media_id ON media_tags(media_id);
