# Feature 49: Drawing & Annotation Tools

Freehand drawing and shape annotation layer on the map canvas, usable by
players as well as the GM.

## Description

Add a drawing/annotation layer for spell effects, tactical notes, visual aids,
and player-drawn maps:

- **Freehand drawing**: Pen tool for free drawing on the map
- **Shapes**: Circles, cones, lines, rectangles for spell area-of-effect templates
- **Color picker**: Choose drawing color
- **Persistence**: Drawings saved to server and broadcast to all players
- **Clear/undo**: GM can clear all drawings or undo last stroke
- **Layer ordering**: Drawings render above the map grid but below tokens

Useful for marking spell areas (fireball radius, cone of cold), tactical plans,
and notes during play.

## Players drawing their own maps

Some adventures expect the party to map as they go, and the referee to show
them nothing. The Sky-Blind Spire is the case in hand: its whole puzzle is that
the tower's interior geometry does not add up, the GM guide explicitly says not
to hand out a floor plan or maintain a shared map, and the players are meant to
argue over their own drawing until it makes sense. That only works if the
players can draw.

What that needs beyond the annotation case above:

- **Players draw by default.** Drawing is not a GM tool with player access
  bolted on. A player opens the map and can draw, on a map with no background
  at all.
- **Whose drawing is it.** A stroke belongs to whoever drew it. At minimum a
  player can erase their own strokes without touching anyone else's. Worth
  deciding: one shared canvas the party draws on together, or a per-player
  canvas each sees only their own version of. The shared one suits a table
  arguing over a map; the private one suits players who disagree about what
  they saw. A toggle between them may be the honest answer.
- **A blank surface to draw on.** Mapping needs an empty grid that is not
  pretending to be a location, and it should not require the GM to create a map
  first.
- **It has to survive.** A map drawn over three hours and lost to a reload is
  worse than no map. Strokes persist with the session like any other state.
- **Grid snapping, optionally.** Drawing dungeon corridors on a square grid is
  much faster when lines snap to it, but freehand has to stay available for the
  bits that are not square.
- **The GM can be locked out of it.** Or at least should not be tempted: if the
  referee corrects the players' map, the puzzle stops being a puzzle.

## Related

- **Feature 44: Layer Toolbar** — the drawing layer would be managed through the
  layer system, and player-drawn maps want a layer players own.
- **Feature 72: Viewport Undo** — undo behaviour should be consistent.

## Dependencies

- **Feature 44: Layer Toolbar**

## Status: Not Started

## Plan

(none yet)

## Findings

- Movement and positioning are deliberately unenforced (see the game module
  work), so drawing is an aid rather than a rules surface. It does not need to
  interact with tokens, measurement, or fog beyond render order.
