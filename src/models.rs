use serde::{Deserialize, Serialize};

// ===== Shared DTOs (compiled for both server and client) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: i32,
    pub name: String,
    pub gm_username: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageInfo {
    pub id: i32,
    pub username: String,
    pub message: String,
    pub is_dice_roll: bool,
    pub dice_result: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapInfo {
    pub id: i32,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub cell_size: i32,
    pub background_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub id: i32,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub color: String,
    pub size: i32,
    pub visible: bool,
    pub current_hp: Option<i32>,
    pub max_hp: Option<i32>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub id: i32,
    pub hash: String,
    pub url: String,
    pub content_type: String,
    pub media_type: String,
    pub size_bytes: i32,
    pub tags: Vec<String>,
}

/// A field definition within an RPG template schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub category: String,
    #[serde(default)]
    pub default: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Number,
    Text,
    Boolean,
    Textarea,
}

/// An RPG template with its parsed field schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub fields: Vec<TemplateField>,
}

/// A character with its data parsed according to the template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub id: i32,
    pub session_id: i32,
    pub user_id: i32,
    pub name: String,
    pub data: serde_json::Value,
    pub resources: Vec<ResourceInfo>,
    pub portrait_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInfo {
    pub id: i32,
    pub name: String,
    pub current_value: i32,
    pub max_value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatureInfo {
    pub id: i32,
    pub name: String,
    pub stat_data: serde_json::Value,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItemInfo {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub quantity: i32,
    pub is_party_item: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeEntryInfo {
    pub id: i32,
    pub label: String,
    pub initiative_value: f32,
    pub is_current_turn: bool,
    /// Portrait/icon URL for display (not persisted in DB).
    pub portrait_url: Option<String>,
}

// ===== Diesel models (server only) =====

#[cfg(feature = "ssr")]
pub mod db_models {
    use diesel::prelude::*;

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::users)]
    pub struct User {
        pub id: i32,
        pub username: String,
        pub display_name: String,
        pub email: String,
        pub access_level: i32,
        pub locked: bool,
        pub passcrypt: Option<String>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::users)]
    pub struct NewUser<'a> {
        pub username: &'a str,
        pub display_name: &'a str,
        pub email: &'a str,
        pub passcrypt: &'a str,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::sessions)]
    pub struct Session {
        pub id: i32,
        pub name: String,
        pub gm_user_id: i32,
        pub template_id: Option<i32>,
        pub active: bool,
        pub created_at: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::sessions)]
    pub struct NewSession<'a> {
        pub name: &'a str,
        pub gm_user_id: i32,
        pub template_id: Option<i32>,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::session_players)]
    pub struct SessionPlayer {
        pub id: i32,
        pub session_id: i32,
        pub user_id: i32,
        pub role: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::session_players)]
    pub struct NewSessionPlayer {
        pub session_id: i32,
        pub user_id: i32,
        pub role: String,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::rpg_templates)]
    pub struct RpgTemplate {
        pub id: i32,
        pub name: String,
        pub description: String,
        pub schema_json: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::rpg_templates)]
    pub struct NewRpgTemplate<'a> {
        pub name: &'a str,
        pub description: &'a str,
        pub schema_json: &'a str,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::characters)]
    pub struct Character {
        pub id: i32,
        pub session_id: i32,
        pub user_id: i32,
        pub name: String,
        pub data_json: String,
        pub created_at: String,
        pub portrait_url: Option<String>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::characters)]
    pub struct NewCharacter<'a> {
        pub session_id: i32,
        pub user_id: i32,
        pub name: &'a str,
        pub data_json: &'a str,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::character_resources)]
    pub struct CharacterResource {
        pub id: i32,
        pub character_id: i32,
        pub name: String,
        pub current_value: i32,
        pub max_value: i32,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::character_resources)]
    pub struct NewCharacterResource<'a> {
        pub character_id: i32,
        pub name: &'a str,
        pub current_value: i32,
        pub max_value: i32,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::creatures)]
    pub struct NewCreature<'a> {
        pub session_id: i32,
        pub template_id: Option<i32>,
        pub name: &'a str,
        pub stat_data_json: &'a str,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::maps)]
    pub struct Map {
        pub id: i32,
        pub session_id: i32,
        pub name: String,
        pub width: i32,
        pub height: i32,
        pub cell_size: i32,
        pub background_url: Option<String>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::maps)]
    pub struct NewMap<'a> {
        pub session_id: i32,
        pub name: &'a str,
        pub width: i32,
        pub height: i32,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::creatures)]
    pub struct Creature {
        pub id: i32,
        pub session_id: i32,
        pub template_id: Option<i32>,
        pub name: String,
        pub stat_data_json: String,
        pub image_url: Option<String>,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::tokens)]
    pub struct Token {
        pub id: i32,
        pub map_id: i32,
        pub label: String,
        pub x: f32,
        pub y: f32,
        pub color: String,
        pub size: i32,
        pub visible: bool,
        pub character_id: Option<i32>,
        pub creature_id: Option<i32>,
        pub image_url: Option<String>,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::token_instances)]
    pub struct TokenInstance {
        pub id: i32,
        pub token_id: i32,
        pub creature_id: i32,
        pub current_hp: i32,
        pub max_hp: i32,
        pub conditions_json: String,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::chat_messages)]
    pub struct ChatMessage {
        pub id: i32,
        pub session_id: i32,
        pub user_id: i32,
        pub message: String,
        pub is_dice_roll: bool,
        pub dice_result: Option<String>,
        pub created_at: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::chat_messages)]
    pub struct NewChatMessage<'a> {
        pub session_id: i32,
        pub user_id: i32,
        pub message: &'a str,
        pub is_dice_roll: bool,
        pub dice_result: Option<&'a str>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::tokens)]
    pub struct NewToken<'a> {
        pub map_id: i32,
        pub label: &'a str,
        pub x: f32,
        pub y: f32,
        pub color: &'a str,
        pub size: i32,
        pub visible: bool,
        pub creature_id: Option<i32>,
        pub image_url: Option<&'a str>,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::token_instances)]
    pub struct NewTokenInstance {
        pub token_id: i32,
        pub creature_id: i32,
        pub current_hp: i32,
        pub max_hp: i32,
        pub conditions_json: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::inventory_items)]
    pub struct NewInventoryItem<'a> {
        pub session_id: i32,
        pub name: &'a str,
        pub description: &'a str,
        pub quantity: i32,
        pub is_party_item: bool,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::initiative)]
    pub struct NewInitiativeEntry<'a> {
        pub session_id: i32,
        pub label: &'a str,
        pub initiative_value: f32,
        pub is_current_turn: bool,
        pub sort_order: i32,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::inventory_items)]
    pub struct InventoryItem {
        pub id: i32,
        pub session_id: i32,
        pub name: String,
        pub description: String,
        pub quantity: i32,
        pub owner_character_id: Option<i32>,
        pub is_party_item: bool,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::initiative)]
    pub struct InitiativeEntry {
        pub id: i32,
        pub session_id: i32,
        pub label: String,
        pub initiative_value: f32,
        pub is_current_turn: bool,
        pub token_id: Option<i32>,
        pub character_id: Option<i32>,
        pub sort_order: i32,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::media)]
    pub struct Media {
        pub id: i32,
        pub hash: String,
        pub content_type: String,
        pub media_type: String,
        pub size_bytes: i32,
        pub uploaded_by: i32,
        pub created_at: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::media)]
    pub struct NewMedia<'a> {
        pub hash: &'a str,
        pub content_type: &'a str,
        pub media_type: &'a str,
        pub size_bytes: i32,
        pub uploaded_by: i32,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::media_tags)]
    pub struct MediaTag {
        pub id: i32,
        pub media_id: i32,
        pub tag: String,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::media_tags)]
    pub struct NewMediaTag<'a> {
        pub media_id: i32,
        pub tag: &'a str,
    }

    // ===== VFS models =====

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::vfs_files)]
    pub struct VfsFile {
        pub id: i32,
        pub drive: String,
        pub connection_id: Option<String>,
        pub session_id: Option<i32>,
        pub user_id: Option<i32>,
        pub path: String,
        pub is_directory: bool,
        pub size_bytes: i32,
        pub content_type: Option<String>,
        pub inline_data: Option<Vec<u8>>,
        pub media_hash: Option<String>,
        pub modified_by: Option<i32>,
        pub created_at: i32,
        pub updated_at: i32,
        pub mode: i32,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::vfs_files)]
    pub struct NewVfsFile<'a> {
        pub drive: &'a str,
        pub connection_id: Option<&'a str>,
        pub session_id: Option<i32>,
        pub user_id: Option<i32>,
        pub path: &'a str,
        pub is_directory: bool,
        pub size_bytes: i32,
        pub content_type: Option<&'a str>,
        pub inline_data: Option<&'a [u8]>,
        pub media_hash: Option<&'a str>,
        pub modified_by: Option<i32>,
        pub mode: i32,
    }

    #[derive(Debug, Queryable, Selectable)]
    #[diesel(table_name = crate::schema::vfs_archive)]
    pub struct VfsArchive {
        pub id: i32,
        pub original_session_id: i32,
        pub session_name: String,
        pub path: String,
        pub size_bytes: i32,
        pub content_type: Option<String>,
        pub inline_data: Option<Vec<u8>>,
        pub media_hash: Option<String>,
        pub archived_at: i32,
        pub expires_at: i32,
    }

    #[derive(Debug, Insertable)]
    #[diesel(table_name = crate::schema::vfs_archive)]
    pub struct NewVfsArchive<'a> {
        pub original_session_id: i32,
        pub session_name: &'a str,
        pub path: &'a str,
        pub size_bytes: i32,
        pub content_type: Option<&'a str>,
        pub inline_data: Option<&'a [u8]>,
        pub media_hash: Option<&'a str>,
        pub expires_at: i32,
    }
}
