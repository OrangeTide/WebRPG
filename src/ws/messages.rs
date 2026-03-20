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
        character_id: Option<i32>,
        creature_id: Option<i32>,
        image_url: Option<String>,
    },
    /// GM places all player characters on the map at once, centered at (x, y).
    PlaceAllPlayerTokens {
        x: f32,
        y: f32,
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
    MoveTokens {
        moves: Vec<(i32, f32, f32)>,
    },
    RotateTokens {
        rotations: Vec<(i32, f32)>,
    },
    UpdateTokenConditions {
        token_id: i32,
        conditions: Vec<String>,
    },
    /// Ping the map at a world-space position.
    Ping {
        x: f64,
        y: f64,
    },
    /// GM broadcasts their viewport to all players.
    SyncViewport {
        x: f64,
        y: f64,
        zoom: f64,
    },
    /// Update the user's ping color preference.
    SetPingColor {
        color: String,
    },
    /// Update the user's suppress_tooltips preference.
    SetSuppressTooltips {
        suppress: bool,
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
    TokensMoved {
        moves: Vec<(i32, f32, f32)>,
    },
    TokensRotated {
        rotations: Vec<(i32, f32)>,
    },
    TokenConditionsUpdated {
        token_id: i32,
        conditions: Vec<String>,
    },
    /// A player pinged a location on the map.
    PingBroadcast {
        username: String,
        x: f64,
        y: f64,
        color: String,
    },
    /// GM synced their viewport to all players.
    ViewportSynced {
        x: f64,
        y: f64,
        zoom: f64,
    },
    /// Notifies clients that a file on C: drive has changed.
    VfsChanged {
        /// The path that was affected (e.g. "/maps/dungeon.png").
        path: String,
        /// What happened: "write", "delete", "mkdir", "rename", "chmod".
        action: String,
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
    pub is_gm: bool,
    pub ping_color: String,
    pub suppress_tooltips: bool,
}
