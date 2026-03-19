#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
BIN_TARGET="x86_64-unknown-linux-musl"
RELEASE_NAME="webrpg-${VERSION}"
RELEASE_DIR="target/release-package"
TARBALL="target/${RELEASE_NAME}.tar.gz"
SRC_TARBALL="target/${RELEASE_NAME}-src.tar.gz"

echo "Building WebRPG v${VERSION} release (static musl binary)..."

LEPTOS_BIN_TARGET_TRIPLE="${BIN_TARGET}" cargo leptos build --release

echo "Packaging release..."

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR/${RELEASE_NAME}"

# Server binary (statically linked via musl)
cp "target/${BIN_TARGET}/release/webrpg" "$RELEASE_DIR/${RELEASE_NAME}/"

# Work around cargo-leptos renaming webrpg_bg.wasm to webrpg.wasm without
# patching the JS import that still references webrpg_bg.wasm. Incremental
# compilation can also cache a stale option_env!("LEPTOS_OUTPUT_NAME") result,
# causing Leptos HydrationScripts to request the _bg variant.
# See: https://github.com/leptos-rs/leptos/issues/1337
if [ -f target/site/pkg/webrpg.wasm ] && [ ! -f target/site/pkg/webrpg_bg.wasm ]; then
    cp target/site/pkg/webrpg.wasm target/site/pkg/webrpg_bg.wasm
fi

# Site assets (WASM, CSS, JS, public files)
cp -r target/site "$RELEASE_DIR/${RELEASE_NAME}/site"

# Diesel migrations for database setup
cp -r migrations "$RELEASE_DIR/${RELEASE_NAME}/migrations"

# Include a minimal env template
cat > "$RELEASE_DIR/${RELEASE_NAME}/env.example" <<'EOF'
LEPTOS_OUTPUT_NAME=webrpg
LEPTOS_SITE_ROOT=site
LEPTOS_SITE_PKG_DIR=pkg
LEPTOS_SITE_ADDR=0.0.0.0:3000
DATABASE_URL=database.db
SECRET_KEY=change-me-in-production
EOF

# Tar it up
tar -czf "$TARBALL" -C "$RELEASE_DIR" "$RELEASE_NAME"

# Source code snapshot via git archive
echo "Creating source snapshot..."
git archive --format=tar.gz --prefix="${RELEASE_NAME}-src/" -o "$SRC_TARBALL" HEAD

echo "Release tarball: $TARBALL"
echo "Size: $(du -h "$TARBALL" | cut -f1)"
echo "Source tarball:  $SRC_TARBALL"
echo "Size: $(du -h "$SRC_TARBALL" | cut -f1)"
