# Feature 3: Window Manager Dock

Rewrite the WindowToggleToolbar to track minimized windows as draggable tile icons that lock to a Dock area.

The dock area begins in the upper-left corner of the game page. Tiles can be
dragged and dropped in this area and they will snap to an any adjacent tile on
the grid. The dock always has a fixed icon for the system icon in the
upper-left corner and it behaves like other tiles but cannot be dragged or
moved away from the dock. It acts as a starting point for docking the tile
icons.

Tiles are all 64x64 pixel squares.

Theme the tiles with a gray background and transparent icon in the center with
a white label at the bottom. Use black and width edges to give the tiles a 3D
look. See @mockup-dock1.png for how this should look.

## Status: Not Started

## Plan

1. Remove `WindowToggleToolbar` component and related CSS (`.wm-toolbar`, `.wm-toolbar-btn`)
2. Remove `toggle_window()` and `close_window()` from `WindowManagerContext` for static windows — static windows can only be minimized, never closed
3. Remove the close button ("×") from `GameWindow` title bar for static windows; keep it only for dynamic windows (`CharacterEditor`)
4. Remove `visible` field from `WindowState` for static windows — they are always either restored or minimized
5. Remove the existing `.wm-taskbar` (replaced by dock)
6. Implement `Dock` component in upper-left corner of `.wm-viewport` with high z-index
7. Add fixed system icon as the dock anchor at position (0,0) — cannot be dragged
8. Minimized windows appear as 64x64 tile icons in the dock
9. Theme tiles with gray background, transparent icon, white label, and 3D beveled edges (see mockup-dock1.png)
10. Implement drag-and-drop for tiles that snap to adjacent grid positions
11. Clicking a dock tile restores the window to its previous position
12. Persist dock tile layout to localStorage

## Findings

### Current Architecture
- `WindowToggleToolbar` (game.rs:457–506): text buttons in game header toggling window visibility
- `WindowManagerContext` methods: `close_window()`, `minimize_window()`, `restore_window()`, `toggle_window()`
- `WindowState` has both `visible` and `minimized` fields (3 states: visible, minimized, closed)
- Existing taskbar (window_manager.rs:693–740): bottom bar showing buttons for minimized windows
- CSS classes: `.wm-toolbar`, `.wm-toolbar-btn`, `.wm-taskbar`, `.wm-taskbar-btn`
- Dynamic windows (`CharacterEditor`): created via `open_character_editor()`, removed entirely on close
- Static windows persist across sessions via localStorage (`webrpg_window_layout`)

### Design Decisions
- Dynamic windows (CharacterEditor) can be closed (removed entirely)
- Static/persistent windows can only be minimized (never closed) — no `visible` flag needed for them
- Dock orientation: vertical by default, but tiles can be dragged to any 2D grid position (snap to adjacent tiles)
- Dock z-index: dock always floats above windows; windows cannot be moved underneath the dock area; when new dock icons appear they push overlapping windows away
- Tile icons: support Unicode characters, inline SVG, or PNG URLs per WindowId
- System icon: knight's shield (🛡️ or inline SVG)
- Grid snapping: full 2D grid snapping from day one (no simplified vertical-only phase)
