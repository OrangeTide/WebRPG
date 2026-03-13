# Progress: Phase 8 — Multi-Window UI

## Session Log

### 2026-03-13 — Planning
- Explored codebase: no existing window manager code
- Read all 7 game components, game.rs, full CSS (944 lines)
- Created task_plan.md with 6 phases
- Created findings.md with component inventory and architecture notes
- Ready to begin Phase 1: WindowManager + GameWindow components

### 2026-03-13 — Phase 1: WindowManager + GameWindow
- Created `src/components/window_manager.rs` (~330 lines)
  - `WindowId` enum (Map, Chat, CharacterSheet, Initiative, Inventory, Creatures) with titles and min sizes
  - `WindowState` struct with position, size, z-index, minimized, visible
  - `WindowManagerContext` with all window operations: bring_to_front, close, minimize, restore, toggle, drag move, resize (8 edges/corners)
  - `default_window_layout()` with sensible defaults for each window
  - `WindowManager` component: viewport overlay for mouse capture, taskbar for minimized windows
  - `GameWindow` component: title bar with drag, resize handles, minimize/close buttons, body for children
- Key design: children rendered once (FnOnce), visibility via CSS `display:none` instead of conditional DOM removal
- Fixed: clone issue with mouseleave handler, removed unused `is_shown` variable
- All checks pass: `cargo check --features ssr`, `cargo check --features hydrate --target wasm32-unknown-unknown`, `cargo test`
- Added `pub mod window_manager;` to `src/components/mod.rs`

### 2026-03-13 — Phases 2-5: Integration, CSS, localStorage
- Refactored `src/pages/game.rs`: replaced fixed sidebar layout with `WindowManager` + 6 `GameWindow` wrappers
- Added `WindowToggleToolbar` and `WindowToggleButton` components to game header
- Replaced old `.game-layout`/`.game-main`/`.game-sidebar` CSS with window manager styles:
  - `.wm-viewport` — full viewport overlay for mouse capture
  - `.gw` — absolute-positioned window with border, shadow, rounded corners
  - `.gw-titlebar` — drag handle with grab cursor
  - `.gw-btn`/`.gw-btn-close`/`.gw-btn-min` — window control buttons
  - `.gw-resize-*` — 8 resize handles (4 edges + 4 corners) with appropriate cursors
  - `.wm-taskbar` — bottom dock for minimized windows (auto-hides when empty)
  - `.wm-toolbar` — header toolbar buttons to toggle window visibility
  - `.gw-body > *` — flex fill rule for components inside windows
- Added `Storage` feature to web-sys in Cargo.toml
- Added localStorage persistence: load on init (merge with defaults), save via reactive Effect
- Removed old `.chat-panel` border-bottom (sidebar separator)
- All checks pass: ssr, hydrate, cargo test

## Files Modified
- src/components/window_manager.rs (created)
- src/components/mod.rs (added window_manager module)
- src/pages/game.rs (refactored layout, added toolbar components)
- style/main.css (replaced game layout styles with window manager styles)
- Cargo.toml (added Storage to web-sys features)
- PLAN.md (updated Phase 8 checklist)

### 2026-03-13 — Phase 6: Browser Testing
- Tested at 1280x800 in Firefox via WebDriver
- All 6 windows render correctly: Map, Chat, Character Sheet, Initiative, Inventory, Creatures
- Minimize → taskbar → restore cycle works
- Close → toolbar toggle → reopen works
- Drag repositions window correctly, z-index brings to front
- localStorage persists positions across window operations
- Components render correctly inside windows (inputs, buttons, forms all functional)
- Screenshots saved to testing/ directory

## Remaining
- GM role check for Creatures window (needs `is_gm` in GameContext)
- Responsive default positions for common screen sizes
- Resize interaction not tested via WebDriver (hard to simulate edge-grab)
