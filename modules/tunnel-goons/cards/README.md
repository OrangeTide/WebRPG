# Card deck sources

ASCII art card sources, one `.card` file each. They render to PNGs in
`../assets/cards/`, which the item cards in an adventure module reference as
`tunnel-goons:cards/<name>.png`.

Render them with:

```sh
scripts/ascii-cards-to-png.sh
```

The script draws the frame, centres the art, wraps the text, and draws the slot
pips, so a card source only holds its content:

```
name: Torch
kind: gear
slots: 1
uses: 0
effect: +1 Skulker in the dark.
note: It burns out when the referee says so.
art:
    (  )
  (    )
 .-'---'-.
 |=======|
    |_|
```

Keep the art ASCII-only. The frame is drawn in box-drawing characters by the
script, but art containing multibyte characters throws the column arithmetic
off, and the point of these is that the columns line up.

Everything after `art:` is taken literally, so leading spaces are the drawing's
own alignment and are preserved.

## Slot cards and Knave

Tunnel Goons 1.2 does not formally use inventory cards; the deck is borrowed
from Knave, which is a common thing to borrow. It makes the encumbrance rule
that Tunnel Goons already has into something physical: over your Inventory
Score is -1 to every Brute and Skulker roll.

The condition cards (fatigue, stress, scar) are Knave-flavoured too, and are
optional. They occupy a slot each, which is the whole mechanic.

These renders are placeholders. Replace the PNGs with real art and nothing else
has to change, as long as the file names stay the same.
