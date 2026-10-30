# Feature 58: Creatures Undo Button

Undo button on the Creatures table to reverse the last action.

## Description

Add an undo button on the Creatures table to undo the last action. Supported actions to undo:

- Delete
- Field change
- Rename
- Place on map
- Roll initiative

## Dependencies

None.

## Status: Phase 1 done (table-local actions)

## Plan

Phase 1 covers the three table-local actions: delete, field change, and rename.
Place on map and roll initiative reach into the map and initiative subsystems and
are deferred to a later phase.

- Add a client-side undo stack in `CreaturePanel` holding `CreatureUndo` entries
  (`Deleted(CreatureInfo)` and `Updated { id, name, stat_data }`).
- Delete pushes a `Deleted` snapshot before calling the server. Field change and
  rename both go through `update_creature`, so the editor pushes one `Updated`
  entry (the pre-edit name and stats) on each successful save, skipping no-op
  saves.
- An undo button in the panel header, shown only when the stack is non-empty,
  pops the last entry and applies the inverse: recreate the deleted creature, or
  write the previous name and stats back. Recreating a creature gives it a new
  id, so the handler remaps later stack entries from the old id to the new one.

## Findings

- Creature CRUD is GM-local via server functions, not broadcast over the socket,
  so the undo stack is a local, GM-only concern.
- Undoing a delete restores the creature record (name, stats, image) but not any
  map tokens or initiative entries it had, since the server cascades those away
  on delete. Restoring them belongs with the deferred place-on-map undo.
- Fixed a pre-existing bug found while testing: the list card delete button used
  the `on:click:stopPropagation` modifier, which did not stop the parent card's
  click handler, so clicking delete opened the editor instead of deleting. The
  explicit `ev.stop_propagation()` form fixes it. The same modifier is used in
  `file_browser.rs` and may have the same issue; left for a separate change.
