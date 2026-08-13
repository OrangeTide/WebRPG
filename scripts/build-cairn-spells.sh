#!/usr/bin/env bash
# Generate the Cairn spellbook module's JSON from its spell list.
#
# modules/cairn-spellbooks/spells.txt is the source of truth: one spell per
# line, "key|name|description". This writes two files from it:
#
#   tables.json  the d666 spell table, for rolling up a found spellbook
#   items.json   one spellbook item card per spell, for dealing to a character
#
# Keeping both generated means the list is written once and the two views of it
# cannot drift.
#
# PUBLIC DOMAIN (CC0-1.0)
set -euo pipefail

MODULE_DIR="${MODULE_DIR:-modules/cairn-spellbooks}"
SRC="$MODULE_DIR/spells.txt"

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found. Install jq." >&2
    exit 1
fi

if [ ! -f "$SRC" ]; then
    echo "error: no spell list at $SRC" >&2
    exit 1
fi

# Drop comments and blank lines, then split each line on the first two pipes.
parse() {
    grep -v '^#' "$SRC" | grep -v '^[[:space:]]*$' \
        | jq -R -s 'split("\n")
            | map(select(length > 0))
            | map(split("|"))
            | map(select(length == 3))
            | map({key: .[0], name: .[1], text: .[2]})'
}

count=$(parse | jq 'length')
if [ "$count" -eq 0 ]; then
    echo "error: no spells parsed from $SRC" >&2
    exit 1
fi

parse | jq '[{
    id: "spells",
    name: "Spellbooks (d666)",
    die: "d666",
    description: "Roll three d6 in order for the spell in a found spellbook. Cairn spells are one spellbook each: reading it casts it.",
    entries: map({key: .key, text: (.name + ". " + .text)})
}]' > "$MODULE_DIR/tables.json"

# The slug keeps punctuation out of ids while staying readable.
parse | jq 'map({
    id: ("spellbook-" + (.name | ascii_downcase | gsub("[^a-z0-9]+"; "-") | sub("^-"; "") | sub("-$"; ""))),
    name: ("Spellbook: " + .name),
    kind: "spellbook",
    slots: 1,
    bonus: .text,
    note: ("d666 " + .key + ". A spellbook takes one slot. Reading it casts the spell.")
})' > "$MODULE_DIR/items.json"

echo "Wrote $count spells to $MODULE_DIR/tables.json and $MODULE_DIR/items.json" >&2
