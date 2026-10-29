use leptos::prelude::*;
#[cfg(feature = "hydrate")]
use leptos::reactive::owner::LocalStorage;

use crate::components::charsheet::{CharacterEditorPanel, CharacterSelection};
use crate::components::chat::ChatPanel;
use crate::components::creatures::CreaturePanel;
use crate::components::file_browser::FileBrowserPanel;
use crate::components::help_viewer::{HelpContext, HelpViewerPanel};
use crate::components::initiative::InitiativeTracker;
use crate::components::inventory::InventoryPanel;
use crate::components::map::MapCanvas;
use crate::components::terminal::TerminalPanel;
use crate::components::window_manager::{
    GameWindow, WindowId, WindowManager, WindowManagerContext,
};
use crate::models::{
    CharacterInfo, ChatMessageInfo, InitiativeEntryInfo, InventoryItemInfo, MapInfo, TokenInfo,
};
use crate::ws::messages::{ClientMessage, GameStateSnapshot, ServerMessage};

// ---------------------------------------------------------------------------
// Loading state — centralised status messages with severity levels
// ---------------------------------------------------------------------------

/// Severity level for the loading modal, mapped to CSS classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingLevel {
    /// Blue — default progress steps (Initializing, Authenticating).
    Info,
    /// Yellow — transient / in-flight actions (Connecting, Joining).
    Warn,
    /// Red — fatal errors that block the session.
    Error,
    /// Green — success (Connected). Shown briefly before the modal closes.
    Success,
}

impl LoadingLevel {
    /// CSS class suffix applied to the loading modal.
    pub fn css_class(&self) -> &'static str {
        match self {
            LoadingLevel::Info => "loading-info",
            LoadingLevel::Warn => "loading-warn",
            LoadingLevel::Error => "loading-error",
            LoadingLevel::Success => "loading-success",
        }
    }
}

/// A loading-modal state: message + severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadingState {
    pub message: &'static str,
    pub level: LoadingLevel,
    /// Whether to offer a "Reconnect now" action (for recoverable connection
    /// errors, as opposed to a failed login).
    pub retryable: bool,
}

// Pre-defined loading states (single source of truth for all messages).
impl LoadingState {
    pub const INITIALIZING: Self = Self {
        message: "Initializing\u{2026}",
        level: LoadingLevel::Info,
        retryable: false,
    };
    pub const AUTHENTICATING: Self = Self {
        message: "Authenticating\u{2026}",
        level: LoadingLevel::Info,
        retryable: false,
    };
    pub const CONNECTING: Self = Self {
        message: "Connecting\u{2026}",
        level: LoadingLevel::Warn,
        retryable: false,
    };
    pub const JOINING: Self = Self {
        message: "Joining session\u{2026}",
        level: LoadingLevel::Warn,
        retryable: false,
    };
    pub const CONNECTED: Self = Self {
        message: "Connected",
        level: LoadingLevel::Success,
        retryable: false,
    };
    pub const AUTH_FAILED: Self = Self {
        message: "Authentication failed",
        level: LoadingLevel::Error,
        retryable: false,
    };
    pub const CONNECT_FAILED: Self = Self {
        message: "Connection failed",
        level: LoadingLevel::Error,
        retryable: false,
    };
    pub const CONNECTION_LOST: Self = Self {
        message: "Connection lost",
        level: LoadingLevel::Error,
        retryable: false,
    };
    pub const CONNECTION_ERROR: Self = Self {
        message: "Connection error",
        level: LoadingLevel::Error,
        retryable: false,
    };
    /// Shown while automatically reconnecting after an unexpected drop.
    pub const RECONNECTING: Self = Self {
        message: "Reconnecting\u{2026}",
        level: LoadingLevel::Warn,
        retryable: false,
    };
    /// Shown after several failed reconnect attempts. Retries continue in the
    /// background, and the user can also trigger one immediately.
    pub const DISCONNECTED: Self = Self {
        message: "Disconnected. Retrying\u{2026}",
        level: LoadingLevel::Error,
        retryable: true,
    };
}

/// Shared game state provided via Leptos context to all child components.
/// A request for the map to recenter on a specific token. The map resolves the
/// target to a token, centers on it, then clears the request. One command with
/// several lookup keys, rather than one signal per key.
#[derive(Clone, Debug, PartialEq)]
pub enum CenterTarget {
    Character(i32),
    TokenLabel(String),
    TokenId(i32),
}

