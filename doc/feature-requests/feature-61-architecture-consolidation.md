# Feature 61: Architecture Consolidation

Holistic review and refactoring of token/character/creature window interactions.

## Description

Review Map View, Initiative Tracker, Creatures window, Character Selection, and Character Sheet windows. Handle states consistently across all windows.

Key principles to consolidate:
- Tokens have a 1:1 mapping to characters
- Creatures window spawns instances of a creature type (parent class / template)
- Each creature token has a 1:1 mapping to a creature instance
- Creature window items are parent classes/templates for instances

Once a clearer picture of the architecture is reached, refactor to consolidate these ideas.

## Dependencies

- Token instance refactoring (2026-03-20) laid groundwork — universal token_instances, unique creature labels, cascade deletes
- Feature 63 (Armies Window) — natural extension of this architecture
- Feature 60 (Database V1 Rewrite) — may be combined

## Status: Not Started

## Plan

(none yet)

## Findings

- Token instances now universal for both character and creature tokens (2026-03-20 refactor)
- Creature tokens get unique labels ("Wolf", "Wolf 2", etc.) server-side
- Cascade deletes prevent orphaned records
- `token_instances` has both `character_id` and `creature_id` (nullable)
