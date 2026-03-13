# Findings: Phase 8 — Multi-Window UI

## Current Game Layout
- `pages/game.rs` renders a fixed two-column layout: `.game-main` (MapCanvas + CharacterSheet) and `.game-sidebar` (Chat, Initiative, Inventory, Creatures)
- Layout is 100vh with flexbox, sidebar is fixed 320px width
- No existing window_manager.rs or draggable/resizable code

## Component Inventory (to be wrapped in windows)
| Component | File | Default Visibility | Notes |
|-----------|------|-------------------|-------|
| MapCanvas | components/map.rs (17K) | Always open, largest | Canvas-based, sizes to container |
| ChatPanel | components/chat.rs (2.9K) | Always open | Flex column, needs min-height |
| CharacterSheet | components/charsheet.rs (14K) | Open for players | Scrollable, variable height |
| InitiativeTracker | components/initiative.rs (4.6K) | Open when active | Small, compact |
| InventoryPanel | components/inventory.rs (3.6K) | Starts minimized | Medium size |
| CreaturePanel | components/creatures.rs (8.8K) | GM only | Medium-large |
| MediaBrowser | components/media_browser.rs (11K) | Opens as modal | Stays as modal overlay, NOT a window |

## Existing web-sys Features Available
MouseEvent, HtmlElement, Element, Window, Document, DomRect — all already imported. Need to add:
- `PointerEvent` for setPointerCapture (better drag behavior)
- Possibly `CssStyleDeclaration` for cursor changes

## GameContext Structure
- Provided via `provide_context` in GamePage
- Contains all reactive signals for game state
- `send` function wrapped in `StoredValue<_, LocalStorage>` for WASM compatibility
- No `is_gm` field currently — will need to check how GM status is determined

## CSS Theme Colors (for window chrome styling)
- Panel bg: `#16213e`
- Dark bg: `#0d1b30`
- Body bg: `#1a1a2e`
- Border: `#0f3460`
- Border hover: `#1a4a7a`
- Accent: `#e94560`
- Text: `#eee`
- Muted text: `#aaa`
- Shadow style from media browser: `box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6)`

## WinBox.js Evaluation
Considered WinBox.js (~10KB, no-dep JS window manager). Rejected because:
- Imperative DOM management conflicts with Leptos declarative rendering
- Two sources of truth (WinBox internal state vs Leptos signals)
- Would require wasm-bindgen interop for every method/callback
- Adds JS build dependency to pure-Rust project
- Our needs are simple enough to implement natively in Leptos (~200-300 lines)

## Architecture Decisions from PLAN.md
- WindowState: id, title, x, y, width, height, z_index, minimized, visible
- Title bar: drag to move, double-click to maximize/restore
- Edges/corners: drag to resize with minimum size enforced
- Close hides (can reopen from menu/dock), minimize collapses to taskbar
- GM-only windows not rendered for non-GM players
- Persist layout to localStorage
