# NeXTSTEP-Style Dock: Design Notes

Made by a machine. PUBLIC DOMAIN (CC0-1.0)

This document describes the design of the WebRPG dock so the approach can be
reused in other projects, including ones written in C. It is written to be
language-neutral: the concrete implementation is Leptos/Rust/WebAssembly, but
nothing here depends on that. Where the reference implementation is relevant,
it is called out; the algorithms and data model port directly.

Reference source: `src/components/window_manager/dock.rs` and the
`WindowManagerContext` in `src/components/window_manager/mod.rs`.

## 1. What it is

The dock is a small cluster of 64x64 pixel tiles anchored to the top-left corner
of the screen. Each tile represents a minimized window. Clicking a tile restores
its window. Dragging a tile relocates it, snapping to a 2D grid. One fixed
"system" tile at grid origin (0,0) acts as an anchor and a menu launcher and is
never removed.

This mimics the NeXTSTEP dock: fixed-size icon tiles, snap-to-grid arrangement,
and a persistent anchor. It differs from the macOS Dock (which is a centered,
magnifying strip) in that tiles occupy a free 2D grid the user arranges by hand.

## 2. Design goals

- **Minimized windows go somewhere visible and manipulable**, not into a hidden
  taskbar list.
- **The user controls the arrangement.** Tiles snap to a grid but the layout is
  the user's, not auto-flowed.
- **The dock never overlaps live windows.** When a tile appears, any open window
  sitting under the dock is pushed out of the way.
- **The layout survives reloads.** Grid positions persist across sessions.
- **The dock is derived state.** The set of tiles is a projection of "which
  windows are minimized," not an independent list that can drift out of sync.

That last point is the single most important architectural decision. Read
section 5 before porting.

## 3. Data model

Three small structures carry the whole feature.

### Grid position

```
struct DockPos { col: int; row: int; }   // integer grid coordinates, not pixels
```

Grid coordinates are integers. Pixels are `col * TILE_SIZE` and
`row * TILE_SIZE`. Keeping the grid in integer space (not float pixels) makes
equality, occupancy tests, and adjacency trivial and exact. Convert to pixels
only at render time.

### Layout

```
struct DockLayout { tiles: list<(WindowId, DockPos)>; }
```

A flat association list of window-id to grid position. A list (rather than a
hash map keyed by position) was chosen because:

- The tile count is tiny (single digits, rarely dozens). Linear scans are free.
- It serializes trivially.
- It preserves nothing about order that matters, so no ordering bugs.

In C, a fixed-size array of `{ id, col, row, occupied }` structs, or a small
`realloc`-grown vector, is the natural equivalent. Do not reach for a hash map.

### Drag operation

```
struct DockDrag {
    window_id;
    mouse_x, mouse_y;       // current pointer, in dock-container coordinates
    offset_x, offset_y;     // pointer offset within the grabbed tile
    start_x, start_y;       // pointer position at mousedown (for threshold)
    active: bool;           // has the drag passed the movement threshold?
}
```

One nullable instance of this exists at a time. `None`/`NULL` means "no drag in
progress." See section 6 for how `active` disambiguates click from drag.

## 4. The grid and snapping

The grid is infinite in the `+col`/`+row` direction, anchored at (0,0) where the
system tile lives. Occupancy has one special case: (0,0) is always occupied by
the system tile.

Key operations on the layout:

- `is_occupied(pos)` — true if any tile sits at `pos`, or `pos == (0,0)`.
- `has_adjacent_tile(pos)` — true if any of the four orthogonal neighbors of
  `pos` is occupied (the system tile at (0,0) counts). This is the rule that
  keeps the dock a connected blob instead of letting tiles scatter anywhere.
- `next_available_pos()` — default placement for a newly minimized window: walk
  down column 0 (row 1, 2, 3, ...) below the system tile; overflow into column 1.
- `snap_to_grid(x, y)` — given a pixel drop point, find the grid cell to land in.

### Snapping algorithm

```
snap_to_grid(x, y):
    target = ( round(x / TILE_SIZE), round(y / TILE_SIZE) )
    if target off-grid (negative): return NONE
    if target is free AND has_adjacent_tile(target): return target
    for radius in 1..=3:
        for each cell within the square ring at Chebyshev distance <= radius:
            if cell on-grid AND free AND has_adjacent_tile(cell):
                return cell
    return NONE      // no valid drop; caller leaves tile where it was
```

