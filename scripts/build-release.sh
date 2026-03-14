#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
BIN_TARGET="x86_64-unknown-linux-musl"
RELEASE_NAME="webrpg-${VERSION}"
RELEASE_DIR="target/release-package"
TARBALL="target/${RELEASE_NAME}.tar.gz"

echo "Building WebRPG v${VERSION} release (static musl binary)..."

cargo leptos build --release --bin-cargo-args="--target" --bin-cargo-args="${BIN_TARGET}"

echo "Packaging release..."

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR/${RELEASE_NAME}"

# Server binary (statically linked via musl)
cp "target/${BIN_TARGET}/release/webrpg" "$RELEASE_DIR/${RELEASE_NAME}/"

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

echo "Release tarball: $TARBALL"
echo "Size: $(du -h "$TARBALL" | cut -f1)"