#[derive(Clone)]
pub struct GameContext {
    pub session_id: ReadSignal<i32>,
    pub session_name: RwSignal<String>,
    pub players: RwSignal<Vec<String>>,
    pub map: RwSignal<Option<MapInfo>>,
    pub tokens: RwSignal<Vec<TokenInfo>>,
    pub fog: RwSignal<Vec<(i32, i32)>>,
    pub initiative: RwSignal<Vec<InitiativeEntryInfo>>,
    pub chat_messages: RwSignal<Vec<ChatMessageInfo>>,
    pub inventory: RwSignal<Vec<InventoryItemInfo>>,
    /// All characters in the session. The single client-side cache: the server
    /// pushes granular `CharacterUpdated`/`CharacterResourceUpdated` deltas that
    /// are applied here in place, and windows read this signal reactively
    /// instead of each refetching the whole list.
    pub characters: RwSignal<Vec<CharacterInfo>>,
    pub connected: RwSignal<bool>,
    pub send: Sender,
    /// Bumped to force a full reload of `characters` from the server. Used for
    /// changes that are not broadcast as deltas (create/delete). Field and
    /// resource edits are applied in place and do not bump this.
    pub character_revision: RwSignal<u32>,
    /// Whether the current user is the GM of this session.
    pub is_gm: RwSignal<bool>,
    /// User's ping color preference.
    pub ping_color: RwSignal<String>,
    /// Whether to suppress icon tooltips.
    pub suppress_tooltips: RwSignal<bool>,
    /// Active pings on the map: (x, y, color, timestamp_ms).
    pub pings: RwSignal<Vec<(f64, f64, String, f64)>>,
    /// When set, the map component should update its viewport.
    pub viewport_override: RwSignal<Option<(f64, f64, f64)>>,
    /// Current map viewport center in grid coordinates, updated by the map component.
    /// Used by other components to place tokens in view.
    pub map_view_center: RwSignal<(f32, f32)>,
    /// When set, the map centers on the token matching this target, then clears
    /// the signal. One request with several lookup keys (see CenterTarget).
    pub center_on: RwSignal<Option<CenterTarget>>,
    /// When set, the Creatures window is brought to front and scrolls to this creature_id.
    pub focus_creature: RwSignal<Option<i32>>,
    /// Whether initiative rolls from character sheets are locked.
    pub initiative_locked: RwSignal<bool>,
    /// Loading modal state. `Some(…)` shows the modal; `None` hides it.
    pub loading: RwSignal<Option<LoadingState>>,
    /// Set when a new initiative turn begins — holds the entry that became active.
    /// Used by character sheet windows (title flash) and the map (star animation).
    pub turn_notify: RwSignal<Option<InitiativeEntryInfo>>,
    /// Turn-start star animation on map: (world_x, world_y, start_timestamp_ms).
    pub turn_star: RwSignal<Option<(f64, f64, f64)>>,
    /// Token ID of the creature/character whose initiative turn it is (for map highlight).
    pub active_initiative_token_id: RwSignal<Option<i32>>,
    /// Initiative round number (increments each time the turn wraps to the top).
    pub initiative_round: RwSignal<u32>,
    /// Modules this session runs, and the roll model that comes with them.
    /// Empty until the fetch after join completes.
    pub modules: RwSignal<crate::server::modules_api::SessionModules>,
    /// Session members for the GM management panel, populated on demand when the
    /// Members window requests the list.
    pub members: RwSignal<Vec<crate::ws::messages::SessionMemberInfo>>,
    /// Whether the current user may manage the GM list (the session owner only).
    pub can_manage_gms: RwSignal<bool>,
    /// Decremented for each locally-created message to avoid ID collisions with DB rows.
    next_local_id: std::sync::Arc<std::sync::atomic::AtomicI32>,
}

// In WASM (single-threaded), JS types like WebSocket aren't Send+Sync.
// Use LocalStorage to avoid the Send+Sync requirement.
#[cfg(feature = "hydrate")]
type SendFn = Box<dyn Fn(ClientMessage)>;

/// Sends client messages over the session WebSocket. The closure captures the
/// WebSocket, which is not Send on wasm, so on the client it lives in a
/// LocalStorage StoredValue. It is None until the socket connects, and a send
/// before then logs a warning rather than silently dropping the message.
///
/// On the server the sender holds nothing at all. The WebSocket is hydrate-only,
/// so creating a LocalStorage value during SSR would leave a non-Send value in
/// the reactive owner that panics when the streaming render is cleaned up on a
/// different worker thread than it was built on. `send` is a no-op there.
#[derive(Clone, Copy)]
pub struct Sender {
    #[cfg(feature = "hydrate")]
    inner: StoredValue<Option<SendFn>, LocalStorage>,
}

impl Sender {
    #[cfg(feature = "hydrate")]
    fn new() -> Self {
        Sender {
            inner: StoredValue::new_local(None),
        }
    }

    #[cfg(not(feature = "hydrate"))]
    fn new() -> Self {
        Sender {}
    }

    /// Install the real send closure once the socket is open. Hydrate-only.
    #[cfg(feature = "hydrate")]
    fn connect(&self, f: SendFn) {
        self.inner.set_value(Some(f));
    }

