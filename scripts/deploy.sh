#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $0 <user@host> [tarball]"
    echo ""
    echo "Uploads and unpacks a WebRPG release to a remote server via SSH."
    echo ""
    echo "Arguments:"
    echo "  user@host   SSH destination (e.g. deploy@myserver.com)"
    echo "  tarball     Path to release tarball (default: auto-detect latest in target/)"
    echo ""
    echo "The release is deployed to ~/webrpg on the remote host."
    echo "On first deploy, copy env.example to .env and edit it."
    exit 1
}

if [ $# -lt 1 ]; then
    usage
fi

REMOTE="$1"
DEPLOY_DIR="webrpg"

# Find tarball
if [ $# -ge 2 ]; then
    TARBALL="$2"
else
    cd "$(dirname "$0")/.."
    TARBALL=$(ls -t target/webrpg-*.tar.gz 2>/dev/null | head -1)
    if [ -z "$TARBALL" ]; then
        echo "Error: No release tarball found. Run scripts/build-release.sh first."
        exit 1
    fi
fi

if [ ! -f "$TARBALL" ]; then
    echo "Error: Tarball not found: $TARBALL"
    exit 1
fi

RELEASE_NAME=$(basename "$TARBALL" .tar.gz)
echo "Deploying ${RELEASE_NAME} to ${REMOTE}:~/${DEPLOY_DIR}/"

# Upload tarball
echo "Uploading $(du -h "$TARBALL" | cut -f1)..."
scp "$TARBALL" "${REMOTE}:/tmp/${RELEASE_NAME}.tar.gz"

# Unpack and link on remote
ssh "$REMOTE" bash -s "$RELEASE_NAME" "$DEPLOY_DIR" <<'REMOTE_SCRIPT'
set -euo pipefail

RELEASE_NAME="$1"
DEPLOY_DIR="$2"

mkdir -p ~/"$DEPLOY_DIR"
cd ~/"$DEPLOY_DIR"

# Unpack release
tar -xzf "/tmp/${RELEASE_NAME}.tar.gz"
rm "/tmp/${RELEASE_NAME}.tar.gz"

# Symlink current release
ln -sfn "$RELEASE_NAME" current

# Preserve .env and database across deploys
if [ -f .env ]; then
    ln -sf ../../.env "current/.env"
else
    echo "NOTE: No .env found. Copy current/env.example to ~/webrpg/.env and edit it."
fi

if [ -f database.db ]; then
    ln -sf ../../database.db "current/database.db"
fi

# Preserve uploads directory across deploys
if [ -d uploads ]; then
    ln -sfn ../../uploads "current/uploads"
else
    mkdir -p uploads
    ln -sfn ../../uploads "current/uploads"
fi

echo ""
echo "Deployed to ~/${DEPLOY_DIR}/current/"
echo ""
echo "To run:"
echo "  cd ~/${DEPLOY_DIR}/current && ./webrpg"
echo ""
echo "To run migrations (first deploy or after schema changes):"
echo "  cd ~/${DEPLOY_DIR}/current && diesel migration run"
REMOTE_SCRIPT

echo "Done."
