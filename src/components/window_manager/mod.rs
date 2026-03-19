mod dock;
pub(crate) mod persistence;
mod settings;

use leptos::prelude::*;

use crate::components::help_viewer::HelpContext;

use dock::Dock;
#[cfg(feature = "hydrate")]
use persistence::{load_or_default_layout, save_layout};

/// Identifies a game window by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WindowId {
    Map,
    Chat,
    CharacterSelection,
    Initiative,
    Inventory,
    Creatures,
    Terminal,
    FileBrowser,
    HelpViewer,
    /// Dynamic window for editing a specific character (by character_id).
    CharacterEditor(i32),
}

impl WindowId {
    pub fn title(&self) -> &'static str {
        match self {
            WindowId::Map => "Map",
            WindowId::Chat => "Chat",
            WindowId::CharacterSelection => "Character Selection",
            WindowId::Initiative => "Initiative",
            WindowId::Inventory => "Inventory",
            WindowId::Creatures => "Creatures",
            WindowId::Terminal => "COMMAND.COM",
            WindowId::FileBrowser => "File Viewer",
            WindowId::HelpViewer => "Help",
            WindowId::CharacterEditor(_) => "Character Sheet",
        }
    }

    /// Static window IDs used for default layout and toolbar.
    pub fn all() -> &'static [WindowId] {
        &[
            WindowId::Map,
            WindowId::Chat,
            WindowId::CharacterSelection,
            WindowId::Initiative,
            WindowId::Inventory,
            WindowId::Creatures,
            WindowId::Terminal,
            WindowId::FileBrowser,
            WindowId::HelpViewer,
        ]
    }

    /// Minimum size (width, height) in pixels for this window type.
    pub fn min_size(&self) -> (f64, f64) {
        match self {
            WindowId::Map => (400.0, 300.0),
            WindowId::Chat => (250.0, 200.0),
            WindowId::CharacterSelection => (250.0, 200.0),
            WindowId::Initiative => (220.0, 150.0),
            WindowId::Inventory => (250.0, 150.0),
            WindowId::Creatures => (280.0, 250.0),
            WindowId::Terminal => (500.0, 300.0),
            WindowId::FileBrowser => (450.0, 350.0),
            WindowId::HelpViewer => (400.0, 300.0),
            WindowId::CharacterEditor(_) => (300.0, 350.0),
        }
    }

    /// Returns true for dynamic windows that should not be persisted to localStorage.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, WindowId::CharacterEditor(_))
    }

    /// Unicode icon for dock tile display.
    pub fn dock_icon(&self) -> &'static str {
        match self {
            WindowId::Map => "\u{1f5fa}",                // 🗺 world map
            WindowId::Chat => "\u{1f4ac}",               // 💬 speech balloon
            WindowId::CharacterSelection => "\u{1f464}", // 👤 bust in silhouette
            WindowId::Initiative => "\u{2694}",          // ⚔ crossed swords
            WindowId::Inventory => "\u{1f392}",          // 🎒 backpack
            WindowId::Creatures => "\u{1f409}",          // 🐉 dragon
            WindowId::Terminal => "\u{1f4bb}",           // 💻 personal computer
            WindowId::FileBrowser => "\u{1f4c2}",        // 📂 open file folder
            WindowId::HelpViewer => "\u{1f4d6}",         // 📖 open book
            WindowId::CharacterEditor(_) => "\u{1f4dc}", // 📜 scroll
        }
    }

    /// Help topic slug associated with this window, if any.
    /// Used to show the ? help button in the title bar.
    pub fn help_topic(&self) -> Option<&'static str> {
        match self {
            WindowId::FileBrowser => Some("file-viewer"),
            _ => None,
        }
    }

    /// Short label for dock tile (truncated for 64px tile width).
    pub fn dock_label(&self) -> &'static str {
        match self {
            WindowId::Map => "Map",
            WindowId::Chat => "Chat",
            WindowId::CharacterSelection => "Chars",
            WindowId::Initiative => "Init",
            WindowId::Inventory => "Items",
            WindowId::Creatures => "Beasts",
            WindowId::Terminal => "Term",
            WindowId::FileBrowser => "Files",
            WindowId::HelpViewer => "Help",
            WindowId::CharacterEditor(_) => "Sheet",
        }
    }
}