    pub fn send(&self, msg: ClientMessage) {
        #[cfg(feature = "hydrate")]
        self.inner.with_value(|f| match f {
            Some(f) => f(msg),
            None => log::warn!("message sent before the WebSocket connected; dropping it"),
        });
        #[cfg(not(feature = "hydrate"))]
        let _ = msg;
    }
}

#[allow(dead_code)]
impl GameContext {
    pub fn send_message(&self, msg: ClientMessage) {
        self.send.send(msg);
    }

    fn apply_snapshot(&self, snapshot: GameStateSnapshot) {
        self.session_name.set(snapshot.session_name);
        self.players.set(snapshot.players);
        self.map.set(snapshot.map);
        self.tokens.set(snapshot.tokens);
        self.fog.set(snapshot.fog);
        self.active_initiative_token_id.set(
            snapshot
                .initiative
                .iter()
                .find(|e| e.is_current_turn)
                .and_then(|e| e.token_id),
        );
        self.initiative.set(snapshot.initiative);
        self.chat_messages.set(snapshot.recent_chat);
        self.inventory.set(snapshot.inventory);
        self.initiative_locked.set(snapshot.initiative_locked);
        self.is_gm.set(snapshot.is_gm);
        self.ping_color.set(snapshot.ping_color);
        self.suppress_tooltips.set(snapshot.suppress_tooltips);
        self.connected.set(true);
        // Clear loading modal — game is ready
        self.loading.set(None);
    }

