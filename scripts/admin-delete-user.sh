#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DATABASE_URL:-database.db}"

if [ ! -f "$DB" ]; then
    echo "Database not found: $DB" >&2
    exit 1
fi

if [ $# -lt 1 ]; then
    echo "Usage: $0 <user_id> [--force]" >&2
    echo "  Deletes a user and all their owned data." >&2
    echo "  Use --force to skip confirmation." >&2
    exit 1
fi

USER_ID="$1"
FORCE="${2:-}"

# Verify user exists
USER_INFO=$(sqlite3 "$DB" "SELECT username, display_name FROM users WHERE id = $USER_ID;" 2>/dev/null)
if [ -z "$USER_INFO" ]; then
    echo "User $USER_ID not found." >&2
    exit 1
fi

echo "User to delete: $USER_INFO (id=$USER_ID)"

# Show what will be deleted
echo ""
echo "This will delete:"
sqlite3 "$DB" "
SELECT '  Sessions owned (GM):    ' || COUNT(*) FROM sessions WHERE gm_user_id = $USER_ID;
SELECT '  Session memberships:    ' || COUNT(*) FROM session_players WHERE user_id = $USER_ID;
SELECT '  Characters:             ' || COUNT(*) FROM characters WHERE user_id = $USER_ID;
SELECT '  Chat messages:          ' || COUNT(*) FROM chat_messages WHERE user_id = $USER_ID;
SELECT '  Media uploads:          ' || COUNT(*) FROM media WHERE uploaded_by = $USER_ID;
SELECT '  VFS files (user drive): ' || COUNT(*) FROM vfs_files WHERE user_id = $USER_ID;
"

# Check if user is GM of sessions — those sessions will be orphaned or deleted
GM_SESSIONS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM sessions WHERE gm_user_id = $USER_ID;")
if [ "$GM_SESSIONS" -gt 0 ]; then
    echo ""
    echo "WARNING: User is GM of $GM_SESSIONS session(s). Those sessions and ALL their"
    echo "data (maps, tokens, creatures, initiative, inventory, chat, fog, VFS) will be deleted."
    sqlite3 -header -column "$DB" "SELECT id, name FROM sessions WHERE gm_user_id = $USER_ID;"
fi

if [ "$FORCE" != "--force" ]; then
    echo ""
    read -rp "Type 'yes' to confirm deletion: " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Aborted."
        exit 0
    fi
fi

echo ""
echo "Deleting user $USER_ID..."

sqlite3 "$DB" "
BEGIN TRANSACTION;

-- Delete character resources for user's characters
DELETE FROM character_resources WHERE character_id IN
    (SELECT id FROM characters WHERE user_id = $USER_ID);

-- Delete user's characters
DELETE FROM characters WHERE user_id = $USER_ID;

-- Delete chat messages by this user
DELETE FROM chat_messages WHERE user_id = $USER_ID;

-- Delete session memberships
DELETE FROM session_players WHERE user_id = $USER_ID;

-- Delete VFS files owned by user
DELETE FROM vfs_files WHERE user_id = $USER_ID;

-- For sessions where user is GM: cascade delete everything
-- Token instances for tokens on maps in GM's sessions
DELETE FROM token_instances WHERE token_id IN
    (SELECT t.id FROM tokens t
     JOIN maps m ON t.map_id = m.id
     JOIN sessions s ON m.session_id = s.id
     WHERE s.gm_user_id = $USER_ID);

-- Tokens on maps in GM's sessions
DELETE FROM tokens WHERE map_id IN
    (SELECT m.id FROM maps m
     JOIN sessions s ON m.session_id = s.id
     WHERE s.gm_user_id = $USER_ID);

-- Fog of war on maps in GM's sessions
DELETE FROM fog_of_war WHERE map_id IN
    (SELECT m.id FROM maps m
     JOIN sessions s ON m.session_id = s.id
     WHERE s.gm_user_id = $USER_ID);

-- Maps in GM's sessions
DELETE FROM maps WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Creatures in GM's sessions
DELETE FROM creatures WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Initiative in GM's sessions
DELETE FROM initiative WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Inventory in GM's sessions
DELETE FROM inventory_items WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Chat messages in GM's sessions (from all users)
DELETE FROM chat_messages WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Character resources for characters in GM's sessions
DELETE FROM character_resources WHERE character_id IN
    (SELECT c.id FROM characters c
     JOIN sessions s ON c.session_id = s.id
     WHERE s.gm_user_id = $USER_ID);

-- Characters in GM's sessions (from all users)
DELETE FROM characters WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- Session players in GM's sessions
DELETE FROM session_players WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- VFS files in GM's sessions
DELETE FROM vfs_files WHERE session_id IN
    (SELECT id FROM sessions WHERE gm_user_id = $USER_ID);

-- The sessions themselves
DELETE FROM sessions WHERE gm_user_id = $USER_ID;

-- Finally, delete the user
DELETE FROM users WHERE id = $USER_ID;

COMMIT;
"

echo "Done. User $USER_ID deleted."
