use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;

use crate::components::charsheet::CharacterSheet;
use crate::components::chat::ChatPanel;
use crate::components::creatures::CreaturePanel;
use crate::components::initiative::InitiativeTracker;
use crate::components::inventory::InventoryPanel;
use crate::components::map::MapCanvas;
use crate::components::window_manager::{GameWindow, WindowId, WindowManager, WindowManagerContext};
use crate::models::{
    ChatMessageInfo, InitiativeEntryInfo, InventoryItemInfo, MapInfo, TokenInfo,
};
use crate::ws::messages::{ClientMessage, GameStateSnapshot, ServerMessage};

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
        self.initiative.set(snapshot.initiative);
        self.chat_messages.set(snapshot.recent_chat);
        self.inventory.set(snapshot.inventory);
        self.connected.set(true);
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
                let local_id = self.next_local_id.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
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
                self.tokens.update(|tokens| tokens.retain(|t| t.id != token_id));
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
                self.initiative.set(entries);
            }
            ServerMessage::CharacterUpdated { .. } => {
                // Character sheets not yet implemented on client
            }
            ServerMessage::InventoryUpdated { items } => {
                self.inventory.set(items);
            }
            ServerMessage::Error { message } => {
                log::warn!("Server error: {message}");
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
        next_local_id: std::sync::Arc::new(std::sync::atomic::AtomicI32::new(-1)),
    };

    provide_context(ctx.clone());

    // Fetch WS token and connect
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let ctx_ws = ctx.clone();

        Effect::new(move |_| {
            let sid = session_id_val();
            if sid == 0 {
                return;
            }
            session_id_w.set(sid);

            let ctx = ctx_ws.clone();

            leptos::task::spawn_local(async move {
                let token = match crate::server::api::get_ws_token().await {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Failed to get WS token: {e}");
                        return;
                    }
                };

                let window = web_sys::window().expect("no window");
                let location = window.location();
                let protocol = location.protocol().unwrap_or_default();
                let host = location.host().unwrap_or_default();
                let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
                let ws_url =
                    format!("{ws_protocol}//{host}/api/ws?token={token}");

                let ws = match web_sys::WebSocket::new(&ws_url) {
                    Ok(ws) => ws,
                    Err(e) => {
                        log::error!("Failed to create WebSocket: {e:?}");
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
                    ctx_open.send_message(ClientMessage::JoinSession {
                        session_id: sid,
                    });
                });
                ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                on_open.forget();

                // On message: parse and apply
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

                // On close
                let ctx_close = ctx.clone();
                let on_close = Closure::<dyn Fn()>::new(move || {
                    ctx_close.connected.set(false);
                });
                ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
                on_close.forget();

                // On error
                let on_error = Closure::<dyn Fn()>::new(move || {
                    log::error!("WebSocket error");
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

    view! {
        <div class="game-page">
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
                    <WindowToggleToolbar />
                    <span class="connection-status">
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
                <GameWindow id=WindowId::CharacterSheet>
                    <CharacterSheet />
                </GameWindow>
                <GameWindow id=WindowId::Initiative>
                    <InitiativeTracker />
                </GameWindow>
                <GameWindow id=WindowId::Inventory>
                    <InventoryPanel />
                </GameWindow>
                <GameWindow id=WindowId::Creatures>
                    <CreaturePanel />
                </GameWindow>
            </WindowManager>
        </div>
    }
}

/// Toolbar buttons to toggle window visibility.
#[component]
fn WindowToggleToolbar() -> impl IntoView {
    // WindowManagerContext won't be available during SSR render (it's provided
    // by WindowManager which renders after this). Use try_use_context to avoid panic.
    // On the client after hydration, the context will be available via effects.

    view! {
        <div class="wm-toolbar">
            {WindowId::all()
                .iter()
                .map(|&id| {
                    view! { <WindowToggleButton id=id /> }
                })
                .collect::<Vec<_>>()}
        </div>
    }
}

/// A single toggle button for a window.
#[component]
fn WindowToggleButton(id: WindowId) -> impl IntoView {
    let on_click = move |_| {
        if let Some(wm) = use_context::<WindowManagerContext>() {
            wm.toggle_window(id);
        }
    };

    let is_active = move || {
        use_context::<WindowManagerContext>()
            .map(|wm| {
                wm.windows.with(|wins| {
                    wins.iter()
                        .find(|w| w.id == id)
                        .map(|w| w.visible && !w.minimized)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    };

    view! {
        <button
            class="wm-toolbar-btn"
            class:active=is_active
            on:click=on_click
            title=format!("Toggle {}", id.title())
        >
            {id.title()}
        </button>
    }
}