    fn apply_server_message(&self, msg: ServerMessage) {
        match msg {
            ServerMessage::SessionJoined { snapshot } => {
                self.apply_snapshot(snapshot);
            }
            ServerMessage::PlayerJoined { username } => {
                self.players.update(|p| {
                    if !p.contains(&username) {
                        p.push(username);
                    }
                });
            }
            ServerMessage::PlayerLeft { username } => {
                self.players.update(|p| p.retain(|u| u != &username));
            }
            ServerMessage::MemberList {
                members,
                can_manage,
            } => {
                self.members.set(members);
                self.can_manage_gms.set(can_manage);
            }
            ServerMessage::GmStatusChanged { is_gm } => {
                self.is_gm.set(is_gm);
            }
            ServerMessage::MembersChanged => {
                // Only GMs see the member list; refetch it if we are one.
                if self.is_gm.get_untracked() {
                    self.send.send(ClientMessage::ListMembers);
                }
            }
            ServerMessage::ChatBroadcast { message } => {
                self.chat_messages.update(|msgs| msgs.push(message));
            }
            ServerMessage::DiceResult {
                username,
                expression,
                rolls,
                total,
            } => {
                let rolls_str = rolls
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                // Use negative IDs for locally-created messages to avoid
                // collisions with DB-assigned positive IDs.
                let local_id = self
                    .next_local_id
                    .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                self.chat_messages.update(|msgs| {
                    msgs.push(ChatMessageInfo {
                        id: local_id,
                        username: username.clone(),
                        message: format!("rolled {expression}: [{rolls_str}] = {total}"),
                        is_dice_roll: true,
                        dice_result: Some(format!("{total}")),
                        created_at: String::new(),
                    });
                });
            }
            ServerMessage::TokenMoved { token_id, x, y } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.x = x;
                        t.y = y;
                    }
                });
            }
            ServerMessage::TokenPlaced { token } => {
                self.tokens.update(|tokens| tokens.push(token));
            }
            ServerMessage::TokenRemoved { token_id } => {
                self.tokens
                    .update(|tokens| tokens.retain(|t| t.id != token_id));
            }
            ServerMessage::TokenHpUpdated {
                token_id,
                current_hp,
                max_hp,
            } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.current_hp = Some(current_hp);
                        t.max_hp = Some(max_hp);
                    }
                });
            }
            ServerMessage::TokenImageUpdated {
                token_id,
                image_url,
            } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.image_url = image_url;
                    }
                });
            }
            ServerMessage::FogUpdated { revealed, hidden } => {
                self.fog.update(|fog| {
                    for cell in revealed {
                        if !fog.contains(&cell) {
                            fog.push(cell);
                        }
                    }
                    for cell in &hidden {
                        fog.retain(|c| c != cell);
                    }
                });
            }
            ServerMessage::MapChanged { map, tokens, fog } => {
                self.map.set(Some(map));
                self.tokens.set(tokens);
                self.fog.set(fog);
            }
            ServerMessage::InitiativeUpdated { entries } => {
                // Detect turn change: compare old current turn to new
                let old_current = self
                    .initiative
                    .with_untracked(|old| old.iter().find(|e| e.is_current_turn).cloned());
                let new_current = entries.iter().find(|e| e.is_current_turn).cloned();

                // Fire notification if the current turn entry changed
                let changed = match (&old_current, &new_current) {
                    (Some(old), Some(new)) => {
                        old.label != new.label
                            || old.character_id != new.character_id
                            || old.token_id != new.token_id
                    }
                    (None, Some(_)) => true,
                    _ => false,
                };

                // Reset round when initiative is cleared
                if entries.is_empty() {
                    self.initiative_round.set(1);
                }

                self.initiative.set(entries);

                // Always track the active initiative token for map highlight
                self.active_initiative_token_id
                    .set(new_current.as_ref().and_then(|e| e.token_id));

                if changed && let Some(ref entry) = new_current {
                    // Increment round when turn wraps to the first entry
                    let entries_ref = self.initiative.get_untracked();
                    if let Some(new_idx) = entries_ref.iter().position(|e| e.is_current_turn)
                        && new_idx == 0
                        && old_current.is_some()
                    {
                        self.initiative_round.update(|r| *r += 1);
                    }

                    self.turn_notify.set(Some(entry.clone()));

                    // Compute star position and auto-center on active token
                    if let Some(tid) = entry.token_id {
                        let tokens = self.tokens.get_untracked();
                        let map = self.map.get_untracked();
                        if let (Some(t), Some(m)) =
                            (tokens.iter().find(|t| t.id == tid), map.as_ref())
                        {
                            let cell = m.cell_size as f64;
                            let wx = (t.x as f64 + 0.5) * cell;
                            let wy = (t.y as f64 + 0.5) * cell;
                            #[cfg(feature = "hydrate")]
                            let now = web_sys::js_sys::Date::now();
                            #[cfg(not(feature = "hydrate"))]
                            let now = 0.0;
                            self.turn_star.set(Some((wx, wy, now)));
                        }
                        // Auto-center map on the active token
                        self.center_on.set(Some(CenterTarget::TokenId(tid)));
                    }
                }
            }
            ServerMessage::CharacterUpdated {
                character_id,
                field_path,
                value,
            } => {
                // Apply the delta in place, mirroring the dot-path write the
                // server does in ws_handler::character::update_character_field.
                self.characters.update(|chars| {
                    let Some(c) = chars.iter_mut().find(|c| c.id == character_id) else {
                        return;
                    };
                    let parts: Vec<&str> = field_path.split('.').collect();
                    let mut current = &mut c.data;
                    for (i, part) in parts.iter().enumerate() {
                        if i == parts.len() - 1 {
                            current[part] = value.clone();
                        } else {
                            if !current.get(part).is_some_and(|v| v.is_object()) {
                                current[part] = serde_json::json!({});
                            }
                            current = &mut current[part];
                        }
                    }
                });
            }
            ServerMessage::CharacterResourceUpdated {
                character_id,
                resource_id,
                current_value,
                max_value,
            } => {
                self.characters.update(|chars| {
                    if let Some(c) = chars.iter_mut().find(|c| c.id == character_id)
                        && let Some(r) = c.resources.iter_mut().find(|r| r.id == resource_id)
                    {
                        r.current_value = current_value;
                        r.max_value = max_value;
                    }
                });
            }
            ServerMessage::InventoryUpdated { items } => {
                self.inventory.set(items);
            }
            ServerMessage::InitiativeLockChanged { locked } => {
                self.initiative_locked.set(locked);
            }
            ServerMessage::MapBackgroundChanged { background_url } => {
                self.map.update(|m_opt| {
                    if let Some(m) = m_opt {
                        m.background_url = background_url;
                    }
                });
            }
            ServerMessage::MapDefaultColorChanged {
                default_token_color,
            } => {
                self.map.update(|m_opt| {
                    if let Some(m) = m_opt {
                        m.default_token_color = default_token_color;
                    }
                });
            }
            ServerMessage::TokensMoved { moves } => {
                self.tokens.update(|tokens| {
                    for (token_id, x, y) in &moves {
                        if let Some(t) = tokens.iter_mut().find(|t| t.id == *token_id) {
                            t.x = *x;
                            t.y = *y;
                        }
                    }
                });
            }
            ServerMessage::TokensRotated { rotations } => {
                self.tokens.update(|tokens| {
                    for (token_id, rotation) in &rotations {
                        if let Some(t) = tokens.iter_mut().find(|t| t.id == *token_id) {
                            t.rotation = *rotation;
                        }
                    }
                });
            }
            ServerMessage::TokenConditionsUpdated {
                token_id,
                conditions,
            } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.conditions = conditions;
                    }
                });
            }
            ServerMessage::TokenLabelUpdated { token_id, label } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.label = label;
                    }
                });
            }
            ServerMessage::TokenVisibilityUpdated { token_id, visible } => {
                self.tokens.update(|tokens| {
                    if let Some(t) = tokens.iter_mut().find(|t| t.id == token_id) {
                        t.visible = visible;
                    }
                });
            }
            ServerMessage::PingBroadcast {
                username: _,
                x,
                y,
                color,
            } => {
                let now = {
                    #[cfg(feature = "hydrate")]
                    {
                        web_sys::js_sys::Date::now()
                    }
                    #[cfg(not(feature = "hydrate"))]
                    {
                        0.0
                    }
                };
                self.pings.update(|pings| {
                    pings.push((x, y, color, now));
                });
            }
            ServerMessage::ViewportSynced { x, y, zoom } => {
                self.viewport_override.set(Some((x, y, zoom)));
            }
            ServerMessage::Error { message } => {
                log::warn!("Server error: {message}");
            }
            ServerMessage::VfsChanged { .. } => {
                // VFS change notifications will be handled by the file browser
                // and terminal components once they are implemented.
            }
        }
    }
}

