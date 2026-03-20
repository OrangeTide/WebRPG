#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DATABASE_URL:-database.db}"

if [ ! -f "$DB" ]; then
    echo "Database not found: $DB" >&2
    exit 1
fi

echo "=== Sessions ==="
sqlite3 -header -column "$DB" "
SELECT s.id, s.name,
       u.username AS gm,
       CASE s.active WHEN 1 THEN 'active' ELSE 'inactive' END AS status,
       (SELECT COUNT(*) FROM session_players sp WHERE sp.session_id = s.id) AS players,
       (SELECT COUNT(*) FROM characters c WHERE c.session_id = s.id) AS chars,
       (SELECT COUNT(*) FROM maps m WHERE m.session_id = s.id) AS maps,
       (SELECT COUNT(*) FROM creatures cr WHERE cr.session_id = s.id) AS creatures,
       (SELECT COUNT(*) FROM chat_messages cm WHERE cm.session_id = s.id) AS messages,
       s.created_at
FROM sessions s
JOIN users u ON s.gm_user_id = u.id
ORDER BY s.id;
"