The expanding-radius search means a sloppy drop near the cluster still finds the
nearest legal cell. The `has_adjacent_tile` gate enforces connectivity: you can
grow the dock outward but cannot leave a floating island. A `NONE` result is a
no-op drop, the tile snaps back to its original position.

The radius cap (3) is arbitrary and cheap; it bounds the search to a 7x7
neighborhood. For a hand-arranged dock that is always more than enough.

## 5. Tiles are derived, not owned

The dock does not maintain its own list of "what is docked." The authoritative
state lives in the window manager: each window has a `minimized` boolean. The
dock's tile set is recomputed as: *for every window where `minimized == true`,
show a tile.*

A reconciliation step runs whenever the window list changes:

1. For each minimized window **not** already in the layout, assign it
   `next_available_pos()` and add it. (New minimize → new tile.)
2. Remove any layout entry whose window is no longer minimized or no longer
   exists. (Restore or close → tile disappears.)
3. If anything changed, recompute the dock's pixel bounds and push overlapping
   windows out (section 7).

The layout structure therefore stores **only positions** (the arrangement the
user chose); it is not the source of truth for membership. This split is what
prevents the classic taskbar bug where the button list and the real window list
disagree.

In the reference implementation this is a reactive effect that fires when the
window signal changes. In C with a manual event loop, call a
`dock_reconcile(dock, windows)` function at the end of any operation that
minimizes, restores, closes, or opens a window. Same logic, explicit call site.

## 6. Interaction: click vs drag

A tile must serve two gestures from the same mousedown: a **click** (restore the
window) and a **drag** (relocate the tile). They are disambiguated by a movement
threshold.

- **mousedown** on a tile records a `DockDrag` with `active = false` and captures
  the pointer offset within the tile.
- **mousemove** updates the pointer position. Once the pointer has moved
  `>= DRAG_THRESHOLD` pixels (5px) from the start point, set `active = true`.
- **mouseup**:
  - if `active` → this was a drag: snap to grid, commit the new position.
  - if not `active` → this was a click: restore the window.
- **mouseleave** on the dock cancels the drag (tile returns to origin).

The 5px threshold is what makes a tile feel like a button *and* a draggable
object without a mode switch. Below threshold it is a button; past it, an object.

### Visual feedback during drag

Three things render while `active`:

1. The **original tile** is hidden (skipped in the tile list) so it does not
   appear in two places.
2. A **dragged tile** follows the pointer at `mouse - offset`, drawn on top,
   semi-transparent, with a drop shadow. This is a free-floating copy.
3. A **ghost tile** renders at the *snap target* the tile would land in if
   dropped now, styled as a dashed outline. This is the snap preview and is what
   makes grid snapping legible to the user.

The ghost is computed by running `snap_to_grid` against a temporary copy of the
layout with the dragged tile removed (so it does not block its own landing cell).

One implementation detail: the dock container is temporarily enlarged during an
active drag (by ~2 tile widths) so the pointer can reach and snap to cells beyond
the current cluster bounds. Without this, the container clips the reachable area.

## 7. Keeping windows clear of the dock

When tiles occupy the top-left, an open window can sit underneath them. After any
change that alters the dock's footprint, compute the dock's bounding rectangle in
pixels and push offending windows out:

```
push_windows_from_dock(dock_w, dock_h):
    for each non-minimized window w:
        if w.x < dock_w AND w.y < dock_h:            // top-left corner is under dock
            push_right = dock_w - w.x
            push_down  = dock_h - w.y
            if push_right <= push_down: w.x = dock_w  // move right, shorter distance
            else:                       w.y = dock_h  // else move down
```

It pushes each window the minimum distance, right or down, to clear the dock. It
only tests the window's top-left corner, which is cheap and good enough in
practice; see limitations.

## 8. Persistence

Only the grid positions are persisted (window-id to `DockPos`), serialized to
JSON and stored under a single key. On load, the saved layout is read back;
missing or corrupt data falls back to an empty layout and positions are
reassigned by `next_available_pos()` as windows minimize.

Membership is deliberately **not** persisted, only arrangement. Which windows are
minimized is restored from the window manager's own persisted state, and
reconciliation (section 5) rebuilds the tile set from that. This avoids a stale
saved dock referencing a window that no longer exists.

