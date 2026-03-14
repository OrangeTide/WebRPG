# Feature 4: Macro Bar

Add a macro button bar to the game page.

The buttons can be assigned by the user to a number of actions, have a configurable text label,
and optional icon.

Clicking or tapping triggers the action.

Right clicking or long press opens the configuration window. Where an action type and arguments can be assigned.

Initially place this bar at the bottom of the game page. Provide a small grab area on each side of the bar to allow the user to move the bar. Do not permit the window to move completely off the screen, it must always be possible to click on the grab area of the bar.

One item at the end of the button bar is always a plus (+) sign. Clicking this will create a new button and expand the bar. After 10 buttons are created, a second row a buttons begin. Maximum number of buttons is 30 (3 rows).
Minimum size of the button bar is one button (for the plus sign button).

Assignable Actions include:

 * Rolling dice. A dice expression is configurable during action creation.
 * Opening/closing a set window. Target window is configurable during action creation. (inventory, character sheet, etc.)
 * Running a javascript expression.
 * print a message to chat
 * center the map on a set token.

## Dependencies

- **Feature 32: Map Viewport Panning** — Required for the "center map on token" macro action. This action can be deferred until Feature 32 is implemented.

## Status: Not Started

## Plan

### Phase 1: Data Model & Persistence

1. **Database migration** — `diesel migration generate create_user_macros`
   - Table `user_macros`: `id INTEGER PRIMARY KEY`, `user_id INTEGER NOT NULL REFERENCES users(id)`, `session_id INTEGER NOT NULL REFERENCES sessions(id)`, `slot INTEGER NOT NULL` (0–29), `label TEXT NOT NULL`, `icon_url TEXT`, `action_type TEXT NOT NULL` (one of: `dice`, `toggle_window`, `javascript`, `chat`, `center_token`), `action_data TEXT NOT NULL` (JSON-encoded action arguments)
   - Unique constraint on `(user_id, session_id, slot)`
   - Macros are per-user, per-session so different games can have different bars

2. **Diesel models** in `src/models.rs`
   - Shared DTO: `MacroInfo { slot, label, icon_url, action_type, action_data }`
   - Server-only: `UserMacro` (Queryable) and `NewUserMacro` (Insertable)

3. **Server functions** (Leptos `#[server]`)
   - `get_macros(session_id) -> Vec<MacroInfo>` — load all macros for current user + session
   - `save_macro(session_id, MacroInfo) -> ()` — upsert a single macro by slot
   - `delete_macro(session_id, slot) -> ()` — remove a macro

### Phase 2: Macro Bar Component (`src/components/macro_bar.rs`)

4. **MacroBar component**
   - Renders a positioned `div.macro-bar` at the bottom of the game viewport
   - Grab handles on left and right edges (`.macro-bar-grip`) for dragging
   - Loads macros from server on mount via `get_macros`, stores in `RwSignal<Vec<MacroInfo>>`
   - Lays out buttons in rows of 10 (grid or flex-wrap), up to 3 rows
   - Final button is always "+" — clicking it opens the config dialog with the next free slot
   - Bar position stored in `RwSignal<(f64, f64)>`, persisted to localStorage (`webrpg_macro_bar_pos`)

5. **MacroButton sub-component**
   - Displays label (and optional icon via `<img>` if `icon_url` is set)
   - `on:click` → execute the macro action
   - `on:contextmenu` (+ long-press via `on:touchstart`/`on:touchend` timer) → open config dialog for this slot
   - Tooltip showing action summary on hover

6. **Drag behavior** for the bar
   - Reuse the same mousedown/mousemove/mouseup pattern from `window_manager.rs`
   - On mousedown on `.macro-bar-grip`: record offset, set `dragging` signal
   - On mousemove: update position, clamping so grips stay on-screen
   - On mouseup: clear dragging, save position to localStorage

### Phase 3: Action Execution

7. **Action dispatch function** `execute_macro(macro_info: &MacroInfo, ctx: &GameContext, wm: &WindowManagerContext)`
   - Match on `action_type`:
     - `"dice"` → `ctx.send_message(ClientMessage::RollDice { expression: action_data })`
     - `"toggle_window"` → parse `action_data` as `WindowId` name, call `wm.toggle_window(id)`
     - `"javascript"` → call `js_sys::eval(&action_data)` (gated behind `#[cfg(feature = "hydrate")]`)
     - `"chat"` → `ctx.send_message(ClientMessage::ChatMessage { message: action_data })`
     - `"center_token"` → parse `action_data` as token_id, find token in `ctx.tokens`, scroll/pan map to its position (requires adding a scroll signal or method to the map component)

