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
- Map render effect receives fog data but only draws it as semi-transparent overlay for GM
- No fog editing tools or player-side culling exist
