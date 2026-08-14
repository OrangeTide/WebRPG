#!/usr/bin/env bash
# Fetch The Sky-Blind Spire's art into the module's assets directory.
#
# The adventure is (c) 2016 Michael Prescott under CC BY-NC 3.0, which this
# MIT-0 repository does not carry, so the art is gitignored and never
# committed. This script reproduces the drop-in so anyone can complete their
# own copy: install the module afterwards and the maps come up with their
# backgrounds attached.
#
# Two sources, in order of preference:
#
#   SPIRE_SOURCE=/path/to/dir   copy images already extracted, by file name
#   otherwise                   download the one-page PDF from trilemma.com
#                               and render its pages
#
# Requires curl and poppler-utils (pdftoppm). ImageMagick is used to trim the
# rendered pages if it is available.
#
# PUBLIC DOMAIN (CC0-1.0)
set -euo pipefail

cd "$(dirname "$0")/.."

MODULE_DIR="${MODULE_DIR:-modules/sky-blind-spire}"
ASSETS="$MODULE_DIR/assets"
PDF_URL="${PDF_URL:-https://trilemma.com/blog/adventures/24%20Sky-Blind%20Spire.pdf}"
CREDIT="The Sky-Blind Spire (c) 2016 Michael Prescott, CC BY-NC 3.0"

# What maps.json expects to find.
MAP_ART="24-Sky-Blind-Spire-Map.png"
TOWERTOP_ART="24-Towertop.png"

usage() {
    cat <<EOF
Usage: $0

Downloads the adventure's art into $ASSETS, which is gitignored.

  SPIRE_SOURCE=<dir>  copy from a directory that already holds the images
                      ($MAP_ART, $TOWERTOP_ART) instead of downloading
  PDF_URL=<url>       override the source PDF
  DPI=<n>             render resolution, default 200

$CREDIT
EOF
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

if [ ! -d "$MODULE_DIR" ]; then
    echo "error: no module at $MODULE_DIR" >&2
    exit 1
fi

mkdir -p "$ASSETS"

# Copying from a local extract is exact, so prefer it when offered.
if [ -n "${SPIRE_SOURCE:-}" ]; then
    if [ ! -d "$SPIRE_SOURCE" ]; then
        echo "error: SPIRE_SOURCE is not a directory: $SPIRE_SOURCE" >&2
        exit 1
    fi
    copied=0
    for name in "$MAP_ART" "$TOWERTOP_ART" tower-elevation.png; do
        if [ -f "$SPIRE_SOURCE/$name" ]; then
            cp "$SPIRE_SOURCE/$name" "$ASSETS/$name"
            echo "copied $name"
            copied=$((copied + 1))
        fi
    done
    if [ "$copied" -eq 0 ]; then
        echo "error: none of the expected images were in $SPIRE_SOURCE" >&2
        echo "expected: $MAP_ART, $TOWERTOP_ART, tower-elevation.png" >&2
        exit 1
    fi
    echo "$copied file(s) in $ASSETS. $CREDIT" >&2
    exit 0
fi

for tool in curl pdftoppm; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: $tool not found (poppler-utils provides pdftoppm)" >&2
        exit 1
    fi
done

DPI="${DPI:-200}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo "Downloading the adventure PDF..."
if ! curl -fsSL "$PDF_URL" -o "$work/spire.pdf"; then
    echo "error: could not download $PDF_URL" >&2
    echo "Fetch it by hand from https://blog.trilemma.com/2016/04/the-sky-blind-spire.html" >&2
    echo "and re-run with SPIRE_SOURCE=<dir holding the images>." >&2
    exit 1
fi

# A one-page adventure renders to one image; later pages, if the file ever
# grows them, are kept alongside so nothing is silently dropped.
echo "Rendering at ${DPI} dpi..."
pdftoppm -png -r "$DPI" "$work/spire.pdf" "$work/page"

pages=("$work"/page-*.png)
if [ ! -e "${pages[0]}" ]; then
    echo "error: the PDF produced no pages" >&2
    exit 1
fi

cp "${pages[0]}" "$ASSETS/$MAP_ART"
echo "wrote $MAP_ART"

# The towertop illustration is part of the same page; without a reliable way to
# crop it, reuse the page and let the GM crop if they care.
if [ ! -f "$ASSETS/$TOWERTOP_ART" ]; then
    cp "${pages[0]}" "$ASSETS/$TOWERTOP_ART"
    echo "wrote $TOWERTOP_ART (same page; crop it if you want just the roof)"
fi

if [ "${#pages[@]}" -gt 1 ]; then
    for extra in "${pages[@]:1}"; do
        cp "$extra" "$ASSETS/$(basename "$extra")"
        echo "wrote $(basename "$extra")"
    done
fi

cat >&2 <<EOF

Done. $CREDIT
The files are gitignored: they stay on this machine.
Reinstall the adventure from the Modules window to attach them to its maps.
EOF