Dynamic windows (ones with a runtime-allocated id, e.g. per-object editors) are
excluded from persistence because their ids are not stable across sessions.

In C, serialize the position list to whatever format the project already uses.
The structure is a flat list of `(stable_id, col, row)` triples.

## 9. Visual design

- Fixed **64x64** tiles. No magnification, no reflow. Predictable hit targets.
- Beveled borders (light top/left, dark bottom/right) give the raised NeXTSTEP
  look; the bevel inverts on `:active` to read as "pressed."
- Each tile shows a large glyph icon plus a short truncated text label.
- The container uses `pointer-events: none` while individual tiles re-enable
  `pointer-events: auto`, so the dead space between/around tiles does not eat
  clicks meant for windows behind it. (A UI-toolkit-specific trick; in an
  immediate-mode or custom C UI, just hit-test the tile rects directly.)

## 10. Lessons learned

- **Derive the tile set from window state; never maintain it in parallel.** This
  removed an entire class of desync bugs. The dock became a pure view of
  `minimized == true`. If you take one idea from this document, take this one.

- **Store the grid in integers, convert to pixels only at the edge.** Occupancy,
  adjacency, and equality are exact and obvious. Every bug we might have had from
  float rounding in grid math simply never existed.

- **A movement threshold cleanly separates click from drag.** No long-press, no
  modifier key, no separate drag handle. One gesture stream, split by distance.

- **The ghost/snap-preview is worth the extra render.** Snap-to-grid without a
  preview feels random; with the preview the user sees exactly where a drop
  lands and the grid becomes self-explanatory.

- **A connectivity rule (`has_adjacent_tile`) keeps the layout coherent** with
  almost no code. It turns "free 2D placement" (which looks messy) into "grow a
  connected cluster" (which looks intentional) without constraining the user to a
  single row or column.

- **Small-N means simple data structures win.** A flat list beats a hash map here
  on every axis that matters: less code, trivial serialization, no ordering
  surprises, and the linear scans are unmeasurable.

## 11. Limitations

- **Overlap test is corner-only.** `push_windows_from_dock` checks a window's
  top-left corner against the dock rectangle. A window whose top-left is outside
  the dock but whose body still overlaps a protruding tile is not pushed. Fine in
  practice; fix by testing full rectangle intersection if it matters.

- **The grid is unbounded and unaware of screen size.** Nothing stops the dock
  from growing off-screen if enough windows are minimized. There is no wrapping
  or scrolling. Bound the grid to viewport dimensions if you need it.

- **No tile reordering/insertion.** Dropping a tile onto an occupied cell does not
  displace the occupant; the snap search just finds the nearest *free* cell. A
  true insert-and-shift would need a different algorithm.

- **Fixed anchor position.** The dock is hard-anchored top-left. Moving it to
  another corner means changing the growth direction of `next_available_pos` and
  the sign conventions in the snap search. It is not user-relocatable.

- **`offset_x/offset_y` from the toolkit's event coordinates.** The reference
  code reads pointer offset from DOM event fields. In a C UI you must compute the
  pointer position relative to the dock container yourself; the coordinate space
  matters (all of `mouse_x/y`, `offset`, and snap math must share one origin).

- **Label truncation is naive.** Labels are cut to a fixed character count with an
  ellipsis. No measurement of actual rendered width. Adequate for short glyph +
  label tiles; revisit for proportional fonts or i18n.

## 12. Porting checklist for a C project

1. Define `DockPos { int col, row; }` and a small growable list of
   `{ stable_window_id, DockPos }`.
2. Implement `is_occupied`, `has_adjacent_tile`, `next_available_pos`,
   `snap_to_grid`, and `bounds_px` as plain functions over that list. They are
   pure and easy to unit-test.
3. Keep `minimized` on your window struct as the source of truth. Write
   `dock_reconcile(dock, windows)` and call it after every minimize / restore /
   close / open.
4. Handle the three pointer events (down/move/up) with the 5px threshold to split
   click from drag. Cancel on pointer-leave.
5. Render three things during a drag: the floating tile, the ghost at the snap
   target, and everything else statically. Hide the original.
6. After reconcile, run the push-out pass so live windows never hide under tiles.
7. Serialize only the position list; rebuild membership from window state on load.
