use leptos::prelude::*;

/// Identifies a game window by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WindowId {
    Map,
    Chat,
    CharacterSheet,
    Initiative,
    Inventory,
    Creatures,
}

impl WindowId {
    pub fn title(&self) -> &'static str {
        match self {
            WindowId::Map => "Map",
            WindowId::Chat => "Chat",
            WindowId::CharacterSheet => "Character Sheet",
            WindowId::Initiative => "Initiative",
            WindowId::Inventory => "Inventory",
            WindowId::Creatures => "Creatures",
        }
    }

    pub fn all() -> &'static [WindowId] {
        &[
            WindowId::Map,
            WindowId::Chat,
            WindowId::CharacterSheet,
            WindowId::Initiative,
            WindowId::Inventory,
            WindowId::Creatures,
        ]
    }

    /// Minimum size (width, height) in pixels for this window type.
    pub fn min_size(&self) -> (f64, f64) {
        match self {
            WindowId::Map => (400.0, 300.0),
            WindowId::Chat => (250.0, 200.0),
            WindowId::CharacterSheet => (280.0, 300.0),
            WindowId::Initiative => (220.0, 150.0),
            WindowId::Inventory => (250.0, 150.0),
            WindowId::Creatures => (280.0, 250.0),
        }
    }
}

/// Persistent state for a single window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    pub id: WindowId,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: u32,
    pub minimized: bool,
    pub visible: bool,
}

/// Which edge/corner is being resized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Active drag or resize operation.
#[derive(Debug, Clone, Copy)]
enum DragOp {
    Move {
        window_id: WindowId,
        offset_x: f64,
        offset_y: f64,
    },
    Resize {
        window_id: WindowId,
        edge: ResizeEdge,
        start_x: f64,
        start_y: f64,
        orig_x: f64,
        orig_y: f64,
        orig_w: f64,
        orig_h: f64,
    },
}

/// Context provided to child components for window operations.
#[derive(Clone)]
pub struct WindowManagerContext {
    pub windows: RwSignal<Vec<WindowState>>,
    drag_op: RwSignal<Option<DragOp>>,
    next_z: RwSignal<u32>,
}

impl WindowManagerContext {
    pub fn bring_to_front(&self, id: WindowId) {
        let z = self.next_z.get_untracked();
        self.next_z.set(z + 1);
        self.windows.update(|wins| {
            if let Some(w) = wins.iter_mut().find(|w| w.id == id) {
                w.z_index = z;
            }
        });
    }

    pub fn close_window(&self, id: WindowId) {
        self.windows.update(|wins| {
            if let Some(w) = wins.iter_mut().find(|w| w.id == id) {
                w.visible = false;
            }
        });
    }

    pub fn minimize_window(&self, id: WindowId) {
        self.windows.update(|wins| {
            if let Some(w) = wins.iter_mut().find(|w| w.id == id) {
                w.minimized = true;
            }
        });
    }

    pub fn restore_window(&self, id: WindowId) {
        let z = self.next_z.get_untracked();
        self.next_z.set(z + 1);
        self.windows.update(|wins| {
            if let Some(w) = wins.iter_mut().find(|w| w.id == id) {
                w.minimized = false;
                w.visible = true;
                w.z_index = z;
            }
        });
    }

    pub fn toggle_window(&self, id: WindowId) {
        let is_visible = self
            .windows
            .with_untracked(|wins| wins.iter().find(|w| w.id == id).map(|w| w.visible && !w.minimized));
        match is_visible {
            Some(true) => self.close_window(id),
            _ => self.restore_window(id),
        }
    }

    fn start_move(&self, id: WindowId, offset_x: f64, offset_y: f64) {
        self.bring_to_front(id);
        self.drag_op.set(Some(DragOp::Move {
            window_id: id,
            offset_x,
            offset_y,
        }));
    }

    fn start_resize(&self, id: WindowId, edge: ResizeEdge, mouse_x: f64, mouse_y: f64) {
        self.bring_to_front(id);
        let (ox, oy, ow, oh) = self.windows.with_untracked(|wins| {
            wins.iter()
                .find(|w| w.id == id)
                .map(|w| (w.x, w.y, w.width, w.height))
                .unwrap_or((0.0, 0.0, 400.0, 300.0))
        });
        self.drag_op.set(Some(DragOp::Resize {
            window_id: id,
            edge,
            start_x: mouse_x,
            start_y: mouse_y,
            orig_x: ox,
            orig_y: oy,
            orig_w: ow,
            orig_h: oh,
        }));
    }