/// Open a WebSocket for the session and wire its handlers, with automatic
/// reconnection (feature 55). Each attempt takes a `generation`: a socket's
/// handlers only act while they are still the current generation, so a superseded
/// socket (after a manual retry or a newer attempt) never reconnects or fights
/// the live one. On an unexpected close it schedules another attempt with
/// exponential backoff; a successful open resets the attempt counter.
#[cfg(feature = "hydrate")]
fn ws_connect(
    ctx: GameContext,
    sid: i32,
    current_ws: StoredValue<Option<web_sys::WebSocket>, LocalStorage>,
    closing: StoredValue<bool>,
    attempt: StoredValue<u32>,
    generation: StoredValue<u32>,
) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    if closing.get_value() {
        return;
    }
    generation.update_value(|g| *g += 1);
    let my_gen = generation.get_value();
    let stale = move || closing.get_value() || generation.get_value() != my_gen;

    let first = attempt.get_value() == 0;
    ctx.loading.set(Some(if first {
        LoadingState::AUTHENTICATING
    } else {
        LoadingState::RECONNECTING
    }));

    leptos::task::spawn_local(async move {
        if stale() {
            return;
        }
        let token = match crate::server::api::get_ws_token().await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("WS token fetch failed: {e}");
                schedule_reconnect(ctx, sid, current_ws, closing, attempt, generation, my_gen);
                return;
            }
        };
        if stale() {
            return;
        }
        if first {
            ctx.loading.set(Some(LoadingState::CONNECTING));
        }

        let window = web_sys::window().expect("no window");
        let location = window.location();
        let protocol = location.protocol().unwrap_or_default();
        let host = location.host().unwrap_or_default();
        let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
        let ws_url = format!("{ws_protocol}//{host}/api/ws?token={token}");

        let ws = match web_sys::WebSocket::new(&ws_url) {
            Ok(ws) => ws,
            Err(e) => {
                log::warn!("WebSocket create failed: {e:?}");
                schedule_reconnect(ctx, sid, current_ws, closing, attempt, generation, my_gen);
                return;
            }
        };
        current_ws.set_value(Some(ws.clone()));

        let ws_send = ws.clone();
        let send_fn: SendFn = Box::new(move |msg: ClientMessage| {
            if let Ok(json) = serde_json::to_string(&msg) {
                let _ = ws_send.send_with_str(&json);
            }
        });
        ctx.send.connect(send_fn);

        let ctx_open = ctx.clone();
        let on_open = Closure::<dyn Fn()>::new(move || {
            attempt.set_value(0);
            ctx_open.loading.set(Some(LoadingState::JOINING));
            ctx_open.send_message(ClientMessage::JoinSession { session_id: sid });
        });
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        on_open.forget();

        let ctx_msg = ctx.clone();
        let on_message =
            Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |e: web_sys::MessageEvent| {
                if let Some(text) = e.data().as_string() {
                    match serde_json::from_str::<ServerMessage>(&text) {
                        Ok(msg) => ctx_msg.apply_server_message(msg),
                        Err(err) => log::warn!("Failed to parse server message: {err}"),
                    }
                }
            });
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();

        let ctx_close = ctx.clone();
        let on_close = Closure::<dyn Fn()>::new(move || {
            ctx_close.connected.set(false);
            if stale() {
                return;
            }
            attempt.update_value(|a| *a += 1);
            schedule_reconnect(
                ctx_close.clone(),
                sid,
                current_ws,
                closing,
                attempt,
                generation,
                my_gen,
            );
        });
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        on_close.forget();

        let on_error = Closure::<dyn Fn()>::new(move || {
            log::warn!("WebSocket error");
        });
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    });
}

