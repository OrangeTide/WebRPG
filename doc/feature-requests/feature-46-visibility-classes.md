# Feature 46: Visibility Classes

Per-player token/area visibility based on character vision types and light conditions. RPG-template-configurable visibility classes (e.g. 5e: bright/dim/darkness/magical darkness × normal/darkvision/truesight/devil's sight, plus invisibility).

## Description

The map should support multiple visibility layers that interact to determine what each player can see. Visibility is a function of two axes:

### Light Conditions (applied to map areas)
Areas of the map can have different light levels. In D&D 5e (2024 PHB), these are:
- **Bright Light** — normal visibility for all vision types
- **Dim Light** — lightly obscured; Perception checks at disadvantage for normal vision
- **Darkness** — heavily obscured; effectively blind for normal vision
- **Magical Darkness** — heavily obscured; blocks even darkvision (only Truesight/Devil's Sight penetrate)

### Vision Types (applied to characters/creatures)
Each character or creature has one or more vision capabilities. In D&D 5e:
- **Normal Vision** — sees in bright light, dim light is lightly obscured, darkness is heavily obscured
- **Darkvision** — treats darkness as dim light and dim light as bright light (within range), but cannot see color in darkness. Does NOT penetrate magical darkness.
- **Truesight** — sees in normal and magical darkness, sees invisible creatures, sees into the Ethereal Plane, sees through illusions, perceives true form of shapechangers/transformed creatures
- **Devil's Sight** — sees normally in darkness AND magical darkness (within range), but does not grant other Truesight benefits

### Invisibility (applied to creatures/objects)
Invisibility is a condition on a creature or object, not a map area property. An invisible creature:
- Cannot be seen by normal vision or darkvision
- CAN be seen by Truesight
- Is still detectable by other means (sound, tracks, etc.) — but that's game logic, not map rendering

### How it works on the map
- The GM paints light condition zones on the map (like fog of war, but for light levels)
- Each player's view is computed from their character's vision type(s) intersected with the light conditions
- Tokens with the "invisible" condition are hidden from players whose characters lack Truesight
- The GM always sees everything (with visual indicators of light zones)

### Template configurability
The light conditions and vision types are defined by the RPG template, not hardcoded. The template specifies:
- Available light conditions (names, severity/ordering)
- Available vision types (names, what light conditions they can see through)
- The visibility matrix: for each (vision_type, light_condition) pair, the rendering effect (clear / dim / hidden)
- Available visibility-affecting conditions (e.g. invisibility) and which vision types bypass them

This allows non-D&D systems to define their own visibility rules.

### References
- D&D Beyond PHB 2024 — Obscured Areas: https://www.dndbeyond.com/sources/dnd/phb-2024/playing-the-game#ObscuredAreas
- D&D Beyond PHB 2024 — Truesight: https://www.dndbeyond.com/sources/dnd/phb-2024/rules-glossary#Truesight

## Dependencies

- **Feature 44: Layer Toolbar** — light condition zones are painted on a dedicated map layer
- **Feature 38: Token Conditions** — invisibility condition on tokens feeds into visibility computation
- Template system — vision types need to be defined per-character in the RPG template fields

## Status: Not Started

Long-term feature. Complex interaction model requiring template schema extensions, map layer painting tools, per-player server-side view computation, and client rendering effects.

## Plan

(none yet)

## Findings

(none yet)
