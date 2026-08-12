use serde::{Deserialize, Serialize};

use crate::models::{ChatMessageInfo, InitiativeEntryInfo, InventoryItemInfo, MapInfo, TokenInfo};

/// One modifier a player applied to a check, and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollBonus {
    pub label: String,
    pub value: i32,
}

/// A resolved check, stored as the chat message's structured dice result.
///
/// `margin` is the whole point in a system where the gap between the total and
/// the DS is damage. It is `None` when the players were not told the DS, which
/// is the usual case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Always "check", to tell this apart from a plain dice roll.
    pub kind: String,
    pub label: String,
    pub dice: String,
    pub rolls: Vec<i32>,
    pub ability: String,
    pub ability_mod: i32,
    pub bonuses: Vec<RollBonus>,
    pub total: i32,
    pub ds: Option<i32>,
    pub margin: Option<i32>,
    pub dangerous: bool,
}

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
        /// gear, weapon, armour, treasure, or consumable.
        #[serde(default)]
        kind: String,
        /// What the item gives, in the language of the roll.
        #[serde(default)]
        bonus: String,
        /// Carrying capacity the item eats. Defaults to one.
        #[serde(default)]
        slots: Option<i32>,
        /// Uses for a consumable, filled to full when dealt.
        #[serde(default)]
        uses: Option<i32>,
        /// Character carrying it. `None` leaves it with the party.
        #[serde(default)]
        owner_character_id: Option<i32>,
    },
    /// Spend or restore uses on a consumable.
    SetInventoryItemUses {
        item_id: i32,
        uses_left: i32,
    },
    /// Hand an item to a character, or back to the party with `None`.
    AssignInventoryItem {
        item_id: i32,
        character_id: Option<i32>,
    },
    /// Roll the system's check: dice + ability + the bonuses the player picked,
    /// against a DS the player may or may not know.
    RollCheck {
        /// What is being attempted, shown in the chat line.
        label: String,
        /// Ability label, e.g. "Brute". Empty rolls without one.
        ability: String,
        ability_mod: i32,
        /// Bonuses the player ticked, each with what justifies it.
        bonuses: Vec<RollBonus>,
        /// Difficulty score, when the players have been told it.
        ds: Option<i32>,
        /// Whether missing costs hit points.
        dangerous: bool,
        /// Dice to roll, in `NdN` form. Comes from the system module.
        dice: String,
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
    /// GM sets the map's default token color (used for facing arrows on generic tokens).
    SetMapDefaultColor {
        color: String,
    },
    /// GM renames a token.
    UpdateTokenLabel {
        token_id: i32,
        label: String,
    },
    /// GM toggles token visibility (hidden tokens are invisible to players).
    UpdateTokenVisibility {
        token_id: i32,
        visible: bool,
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
    /// Token image changed (e.g. character portrait or creature icon updated).
    TokenImageUpdated {
        token_id: i32,
        image_url: Option<String>,
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
    /// Map default token color was changed by the GM.
    MapDefaultColorChanged {
        default_token_color: String,
    },
    /// Token label was changed.
    TokenLabelUpdated {
        token_id: i32,
        label: String,
    },
    /// Token visibility was changed.
    TokenVisibilityUpdated {
        token_id: i32,
        visible: bool,
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
