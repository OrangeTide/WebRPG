#!/usr/bin/env bash
# Render ASCII art inventory cards as PNGs, sized for use as card art in game.
#
# Two input shapes are accepted:
#
#   *.card   A card source: `key: value` headers, then `art:` and the drawing.
#            The script draws the frame, so the art does not have to be padded
#            by hand and every card comes out the same size.
#
#   *.md     Markdown with ```text blocks, each block rendered as-is. The card
#            name comes from a heading inside the block ("[ITEM: SWORD]" or
#            "* ENCHANTED RING *"), or from the prose line above the fence.
#
# Requires ImageMagick and a monospace font with box-drawing glyphs.
#
# PUBLIC DOMAIN (CC0-1.0)
set -euo pipefail

IN_DIR="${IN_DIR:-modules/tunnel-goons/cards}"
OUT_DIR="${OUT_DIR:-modules/tunnel-goons/assets/cards}"
# Rendered width in pixels. Cards land near this, give or take a pixel from
# the aspect-preserving resize.
WIDTH="${WIDTH:-512}"
# Point size to rasterise at before the resize. Larger is sharper and slower.
POINTSIZE="${POINTSIZE:-28}"
FONT="${FONT:-/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf}"
BG="${BG:-#12101a}"
FG="${FG:-#e8e2d0}"
PAD="${PAD:-18}"
# Interior width of a framed card, in characters.
CARD_WIDTH="${CARD_WIDTH:-23}"

