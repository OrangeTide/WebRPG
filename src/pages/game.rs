use leptos::prelude::*;
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
use crate::models::{ChatMessageInfo, InitiativeEntryInfo, InventoryItemInfo, MapInfo, TokenInfo};
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
}

// Pre-defined loading states (single source of truth for all messages).
impl LoadingState {
    pub const INITIALIZING: Self = Self {
        message: "Initializing\u{2026}",
        level: LoadingLevel::Info,
    };
    pub const AUTHENTICATING: Self = Self {
        message: "Authenticating\u{2026}",
        level: LoadingLevel::Info,
    };
    pub const CONNECTING: Self = Self {
        message: "Connecting\u{2026}",
        level: LoadingLevel::Warn,
    };
    pub const JOINING: Self = Self {
        message: "Joining session\u{2026}",
        level: LoadingLevel::Warn,
    };
    pub const CONNECTED: Self = Self {
        message: "Connected",
        level: LoadingLevel::Success,
    };
    pub const AUTH_FAILED: Self = Self {
        message: "Authentication failed",
        level: LoadingLevel::Error,
    };
    pub const CONNECT_FAILED: Self = Self {
        message: "Connection failed",
        level: LoadingLevel::Error,
    };
    pub const CONNECTION_LOST: Self = Self {
        message: "Connection lost",
        level: LoadingLevel::Error,
    };
    pub const CONNECTION_ERROR: Self = Self {
        message: "Connection error",
        level: LoadingLevel::Error,
    };
}

/// Shared game state provided via Leptos context to all child components.
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
    pub connected: RwSignal<bool>,
    pub send: StoredValue<Option<SendFn>, LocalStorage>,
    /// Bumped when any character data/resource changes (triggers refetch in listeners).
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
    /// When set, the map component centers on the token with this character_id.
    pub center_on_character: RwSignal<Option<i32>>,
    /// When set, the map component centers on the first token matching this label.
    pub center_on_token_label: RwSignal<Option<String>>,
    /// When set, the map component centers on the token with this ID.
    pub center_on_token_id: RwSignal<Option<i32>>,
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
    /// Decremented for each locally-created message to avoid ID collisions with DB rows.
    next_local_id: std::sync::Arc<std::sync::atomic::AtomicI32>,
}

// In WASM (single-threaded), JS types like WebSocket aren't Send+Sync.
// Use LocalStorage to avoid the Send+Sync requirement.
type SendFn = Box<dyn Fn(ClientMessage)>;