### Phase 4: Configuration Dialog

8. **MacroConfigDialog component**
   - Modal overlay (similar to existing dialog patterns in the codebase)
   - Fields: Label (text input), Icon URL (optional text input), Action Type (dropdown select), Action Data (context-dependent input)
   - Action-specific inputs:
     - `dice`: text field for expression (e.g. "2d6+3"), with validation via `is_dice_roll()`
     - `toggle_window`: dropdown populated from `WindowId` variants
     - `javascript`: textarea for JS code
     - `chat`: text field for the message
     - `center_token`: dropdown populated from current `ctx.tokens` list
   - Save button → calls `save_macro` server function, updates local signal
   - Delete button → calls `delete_macro`, removes from local signal, reflows remaining buttons

### Phase 5: Integration & Styling

9. **Wire into game page** (`src/pages/game.rs`)
   - Add `<MacroBar/>` inside the `.wm-viewport` div, after the existing window manager content
   - Register the new component module in `src/components/mod.rs`

10. **CSS** (`style/main.css`)
    - `.macro-bar`: `position: absolute`, `bottom: 40px`, `left: 50%`, `transform: translateX(-50%)`, dark panel background, rounded corners, flex-wrap layout, `z-index` above windows but below modals
    - `.macro-bar-grip`: narrow vertical strip on each side, `cursor: grab`, subtle visual indicator (dots or lines)
    - `.macro-btn`: consistent with existing button styling but slightly larger for touch targets (~36px min), truncated text
    - `.macro-btn-add`: "+" button styled distinctly (dashed border or accent color)
    - `.macro-config-dialog`: modal overlay with form styling matching existing theme
    - Responsive: On small screens, reduce button size or max columns

### Phase 6: Center-on-Token Support

11. **Map scrolling** — the map component currently renders at full size without pan/scroll support for centering. Add:
    - A `scroll_to_token` signal or callback in `GameContext` that the map component listens to
    - When triggered, calculate the token's pixel position and scroll the map container (or adjust a viewport transform) to center it
    - This is the most architecturally involved action and can be deferred to a follow-up if needed

### Implementation Order

Start with Phases 1–2 to get the bar rendering with localStorage-only persistence, then Phase 3 for action execution, Phase 4 for the config dialog, and Phase 5 for polish. Phase 6 (center-on-token) can ship later since it requires map viewport changes.

## Findings

### Relevant Files
- **Game page**: `src/pages/game.rs` — `GameContext` struct, `WindowToggleToolbar`, `apply_server_message()`
- **Window manager**: `src/components/window_manager.rs` — `WindowId` enum, `WindowManagerContext` (toggle/close/open methods), drag/resize system via `DragOp`
- **Chat**: `src/components/chat.rs` — `ChatPanel`, `is_dice_roll()` for dice pattern detection
- **Map**: `src/components/map.rs` — `MapCanvas`, canvas rendering, token drawing
- **Models**: `src/models.rs` — shared DTOs + SSR-only Diesel structs pattern
- **Messages**: `src/ws/messages.rs` — `ClientMessage`/`ServerMessage` enums
- **WS handler**: `src/server/ws_handler.rs` — `parse_and_roll()`, message processing
- **Schema**: `src/schema.rs` — auto-generated Diesel table definitions
- **Styling**: `style/main.css` — dark theme, `.wm-toolbar` buttons, `.gw` window positioning

### Key Patterns to Follow
- **Feature gating**: Server-only code behind `#[cfg(feature = "ssr")]`, imports inside `#[server]` functions
- **Drag behavior**: mousedown records offset, mousemove updates position with clamping, mouseup clears state — pattern from `window_manager.rs`
- **Database**: Queryable/Insertable derives, `get_conn()` for DB pool access
- **WebSocket messaging**: `ctx.send_message(ClientMessage::Variant { ... })` from components
- **Window toggling**: `wm.toggle_window(WindowId::Variant)` from `WindowManagerContext`
- **localStorage**: Used for window layout persistence (`webrpg_window_layout` key pattern)
