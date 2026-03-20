/// NeXTSTEP-style dock with snap-to-grid tile management.
use leptos::prelude::*;

use super::settings::SettingsDialog;
use super::{WindowId, WindowManagerContext};

/// Dock tile size in pixels.
const DOCK_TILE_SIZE: f64 = 64.0;

/// Position of a tile in the dock grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
struct DockPos {
    col: i32,
    row: i32,
}

/// Persistent dock layout: maps window IDs to grid positions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DockLayout {
    tiles: Vec<(WindowId, DockPos)>,
}

#[allow(dead_code)]
impl DockLayout {
    fn new() -> Self {
        Self { tiles: vec![] }
    }

    /// Get the grid position for a window, or None if not placed yet.
    fn get_pos(&self, id: WindowId) -> Option<DockPos> {
        self.tiles
            .iter()
            .find(|(wid, _)| *wid == id)
            .map(|(_, p)| *p)
    }

    /// Check if a grid position is occupied (by any tile or the system icon at 0,0).
    fn is_occupied(&self, pos: DockPos) -> bool {
        if pos.col == 0 && pos.row == 0 {
            return true; // system icon
        }
        self.tiles.iter().any(|(_, p)| *p == pos)
    }

    /// Place a window at a specific grid position, replacing any existing placement.
    fn set_pos(&mut self, id: WindowId, pos: DockPos) {
        self.tiles.retain(|(wid, _)| *wid != id);
        self.tiles.push((id, pos));
    }

    /// Remove a window from the dock (when restored).
    fn remove(&mut self, id: WindowId) {
        self.tiles.retain(|(wid, _)| *wid != id);
    }

    /// Find the next available position adjacent to existing tiles.
    /// Default placement: vertical column below the system icon.
    fn next_available_pos(&self) -> DockPos {
        // Try positions in column 0, starting from row 1 (below system icon)
        for row in 1..50 {
            let pos = DockPos { col: 0, row };
            if !self.is_occupied(pos) {
                return pos;
            }
        }
        // Overflow: try column 1
        for row in 0..50 {
            let pos = DockPos { col: 1, row };
            if !self.is_occupied(pos) {
                return pos;
            }
        }
        DockPos { col: 0, row: 50 }
    }

    /// Compute the bounding rectangle of all dock tiles (including system icon)
    /// as (width_px, height_px).
    fn bounds_px(&self) -> (f64, f64) {
        let mut max_col = 0i32;
        let mut max_row = 0i32;
        for (_, pos) in &self.tiles {
            max_col = max_col.max(pos.col);
            max_row = max_row.max(pos.row);
        }
        (
            (max_col + 1) as f64 * DOCK_TILE_SIZE,
            (max_row + 1) as f64 * DOCK_TILE_SIZE,
        )
    }

    /// Find the nearest unoccupied grid position adjacent to at least one
    /// existing tile (or the system icon). Used for drag-and-drop snapping.
    fn snap_to_grid(&self, x: f64, y: f64) -> Option<DockPos> {
        let target_col = (x / DOCK_TILE_SIZE).round() as i32;
        let target_row = (y / DOCK_TILE_SIZE).round() as i32;

        if target_col < 0 || target_row < 0 {
            return None;
        }

        let candidate = DockPos {
            col: target_col,
            row: target_row,
        };

        // Must be adjacent to an existing tile or the system icon
        if !self.is_occupied(candidate) && self.has_adjacent_tile(candidate) {
            return Some(candidate);
        }

        // Search nearby positions in expanding radius
        for radius in 1..=3 {
            for dc in -radius..=radius {
                for dr in -radius..=radius {
                    let pos = DockPos {
                        col: target_col + dc,
                        row: target_row + dr,
                    };
                    if pos.col >= 0
                        && pos.row >= 0
                        && !self.is_occupied(pos)
                        && self.has_adjacent_tile(pos)
                    {
                        return Some(pos);
                    }
                }
            }
        }
        None
    }

    /// Check if a position has at least one adjacent occupied tile.
    fn has_adjacent_tile(&self, pos: DockPos) -> bool {
        let neighbors = [
            DockPos {
                col: pos.col - 1,
                row: pos.row,
            },
            DockPos {
                col: pos.col + 1,
                row: pos.row,
            },
            DockPos {
                col: pos.col,
                row: pos.row - 1,
            },
            DockPos {
                col: pos.col,
                row: pos.row + 1,
            },
        ];
        for n in &neighbors {
            // System icon at (0,0) counts as adjacent
            if n.col == 0 && n.row == 0 {
                return true;
            }
            if self.tiles.iter().any(|(_, p)| p == n) {
                return true;
            }
        }
        false
    }
}