#[allow(dead_code)]
impl GameContext {
    pub fn send_message(&self, msg: ClientMessage) {
        self.send.with_value(|f| {
            if let Some(f) = f {
                f(msg);
            }
        });
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

                if changed {
                    if let Some(ref entry) = new_current {
                        // Increment round when turn wraps to the first entry
                        let entries_ref = self.initiative.get_untracked();
                        if let Some(new_idx) = entries_ref.iter().position(|e| e.is_current_turn) {
                            if new_idx == 0 && old_current.is_some() {
                                self.initiative_round.update(|r| *r += 1);
                            }
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
                            self.center_on_token_id.set(Some(tid));
                        }
                    }
                }
            }
            ServerMessage::CharacterUpdated { .. } => {
                self.character_revision.update(|n| *n += 1);
            }
            ServerMessage::CharacterResourceUpdated { .. } => {
                self.character_revision.update(|n| *n += 1);
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
        connected: RwSignal::new(false),
        send: StoredValue::new_local(None),
        character_revision: RwSignal::new(0),
        is_gm: RwSignal::new(false),
        ping_color: RwSignal::new("#ffcc00".to_string()),
        suppress_tooltips: RwSignal::new(false),
        pings: RwSignal::new(vec![]),
        viewport_override: RwSignal::new(None),
        map_view_center: RwSignal::new((0.0, 0.0)),
        center_on_character: RwSignal::new(None),
        center_on_token_label: RwSignal::new(None),
        center_on_token_id: RwSignal::new(None),
        focus_creature: RwSignal::new(None),
        initiative_locked: RwSignal::new(false),
        loading: RwSignal::new(Some(LoadingState::INITIALIZING)),
        turn_notify: RwSignal::new(None),
        turn_star: RwSignal::new(None),
        active_initiative_token_id: RwSignal::new(None),
        initiative_round: RwSignal::new(1),
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

    // Fetch WS token and connect
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        let ctx_ws = ctx.clone();

        Effect::new(move |_| {
            let sid = session_id_val();
            if sid == 0 {
                return;
            }
            session_id_w.set(sid);

            let ctx = ctx_ws.clone();

            leptos::task::spawn_local(async move {
                ctx.loading.set(Some(LoadingState::AUTHENTICATING));

                let token = match crate::server::api::get_ws_token().await {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Failed to get WS token: {e}");
                        ctx.loading.set(Some(LoadingState::AUTH_FAILED));
                        return;
                    }
                };

                ctx.loading.set(Some(LoadingState::CONNECTING));

                let window = web_sys::window().expect("no window");
                let location = window.location();
                let protocol = location.protocol().unwrap_or_default();
                let host = location.host().unwrap_or_default();
                let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
                let ws_url = format!("{ws_protocol}//{host}/api/ws?token={token}");

                let ws = match web_sys::WebSocket::new(&ws_url) {
                    Ok(ws) => ws,
                    Err(e) => {
                        log::error!("Failed to create WebSocket: {e:?}");
                        ctx.loading.set(Some(LoadingState::CONNECT_FAILED));
                        return;
                    }
                };

                // Store the send function
                let ws_clone = ws.clone();
                let send_fn: SendFn = Box::new(move |msg: ClientMessage| {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = ws_clone.send_with_str(&json);
                    }
                });
                ctx.send.set_value(Some(send_fn));

                // On open: send JoinSession
                let ctx_open = ctx.clone();
                let on_open = Closure::<dyn Fn()>::new(move || {
                    ctx_open.loading.set(Some(LoadingState::JOINING));
                    ctx_open.send_message(ClientMessage::JoinSession { session_id: sid });
                });
                ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                on_open.forget();

                // On message: parse and apply
                let ctx_msg = ctx.clone();
                let on_message = Closure::<dyn Fn(web_sys::MessageEvent)>::new(
                    move |e: web_sys::MessageEvent| {
                        if let Some(text) = e.data().as_string() {
                            match serde_json::from_str::<ServerMessage>(&text) {
                                Ok(msg) => ctx_msg.apply_server_message(msg),
                                Err(err) => log::warn!("Failed to parse server message: {err}"),
                            }
                        }
                    },
                );
                ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
                on_message.forget();

                // On close
                let ctx_close = ctx.clone();
                let on_close = Closure::<dyn Fn()>::new(move || {
                    ctx_close.connected.set(false);
                    // Only show error if we never successfully connected
                    // (loading is cleared on successful snapshot)
                    if ctx_close.loading.get_untracked().is_some() {
                        ctx_close.loading.set(Some(LoadingState::CONNECTION_LOST));
                    }
                });
                ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
                on_close.forget();

                // On error
                let ctx_err = ctx.clone();
                let on_error = Closure::<dyn Fn()>::new(move || {
                    log::error!("WebSocket error");
                    if ctx_err.loading.get_untracked().is_some() {
                        ctx_err.loading.set(Some(LoadingState::CONNECTION_ERROR));
                    }
                });
                ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                on_error.forget();
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
                        let msg = s.message.to_string();
                        Some(view! {
                            <div class="loading-overlay">
                                <div class={css}>
                                    {if is_error {
                                        view! {
                                            <div class="loading-error">
                                                <div class="loading-error-icon">"!"</div>
                                                <p>{msg}</p>
                                                <a href="/sessions" class="btn-back-sessions">"Back to Sessions"</a>
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
                <GameWindow id=WindowId::Terminal>
                    <TerminalPanel />
                </GameWindow>
                <GameWindow id=WindowId::FileBrowser>
                    <FileBrowserPanel />
                </GameWindow>
                <GameWindow id=WindowId::HelpViewer>
                    <HelpViewerPanel />
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
