#!/usr/bin/env bash
# Verify diesel migrations run cleanly on a fresh database.
set -euo pipefail

TMPDB=$(mktemp /tmp/webrpg-test-XXXXXX.db)
trap "rm -f $TMPDB" EXIT

echo "=== Running migrations on fresh DB: $TMPDB ==="
DATABASE_URL="$TMPDB" diesel migration run 2>&1
echo "PASS: migrations applied"

echo ""
echo "=== Verifying expected tables ==="
EXPECTED_TABLES="users sessions session_players rpg_templates characters character_resources maps fog_of_war creatures tokens token_instances chat_messages inventory_items initiative media media_tags"

for table in $EXPECTED_TABLES; do
    if ! sqlite3 "$TMPDB" ".tables" | grep -qw "$table"; then
        echo "FAIL: table '$table' not found"
        exit 1
    fi
done
echo "PASS: all expected tables present"

echo ""
echo "=== Verifying image_url column on tokens ==="
if ! sqlite3 "$TMPDB" "PRAGMA table_info(tokens);" | grep -q "image_url"; then
    echo "FAIL: tokens.image_url column missing"
    exit 1
fi
echo "PASS: tokens.image_url exists"

echo ""
echo "=== Verifying portrait_url column on characters ==="
if ! sqlite3 "$TMPDB" "PRAGMA table_info(characters);" | grep -q "portrait_url"; then
    echo "FAIL: characters.portrait_url column missing"
    exit 1
fi
echo "PASS: characters.portrait_url exists"

echo ""
echo "=== Testing migration rollback ==="
DATABASE_URL="$TMPDB" diesel migration redo 2>&1
echo "PASS: migration redo succeeded"