#[cfg(feature = "hydrate")]
const DOCK_STORAGE_KEY: &str = "webrpg_dock_layout";

#[cfg(feature = "hydrate")]
fn load_dock_layout() -> DockLayout {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item(DOCK_STORAGE_KEY) {
                if let Ok(layout) = serde_json::from_str::<DockLayout>(&json) {
                    return layout;
                }
            }
        }
    }
    DockLayout::new()
}

#[cfg(feature = "hydrate")]
fn save_dock_layout(layout: &DockLayout) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(layout) {
                let _ = storage.set_item(DOCK_STORAGE_KEY, &json);
            }
        }
    }
}

/// Active dock tile drag operation.
#[derive(Debug, Clone, Copy)]
struct DockDrag {
    window_id: WindowId,
    /// Current mouse position relative to the dock container.
    mouse_x: f64,
    mouse_y: f64,
    /// Offset from the tile's top-left corner to the mouse position.
    offset_x: f64,
    offset_y: f64,
    /// Starting mouse position for distance threshold check.
    start_x: f64,
    start_y: f64,
    /// Whether the drag has actually started (mouse moved enough).
    active: bool,
}

/// NeXTSTEP-style dock that shows minimized windows as 64x64 tiles.
/// The dock sits in the upper-left corner of the viewport with a fixed
/// system icon anchor. Tiles snap to a 2D grid adjacent to existing tiles.
#[component]
pub(super) fn Dock() -> impl IntoView {
    let wm = expect_context::<WindowManagerContext>();
    let dock_layout = RwSignal::new(DockLayout::new());
    let dock_drag = RwSignal::new(None::<DockDrag>);
    let show_settings = RwSignal::new(false);
    // Position (x, y) for the shield context menu, or None if hidden.
    let context_menu = RwSignal::new(None::<(i32, i32)>);

    // Load dock layout from localStorage after hydration
    #[cfg(feature = "hydrate")]
    {
        let dock = dock_layout;
        Effect::new(move |first: Option<bool>| {
            let _ = dock.get(); // track for saves
            if first == Some(true) {
                save_dock_layout(&dock.get_untracked());
                return true;
            }
            dock.set(load_dock_layout());
            true
        });
    }

    // Sync dock tiles with minimized windows:
    // - Add tiles for newly minimized windows
    // - Remove tiles for windows that are no longer minimized
    // Also push windows away from the dock area when tiles appear.
    let wm_sync = wm.clone();
    Effect::new(move |_| {
        let wins = wm_sync.windows.get();
        let mut changed = false;
        dock_layout.update(|layout| {
            // Add tiles for minimized windows not yet in dock
            for w in &wins {
                if w.minimized && layout.get_pos(w.id).is_none() {
                    let pos = layout.next_available_pos();
                    layout.set_pos(w.id, pos);
                    changed = true;
                }
            }
            // Remove tiles for windows that are no longer minimized (or removed)
            let before = layout.tiles.len();
            let minimized_ids: Vec<WindowId> =
                wins.iter().filter(|w| w.minimized).map(|w| w.id).collect();
            layout.tiles.retain(|(id, _)| minimized_ids.contains(id));
            if layout.tiles.len() != before {
                changed = true;
            }
        });

        // Push non-minimized windows out of the dock area
        if changed {
            let layout = dock_layout.get_untracked();
            let (dock_w, dock_h) = layout.bounds_px();
            // Also account for the system icon minimum
            let dock_w = dock_w.max(DOCK_TILE_SIZE);
            let dock_h = dock_h.max(DOCK_TILE_SIZE);
            wm_sync.push_windows_from_dock(dock_w, dock_h);
        }
    });

    // Derive minimized windows list with their dock positions
    let dock_tiles = move || {
        let layout = dock_layout.get();
        let wins = wm.windows.get();
        let drag = dock_drag.get();
        let mut tiles: Vec<(WindowId, String, &'static str, DockPos)> = vec![];
        for w in &wins {
            if w.minimized {
                if let Some(pos) = layout.get_pos(w.id) {
                    // Skip the tile being actively dragged (it's rendered separately)
                    if drag
                        .as_ref()
                        .is_some_and(|d| d.window_id == w.id && d.active)
                    {
                        continue;
                    }
                    let label = w.title.as_deref().unwrap_or(w.id.dock_label());
                    let label = if label.len() > 8 {
                        format!("{}...", &label[..6])
                    } else {
                        label.to_string()
                    };
                    tiles.push((w.id, label, w.id.dock_icon(), pos));
                }
            }
        }
        tiles
    };

    // Ghost tile position (snap preview during active drag)
    let ghost_pos = move || {
        let drag = dock_drag.get()?;
        if !drag.active {
            return None;
        }
        let layout = dock_layout.get_untracked();
        let snap_x = drag.mouse_x - drag.offset_x + DOCK_TILE_SIZE / 2.0;
        let snap_y = drag.mouse_y - drag.offset_y + DOCK_TILE_SIZE / 2.0;
        // Create a temporary layout without the dragged tile for snapping
        let mut temp_layout = layout.clone();
        temp_layout.remove(drag.window_id);
        temp_layout.snap_to_grid(snap_x, snap_y)
    };

    // Dragged tile visual (follows mouse, only when active)
    let drag_tile_info = move || {
        let drag = dock_drag.get()?;
        if !drag.active {
            return None;
        }
        let wins = wm.windows.get();
        let w = wins.iter().find(|w| w.id == drag.window_id)?;
        let label = w.title.as_deref().unwrap_or(w.id.dock_label());
        let label = if label.len() > 8 {
            format!("{}...", &label[..6])
        } else {
            label.to_string()
        };
        let x = drag.mouse_x - drag.offset_x;
        let y = drag.mouse_y - drag.offset_y;
        Some((drag.window_id, label, w.id.dock_icon(), x, y))
    };

    // Compute dock area bounds for the container size
    let dock_bounds = move || {
        let layout = dock_layout.get();
        let wins = wm.windows.get();
        let has_minimized = wins.iter().any(|w| w.minimized);
        if !has_minimized {
            (DOCK_TILE_SIZE, DOCK_TILE_SIZE)
        } else {
            let (w, h) = layout.bounds_px();
            (w.max(DOCK_TILE_SIZE), h.max(DOCK_TILE_SIZE))
        }
    };

    // Minimum distance (px) mouse must move before drag activates
    const DRAG_THRESHOLD: f64 = 5.0;

    // Mouse handlers for dock tile drag
    let on_dock_mousemove = move |ev: leptos::ev::MouseEvent| {
        if dock_drag.get_untracked().is_some() {
            ev.prevent_default();
            dock_drag.update(|d| {
                if let Some(d) = d {
                    d.mouse_x = ev.offset_x() as f64;
                    d.mouse_y = ev.offset_y() as f64;
                    // Activate drag once mouse moves past threshold
                    if !d.active {
                        let dx = d.mouse_x - d.start_x;
                        let dy = d.mouse_y - d.start_y;
                        if (dx * dx + dy * dy).sqrt() >= DRAG_THRESHOLD {
                            d.active = true;
                        }
                    }
                }
            });
        }
    };

    let wm_mouseup = wm.clone();
    let on_dock_mouseup = move |_: leptos::ev::MouseEvent| {
        if let Some(drag) = dock_drag.get_untracked() {
            if drag.active {
                // Complete the drag — snap tile to new position
                let snap_x = drag.mouse_x - drag.offset_x + DOCK_TILE_SIZE / 2.0;
                let snap_y = drag.mouse_y - drag.offset_y + DOCK_TILE_SIZE / 2.0;
                dock_layout.update(|layout| {
                    let mut temp = layout.clone();
                    temp.remove(drag.window_id);
                    if let Some(new_pos) = temp.snap_to_grid(snap_x, snap_y) {
                        layout.set_pos(drag.window_id, new_pos);
                    }
                });
            } else {
                // Short click — restore the window
                wm_mouseup.restore_window(drag.window_id);
            }
            dock_drag.set(None);
        }
    };

    let on_dock_mouseleave = move |_: leptos::ev::MouseEvent| {
        // Cancel drag if mouse leaves the dock area
        dock_drag.set(None);
    };

    view! {
        <div
            class="dock"
            style=move || {
                let (w, h) = dock_bounds();
                // Expand during active drag to allow snapping to new positions
                let drag_extra = if dock_drag.get().is_some_and(|d| d.active) {
                    DOCK_TILE_SIZE * 2.0
                } else {
                    0.0
                };
                format!(
                    "width:{}px;height:{}px;",
                    w + drag_extra,
                    h + drag_extra
                )
            }
            on:mousemove=on_dock_mousemove
            on:mouseup=on_dock_mouseup
            on:mouseleave=on_dock_mouseleave
        >
            // System icon (anchor tile at 0,0)
            <div
                class="dock-tile dock-tile-system"
                data-tooltip="Settings"
                on:click=move |_| show_settings.set(true)
                on:contextmenu=move |ev: leptos::ev::MouseEvent| {
                    ev.prevent_default();
                    context_menu.set(Some((ev.client_x(), ev.client_y())));
                }
                style="cursor: pointer;"
            >
                <span class="dock-tile-icon">{"\u{1f6e1}"}</span>
                <span class="dock-tile-label">"WebRPG"</span>
            </div>

            // Minimized window tiles
            <For
                each=dock_tiles
                key=|t| t.0
                let:tile
            >
                {
                    let id = tile.0;
                    let label = tile.1.clone();
                    let icon = tile.2;
                    let pos = tile.3;
                    let left = pos.col as f64 * DOCK_TILE_SIZE;
                    let top = pos.row as f64 * DOCK_TILE_SIZE;
                    view! {
                        <div
                            class="dock-tile"
                            style=format!("left:{}px;top:{}px;", left, top)
                            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                                ev.prevent_default();
                                let tile_left = pos.col as f64 * DOCK_TILE_SIZE;
                                let tile_top = pos.row as f64 * DOCK_TILE_SIZE;
                                let mx = ev.offset_x() as f64 + tile_left;
                                let my = ev.offset_y() as f64 + tile_top;
                                dock_drag.set(Some(DockDrag {
                                    window_id: id,
                                    mouse_x: mx,
                                    mouse_y: my,
                                    offset_x: ev.offset_x() as f64,
                                    offset_y: ev.offset_y() as f64,
                                    start_x: mx,
                                    start_y: my,
                                    active: false,
                                }));
                            }
                            title=id.title()
                        >
                            <span class="dock-tile-icon">{icon}</span>
                            <span class="dock-tile-label">{label}</span>
                        </div>
                    }
                }
            </For>

            // Ghost tile (snap preview during drag)
            {move || {
                ghost_pos().map(|gp| {
                    let left = gp.col as f64 * DOCK_TILE_SIZE;
                    let top = gp.row as f64 * DOCK_TILE_SIZE;
                    view! {
                        <div
                            class="dock-tile dock-tile-ghost"
                            style=format!("left:{}px;top:{}px;", left, top)
                        />
                    }
                })
            }}

            // Dragged tile (follows mouse)
            {move || {
                drag_tile_info().map(|(_, label, icon, x, y)| {
                    view! {
                        <div
                            class="dock-tile dock-tile-dragging"
                            style=format!("left:{}px;top:{}px;", x, y)
                        >
                            <span class="dock-tile-icon">{icon}</span>
                            <span class="dock-tile-label">{label}</span>
                        </div>
                    }
                })
            }}
        </div>

        // Settings dialog
        {move || {
            show_settings.get().then(|| {
                view! {
                    <SettingsDialog on_close=move || show_settings.set(false) />
                }
            })
        }}

        // Shield context menu
        {move || {
            let wm_ctx = wm.clone();
            context_menu.get().map(|(x, y)| {
                view! {
                    <div class="dock-context-backdrop" on:click=move |_| context_menu.set(None) on:contextmenu=move |ev: leptos::ev::MouseEvent| { ev.prevent_default(); context_menu.set(None); }>
                        <div
                            class="dock-context-menu"
                            style=format!("left:{}px;top:{}px;", x, y)
                            on:click:stopPropagation=|_: leptos::ev::MouseEvent| {}
                        >
                            <button class="dock-context-item" on:click=move |_| {
                                wm_ctx.minimize_all();
                                context_menu.set(None);
                            }>"Return All Windows"</button>
                        </div>
                    </div>
                }
            })
        }}
    }
}
