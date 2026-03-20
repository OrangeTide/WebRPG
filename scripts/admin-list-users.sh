#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DATABASE_URL:-database.db}"

if [ ! -f "$DB" ]; then
    echo "Database not found: $DB" >&2
    exit 1
fi

echo "=== Users ==="
sqlite3 -header -column "$DB" "
SELECT u.id, u.username, u.display_name, u.email,
       CASE u.access_level WHEN 0 THEN 'user' WHEN 1 THEN 'admin' ELSE u.access_level END AS access,
       CASE u.locked WHEN 1 THEN 'LOCKED' ELSE '' END AS locked,
       (SELECT COUNT(*) FROM session_players sp WHERE sp.user_id = u.id) AS sessions,
       (SELECT COUNT(*) FROM characters c WHERE c.user_id = u.id) AS characters
FROM users u
ORDER BY u.id;
"
