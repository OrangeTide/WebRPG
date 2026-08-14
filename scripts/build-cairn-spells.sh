#!/usr/bin/env bash
# Generate the Cairn spellbook module's JSON from its spell lists.
#
# Two sources, both "key|name|description" per line:
#
#   spells-core.txt       the 100 spells from the Cairn 2e Warden's Guide,
#                         rolled as d100. This is the primary list.
#   spells-community.txt  216 community-contributed spells of uneven quality,
#                         rolled as d666. Secondary, kept for variety.
#
# Written out as:
#
#   tables.json  both tables, core first
#   items.json   one spellbook card per spell. Core wording wins where a name
#                appears in both lists, so a spell has exactly one card.
#
# Keeping these generated means each list is written once and the table and the
# cards cannot drift apart.
#
# PUBLIC DOMAIN (CC0-1.0)
set -euo pipefail

MODULE_DIR="${MODULE_DIR:-modules/cairn-spellbooks}"
CORE="$MODULE_DIR/spells-core.txt"
COMMUNITY="$MODULE_DIR/spells-community.txt"

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found. Install jq." >&2
    exit 1
fi

for f in "$CORE" "$COMMUNITY"; do
    if [ ! -f "$f" ]; then
        echo "error: no spell list at $f" >&2
        exit 1
    fi
done

# Drop comments and blank lines, then split each line on the first two pipes.
parse() {
    grep -v '^#' "$1" | grep -v '^[[:space:]]*$' \
        | jq -R -s 'split("\n")
            | map(select(length > 0))
            | map(split("|"))
            | map(select(length == 3))
            | map({key: .[0], name: .[1], text: .[2]})'
}

core=$(parse "$CORE")
community=$(parse "$COMMUNITY")

core_count=$(jq 'length' <<<"$core")
community_count=$(jq 'length' <<<"$community")
if [ "$core_count" -eq 0 ] || [ "$community_count" -eq 0 ]; then
    echo "error: a spell list parsed as empty" >&2
    exit 1
fi

jq -n --argjson core "$core" --argjson community "$community" '[
    {
        id: "spells-core",
        name: "Spellbooks (d100)",
        die: "d100",
        description: "The core spell list. Roll for the spell in a found spellbook. Cairn spells are one spellbook each: reading it casts it.",
        entries: ($core | map({key: .key, text: (.name + ". " + .text)}))
    },
    {
        id: "spells-community",
        name: "More spellbooks (d666)",
        die: "d666",
        description: "A larger community-contributed list, of uneven quality. Roll three d6 in order. Use it when you want a spell the table will not recognise; skim what you roll before handing it over.",
        entries: ($community | map({key: .key, text: (.name + ". " + .text)}))
    }
]' > "$MODULE_DIR/tables.json"

# One card per spell name. Core entries are emitted first and win ties, so a
# spell in both lists gets the Warden's Guide wording rather than a variant.
jq -n --argjson core "$core" --argjson community "$community" '
    def slug: ascii_downcase | gsub("[^a-z0-9]+"; "-") | sub("^-"; "") | sub("-$"; "");
    ($core | map(. + {source: "core"}))
    + ($community | map(. + {source: "community"}))
    | unique_by(.name | ascii_downcase)
    | sort_by(.name | ascii_downcase)
    | map({
        id: ("spellbook-" + (.name | slug)),
        name: ("Spellbook: " + .name),
        kind: "spellbook",
        slots: 1,
        bonus: .text,
        note: (if .source == "core"
               then "d100 " + .key + ". A spellbook takes one slot. Reading it casts the spell."
               else "d666 " + .key + ", community list. A spellbook takes one slot. Reading it casts the spell."
               end)
    })' > "$MODULE_DIR/items.json"

card_count=$(jq 'length' "$MODULE_DIR/items.json")
echo "Wrote $core_count core and $community_count community spells" >&2
echo "as $card_count cards in $MODULE_DIR/items.json and 2 tables in $MODULE_DIR/tables.json" >&2