usage() {
    cat <<EOF
Usage: $0 [file.card|file.md ...]

With no arguments, every .card and .md in $IN_DIR is rendered.

Environment overrides:
  IN_DIR=$IN_DIR
  OUT_DIR=$OUT_DIR
  WIDTH=$WIDTH        output width in pixels
  POINTSIZE=$POINTSIZE       rasterisation size before resize
  CARD_WIDTH=$CARD_WIDTH      interior width of a framed card, in characters
  FONT=$FONT
  BG=$BG  FG=$FG  PAD=$PAD

Examples:
  $0                                    # the Tunnel Goons card deck
  IN_DIR=REF/inv-cards OUT_DIR=/tmp/c \$0   # the raw reference art
EOF
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

if ! command -v convert >/dev/null 2>&1; then
    echo "error: ImageMagick 'convert' not found. Install imagemagick." >&2
    exit 1
fi

if [ ! -f "$FONT" ]; then
    echo "error: font not found: $FONT" >&2
    echo "Set FONT=/path/to/a/monospace.ttf (needs box-drawing glyphs)." >&2
    exit 1
fi

# Turn a card heading into a file name: lowercase, punctuation to dashes.
slugify() {
    printf '%s' "$1" \
        | tr '[:upper:]' '[:lower:]' \
        | sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//' \
        | cut -c1-48
}

# Build the framed text of a .card file on stdout.
#
# Headers are `key: value` up to the `art:` line; everything after it is the
# drawing. The frame, the centring, and the slot pips are drawn here so that
# card sources stay easy to edit.
frame_card() {
    awk -v w="$CARD_WIDTH" '
        function pad(s,   n) {
            n = w - length(s)
            if (n < 0) { s = substr(s, 1, w); n = 0 }
            return s sprintf("%*s", n, "")
        }
        function row(s) { printf "│%s│\n", pad(" " s) }
        function centre(s,   n) {
            n = int((w - length(s)) / 2)
            if (n < 0) n = 0
            return sprintf("%*s", n, "") s
        }
        function rule(   i, s) {
            s = ""
            for (i = 0; i < w; i++) s = s "─"
            return s
        }
        function wrap(s,   words, i, line, out) {
            # Fit prose to the card without splitting words.
            split(s, words, " ")
            line = ""
            for (i = 1; i <= length(words); i++) {
                if (line == "") { line = words[i] }
                else if (length(line) + 1 + length(words[i]) <= w - 2) {
                    line = line " " words[i]
                } else { row(line); line = words[i] }
            }
            if (line != "") row(line)
        }
        function pips(n,   i, s) {
            s = ""
            for (i = 0; i < n; i++) s = s "[*]"
            if (n == 0) s = "none"
            return s
        }
        function boxes(n,   i, s) {
            s = ""
            for (i = 0; i < n; i++) s = s "[ ]"
            return s
        }

        BEGIN { inart = 0; slots = 1; uses = 0 }

        !inart && /^art:[[:space:]]*$/ {
            inart = 1
            printf "┌%s┐\n", rule()
            row(toupper(name))
            if (kind != "") row(toupper(kind))
            row("SLOTS: " pips(slots))
            printf "├%s┤\n", rule()
            next
        }
        !inart && /^[a-z_]+:/ {
            key = $0
            sub(/:.*/, "", key)
            val = $0
            sub(/^[a-z_]+:[[:space:]]*/, "", val)
            if (key == "name") name = val
            else if (key == "kind") kind = val
            else if (key == "slots") slots = val + 0
            else if (key == "uses") uses = val + 0
            else if (key == "effect") effect = val
            else if (key == "note") note = val
            next
        }
        inart {
            art[++artn] = $0
            next
        }

        END {
            # Trim blank lines off both ends of the drawing.
            first = 1; last = artn
            while (first <= last && art[first] ~ /^[[:space:]]*$/) first++
            while (last >= first && art[last] ~ /^[[:space:]]*$/) last--

            # Centre the drawing as a block, so its own alignment survives.
            wide = 0
            for (i = first; i <= last; i++) if (length(art[i]) > wide) wide = length(art[i])
            indent = int((w - wide) / 2) - 1
            if (indent < 0) indent = 0

            row("")
            for (i = first; i <= last; i++) row(sprintf("%*s", indent, "") art[i])
            row("")

            printf "├%s┤\n", rule()
            if (effect != "") wrap(effect)
            if (note != "") wrap(note)
            if (uses > 0) row("USES: " boxes(uses))
            printf "└%s┘\n", rule()
        }
    ' "$1"
}

# Render one block of text to one PNG.
render() {
    local block="$1" out="$2" text
    # The text goes in as an argument rather than as label:@file, because
    # ImageMagick's default policy refuses to read text from a file.
    #
    # Two things in the art would otherwise be read as markup rather than
    # drawn: a backslash starts an escape sequence, so "/  \" loses its right
    # hand side, and % starts a format specifier. Double both.
    text=$(sed -e 's/\\/\\\\/g' -e 's/%/%%/g' "$block")

    # label: keeps the text exactly as written, one glyph cell per character,
    # which is what makes the box-drawing line up.
    convert \
        -background "$BG" \
        -fill "$FG" \
        -font "$FONT" \
        -pointsize "$POINTSIZE" \
        "label:$text" \
        -bordercolor "$BG" -border "$PAD" \
        -resize "${WIDTH}x" \
        "$out"
}

files=("$@")
if [ ${#files[@]} -eq 0 ]; then
    # README.md documents the deck rather than holding cards.
    mapfile -t files < <(
        find "$IN_DIR" -maxdepth 1 \( -name '*.card' -o -name '*.md' \) \
            ! -name 'README.md' | sort
    )
fi

if [ ${#files[@]} -eq 0 ]; then
    echo "error: no .card or .md input found in $IN_DIR" >&2
    exit 1
fi

mkdir -p "$OUT_DIR"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

count=0
for src in "${files[@]}"; do
    [ -f "$src" ] || { echo "skipping missing $src" >&2; continue; }
    base=$(basename "$src")
    base=${base%.*}

    case "$src" in
    *.card)
        block="$work/$base.txt"
        frame_card "$src" > "$block"
        # Named after the source file, not the card's title, so that references
        # like "tunnel-goons:cards/ring.png" stay put when a title is reworded.
        out="$OUT_DIR/$base.png"
        render "$block" "$out"
        count=$((count + 1))
        echo "$out"
        ;;
    *)
        # Split the markdown into blocks. awk writes each fenced block to its
        # own file so the PNG step never has to think about markdown. The last
        # line of prose before a fence is kept beside it, since raw art is
        # often labelled that way ("dungeon torch:") rather than inside the box.
        awk -v dir="$work" -v base="$base" '
            /^```/ {
                if (inblock) { inblock = 0; close(out); next }
                inblock = 1; n++
                out = sprintf("%s/%s-%03d.txt", dir, base, n)
                if (caption != "") {
                    print caption > (out ".name")
                    close(out ".name")
                }
                caption = ""
                next
            }
            inblock { print > out; next }
            /[^[:space:]]/ {
                line = $0
                sub(/^#+[[:space:]]*/, "", line)
                sub(/:[[:space:]]*$/, "", line)
                caption = line
            }
        ' "$src"

        for block in "$work/$base"-*.txt; do
            [ -e "$block" ] || continue
            # Blank or whitespace-only blocks render as an empty image; skip.
            grep -q '[^[:space:]]' "$block" || continue

            heading=$(grep -m1 -oE '\[[^]]+\]|✦[^✦]+✦' "$block" 2>/dev/null | head -1 || true)
            heading=${heading//[\[\]✦]/}
            name=$(slugify "${heading:-}")
            if [ -z "$name" ] && [ -f "$block.name" ]; then
                name=$(slugify "$(cat "$block.name")")
            fi
            if [ -z "$name" ]; then
                name="${base}-$(basename "$block" .txt | sed "s/^${base}-//")"
            fi

            out="$OUT_DIR/$name.png"
            # Two cards with the same heading would overwrite each other.
            if [ -e "$out" ]; then
                out="$OUT_DIR/$name-$(basename "$block" .txt | sed "s/^${base}-//").png"
            fi

            render "$block" "$out"
            count=$((count + 1))
            echo "$out"
        done
        ;;
    esac
done

echo "Rendered $count card(s) into $OUT_DIR" >&2