    fn on_mouse_move(&self, mouse_x: f64, mouse_y: f64) {
        let Some(op) = self.drag_op.get_untracked() else {
            return;
        };
        match op {
            DragOp::Move {
                window_id,
                offset_x,
                offset_y,
            } => {
                let new_x = (mouse_x - offset_x).max(0.0);
                let new_y = (mouse_y - offset_y).max(0.0);
                self.windows.update(|wins| {
                    if let Some(w) = wins.iter_mut().find(|w| w.id == window_id) {
                        w.x = new_x;
                        w.y = new_y;
                    }
                });
            }
            DragOp::Resize {
                window_id,
                edge,
                start_x,
                start_y,
                orig_x,
                orig_y,
                orig_w,
                orig_h,
            } => {
                let dx = mouse_x - start_x;
                let dy = mouse_y - start_y;
                let (min_w, min_h) = window_id.min_size();

                self.windows.update(|wins| {
                    if let Some(w) = wins.iter_mut().find(|w| w.id == window_id) {
                        match edge {
                            ResizeEdge::Right => {
                                w.width = (orig_w + dx).max(min_w);
                            }
                            ResizeEdge::Bottom => {
                                w.height = (orig_h + dy).max(min_h);
                            }
                            ResizeEdge::Left => {
                                let new_w = (orig_w - dx).max(min_w);
                                w.x = orig_x + orig_w - new_w;
                                w.width = new_w;
                            }
                            ResizeEdge::Top => {
                                let new_h = (orig_h - dy).max(min_h);
                                w.y = orig_y + orig_h - new_h;
                                w.height = new_h;
                            }
                            ResizeEdge::BottomRight => {
                                w.width = (orig_w + dx).max(min_w);
                                w.height = (orig_h + dy).max(min_h);
                            }
                            ResizeEdge::BottomLeft => {
                                let new_w = (orig_w - dx).max(min_w);
                                w.x = orig_x + orig_w - new_w;
                                w.width = new_w;
                                w.height = (orig_h + dy).max(min_h);
                            }
                            ResizeEdge::TopRight => {
                                w.width = (orig_w + dx).max(min_w);
                                let new_h = (orig_h - dy).max(min_h);
                                w.y = orig_y + orig_h - new_h;
                                w.height = new_h;
                            }
                            ResizeEdge::TopLeft => {
                                let new_w = (orig_w - dx).max(min_w);
                                w.x = orig_x + orig_w - new_w;
                                w.width = new_w;
                                let new_h = (orig_h - dy).max(min_h);
                                w.y = orig_y + orig_h - new_h;
                                w.height = new_h;
                            }
                        }
                    }
                });
            }
        }
    }

    fn on_mouse_up(&self) {
        self.drag_op.set(None);
    }
}

/// Default window layout for a typical screen.
pub fn default_window_layout() -> Vec<WindowState> {
    vec![
        WindowState {
            id: WindowId::Map,
            x: 10.0,
            y: 10.0,
            width: 700.0,
            height: 500.0,
            z_index: 1,
            minimized: false,
            visible: true,
        },
        WindowState {
            id: WindowId::Chat,
            x: 720.0,
            y: 10.0,
            width: 320.0,
            height: 400.0,
            z_index: 2,
            minimized: false,
            visible: true,
        },
        WindowState {
            id: WindowId::CharacterSheet,
            x: 720.0,
            y: 420.0,
            width: 320.0,
            height: 350.0,
            z_index: 3,
            minimized: false,
            visible: true,
        },
        WindowState {
            id: WindowId::Initiative,
            x: 10.0,
            y: 520.0,
            width: 280.0,
            height: 200.0,
            z_index: 4,
            minimized: false,
            visible: true,
        },
        WindowState {
            id: WindowId::Inventory,
            x: 300.0,
            y: 520.0,
            width: 300.0,
            height: 200.0,
            z_index: 5,
            minimized: true,
            visible: true,
        },
        WindowState {
            id: WindowId::Creatures,
            x: 50.0,
            y: 50.0,
            width: 350.0,
            height: 400.0,
            z_index: 6,
            minimized: false,
            visible: false, // GM only — toggled on by GamePage when user is GM
        },
    ]
}

