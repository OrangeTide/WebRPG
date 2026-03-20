# Feature 44: Layer Toolbar

Create a layers toolbar on the map viewer to switch and hide/show layers. Right-click a layer icon to toggle its visibility. Left-click to switch the active (editing) layer. The toolbar is a compact grid: GM sees 2 rows, players see 1 row. Each layer button shows a single-letter icon; the tooltip provides the full name. Full descriptions go in the help file.

## Layer Definitions

| Index | Icon | Name             | Description                                                                                                    | Player Visible | Player Editable |
|-------|------|------------------|----------------------------------------------------------------------------------------------------------------|----------------|-----------------|
| 0     | G    | Ground           | Background image layer. In the future, "Set Background" will set this layer's image. Multiple stacked images with alpha blending planned. | Yes (can hide)  | No              |
| 1     | A    | GM Tokens A      | GM-placed background map items: doors, rugs, furniture details, etc.                                           | No             | No              |
| 2     | R    | Ridable Tokens   | Furniture, chairs, rideable tokens (horses, carts). Players can place tokens here.                             | Yes (can hide)  | Yes             |
| 3     | B    | GM Tokens B      | Typically for GM's creatures.                                                                                  | No             | No              |
| 4     | P    | Player Tokens    | Default active layer. Players place their tokens here.                                                         | Yes (can hide)  | Yes             |
| 5     | C    | GM Tokens C      | Creatures with multiple parts that tower over the player (e.g. heads of a hydra).                              | No             | No              |
| 6     | T    | Top Layer        | Roof, treetops, canopy. Player can toggle visibility but cannot place tokens.                                  | Yes (can hide)  | No              |
| 7     | D    | GM Layer D       | Roof layers that cannot be altered by the player. Future: players will not be able to move tokens under obscured parts of this layer (requires client-side alpha mask detection). | No             | No              |
| 8     |      | (reserved)       | Reserved for future use. Not shown in the toolbar — exists only to preserve the even/odd GM/player alternation convention in the numbering. | —              | —               |
| 9     | F    | Fog of War       | Image/composite-based fog layer with smooth edges and arbitrary shapes. Needs a redesign from the current cell-based system (see Design Notes). GM only. | No             | No              |

## Toolbar Layout

The toolbar is a grid pattern. GM sees both rows; players see only the bottom row. Layer 8 (reserved) has no button.

```
GM row:     [A] [B] [C] [D] [F]
Player row: [G] [R] [P] [T]
```

- Left-click a layer icon to switch the active editing layer.
- Right-click a layer icon to toggle that layer's visibility on/off.
- Players can only switch to layers they have edit permission on (R, P).
- Players can toggle visibility on layers marked "can hide" (G, R, P, T).
- GM has full access to every layer: switch, hide/show, and edit all.
- No keyboard shortcuts for layers in this feature — shortcuts will be planned holistically across all map viewer tools in a future pass.
- Layers render bottom-to-top by index (0 = Ground at the bottom, 9 = Fog of War on top).

## Design Notes

### Token placement and the active layer
When a token is placed (via the map canvas or the "Place on Map" button in Creatures), it lands on the **active layer**. The server must validate that the user has edit permission on the target layer — if not, reject the placement. The `PlaceToken` and `MoveToken`/`MoveTokens` messages will need a `layer` field (or the server infers it from the token's existing layer for moves). The `TokenInfo` DTO also needs a `layer` field so the client knows which layer to render each token on.

### Token storage
The `tokens` table needs a `layer INTEGER NOT NULL DEFAULT 4` column. The `PlaceToken` client message needs a `layer` field. `TokenInfo` DTO needs `pub layer: i32`. Server-side placement handlers must validate layer permissions before inserting.

### Fog of War redesign
The current fog system is cell-based (`fog_of_war` table stores `(map_id, x, y)` pairs rendered as black grid rectangles). Layer 9 implies an image/composite-based fog with smooth edges and arbitrary shapes. This is a significant redesign that needs its own scoping — the current cell-based system will need to be replaced or supplemented with a texture/mask approach. Consider: canvas compositing with a fog texture, brush-based reveal/hide tools, and how to store the fog mask (e.g. PNG blob in VFS vs. vector paths in DB).

### Hit testing
Layers imply that click-to-select should be layer-aware (e.g. clicking a player token on layer 4 shouldn't accidentally select a door on layer 1 underneath it). This requires a more sophisticated hit-testing architecture than the current flat token list scan. Needs further scoping on feasibility before implementation.

### Migration strategy
Existing tokens with no layer field default to layer 4 (Player Tokens) via `DEFAULT 4` in the migration. No attempt to reclassify existing tokens — GM can manually move them after upgrade.

### Layer visibility
Layer visibility is temporary client-side state (not persisted). Defaults are loaded on each connection and reset on scene change.

## Future Notes

- "Set Background" button will set the active layer's image (layer 0 initially).
- Multiple stacked images per layer with alpha blending.
- GM Layer D (7) will enforce movement restrictions under obscured areas via client-side alpha mask detection.

## Dependencies

- **Feature 32: Map Viewport Panning** — provides the canvas rendering and tool palette infrastructure that layers will integrate with

## Status: Not Started

## Plan

(none yet)

## Findings

No layer support exists in the codebase:
- The `tokens` table in `src/schema.rs` has no `layer` column.
- No layer toolbar UI, layer visibility state, or layer switching logic in `src/components/map.rs`.
- No `layer` field in `TokenInfo` DTO or `PlaceToken`/`MoveTokens` messages.
- The prerequisite Feature 32 (Map Viewport Panning) is complete, so this feature is unblocked.
