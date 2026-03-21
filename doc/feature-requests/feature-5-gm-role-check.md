# Feature 5: GM role check

GM role check: Creatures window only visible to GM (needs `is_gm` in GameContext)

## Status: Done

## Implementation

- `is_gm` was already present in `GameContext` and populated from `GameStateSnapshot`
- Wrapped the Creatures `<GameWindow>` in `<Show when=is_gm>` so it only renders for GMs
- Filtered Creatures from the Settings startup checklist for non-GM users
- Non-GMs cannot see the Creatures window or its dock tile

## Related

- FR73: Multiple GMs per session (future)
