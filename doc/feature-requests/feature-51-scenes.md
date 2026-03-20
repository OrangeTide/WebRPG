# Feature 51: Scenes

Scene management for saving and restoring map + token configurations.

## Description

Each scene holds the settings for a map and token positions. Switching to a different scene saves the current scene and loads the selected one.

- **Scene entity**: Associates a map with a set of token positions, fog state, and camera position
- **Scene list**: GM can create, rename, delete, and switch between scenes
- **Save/restore**: Switching scenes saves the current token layout and restores the target scene's layout
- **GM-only or all-players**: When the GM selects a scene, it can be for all players or for only the GM's view
- **Future**: GM may be able to set a different scene for each player (not yet designed)

Scenes are distinct from maps — a single map image could be used in multiple scenes with different token/fog configurations (e.g., "before the battle" vs "during the battle").

## Dependencies

- **Feature 47: Fog of War UI** — scenes save/restore fog state
- Map management (create/switch/delete) is already implemented

## Status: Not Started

Long-term feature.

## Plan

(none yet)

## Findings

(none yet)
