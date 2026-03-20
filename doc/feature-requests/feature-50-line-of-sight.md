# Feature 50: Line of Sight

Automatic fog of war based on token position and wall geometry.

## Description

Long-term feature. Compute dynamic visibility from a token's position using wall/obstacle definitions:

- **Wall placement tool**: GM draws walls on the map (along grid edges or freeform)
- **LoS computation**: Raycasting or shadow-casting from each player's token position
- **Dynamic fog**: Areas not visible from the player's token are fogged
- **Explored fog**: Previously seen areas shown dimmed (explored but not currently visible)
- **Door entities**: Walls that can be opened/closed, changing visibility dynamically

This is a significant feature that builds on basic fog of war (Feature 47) and the drawing/annotation layer (Feature 49).

## Dependencies

- **Feature 47: Fog of War UI** — basic fog system needed first
- **Feature 49: Drawing & Annotation Tools** — wall drawing tools share infrastructure
- **Feature 46: Visibility Classes** — LoS interacts with vision types (darkvision range, etc.)

## Status: Not Started

Long-term feature requiring significant design work.

## Plan

(none yet)

## Findings

(none yet)
