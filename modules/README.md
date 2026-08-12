# Game modules

A module is a directory of JSON that teaches the server a game system or gives
it an adventure to run. Nothing here is compiled in: drop a directory beside
these, reload the Modules window, and it appears.

The server reads modules from `MODULES_DIR`, which defaults to this directory.
Files are read per request rather than cached, so editing a pack takes effect
without restarting.

## Two kinds

| Kind | Supplies | Bound to a session by |
|------|----------|-----------------------|
| `system` | Character sheet schema, creature schema, roll model | Installing it, which also seeds the sheet as an RPG template |
| `adventure` | Bestiary, pregens, item cards, room key, tables, maps | Installing it, which seeds creatures and maps |

An adventure names the system it was written for in `requires`. Installing it
over a different system still works, and the install report says so.

## module.json

```json
{
  "name": "Tunnel Goons",
  "version": "1.2",
  "kind": "system",
  "description": "One line for the library list.",
  "authors": ["Who wrote the game"],
  "license": "What the content is under.",
  "links": [{ "label": "Buy the rulebook", "url": "https://..." }],
  "files": { "rules": "rules.json", "sheet": "sheet.json" },
  "help": [{ "slug": "tunnel-goons", "title": "Tunnel Goons", "file": "tunnel-goons.md" }]
}
```

The directory name is the module id, whatever the file says. Ids must be
alphanumeric with dashes or underscores.

`files` entries are all optional; a module names only the parts it has.
System modules recognise `rules`, `sheet`, `creature_sheet`, and `tables`.
Adventures recognise `bestiary`, `pregens`, `items`, `rooms`, `tables`, and
`maps`.

Pages listed under `help` are served by the online help viewer at their slug, so
`[link](help:tunnel-goons)` resolves from any other help page. They live under
the module's `help/` directory.

## rules.json

The roll model. This is what the check roller on the character sheet builds
itself from, so a system with different abilities or a different die needs no
code change.

```json
{
  "dice": "2d6",
  "summary": "Shown above the roller.",
  "abilities": [{ "name": "brute", "label": "Brute", "help": "When it applies." }],
  "difficulties": [{ "label": "Easy", "value": 8 }],
  "item_bonus": 1,
  "margin_is_damage": true,
  "monster_ds_is_hp": true,
  "inventory": {
    "capacity_field": "inventory",
    "default_slots": 1,
    "over_capacity_penalty": -1,
    "penalty_applies_to": ["brute", "skulker"],
    "note": "Shown when a character is over capacity."
  }
}
```

`abilities[].name` and `inventory.capacity_field` must match field names in
`sheet.json`, because that is where the roller reads a character's scores and
carrying capacity.

## sheet.json and creature.json

A JSON array of template fields, the same shape the RPG template system already
uses:

```json
[{ "name": "brute", "label": "Brute", "type": "number", "category": "Abilities", "default": 0 }]
```

`type` is one of `number`, `text`, `boolean`, or `textarea`. A field named
`hp_max` becomes the character's HP resource bar.

## Adventure files

- **bestiary.json** — `{ id, name, ds, hp?, notes, tags }`. Seeded as creatures
  on install. A `ds` of 0 marks something that is never fought.
- **pregens.json** — `{ id, name, summary, sheet, items }`. `sheet` holds field
  values by name; a player picks one and gets a character with its gear.
- **items.json** — `{ id, name, kind, slots, bonus, note, uses? }`. Dealt into
  inventories as cards.
- **rooms.json** — `{ number, title, card, read_aloud, gm, exits }`. Only `card`
  and `read_aloud` are ever shown to players, and only when the GM deals them.
- **tables.json** — `{ id, name, die, description, entries: [{ key, text }] }`.
  An empty `die` makes it a reference table rather than a random one.
- **maps.json** — `{ name, width, height, cell_size, asset?, asset_source, notes }`.
  Art named in `asset` is ingested from the module's `assets/` directory if it
  is there; if not, the map is created without a background and the install
  report says which ones need art.

## Who sees what

The room key, bestiary, and tables come from a server function that refuses
anyone but the session's GM. Players get pregens and item cards. Keep secrets
in `gm`, not in `card`.

## Licensing

Put the content's real licence in `license`; the module browser shows it next to
the authors. If a module's art is under terms this repository cannot carry,
leave it out and say where to get it, the way `sky-blind-spire/assets/README.md`
does.
