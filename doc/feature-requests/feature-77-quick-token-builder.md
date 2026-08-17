# Feature 77: Quick Token Builder

Make a plain labelled token in two clicks, from the map toolbar, without
browsing an icon library.

## Description

Most tokens on a map are not creatures. They are the door, the bookcase, the
rock, the barrel, the thing the party is about to lever open. Those need a
name and a place on the grid, and nothing else.

The pieces already exist: tokens carry a label, a colour, a size, and an
optional image. What is missing is a fast path to them. Placing scenery today
means going through the flow built for creatures and characters, and the
comparable feature elsewhere (D&D Beyond) buries it under icon browsing, which
is slower than typing the word "door".

The ask:

- **On the map toolbar.** One button beside the existing tools, not inside a
  dialog reached from somewhere else.
- **Type a label, click a square, done.** The label is the token. A colour and
  a size (defaulting to 1 inch, one grid square) are the only other controls.
- **An icon is optional.** If someone wants a glyph, offer a short list of
  common ones. Never require picking one.
- **Repeatable.** Placing six barrels should not mean retyping "barrel" six
  times: after placing, the tool stays armed with the same label so the next
  click drops another.

## Who can use it

Players and the GM both, by default. Marking "we barricaded this door" is
exactly the sort of thing a player should be able to do without asking.

The existing permissions still decide: if the map is locked for editing, or the
active layer is not one players may place on, then players cannot. The tool
follows those rules rather than introducing its own.

## Related

- **Feature 44: Layer Toolbar** — which layer a quick token lands on. Scenery
  belongs on a furniture or ground-item layer, not among the player tokens.
- **Feature 49: Drawing & Annotation Tools** — the other half of letting a
  table build out a map at the table.
- **Feature 53: Map Settings Window** — where the map's edit lock lives.

## Dependencies

None strictly, though it lands better after Feature 44 decides which layer
receives scenery.

## Status: Not Started

## Plan

(none yet)

## Findings

- Tokens already carry label, colour, size, visibility, rotation, and an
  optional image URL, so this needs no schema change: it is a faster way to
  create what the token model already supports.
- `PlaceToken` takes an optional `character_id` and `creature_id`; a scenery
  token simply leaves both unset, which the protocol already allows.
