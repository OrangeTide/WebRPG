#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DATABASE_URL:-database.db}"

if [ ! -f "$DB" ]; then
    echo "Database not found: $DB" >&2
    exit 1
fi

if [ $# -lt 1 ]; then
    echo "Usage: $0 <session_id> [--force]" >&2
    echo "  Deletes a game session and all its data." >&2
    echo "  Use --force to skip confirmation." >&2
    exit 1
fi

SESSION_ID="$1"
FORCE="${2:-}"

# Verify session exists
SESSION_INFO=$(sqlite3 "$DB" "
SELECT s.name || ' (GM: ' || u.username || ')'
FROM sessions s JOIN users u ON s.gm_user_id = u.id
WHERE s.id = $SESSION_ID;
" 2>/dev/null)
if [ -z "$SESSION_INFO" ]; then
    echo "Session $SESSION_ID not found." >&2
    exit 1
fi

echo "Session to delete: $SESSION_INFO (id=$SESSION_ID)"

# Show what will be deleted
echo ""
echo "This will delete:"
sqlite3 "$DB" "
SELECT '  Players:          ' || COUNT(*) FROM session_players WHERE session_id = $SESSION_ID;
SELECT '  Characters:       ' || COUNT(*) FROM characters WHERE session_id = $SESSION_ID;
SELECT '  Maps:             ' || COUNT(*) FROM maps WHERE session_id = $SESSION_ID;
SELECT '  Creatures:        ' || COUNT(*) FROM creatures WHERE session_id = $SESSION_ID;
SELECT '  Initiative:       ' || COUNT(*) FROM initiative WHERE session_id = $SESSION_ID;
SELECT '  Inventory items:  ' || COUNT(*) FROM inventory_items WHERE session_id = $SESSION_ID;
SELECT '  Chat messages:    ' || COUNT(*) FROM chat_messages WHERE session_id = $SESSION_ID;
SELECT '  VFS files:        ' || COUNT(*) FROM vfs_files WHERE session_id = $SESSION_ID;
"

if [ "$FORCE" != "--force" ]; then
    echo ""
    read -rp "Type 'yes' to confirm deletion: " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Aborted."
        exit 0
    fi
fi

echo ""
echo "Deleting session $SESSION_ID..."

sqlite3 "$DB" "
BEGIN TRANSACTION;

-- Token instances for tokens on maps in this session
DELETE FROM token_instances WHERE token_id IN
    (SELECT t.id FROM tokens t
     JOIN maps m ON t.map_id = m.id
     WHERE m.session_id = $SESSION_ID);

-- Tokens on maps in this session
DELETE FROM tokens WHERE map_id IN
    (SELECT id FROM maps WHERE session_id = $SESSION_ID);

-- Fog of war on maps in this session
DELETE FROM fog_of_war WHERE map_id IN
    (SELECT id FROM maps WHERE session_id = $SESSION_ID);

-- Maps
DELETE FROM maps WHERE session_id = $SESSION_ID;

-- Creatures
DELETE FROM creatures WHERE session_id = $SESSION_ID;

-- Initiative
DELETE FROM initiative WHERE session_id = $SESSION_ID;

-- Inventory
DELETE FROM inventory_items WHERE session_id = $SESSION_ID;

-- Chat messages
DELETE FROM chat_messages WHERE session_id = $SESSION_ID;

-- Character resources
DELETE FROM character_resources WHERE character_id IN
    (SELECT id FROM characters WHERE session_id = $SESSION_ID);

-- Characters
DELETE FROM characters WHERE session_id = $SESSION_ID;

-- Session players
DELETE FROM session_players WHERE session_id = $SESSION_ID;

-- VFS files
DELETE FROM vfs_files WHERE session_id = $SESSION_ID;

-- The session itself
DELETE FROM sessions WHERE id = $SESSION_ID;

COMMIT;
"

echo "Done. Session $SESSION_ID deleted."