/// Schedule the next reconnection attempt after an unexpected close, with
/// exponential backoff capped at 15s. After several tries the modal switches to
/// the disconnected error state, but attempts keep running in the background so
/// the client recovers on its own once the server returns.
#[cfg(feature = "hydrate")]
fn schedule_reconnect(
    ctx: GameContext,
    sid: i32,
    current_ws: StoredValue<Option<web_sys::WebSocket>, LocalStorage>,
    closing: StoredValue<bool>,
    attempt: StoredValue<u32>,
    generation: StoredValue<u32>,
    sched_gen: u32,
) {
    if closing.get_value() || generation.get_value() != sched_gen {
        return;
    }
    let n = attempt.get_value().max(1);
    ctx.loading.set(Some(if n < 5 {
        LoadingState::RECONNECTING
    } else {
        LoadingState::DISCONNECTED
    }));
    let shift = (n - 1).min(4);
    let delay_ms = (1000u64 << shift).min(15_000);
    set_timeout(
        move || {
            if closing.get_value() || generation.get_value() != sched_gen {
                return;
            }
            ws_connect(ctx, sid, current_ws, closing, attempt, generation);
        },
        std::time::Duration::from_millis(delay_ms),
    );
}

#[component]
pub fn GamePage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let session_id_val = move || {
        params
            .read()
            .get("id")
            .and_then(|id| id.parse::<i32>().ok())
            .unwrap_or(0)
    };

    let (session_id_r, session_id_w) = signal(0i32);

    let ctx = GameContext {
        session_id: session_id_r,
        session_name: RwSignal::new(String::new()),
        players: RwSignal::new(vec![]),
        map: RwSignal::new(None),
        tokens: RwSignal::new(vec![]),
        fog: RwSignal::new(vec![]),
        initiative: RwSignal::new(vec![]),
        chat_messages: RwSignal::new(vec![]),
        inventory: RwSignal::new(vec![]),
        characters: RwSignal::new(vec![]),
        connected: RwSignal::new(false),
        send: Sender::new(),
        character_revision: RwSignal::new(0),
        is_gm: RwSignal::new(false),
        ping_color: RwSignal::new("#ffcc00".to_string()),
        suppress_tooltips: RwSignal::new(false),
        pings: RwSignal::new(vec![]),
        viewport_override: RwSignal::new(None),
        map_view_center: RwSignal::new((0.0, 0.0)),
        center_on: RwSignal::new(None),
        focus_creature: RwSignal::new(None),
        initiative_locked: RwSignal::new(false),
        loading: RwSignal::new(Some(LoadingState::INITIALIZING)),
        turn_notify: RwSignal::new(None),
        turn_star: RwSignal::new(None),
        active_initiative_token_id: RwSignal::new(None),
        initiative_round: RwSignal::new(1),
        modules: RwSignal::new(Default::default()),
        members: RwSignal::new(vec![]),
        can_manage_gms: RwSignal::new(false),
        next_local_id: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(-1)),
    };

    provide_context(ctx.clone());
    provide_context(HelpContext::new());

    #[cfg(feature = "hydrate")]
    {
        let scratch_ctx: RwSignal<crate::scratch_drive::ScratchDrives, LocalStorage> =
            RwSignal::new_local(crate::scratch_drive::ScratchDrives { a: None, b: None });
        provide_context(scratch_ctx);
        leptos::task::spawn_local(async move {
            let random = (js_sys::Math::random() * 1_000_000.0) as u64;
            let a = crate::scratch_drive::open_scratch_db(&format!("webrpg_scratch_A_{random}"))
                .await
                .ok();
            let b = crate::scratch_drive::open_scratch_db(&format!("webrpg_scratch_B_{random}"))
                .await
                .ok();
            scratch_ctx.set(crate::scratch_drive::ScratchDrives { a, b });
        });
    }

    // Position [data-tooltip] pseudo-elements using CSS custom properties.
    // The ::after uses position:fixed, so we set --tt-left/--tt-top on mouseover.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;

        let cb = Closure::<dyn Fn(web_sys::MouseEvent)>::new(move |ev: web_sys::MouseEvent| {
            let Some(target) = ev.target() else { return };
            let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() else {
                return;
            };
            // Walk up to find the [data-tooltip] element (may be the target or a parent)
            let mut node = Some(el);
            while let Some(ref n) = node {
                if n.get_attribute("data-tooltip").is_some() {
                    let rect = n.get_bounding_client_rect();
                    let cx = rect.left() + rect.width() / 2.0;
                    let below_top = rect.bottom() + 4.0;
                    let style = n.style();
                    let _ = style.set_property("--tt-left", &format!("{cx}px"));
                    let _ = style.set_property("--tt-top", &format!("{below_top}px"));
                    return;
                }
                node = n
                    .parent_element()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
            }
        });
        let _ = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .add_event_listener_with_callback("mouseover", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Load the session's characters into the single shared cache. Runs once the
    // session id is known, and again whenever character_revision is bumped (for
    // create/delete, which are not broadcast as deltas). Field and resource
    // edits are applied in place by apply_server_message and do not refetch.
    #[cfg(feature = "hydrate")]
    {
        let characters = ctx.characters;
        let character_revision = ctx.character_revision;
        Effect::new(move |_| {
            let sid = session_id_r.get();
            character_revision.track();
            if sid == 0 {
                return;
            }
            leptos::task::spawn_local(async move {
                match crate::server::api::list_characters(sid).await {
                    Ok(chars) => characters.set(chars),
                    Err(e) => log::error!("Failed to load characters: {e}"),
                }
            });
        });
    }

    // Bumped by the "Reconnect now" button on the disconnected modal.
    let reconnect_now = RwSignal::new(0u32);

    // WebSocket connection with automatic reconnection and a keepalive
    // heartbeat (features 55 and 56).
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        // Reconnection state kept across attempts. The socket uses LocalStorage
        // because web_sys::WebSocket is not Send.
        let current_ws: StoredValue<Option<web_sys::WebSocket>, LocalStorage> =
            StoredValue::new_local(None);
        let closing = StoredValue::new(false);
        let attempt = StoredValue::new(0u32);
        let generation = StoredValue::new(0u32);
        let last_activity = StoredValue::new(js_sys::Date::now());

        let ctx_ws = ctx.clone();
        Effect::new(move |_| {
            let sid = session_id_val();
            if sid == 0 {
                return;
            }
            session_id_w.set(sid);

            let ctx = ctx_ws.clone();

            // Which modules this session runs. Not worth blocking connect on.
            let modules_signal = ctx.modules;
            leptos::task::spawn_local(async move {
                match crate::server::modules_api::get_session_modules(sid).await {
                    Ok(m) => modules_signal.set(m),
                    Err(e) => log::warn!("Failed to load session modules: {e}"),
                }
            });

            closing.set_value(false);
            attempt.set_value(0);
            ws_connect(
                ctx_ws.clone(),
                sid,
                current_ws,
                closing,
                attempt,
                generation,
            );
        });

        // Keepalive heartbeat (feature 56): once a minute, carrying an idle flag.
        {
            let ctx_ka = ctx.clone();
            if let Ok(handle) = set_interval_with_handle(
                move || {
                    let idle = (js_sys::Date::now() - last_activity.get_value()) > 60_000.0;
                    ctx_ka.send.send(ClientMessage::Keepalive { idle });
                },
                std::time::Duration::from_secs(60),
            ) {
                on_cleanup(move || handle.clear());
            }
        }

        // Track user activity so the keepalive can report idle status.
        {
            let on_activity = Closure::<dyn Fn()>::new(move || {
                last_activity.set_value(js_sys::Date::now());
            });
            if let Some(win) = web_sys::window() {
                for ev in ["pointerdown", "keydown", "wheel"] {
                    let _ = win
                        .add_event_listener_with_callback(ev, on_activity.as_ref().unchecked_ref());
                }
            }
            on_activity.forget();
        }

        // Manual reconnect from the disconnected modal.
        let ctx_retry = ctx.clone();
        let sid_sig = ctx.session_id;
        Effect::new(move |_| {
            if reconnect_now.get() > 0 {
                let sid = sid_sig.get_untracked();
                if sid != 0 {
                    closing.set_value(false);
                    attempt.set_value(0);
                    current_ws.update_value(|w| {
                        if let Some(ws) = w.take() {
                            let _ = ws.close();
                        }
                    });
                    ws_connect(
                        ctx_retry.clone(),
                        sid,
                        current_ws,
                        closing,
                        attempt,
                        generation,
                    );
                }
            }
        });

        // Stop reconnecting and close the socket when leaving the page.
        on_cleanup(move || {
            closing.set_value(true);
            current_ws.update_value(|w| {
                if let Some(ws) = w.take() {
                    let _ = ws.close();
                }
            });
        });
    }

    // On non-hydrate, just set session_id from params
    #[cfg(not(feature = "hydrate"))]
    {
        Effect::new(move |_| {
            session_id_w.set(session_id_val());
        });
    }

    let session_name = ctx.session_name;
    let connected = ctx.connected;
    let loading = ctx.loading;

    // Delay showing the loading modal so fast reconnects (e.g. hot-reload) don't flash it
    let show_loading_modal = RwSignal::new(false);
    #[cfg(feature = "hydrate")]
    {
        set_timeout(
            move || {
                if loading.get_untracked().is_some() {
                    show_loading_modal.set(true);
                }
            },
            std::time::Duration::from_millis(1000),
        );
        // Hide modal once loading clears
        Effect::new(move |_| {
            let state = loading.get();
            if state.is_none() {
                show_loading_modal.set(false);
            }
        });
    }

    view! {
        <div class=move || if ctx.suppress_tooltips.get() { "game-page suppress-tooltips" } else { "game-page" }>
            // Loading overlay — blocks interaction until session snapshot arrives
            {move || {
                let state = loading.get();
                let show_modal = show_loading_modal.get();
                match state {
                    Some(ref s) if s.level == LoadingLevel::Error || show_modal => {
                        let css = format!("loading-modal {}", s.level.css_class());
                        let is_error = s.level == LoadingLevel::Error;
                        let retryable = s.retryable;
                        let msg = s.message.to_string();
                        Some(view! {
                            <div class="loading-overlay">
                                <div class={css}>
                                    {if is_error {
                                        view! {
                                            <div class="loading-error">
                                                <div class="loading-error-icon">"!"</div>
                                                <p>{msg}</p>
                                                <div class="loading-error-actions">
                                                    <Show when=move || retryable>
                                                        <button
                                                            class="btn-reconnect"
                                                            on:click=move |_| {
                                                                reconnect_now.update(|n| *n += 1)
                                                            }
                                                        >
                                                            "Reconnect now"
                                                        </button>
                                                    </Show>
                                                    <a href="/sessions" class="btn-back-sessions">"Back to Sessions"</a>
                                                </div>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="loading-progress">
                                                <div class="loading-spinner"></div>
                                                <p>{msg}</p>
                                            </div>
                                        }.into_any()
                                    }}
                                </div>
                            </div>
                        })
                    }
                    _ => None,
                }
            }}

            <div class="game-header">
                <h1>{move || {
                    let name = session_name.get();
                    if name.is_empty() {
                        format!("Game Session #{}", session_id_r.get())
                    } else {
                        name
                    }
                }}</h1>
                <div class="game-header-right">
                    <span class=move || if connected.get() { "connection-status connected" } else { "connection-status" }>
                        {move || if connected.get() { "Connected" } else { "Connecting..." }}
                    </span>
                </div>
            </div>
            <WindowManager>
                <GameWindow id=WindowId::Map>
                    <MapCanvas />
                </GameWindow>
                <GameWindow id=WindowId::Chat>
                    <ChatPanel />
                </GameWindow>
                <GameWindow id=WindowId::CharacterSelection>
                    <CharacterSelection />
                </GameWindow>
                <GameWindow id=WindowId::Initiative>
                    <InitiativeTracker />
                </GameWindow>
                <GameWindow id=WindowId::Inventory>
                    <InventoryPanel />
                </GameWindow>
                <Show when=move || ctx.is_gm.get()>
                    <GameWindow id=WindowId::Creatures>
                        <CreaturePanel />
                    </GameWindow>
                </Show>
                <Show when=move || ctx.is_gm.get()>
                    <GameWindow id=WindowId::Members>
                        <crate::components::members_panel::MembersPanel />
                    </GameWindow>
                </Show>
                <GameWindow id=WindowId::Terminal>
                    <TerminalPanel />
                </GameWindow>
                <GameWindow id=WindowId::FileBrowser>
                    <FileBrowserPanel />
                </GameWindow>
                <GameWindow id=WindowId::HelpViewer>
                    <HelpViewerPanel />
                </GameWindow>
                <GameWindow id=WindowId::Modules>
                    <crate::components::modules_panel::ModulesPanel />
                </GameWindow>
                <DynamicCharacterWindows />
                <DynamicFileBrowserWindows />
            </WindowManager>
        </div>
    }
}