/// Persistent state for a single window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindowState {
    pub id: WindowId,
    pub title: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub z_index: u32,
    pub minimized: bool,
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

    /// Close a dynamic window (removes it entirely). Static windows cannot be closed.
    pub fn close_window(&self, id: WindowId) {
        if id.is_dynamic() {
            self.windows.update(|wins| {
                wins.retain(|w| w.id != id);
            });
        }
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
        let (vw, vh) = get_viewport_size();
        self.windows.update(|wins| {
            if let Some(w) = wins.iter_mut().find(|w| w.id == id) {
                w.minimized = false;
                w.z_index = z;
                // Ensure window is at least partially visible on screen
                clamp_to_viewport(w, vw, vh);
            }
        });
    }

    /// Minimize all non-minimized windows (return all to dock).
    pub fn minimize_all(&self) {
        self.windows.update(|wins| {
            for w in wins.iter_mut() {
                if !w.minimized {
                    w.minimized = true;
                }
            }
        });
    }

    /// Push non-minimized windows so they don't overlap the dock area.
    pub fn push_windows_from_dock(&self, dock_w: f64, dock_h: f64) {
        self.windows.update(|wins| {
            for w in wins.iter_mut() {
                if w.minimized {
                    continue;
                }
                // Check if the window's top-left corner is inside the dock area
                if w.x < dock_w && w.y < dock_h {
                    // Push right or down, whichever requires less movement
                    let push_right = dock_w - w.x;
                    let push_down = dock_h - w.y;
                    if push_right <= push_down {
                        w.x = dock_w;
                    } else {
                        w.y = dock_h;
                    }
                }
            }
        });
    }

    /// Open a dynamic character editor window. If already open, brings it to front.
    pub fn open_character_editor(&self, character_id: i32, character_name: &str) {
        let win_id = WindowId::CharacterEditor(character_id);

        // Check if already open
        let exists = self
            .windows
            .with_untracked(|wins| wins.iter().any(|w| w.id == win_id));

        if exists {
            self.restore_window(win_id);
        } else {
            let z = self.next_z.get_untracked();
            self.next_z.set(z + 1);

            // Offset new windows slightly so they don't stack exactly
            let count = self.windows.with_untracked(|wins| {
                wins.iter()
                    .filter(|w| matches!(w.id, WindowId::CharacterEditor(_)))
                    .count()
            });
            let offset = (count as f64) * 25.0;

            self.windows.update(|wins| {
                wins.push(WindowState {
                    id: win_id,
                    title: Some(character_name.to_string()),
                    x: 100.0 + offset,
                    y: 80.0 + offset,
                    width: 340.0,
                    height: 450.0,
                    z_index: z,
                    minimized: false,
                });
            });
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

// ---------------------------------------------------------------------------
// Viewport & default layouts
// ---------------------------------------------------------------------------

/// Get the viewport dimensions available for window layout.
/// Returns (width, height) in pixels, accounting for the game header.
fn get_viewport_size() -> (f64, f64) {
    #[cfg(feature = "hydrate")]
    {
        if let Some(window) = web_sys::window() {
            let w = window
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(1280.0);
            let h = window
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(800.0);
            // Subtract estimated header height (~50px)
            return (w, (h - 50.0).max(400.0));
        }
    }
    (1280.0, 714.0) // SSR fallback
}

/// Reposition a window so it is entirely visible within the viewport.
fn clamp_to_viewport(w: &mut WindowState, vw: f64, vh: f64) {
    // Ensure the window fits; shrink if larger than viewport
    if w.width > vw {
        w.width = vw;
    }
    if w.height > vh {
        w.height = vh;
    }
    // Clamp position so the entire window is on-screen
    if w.x < 0.0 {
        w.x = 0.0;
    } else if w.x + w.width > vw {
        w.x = (vw - w.width).max(0.0);
    }
    if w.y < 0.0 {
        w.y = 0.0;
    } else if w.y + w.height > vh {
        w.y = (vh - w.height).max(0.0);
    }
}

/// Default window layout scaled to the given viewport dimensions.
///
/// Three layout tiers:
/// - **Small** (< 900px wide): Stack windows vertically, all start minimized
///   except Map and Chat.
/// - **Medium** (900-1399px): Two-column layout — Map on the left, sidebar
///   panels on the right.
/// - **Large** (1400px+): Spacious two-column layout with more room for
///   each panel.
pub fn default_window_layout() -> Vec<WindowState> {
    let (vw, vh) = get_viewport_size();
    default_window_layout_for_size(vw, vh)
}

fn default_window_layout_for_size(vw: f64, vh: f64) -> Vec<WindowState> {
    let pad = 10.0;

    if vw < 900.0 {
        // Small screen: stack map and chat, minimize the rest
        let win_w = (vw - 2.0 * pad).max(400.0);
        vec![
            WindowState {
                id: WindowId::Map,
                title: None,
                x: pad,
                y: pad,
                width: win_w,
                height: (vh * 0.55).max(300.0),
                z_index: 1,
                minimized: false,
            },
            WindowState {
                id: WindowId::Chat,
                title: None,
                x: pad,
                y: pad + (vh * 0.55).max(300.0) + pad,
                width: win_w,
                height: (vh * 0.35).max(200.0),
                z_index: 2,
                minimized: false,
            },
            WindowState {
                id: WindowId::CharacterSelection,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(350.0),
                height: 300.0,
                z_index: 3,
                minimized: true,
            },
            WindowState {
                id: WindowId::Initiative,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(300.0),
                height: 200.0,
                z_index: 4,
                minimized: true,
            },
            WindowState {
                id: WindowId::Inventory,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(300.0),
                height: 200.0,
                z_index: 5,
                minimized: true,
            },
            WindowState {
                id: WindowId::Creatures,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(350.0),
                height: 400.0,
                z_index: 6,
                minimized: true,
            },
            WindowState {
                id: WindowId::Terminal,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(500.0),
                height: 300.0,
                z_index: 7,
                minimized: true,
            },
            WindowState {
                id: WindowId::FileBrowser,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(450.0),
                height: 350.0,
                z_index: 8,
                minimized: true,
            },
            WindowState {
                id: WindowId::HelpViewer,
                title: None,
                x: pad,
                y: pad,
                width: win_w.min(500.0),
                height: 400.0,
                z_index: 9,
                minimized: true,
            },
        ]
    } else if vw < 1400.0 {
        // Medium screen: two-column layout
        let sidebar_w = 320.0_f64.min(vw * 0.3);
        let map_w = vw - sidebar_w - 3.0 * pad;
        let sidebar_x = map_w + 2.0 * pad;
        let chat_h = (vh * 0.5).max(250.0);
        let char_sel_h = vh - chat_h - 3.0 * pad;

        vec![
            WindowState {
                id: WindowId::Map,
                title: None,
                x: pad,
                y: pad,
                width: map_w,
                height: vh - 2.0 * pad,
                z_index: 1,
                minimized: false,
            },
            WindowState {
                id: WindowId::Chat,
                title: None,
                x: sidebar_x,
                y: pad,
                width: sidebar_w,
                height: chat_h,
                z_index: 2,
                minimized: false,
            },
            WindowState {
                id: WindowId::CharacterSelection,
                title: None,
                x: sidebar_x,
                y: chat_h + 2.0 * pad,
                width: sidebar_w,
                height: char_sel_h,
                z_index: 3,
                minimized: false,
            },
            WindowState {
                id: WindowId::Initiative,
                title: None,
                x: pad,
                y: pad,
                width: 280.0,
                height: 200.0,
                z_index: 4,
                minimized: true,
            },
            WindowState {
                id: WindowId::Inventory,
                title: None,
                x: pad,
                y: pad,
                width: 300.0,
                height: 200.0,
                z_index: 5,
                minimized: true,
            },
            WindowState {
                id: WindowId::Creatures,
                title: None,
                x: 50.0,
                y: 50.0,
                width: 350.0,
                height: 400.0,
                z_index: 6,
                minimized: true,
            },
            WindowState {
                id: WindowId::Terminal,
                title: None,
                x: 60.0,
                y: 60.0,
                width: 600.0,
                height: 400.0,
                z_index: 7,
                minimized: true,
            },
            WindowState {
                id: WindowId::FileBrowser,
                title: None,
                x: 80.0,
                y: 40.0,
                width: 550.0,
                height: 450.0,
                z_index: 8,
                minimized: true,
            },
            WindowState {
                id: WindowId::HelpViewer,
                title: None,
                x: 100.0,
                y: 60.0,
                width: 550.0,
                height: 450.0,
                z_index: 9,
                minimized: true,
            },
        ]
    } else {
        // Large screen (1400px+): spacious two-column layout
        let sidebar_w = 360.0_f64.min(vw * 0.25);
        let map_w = vw - sidebar_w - 3.0 * pad;
        let sidebar_x = map_w + 2.0 * pad;
        let chat_h = (vh * 0.5).max(300.0);
        let char_sel_h = vh - chat_h - 3.0 * pad;
        let init_w = 280.0;
        let init_h = 220.0;

        vec![
            WindowState {
                id: WindowId::Map,
                title: None,
                x: pad,
                y: pad,
                width: map_w,
                height: vh - 2.0 * pad,
                z_index: 1,
                minimized: false,
            },
            WindowState {
                id: WindowId::Chat,
                title: None,
                x: sidebar_x,
                y: pad,
                width: sidebar_w,
                height: chat_h,
                z_index: 2,
                minimized: false,
            },
            WindowState {
                id: WindowId::CharacterSelection,
                title: None,
                x: sidebar_x,
                y: chat_h + 2.0 * pad,
                width: sidebar_w,
                height: char_sel_h,
                z_index: 3,
                minimized: false,
            },
            WindowState {
                id: WindowId::Initiative,
                title: None,
                x: pad,
                y: vh - init_h - pad,
                width: init_w,
                height: init_h,
                z_index: 4,
                minimized: false,
            },
            WindowState {
                id: WindowId::Inventory,
                title: None,
                x: init_w + 2.0 * pad,
                y: vh - 200.0 - pad,
                width: 300.0,
                height: 200.0,
                z_index: 5,
                minimized: true,
            },
            WindowState {
                id: WindowId::Creatures,
                title: None,
                x: 50.0,
                y: 50.0,
                width: 380.0,
                height: 450.0,
                z_index: 6,
                minimized: true,
            },
            WindowState {
                id: WindowId::Terminal,
                title: None,
                x: 80.0,
                y: 80.0,
                width: 600.0,
                height: 400.0,
                z_index: 7,
                minimized: true,
            },
            WindowState {
                id: WindowId::FileBrowser,
                title: None,
                x: 120.0,
                y: 60.0,
                width: 550.0,
                height: 450.0,
                z_index: 8,
                minimized: true,
            },
            WindowState {
                id: WindowId::HelpViewer,
                title: None,
                x: 140.0,
                y: 80.0,
                width: 600.0,
                height: 500.0,
                z_index: 9,
                minimized: true,
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// WindowManager component
// ---------------------------------------------------------------------------

/// The top-level window manager. Place this inside the game viewport area.
/// It captures mouse events on a full-viewport overlay to handle drag/resize.
#[component]
pub fn WindowManager(children: Children) -> impl IntoView {
    // Always start with the SSR-safe default layout (fixed 1280x714 fallback)
    // to avoid hydration mismatches. The client-side Effect below will apply
    // the real layout (from localStorage / actual viewport) after hydration.
    let initial_layout = default_window_layout_for_size(1280.0, 714.0);
    let max_z = initial_layout.iter().map(|w| w.z_index).max().unwrap_or(0) + 1;

    let wm_ctx = WindowManagerContext {
        windows: RwSignal::new(initial_layout),
        drag_op: RwSignal::new(None),
        next_z: RwSignal::new(max_z),
    };

    provide_context(wm_ctx.clone());

    // After hydration: apply the real layout from localStorage / viewport size,
    // then persist layout changes on every subsequent update.
    #[cfg(feature = "hydrate")]
    {
        let windows = wm_ctx.windows;
        let next_z = wm_ctx.next_z;

        // One-shot effect to load saved layout after hydration
        Effect::new(move |has_loaded: Option<bool>| {
            let _ = windows.get(); // track changes for save_layout
            if has_loaded == Some(true) {
                // Subsequent runs: persist layout changes to localStorage
                save_layout(&windows.get_untracked());
                return true;
            }
            // First run: load from localStorage or compute from real viewport
            let layout = load_or_default_layout();
            let z = layout.iter().map(|w| w.z_index).max().unwrap_or(0) + 1;
            windows.set(layout);
            next_z.set(z);
            true
        });
    }

    let ctx_move = wm_ctx.clone();
    let ctx_up = wm_ctx.clone();
    let ctx_leave = wm_ctx.clone();
    let dragging = wm_ctx.drag_op;

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
            <Dock />
        </div>
    }
}

// ---------------------------------------------------------------------------
// GameWindow component
// ---------------------------------------------------------------------------

/// A single draggable, resizable game window.
///
/// Use like: `<GameWindow id=WindowId::Chat><ChatPanel /></GameWindow>`
#[component]
pub fn GameWindow(
    id: WindowId,
    #[prop(optional, into)] title: Option<String>,
    children: Children,
) -> impl IntoView {
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
                    title: None,
                    x: 100.0,
                    y: 100.0,
                    width: 400.0,
                    height: 300.0,
                    z_index: 1,
                    minimized: false,
                })
        }
    };

    // Resolve display title: prop > window state custom title > default
    let display_title = {
        let title_prop = title.clone();
        let windows = wm.windows;
        move || {
            if let Some(ref t) = title_prop {
                return t.clone();
            }
            let custom = windows.with(|wins| {
                wins.iter()
                    .find(|w| w.id == id)
                    .and_then(|w| w.title.clone())
            });
            custom.unwrap_or_else(|| id.title().to_string())
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

    // Help / minimize / close buttons
    let help_topic = id.help_topic();
    let wm_help = wm.clone();
    let wm_min = wm.clone();
    let wm_close = wm.clone();
    let is_dynamic = id.is_dynamic();

    let style = {
        let windows = wm.windows;
        move || {
            let ws = windows.with(|wins| wins.iter().find(|w| w.id == id).cloned());
            match ws {
                Some(ws) if !ws.minimized => format!(
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
                <span class="gw-title-icon">{id.dock_icon()}</span>
                <span class="gw-title">{display_title}</span>
                <div class="gw-controls">
                    {if let Some(topic) = help_topic {
                        let wm_h = wm_help.clone();
                        Some(view! {
                            <button class="gw-btn gw-btn-help"
                                on:click=move |_| {
                                    let help_ctx = expect_context::<HelpContext>();
                                    help_ctx.navigate_to.set(Some(topic.to_string()));
                                    wm_h.restore_window(WindowId::HelpViewer);
                                }
                                title="Help"
                            >"?"</button>
                        })
                    } else {
                        None
                    }}
                    <button class="gw-btn gw-btn-min"
                        on:click=move |_| wm_min.minimize_window(id)
                        title="Minimize"
                    >"_"</button>
                    {if is_dynamic {
                        Some(view! {
                            <button class="gw-btn gw-btn-close"
                                on:click=move |_| wm_close.close_window(id)
                                title="Close"
                            >"\u{00d7}"</button>
                        })
                    } else {
                        None
                    }}
                </div>
            </div>

            // Window body — children render here
            <div class="gw-body">
                {rendered_children}
            </div>
        </div>
    }
}
