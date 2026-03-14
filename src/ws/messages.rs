use serde::{Deserialize, Serialize};

use crate::models::{ChatMessageInfo, InitiativeEntryInfo, InventoryItemInfo, MapInfo, TokenInfo};

// ===== Client -> Server messages =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    JoinSession {
        session_id: i32,
    },
    LeaveSession,
    ChatMessage {
        message: String,
    },
    RollDice {
        expression: String,
    },
    MoveToken {
        token_id: i32,
        x: f32,
        y: f32,
    },
    PlaceToken {
        label: String,
        x: f32,
        y: f32,
        color: String,
        size: i32,
        creature_id: Option<i32>,
        image_url: Option<String>,
    },
    RemoveToken {
        token_id: i32,
    },
    UpdateTokenHp {
        token_id: i32,
        hp_change: i32,
    },
    RevealFog {
        cells: Vec<(i32, i32)>,
    },
    HideFog {
        cells: Vec<(i32, i32)>,
    },
    SetMap {
        map_id: i32,
    },
    UpdateInitiative {
        entries: Vec<InitiativeEntryInfo>,
    },
    UpdateCharacterField {
        character_id: i32,
        field_path: String,
        value: serde_json::Value,
    },
    AddInventoryItem {
        name: String,
        description: String,
        quantity: i32,
        is_party_item: bool,
    },
    RemoveInventoryItem {
        item_id: i32,
    },
    UpdateInventoryItem {
        item_id: i32,
        name: Option<String>,
        description: Option<String>,
        quantity: Option<i32>,
    },
    RollCharacterInitiative {
        character_id: i32,
    },
    RollCreatureInitiative {
        creature_id: i32,
        label: String,
    },
    SetInitiativeLock {
        locked: bool,
    },
    SetMapBackground {
        background_url: Option<String>,
    },
}

// ===== Server -> Client messages =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    SessionJoined {
        snapshot: GameStateSnapshot,
    },
    Error {
        message: String,
    },
    PlayerJoined {
        username: String,
    },
    PlayerLeft {
        username: String,
    },
    ChatBroadcast {
        message: ChatMessageInfo,
    },
    DiceResult {
        username: String,
        expression: String,
        rolls: Vec<i32>,
        total: i32,
    },
    TokenMoved {
        token_id: i32,
        x: f32,
        y: f32,
    },
    TokenPlaced {
        token: TokenInfo,
    },
    TokenRemoved {
        token_id: i32,
    },
    TokenHpUpdated {
        token_id: i32,
        current_hp: i32,
        max_hp: i32,
    },
    FogUpdated {
        revealed: Vec<(i32, i32)>,
        hidden: Vec<(i32, i32)>,
    },
    MapChanged {
        map: MapInfo,
        tokens: Vec<TokenInfo>,
        fog: Vec<(i32, i32)>,
    },
    InitiativeUpdated {
        entries: Vec<InitiativeEntryInfo>,
    },
    CharacterUpdated {
        character_id: i32,
        field_path: String,
        value: serde_json::Value,
    },
    CharacterResourceUpdated {
        character_id: i32,
        resource_id: i32,
        current_value: i32,
        max_value: i32,
    },
    InventoryUpdated {
        items: Vec<InventoryItemInfo>,
    },
    InitiativeLockChanged {
        locked: bool,
    },
    MapBackgroundChanged {
        background_url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    pub session_id: i32,
    pub session_name: String,
    pub players: Vec<String>,
    pub map: Option<MapInfo>,
    pub tokens: Vec<TokenInfo>,
    pub fog: Vec<(i32, i32)>,
    pub initiative: Vec<InitiativeEntryInfo>,
    pub recent_chat: Vec<ChatMessageInfo>,
    pub inventory: Vec<InventoryItemInfo>,
    pub initiative_locked: bool,
}