#[cfg(feature = "hydrate")]
const LAYOUT_STORAGE_KEY: &str = "webrpg_window_layout";

/// Load window layout from localStorage, falling back to defaults.
/// Handles version mismatches: if a new window ID exists in defaults but not
/// in storage, the default is used; unknown stored windows are dropped.
fn load_or_default_layout() -> Vec<WindowState> {
    #[cfg(feature = "hydrate")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item(LAYOUT_STORAGE_KEY) {
                    if let Ok(mut stored) = serde_json::from_str::<Vec<WindowState>>(&json) {
                        // Merge with defaults: keep stored state for known IDs,
                        // add defaults for any new IDs.
                        let defaults = default_window_layout();
                        for default_win in &defaults {
                            if !stored.iter().any(|w| w.id == default_win.id) {
                                stored.push(default_win.clone());
                            }
                        }
                        // Remove any stored windows with IDs not in defaults
                        stored.retain(|w| defaults.iter().any(|d| d.id == w.id));
                        return stored;
                    }
                }
            }
        }
    }
    default_window_layout()
}

/// Save window layout to localStorage.
#[cfg(feature = "hydrate")]
fn save_layout(windows: &[WindowState]) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(windows) {
                let _ = storage.set_item(LAYOUT_STORAGE_KEY, &json);
            }
        }
    }
}

/// The top-level window manager. Place this inside the game viewport area.
/// It captures mouse events on a full-viewport overlay to handle drag/resize.
#[component]
pub fn WindowManager(children: Children) -> impl IntoView {
    let initial_layout = load_or_default_layout();
    let max_z = initial_layout.iter().map(|w| w.z_index).max().unwrap_or(0) + 1;

    let wm_ctx = WindowManagerContext {
        windows: RwSignal::new(initial_layout),
        drag_op: RwSignal::new(None),
        next_z: RwSignal::new(max_z),
    };

    provide_context(wm_ctx.clone());

    // Persist layout to localStorage on every change (debouncing not needed —
    // localStorage writes are synchronous and fast)
    #[cfg(feature = "hydrate")]
    {
        let windows = wm_ctx.windows;
        Effect::new(move |_| {
            let wins = windows.get();
            save_layout(&wins);
        });
    }

    let ctx_move = wm_ctx.clone();
    let ctx_up = wm_ctx.clone();
    let ctx_leave = wm_ctx.clone();
    let dragging = wm_ctx.drag_op;

    // Taskbar: show minimized windows
    let wm_taskbar = wm_ctx.clone();
    let minimized_windows = move || {
        wm_taskbar
            .windows
            .get()
            .into_iter()
            .filter(|w| w.minimized && w.visible)
            .collect::<Vec<_>>()
    };

    view! {
        <div
            class="wm-viewport"
            on:mousemove=move |ev: leptos::ev::MouseEvent| {
                if dragging.get_untracked().is_some() {
                    ev.prevent_default();
                    ctx_move.on_mouse_move(ev.client_x() as f64, ev.client_y() as f64);
                }
            }
            on:mouseup=move |_| {
                ctx_up.on_mouse_up();
            }
            on:mouseleave=move |_| {
                // Stop dragging if cursor leaves viewport
                ctx_leave.on_mouse_up();
            }
        >
            {children()}
            <div class="wm-taskbar">
                <For
                    each=minimized_windows
                    key=|w| w.id
                    let:win
                >
                    {
                        let wm = wm_taskbar.clone();
                        let id = win.id;
                        view! {
                            <button
                                class="wm-taskbar-btn"
                                on:click=move |_| wm.restore_window(id)
                            >
                                {id.title()}
                            </button>
                        }
                    }
                </For>
            </div>
        </div>
    }
}

