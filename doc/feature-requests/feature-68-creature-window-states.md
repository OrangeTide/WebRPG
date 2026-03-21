# Feature 68: Creature Window List/Edit States

The Creature window should have two clean states: listing creatures, and editing one creature. When editing, the creature list at the bottom should not be shown. Also, remove the duplicate button from the list view — it should only appear in the editor toolbar.

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

- `src/components/creatures.rs` renders both the editor and the `<For>` list simultaneously
- The `editing` signal controls which creature is being edited
- The duplicate button was added to both the card list and the editor toolbar — list-view copy should be removed
