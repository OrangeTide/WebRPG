# Feature 47: Fog of War UI & Visibility

Basic fog-of-war system with GM editing tools and player visibility filtering.

## Description

The database schema already has a `fog_of_war` table with `(map_id, x, y)` cells, and the server loads fog data. What's missing is:

- **GM fog editing UI**: Toolbar to paint/erase fog cells on the map canvas. Brush modes: reveal, hide, toggle. Possibly rectangle select to reveal/hide regions.
- **GM visibility toggle**: Quick toggle to show/hide fog overlay while editing (so GM can see the full map while working).
- **Player-side visibility**: Tokens and map areas covered by fog should be hidden from non-GM players. The server already sends fog data per session — the client needs to render it as an opaque overlay for players.
- **Tokens invisible to player**: GM sets per-token visibility. Tokens marked as hidden are not rendered for non-GM players (but shown with reduced opacity for the GM).

## Dependencies

- **Feature 32: Map Viewport Panning** — Done. Fog rendering uses the same viewport transform system.

## Status: Not Started

Database schema exists. Server loads fog data. No UI or player-side filtering.

## Plan

(none yet)

## Findings

- `fog_of_war` table exists in schema with `(id, map_id, x, y)` columns
- `GameContext.fog` signal exists and is populated from server

### Audit (2026-10-29)

More of the player-facing half exists than the earlier findings implied. The
GM editing UI is the real gap.

- Player-side fog overlay renders on the map canvas
  (`components/map.rs:602-616`), and tokens flagged hidden are culled for
  non-GM players (`components/map.rs:445`, `:449`).
- The server already handles `RevealFog` and `HideFog` requests and broadcasts
  the result (`server/ws_handler/mod.rs:439-480`).
- The missing piece is the GM paint/erase UI: no client code ever *sends*
  `RevealFog` or `HideFog`, and there is no brush toolbar or
  reveal/hide/toggle control. Wiring a GM fog tool onto the existing server
  handlers is the bulk of the remaining work.
