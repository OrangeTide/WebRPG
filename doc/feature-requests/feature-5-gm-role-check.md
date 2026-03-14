# Feature 5: GM role check

GM role check: Creatures window only visible to GM (needs `is_gm` in GameContext)

## Status: In Progress

`is_gm()` exists server-side in `ws_handler.rs` and is used for permission
checks. However, `is_gm` is not exposed in `GameContext`, and the Creatures
window is not visibility-gated on the client side — any user can toggle it
visible.

## Plan

TBD

## Findings

- `is_gm()` function exists in `src/server/ws_handler.rs` and is used for 10+
  server-side permission checks (SetMapBackground, PlaceToken, etc.)
- `GameContext` in `pages/game.rs` does not include an `is_gm` field
- Creatures window is rendered unconditionally in `game.rs` — default layout
  sets `visible: false` but any user can toggle it visible
