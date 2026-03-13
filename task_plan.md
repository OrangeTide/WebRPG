# Task Plan: Phase 8 — Multi-Window UI

## Goal
Replace the fixed sidebar game layout with a draggable, resizable, multi-window UI where each game feature (map, chat, character sheet, etc.) lives in its own window that players can arrange freely.

## Current Phase
Phase 6

## Phases

### Phase 1: WindowManager + GameWindow Components
- [x] Create `src/components/window_manager.rs` with `WindowState`, `WindowId`, `WindowManager`, and `GameWindow` components
- [x] `WindowState` struct: id, title, x, y, width, height, z_index, minimized, visible
- [x] `WindowManager`: holds `RwSignal<Vec<WindowState>>`, provides context, renders children + taskbar
- [x] `GameWindow`: title bar (drag handle, minimize/close buttons), resizable body, z-index stacking
- [x] Drag via mousedown on title bar → mousemove → mouseup (hydrate-only, using web-sys)
- [x] Resize via mousedown on edges/corners → mousemove → mouseup
- [x] Click-to-front z-index behavior
- [x] Add `pub mod window_manager;` to `src/components/mod.rs`
- **Status:** complete

### Phase 2: Integrate into Game Page
- [x] Refactor `GamePage` to use `WindowManager` instead of the fixed sidebar layout
- [x] Wrap each component (MapCanvas, ChatPanel, CharacterSheet, InitiativeTracker, InventoryPanel, CreaturePanel) in `GameWindow`
- [x] Define default window positions/sizes for each window
- [x] Add toolbar/menu bar in game header to toggle window visibility
- [ ] GM role check: CreaturePanel window only visible to GM (deferred — needs `is_gm` in GameContext)
- **Status:** complete

### Phase 3: Minimize Dock + Taskbar
- [x] Minimized windows collapse to a taskbar at the bottom of the game viewport
- [x] Clicking a minimized window in the taskbar restores it
- [x] Taskbar shows window title for each minimized window
- **Status:** complete (built into WindowManager Phase 1)

### Phase 4: CSS Styling
- [x] Style window chrome: title bar, buttons, borders, shadows
- [x] Style resize handles (cursor changes on hover)
- [x] Style taskbar/dock
- [x] Match existing dark theme (#1a1a2e, #16213e, #0f3460, #e94560)
- [x] Ensure existing component styles work inside windows (flex fill rule added)
- [x] Removed old sidebar-specific CSS (chat-panel border-bottom)
- **Status:** complete

### Phase 5: localStorage Persistence
- [x] Save window layout to `localStorage` on every state change (via reactive Effect)
- [x] Load layout from `localStorage` on page load, falling back to defaults
- [x] Handle version mismatches (merge stored + defaults, drop unknown IDs)
- [x] Added `Storage` feature to web-sys in Cargo.toml
- **Status:** complete

### Phase 6: Testing + Verification
- [x] `cargo check --features ssr` — no errors, no warnings
- [x] `cargo check --features hydrate --target wasm32-unknown-unknown` — no errors
- [x] `cargo test` — all pass
- [x] Manual browser testing at 1280x800:
  - [x] Windows render with correct positions, titles, borders, shadows
  - [x] Minimize button hides window, adds to taskbar
  - [x] Taskbar restore button brings window back
  - [x] Close button hides window, toolbar button deactivates
  - [x] Toolbar toggle reopens closed window
  - [x] Drag moves window to new position
  - [x] Z-index stacking: interacted window comes to front
  - [x] localStorage persistence: dragged position survives toggle
  - [x] All 6 window types render their component content correctly
- [x] Update PLAN.md Phase 8 checklist
- **Status:** complete

## Key Questions
1. Should double-click on title bar maximize/restore? (PLAN.md says yes)
2. Should we enforce minimum window sizes per window type? (Yes — map needs more space than initiative)
3. Do we need `is_gm` signal in GameContext for GM-only window filtering? (Check if it exists already)
4. What web-sys features are needed beyond what's already imported? (PointerEvent may be better than MouseEvent for drag)

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Implement drag/resize from scratch with web-sys | No Leptos drag library available; web-sys MouseEvent already imported |
| Use signals for window state, not DOM manipulation | Leptos reactive paradigm; state-driven rendering |
| Window manager provides context (like GameContext does) | Child components need to send window commands (close, minimize) |
| Use PointerEvent + setPointerCapture for drag | More reliable than MouseEvent — captures events even when cursor leaves element |

## Errors Encountered
| Error | Attempt | Resolution |
|-------|---------|------------|
|       | 1       |            |

## Notes
- The existing `.game-layout`, `.game-main`, `.game-sidebar` CSS will be replaced/removed
- Components currently use flex sizing from their parent — they'll need to fill their GameWindow body instead
- Media browser is already a modal overlay — it stays as-is, not a window
- MapCanvas needs special treatment: it sizes its canvas to its container, so it must respond to window resize
