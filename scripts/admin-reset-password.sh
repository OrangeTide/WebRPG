#!/usr/bin/env bash
# Reset a user's password, or everyone's.
#
#   scripts/admin-reset-password.sh <password> [username]
#
# With no username, every account is set to the same password, which is only
# ever sensible on a development database.
#
# The hash is produced by the server's own code (src/bin/hash-password.rs) so
# that it matches what the login path verifies.
#
# PUBLIC DOMAIN (CC0-1.0)
set -euo pipefail

cd "$(dirname "$0")/.."

DB="${DATABASE_URL:-database.db}"

usage() {
    echo "usage: $0 <password> [username]" >&2
    echo "  With no username, resets every account." >&2
}

PASSWORD="${1:-}"
USERNAME="${2:-}"

if [ -z "$PASSWORD" ]; then
    usage
    exit 2
fi

if [ ! -f "$DB" ]; then
    echo "Database not found: $DB" >&2
    exit 1
fi

# A password short enough to be a typo is probably a mistake, and signup
# enforces a minimum too.
if [ "${#PASSWORD}" -lt 8 ]; then
    echo "Password must be at least 8 characters." >&2
    exit 1
fi

if [ -n "$USERNAME" ]; then
    EXISTS=$(sqlite3 "$DB" "SELECT COUNT(*) FROM users WHERE username = '$(printf '%s' "$USERNAME" | sed "s/'/''/g")';")
    if [ "$EXISTS" -eq 0 ]; then
        echo "No such user: $USERNAME" >&2
        exit 1
    fi
fi

# Back up first: the old hashes cannot be recovered afterwards.
BACKUP="$DB.$(date +%Y%m%d-%H%M%S).bak"
cp "$DB" "$BACKUP"
echo "Backed up $DB to $BACKUP"

echo "Building the hashing helper..."
cargo build --quiet --features ssr --bin hash-password

HASH=$(./target/debug/hash-password "$PASSWORD")
if [ -z "$HASH" ]; then
    echo "Failed to hash the password." >&2
    exit 1
fi

# The hash is a PHC string containing $ and /, so it goes in as a bound-ish
# literal with quotes doubled rather than through the shell.
ESCAPED_HASH=$(printf '%s' "$HASH" | sed "s/'/''/g")

if [ -n "$USERNAME" ]; then
    ESCAPED_USER=$(printf '%s' "$USERNAME" | sed "s/'/''/g")
    sqlite3 "$DB" "UPDATE users SET passcrypt = '$ESCAPED_HASH' WHERE username = '$ESCAPED_USER';"
    echo "Reset password for $USERNAME."
else
    sqlite3 "$DB" "UPDATE users SET passcrypt = '$ESCAPED_HASH';"
    COUNT=$(sqlite3 "$DB" "SELECT COUNT(*) FROM users;")
    echo "Reset the password for all $COUNT accounts."
fi
