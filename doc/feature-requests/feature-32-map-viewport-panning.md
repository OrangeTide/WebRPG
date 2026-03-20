# Feature 32: Map Viewport Panning

Add viewport panning to map. Allow users to pan/scroll the map view to navigate large maps. This enables centering on specific tokens or areas and improves usability for maps larger than the visible window area.

## Status: Done

## Plan

(none yet)

## Findings

All described functionality is implemented in `src/components/map.rs` and supporting server/message files:

- **Viewport pan/zoom**: `view_offset` and `view_zoom` signals with `screen_to_world`/`world_to_screen` coordinate transforms.
- **Tool system**: `MapTool` enum with `Select`, `Pan`, `Measure`, `Ping` variants. Keyboard shortcuts: S=Select, M=Measure, space=Pan, P=Ping, G=snap toggle, T=token list.
- **Tool palette UI**: `map-tool-palette` div with buttons for each tool, snap-to-grid toggle, and ping color picker.
- **Area selection / multi-select**: `selected_ids` HashSet, `selection_rect` for rubber-band selection.
- **Multi-drag**: `drag_token_origins` for dragging multiple selected tokens simultaneously.
- **Measurement tool**: `measure_start`, `measure_end`, `measure_cursor` signals for distance measurement.
- **Token rotation**: `rotation` column in schema (`tokens.rotation -> Float`); `RotateTokens` client message; rotation rendering via canvas `rotate()`.
- **Token conditions**: `conditions_json` column in schema; `UpdateTokenConditions` message; condition icons rendered on tokens; condition picker in token popup.
- **Token list dropdown**: `show_token_list` signal toggled via T key or toolbar button.
- **Grid snap toggle**: `snap_to_grid` signal toggled via G key or toolbar button; applied during drag/placement.
- **ResizeObserver**: Used in `map.rs` for canvas resize handling.
- **Map management**: `create_map`, `delete_map`, `list_maps` server functions in `src/server/api.rs`; map list dropdown, create/delete UI in map component.
- **Ping tool**: `MapTool::Ping` variant; `ClientMessage::Ping { x, y }` and `ClientMessage::SetPingColor`; ping color picker in toolbar.
- **Character placement**: `PlaceToken` with `character_id` field; `PlaceAllPlayerTokens` message handled in `src/server/ws_handler.rs` and triggered from `src/components/charsheet.rs`.
- **WS messages**: `MoveTokens`, `TokensMoved`, `PlaceToken` (with `character_id`), `PlaceAllPlayerTokens`, `RotateTokens`, `UpdateTokenConditions`, `Ping` all present in `src/ws/messages.rs`.
