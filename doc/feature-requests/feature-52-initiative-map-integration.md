# Feature 52: Initiative Map Integration

Highlight the active initiative token on the map and auto-center the viewport.

## Description

When the initiative tracker advances to a new turn:

- **Map highlighting**: The token belonging to the current initiative entry gets a distinct visual highlight (pulsing glow, colored ring, or similar) on the map canvas
- **Auto-center**: The map viewport automatically pans to center on the active token (with smooth animation)
- **Optional**: Player preference to disable auto-center (some players prefer manual camera control)

This bridges the initiative tracker and map components so players can immediately see whose turn it is spatially.

## Dependencies

- **Feature 32: Map Viewport Panning** — Done. Uses center-on-token and viewport control.
- Initiative tracker already tracks `is_current_turn` per entry

## Status: Not Started

## Plan

(none yet)

## Findings

- `is_current_turn` flag exists on initiative entries (server-side)
- `center_on_token()` function already exists in map.rs for the token list dropdown
- Initiative entries have optional `character_id` and `creature_id` that could link to tokens
- Tokens have `character_id` and `creature_id` fields for matching