/// Renders dynamic GameWindow instances for each open character editor.
#[component]
fn DynamicCharacterWindows() -> impl IntoView {
    let wm = expect_context::<WindowManagerContext>();

    // Derive the list of open character editor (id, char_id) pairs
    let open_editors = move || {
        wm.windows
            .get()
            .into_iter()
            .filter_map(|w| {
                if let WindowId::CharacterEditor(char_id) = w.id {
                    Some((w.id, char_id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <For
            each=open_editors
            key=|(_, char_id)| *char_id
            let:item
        >
            {
                let (win_id, char_id) = item;
                view! {
                    <GameWindow id=win_id>
                        <CharacterEditorPanel character_id=char_id />
                    </GameWindow>
                }
            }
        </For>
    }
}

/// Renders dynamic GameWindow instances for each extra file browser.
#[component]
fn DynamicFileBrowserWindows() -> impl IntoView {
    let wm = expect_context::<WindowManagerContext>();

    let open_browsers = move || {
        wm.windows
            .get()
            .into_iter()
            .filter_map(|w| {
                if let WindowId::FileBrowserExtra(fb_id) = w.id {
                    Some((w.id, fb_id))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <For
            each=open_browsers
            key=|(_, fb_id)| *fb_id
            let:item
        >
            {
                let (win_id, _fb_id) = item;
                view! {
                    <GameWindow id=win_id>
                        <FileBrowserPanel />
                    </GameWindow>
                }
            }
        </For>
    }
}