/// A single draggable, resizable game window.
///
/// Use like: `<GameWindow id=WindowId::Chat><ChatPanel /></GameWindow>`
#[component]
pub fn GameWindow(id: WindowId, children: Children) -> impl IntoView {
    let wm = expect_context::<WindowManagerContext>();

    // Derive this window's state reactively
    let win_state = {
        let windows = wm.windows;
        move || {
            windows
                .get()
                .into_iter()
                .find(|w| w.id == id)
                .unwrap_or(WindowState {
                    id,
                    x: 100.0,
                    y: 100.0,
                    width: 400.0,
                    height: 300.0,
                    z_index: 1,
                    minimized: false,
                    visible: true,
                })
        }
    };

    // Title bar mousedown → start drag
    let wm_drag = wm.clone();
    let on_titlebar_mousedown = move |ev: leptos::ev::MouseEvent| {
        // Don't start drag if clicking a button
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            if let Some(target) = ev.target() {
                if let Some(el) = target.dyn_ref::<web_sys::HtmlElement>() {
                    if el.tag_name() == "BUTTON" {
                        return;
                    }
                }
            }
        }

        let ws = win_state();
        let offset_x = ev.client_x() as f64 - ws.x;
        let offset_y = ev.client_y() as f64 - ws.y;
        wm_drag.start_move(id, offset_x, offset_y);
    };

    // Resize handle mousedown generators
    let make_resize_handler = {
        let wm = wm.clone();
        move |edge: ResizeEdge| {
            let wm = wm.clone();
            move |ev: leptos::ev::MouseEvent| {
                ev.prevent_default();
                ev.stop_propagation();
                wm.start_resize(id, edge, ev.client_x() as f64, ev.client_y() as f64);
            }
        }
    };

    let on_resize_top = make_resize_handler(ResizeEdge::Top);
    let on_resize_bottom = make_resize_handler(ResizeEdge::Bottom);
    let on_resize_left = make_resize_handler(ResizeEdge::Left);
    let on_resize_right = make_resize_handler(ResizeEdge::Right);
    let on_resize_tl = make_resize_handler(ResizeEdge::TopLeft);
    let on_resize_tr = make_resize_handler(ResizeEdge::TopRight);
    let on_resize_bl = make_resize_handler(ResizeEdge::BottomLeft);
    let on_resize_br = make_resize_handler(ResizeEdge::BottomRight);

    // Bring to front on mousedown anywhere in window
    let wm_focus = wm.clone();
    let on_window_mousedown = move |_: leptos::ev::MouseEvent| {
        wm_focus.bring_to_front(id);
    };

    // Close / minimize buttons
    let wm_close = wm.clone();
    let wm_min = wm.clone();

    let style = {
        let windows = wm.windows;
        move || {
            let ws = windows.with(|wins| {
                wins.iter().find(|w| w.id == id).cloned()
            });
            match ws {
                Some(ws) if ws.visible && !ws.minimized => format!(
                    "left:{}px;top:{}px;width:{}px;height:{}px;z-index:{};",
                    ws.x, ws.y, ws.width, ws.height, ws.z_index
                ),
                _ => "display:none;".to_string(),
            }
        }
    };

    let rendered_children = children();

    view! {
        <div
            class="gw"
            style=style
            on:mousedown=on_window_mousedown
        >
            // Resize handles (edges + corners)
            <div class="gw-resize gw-resize-t" on:mousedown=on_resize_top></div>
            <div class="gw-resize gw-resize-b" on:mousedown=on_resize_bottom></div>
            <div class="gw-resize gw-resize-l" on:mousedown=on_resize_left></div>
            <div class="gw-resize gw-resize-r" on:mousedown=on_resize_right></div>
            <div class="gw-resize gw-resize-tl" on:mousedown=on_resize_tl></div>
            <div class="gw-resize gw-resize-tr" on:mousedown=on_resize_tr></div>
            <div class="gw-resize gw-resize-bl" on:mousedown=on_resize_bl></div>
            <div class="gw-resize gw-resize-br" on:mousedown=on_resize_br></div>

            // Title bar
            <div class="gw-titlebar" on:mousedown=on_titlebar_mousedown>
                <span class="gw-title">{id.title()}</span>
                <div class="gw-controls">
                    <button class="gw-btn gw-btn-min"
                        on:click=move |_| wm_min.minimize_window(id)
                        title="Minimize"
                    >"_"</button>
                    <button class="gw-btn gw-btn-close"
                        on:click=move |_| wm_close.close_window(id)
                        title="Close"
                    >"\u{00d7}"</button>
                </div>
            </div>

            // Window body — children render here
            <div class="gw-body">
                {rendered_children}
            </div>
        </div>
    }
}
