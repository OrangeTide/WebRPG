#!/usr/bin/env bash
# Smoke test: start the server, verify key endpoints, then shut down.
# Requires: cargo-leptos, sqlite3, curl. The test image is ci/fixtures/.
set -euo pipefail

PORT=3199
TMPDB=$(mktemp /tmp/webrpg-smoke-XXXXXX.db)
export DATABASE_URL="$TMPDB"
export SECRET_KEY="ci-test-secret-key-not-for-production"
SERVER_PID=""

cleanup() {
    if [ -n "$SERVER_PID" ]; then
        kill "$SERVER_PID" 2>/dev/null || true
        wait "$SERVER_PID" 2>/dev/null || true
    fi
    rm -f "$TMPDB" /tmp/smoke-cookies.txt /tmp/smoke-cookies2.txt
}
trap cleanup EXIT

echo "=== Setting up test database ==="
diesel migration run

echo ""
echo "=== Building server ==="
# LEPTOS_OUTPUT_NAME must be set while compiling, not just while running: the
# hydration script baked into the server names the wasm file from it, and
# without it the page asks the browser for <name>_bg.wasm, which cargo-leptos
# does not produce. A plain `cargo build` here would leave target/debug/webrpg
# unable to hydrate any page it serves afterwards.
LEPTOS_OUTPUT_NAME=webrpg cargo build --features ssr 2>&1

echo ""
echo "=== Starting server on port $PORT ==="
LEPTOS_SITE_ADDR="127.0.0.1:$PORT" \
LEPTOS_OUTPUT_NAME=webrpg \
LEPTOS_SITE_ROOT=target/site \
LEPTOS_SITE_PKG_DIR=pkg \
  ./target/debug/webrpg &
SERVER_PID=$!

# Wait for server to be ready
echo "Waiting for server..."
for i in $(seq 1 30); do
    if curl -s -o /dev/null -w '' "http://localhost:$PORT/" 2>/dev/null; then
        echo "Server ready after ${i}s"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "FAIL: server did not start within 30s"
        exit 1
    fi
    sleep 1
done

BASE="http://localhost:$PORT"
COOKIES=/tmp/smoke-cookies.txt
PASS=0
FAIL=0

check() {
    local name="$1"
    local result="$2"
    if [ "$result" -eq 0 ]; then
        echo "  PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $name"
        FAIL=$((FAIL + 1))
    fi
}

echo ""
echo "=== Page routes ==="

CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/")
check "Landing page (GET /)" "$([ "$CODE" = "200" ] && echo 0 || echo 1)"

CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/login")
check "Login page (GET /login)" "$([ "$CODE" = "200" ] && echo 0 || echo 1)"

CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/sessions")
check "Sessions page (GET /sessions)" "$([ "$CODE" = "200" ] && echo 0 || echo 1)"

echo ""
echo "=== CSS served ==="
CSS_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/pkg/webrpg.css")
check "Stylesheet (GET /pkg/webrpg.css)" "$([ "$CSS_CODE" = "200" ] && echo 0 || echo 1)"

CSS_LEN=$(curl -s "$BASE/pkg/webrpg.css" | wc -c)
check "Stylesheet not empty (${CSS_LEN} bytes)" "$([ "$CSS_LEN" -gt 100 ] && echo 0 || echo 1)"

# Extract server function URL suffix from the WASM binary
SUFFIX=$(strings target/site/pkg/webrpg.wasm 2>/dev/null | grep -oP '(?<=/api/signup)\d+' | head -1)
if [ -z "$SUFFIX" ]; then
    echo "WARN: Could not extract server fn suffix from WASM, trying login page"
    SUFFIX=$(curl -s "$BASE/login" | grep -oP '(?<=action="/api/login)\d+' | head -1)
fi

if [ -z "$SUFFIX" ]; then
    echo "FAIL: Could not determine server function URL suffix"
    exit 1
fi

# Leptos derives a server function's URL suffix from its module path, so every
# function in one module shares a suffix, and functions in different modules get
# different suffixes. Look each one up in the WASM rather than reusing signup's.
#
# `strings` prints adjacent Rust &str literals with no separator between them,
# so the digit run after a path can swallow the first characters of the next
# literal (list_media_tags shows "...88557" followed by a stray "2"). The real
# suffix is the one its module shares, so the canonical set is the suffixes that
# appear on two or more functions, longest first; a function's true suffix is
# the canonical one that its raw digit run starts with.
#
# A missing name means the bundle is older than the function, and the checks
# that follow would test nothing: an unknown /api/ path falls through to the
# SPA handler, which answers 200 with HTML. Fail loudly instead.
WASM=target/site/pkg/webrpg.wasm
CANONICAL_SUFFIXES=$(strings "$WASM" 2>/dev/null \
    | grep -oP '(?<=/api/)[a-z_]+[0-9]+' | grep -oP '[0-9]+$' \
    | sort | uniq -d | awk '{ print length, $0 }' | sort -rn | cut -d' ' -f2-)

fn_url() {
    local name="$1" run s
    run=$(strings "$WASM" 2>/dev/null | grep -oP "(?<=/api/${name})[0-9]+" | head -1)
    if [ -z "$run" ]; then
        echo "FAIL: server fn '$name' not found in the WASM bundle; rebuild with cargo leptos build" >&2
        exit 1
    fi
    for s in $CANONICAL_SUFFIXES; do
        case "$run" in
        "$s"*)
            printf '%s/api/%s%s' "$BASE" "$name" "$s"
            return 0
            ;;
        esac
    done
    echo "FAIL: could not resolve URL suffix for server fn '$name' (digit run: $run)" >&2
    exit 1
}

# A server function answered, rather than the SPA fallback returning a page.
# Server functions reply with JSON; the fallback replies with HTML.
not_html() {
    case "$1" in
    "<"*) return 1 ;;
    *) return 0 ;;
    esac
}

echo ""
echo "=== Auth (suffix: $SUFFIX) ==="

SIGNUP_RESP=$(curl -s -c "$COOKIES" \
    -X POST "$(fn_url signup)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'username=smoketest&password=smokepass123&display_name=Smoke+Tester&email=smoke%40test.com')
check "Signup returns user" "$(echo "$SIGNUP_RESP" | grep -q '"username":"smoketest"' && echo 0 || echo 1)"

# Signup with a malformed email is rejected (server-side validation).
BAD_EMAIL_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$(fn_url signup)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'username=bademailuser&password=smokepass123&display_name=Bad+Email&email=not-an-email')
check "Signup with malformed email rejected (not 200)" \
    "$([ "$BAD_EMAIL_CODE" != "200" ] && echo 0 || echo 1)"

HAS_TOKEN=$(grep -c "token" "$COOKIES" 2>/dev/null || echo 0)
check "JWT cookie set" "$([ "$HAS_TOKEN" -gt 0 ] && echo 0 || echo 1)"

echo ""
echo "=== Session CRUD ==="

CREATE_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url create_session)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'name=Smoke+Test+Session')
SESSION_ID=$(echo "$CREATE_RESP" | grep -oP '"id":(\d+)' | grep -oP '\d+')
check "Create session (id=$SESSION_ID)" "$([ -n "$SESSION_ID" ] && echo 0 || echo 1)"

LIST_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url list_sessions)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "List sessions returns array" "$(echo "$LIST_RESP" | grep -q '^\[' && echo 0 || echo 1)"

echo ""
echo "=== WebSocket endpoint ==="

WS_TOKEN=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url get_ws_token)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
# WS token is returned as a JSON string (quoted)
check "get_ws_token returns token" "$(echo "$WS_TOKEN" | grep -q 'eyJ' && echo 0 || echo 1)"

# We can't easily test WS upgrade with curl; Axum's WebSocketUpgrade extractor
# rejects non-upgrade requests, so we just verify the route exists (not 404).
WS_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/ws?token=invalid")
check "WS endpoint exists (not 404)" "$([ "$WS_CODE" != "404" ] && echo 0 || echo 1)"

echo ""
echo "=== Media upload + serve ==="

# A committed fixture rather than whatever happens to be in testing/, which is
# gitignored: on a clean checkout that directory does not exist, and under
# `set -o pipefail` a failing find took the whole script down at this point
# rather than skipping the section.
TEST_IMAGE=ci/fixtures/smoke-test.jpg
if [ ! -f "$TEST_IMAGE" ]; then
    echo "SKIP: no test image at $TEST_IMAGE"
else
    UPLOAD_RESP=$(curl -s -b "$COOKIES" \
        -X POST "$BASE/api/media/upload" \
        -F "file=@$TEST_IMAGE" \
        -F "tags=smoke-test,ci")
    MEDIA_HASH=$(echo "$UPLOAD_RESP" | grep -oP '"hash":"([a-f0-9]+)"' | grep -oP '[a-f0-9]{64}')
    check "Upload image" "$([ -n "$MEDIA_HASH" ] && echo 0 || echo 1)"

    if [ -n "$MEDIA_HASH" ]; then
        SERVE_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/media/$MEDIA_HASH")
        check "Serve uploaded image (200)" "$([ "$SERVE_CODE" = "200" ] && echo 0 || echo 1)"

        SERVE_CT=$(curl -sI "$BASE/api/media/$MEDIA_HASH" | grep -i "content-type" | tr -d '\r')
        check "Correct content-type (image/jpeg)" "$(echo "$SERVE_CT" | grep -qi "image/jpeg" && echo 0 || echo 1)"

        SERVE_CACHE=$(curl -sI "$BASE/api/media/$MEDIA_HASH" | grep -i "cache-control" | tr -d '\r')
        check "Immutable cache headers" "$(echo "$SERVE_CACHE" | grep -qi "immutable" && echo 0 || echo 1)"

        # Duplicate upload should return same hash
        DUP_RESP=$(curl -s -b "$COOKIES" \
            -X POST "$BASE/api/media/upload" \
            -F "file=@$TEST_IMAGE" \
            -F "tags=duplicate-test")
        DUP_HASH=$(echo "$DUP_RESP" | grep -oP '"hash":"([a-f0-9]+)"' | grep -oP '[a-f0-9]{64}')
        check "Duplicate upload deduplicates" "$([ "$DUP_HASH" = "$MEDIA_HASH" ] && echo 0 || echo 1)"
    fi

    # list_media
    LIST_MEDIA=$(curl -s -b "$COOKIES" \
        -X POST "$(fn_url list_media)" \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        -H 'Accept: application/json' \
        --data 'media_type=image')
    check "list_media returns images" "$(echo "$LIST_MEDIA" | grep -q '"media_type":"image"' && echo 0 || echo 1)"

    # list_media_tags
    LIST_TAGS=$(curl -s -b "$COOKIES" \
        -X POST "$(fn_url list_media_tags)" \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        -H 'Accept: application/json' \
        --data 'prefix=smoke')
    check "list_media_tags with prefix" "$(echo "$LIST_TAGS" | grep -q 'smoke-test' && echo 0 || echo 1)"
fi

echo ""
echo "=== Invalid media hash ==="
BAD_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/media/0000000000000000000000000000000000000000000000000000000000000000")
check "Non-existent hash returns 404" "$([ "$BAD_CODE" = "404" ] && echo 0 || echo 1)"

BAD_FORMAT_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/media/not-a-hash")
check "Invalid hash format returns 400" "$([ "$BAD_FORMAT_CODE" = "400" ] && echo 0 || echo 1)"

echo ""
echo "=== Game modules ==="

MODULES_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url list_modules)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "list_modules finds the system module" \
    "$(echo "$MODULES_RESP" | grep -q '"id":"tunnel-goons"' && echo 0 || echo 1)"
check "list_modules finds the adventure module" \
    "$(echo "$MODULES_RESP" | grep -q '"id":"sky-blind-spire"' && echo 0 || echo 1)"

INSTALL_SYS=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url install_module)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&module_id=tunnel-goons")
check "Install system module" \
    "$(echo "$INSTALL_SYS" | grep -q '"template_id"' && echo 0 || echo 1)"

INSTALL_ADV=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url install_module)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&module_id=sky-blind-spire")
check "Install adventure module" \
    "$(echo "$INSTALL_ADV" | grep -q '"creatures_added"' && echo 0 || echo 1)"

CREATURE_COUNT=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM creatures WHERE session_id = $SESSION_ID;")
check "Adventure seeded creatures ($CREATURE_COUNT)" \
    "$([ "$CREATURE_COUNT" -gt 0 ] && echo 0 || echo 1)"

MODULE_BINDING=$(sqlite3 "$TMPDB" "SELECT system_module_id || ',' || adventure_module_id FROM sessions WHERE id = $SESSION_ID;")
check "Session bound to both modules" \
    "$([ "$MODULE_BINDING" = "tunnel-goons,sky-blind-spire" ] && echo 0 || echo 1)"

SESSION_MODULES=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url get_session_modules)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}")
check "get_session_modules returns the roll model" \
    "$(echo "$SESSION_MODULES" | grep -q '"dice":"2d6"' && echo 0 || echo 1)"

PREGEN_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url create_character_from_pregen)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&module_id=sky-blind-spire&pregen_id=grog-the-smasher")
check "Create character from pregen" \
    "$(echo "$PREGEN_RESP" | grep -q 'Grog the Smasher' && echo 0 || echo 1)"

PREGEN_ITEMS=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM inventory_items WHERE session_id = $SESSION_ID AND owner_character_id IS NOT NULL;")
check "Pregen gear dealt with slots ($PREGEN_ITEMS items)" \
    "$([ "$PREGEN_ITEMS" -gt 0 ] && echo 0 || echo 1)"

UNINSTALL_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url uninstall_module)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&module_id=sky-blind-spire")
check "Uninstall reports what it did" \
    "$(echo "$UNINSTALL_RESP" | grep -q 'sky-blind-spire' && echo 0 || echo 1)"

UNBOUND=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM sessions WHERE id = $SESSION_ID AND adventure_module_id IS NULL;")
check "Uninstall unbinds the adventure" "$([ "$UNBOUND" -eq 1 ] && echo 0 || echo 1)"

# Unbinding must not throw away work: a GM may have edited these stat blocks.
KEPT=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM creatures WHERE session_id = $SESSION_ID;")
check "Uninstall keeps seeded creatures ($KEPT)" "$([ "$KEPT" -gt 0 ] && echo 0 || echo 1)"

# Put it back so the checks below still have an adventure to read.
curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url install_module)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&module_id=sky-blind-spire"

REFERENCE_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url list_reference_modules)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}")
check "Reference module available without installing" \
    "$(echo "$REFERENCE_RESP" | grep -q '"id":"cairn-spellbooks"' && echo 0 || echo 1)"
check "Reference module carries its spellbook cards" \
    "$(echo "$REFERENCE_RESP" | grep -q 'Spellbook: Word of Pain' && echo 0 || echo 1)"

ASSET_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/modules/tunnel-goons/assets/cards/torch.png")
check "Serve module card art (200)" "$([ "$ASSET_CODE" = "200" ] && echo 0 || echo 1)"

ASSET_TRAVERSAL=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/api/modules/tunnel-goons/assets/../module.json")
check "Module asset traversal refused" \
    "$([ "$ASSET_TRAVERSAL" != "200" ] && echo 0 || echo 1)"

echo ""
echo "=== Virtual file system ==="

# U: is the per-user drive, so these need no session id.
MKDIR_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url vfs_mkdir_dir)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'drive=U&path=/smoke')
check "Create directory on U:" "$(not_html "$MKDIR_RESP" && echo 0 || echo 1)"

# Put a file in it directly, so the check does not depend on how the write
# endpoint encodes its bytes.
SMOKE_UID=$(sqlite3 "$TMPDB" "SELECT id FROM users WHERE username = 'smoketest';")
sqlite3 "$TMPDB" "
INSERT INTO vfs_files (drive, session_id, user_id, path, is_directory, size_bytes,
                       content_type, inline_data, modified_by, created_at, updated_at, mode)
SELECT 'U', NULL, $SMOKE_UID, '/smoke/f.txt', 0, 1, 'text/plain', X'78', $SMOKE_UID,
       strftime('%s','now'), strftime('%s','now'), mode
FROM vfs_files WHERE drive = 'U' AND user_id = $SMOKE_UID AND path = '/smoke';"

LIST_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url vfs_list_dir)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'drive=U&path=/smoke')
check "List directory shows the file" \
    "$(echo "$LIST_RESP" | grep -q '/smoke/f.txt' && echo 0 || echo 1)"

# RMDIR semantics: a non-empty directory is refused, and nothing is removed.
RMDIR_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url vfs_delete_file)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'drive=U&path=/smoke')
check "Deleting a non-empty directory is refused" \
    "$(echo "$RMDIR_RESP" | grep -qi 'not empty' && echo 0 || echo 1)"

SURVIVED=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM vfs_files WHERE drive='U' AND user_id=$SMOKE_UID AND path LIKE '/smoke%';")
check "Refused delete left the tree intact (2 rows)" \
    "$([ "$SURVIVED" -eq 2 ] && echo 0 || echo 1)"

# The file browser's delete: the folder and its contents go together.
TREE_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url vfs_delete_tree)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'drive=U&path=/smoke')
check "Recursive delete succeeds" "$(not_html "$TREE_RESP" && echo 0 || echo 1)"

REMAINING=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM vfs_files WHERE drive='U' AND user_id=$SMOKE_UID AND path LIKE '/smoke%';")
check "Recursive delete removed the whole tree" \
    "$([ "$REMAINING" -eq 0 ] && echo 0 || echo 1)"

echo ""
echo "=== Game page ==="
# Create a map so the game page has something to render
sqlite3 "$TMPDB" "INSERT INTO maps (session_id, name, width, height, cell_size) VALUES ($SESSION_ID, 'Test Map', 20, 15, 40);"

GAME_CODE=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" "$BASE/game/$SESSION_ID")
check "Game page loads (200)" "$([ "$GAME_CODE" = "200" ] && echo 0 || echo 1)"

GAME_HTML=$(curl -s -b "$COOKIES" "$BASE/game/$SESSION_ID")
check "Game page has map container" "$(echo "$GAME_HTML" | grep -q 'map-container' && echo 0 || echo 1)"
# "Set Background" moved inside the map settings dropdown, and the map
# management buttons are GM-gated, so neither is in server-rendered HTML.
# The tool palette is always rendered, so check for that instead.
check "Game page has map tool palette" "$(echo "$GAME_HTML" | grep -q 'map-tool-palette' && echo 0 || echo 1)"
check "Game page has chat panel" "$(echo "$GAME_HTML" | grep -q 'chat-panel' && echo 0 || echo 1)"
check "Game page has character sheet" "$(echo "$GAME_HTML" | grep -q 'character-sheet-panel' && echo 0 || echo 1)"
check "Game page has CSS link" "$(echo "$GAME_HTML" | grep -q 'webrpg.css' && echo 0 || echo 1)"

echo ""
echo "=== Multiple GMs ==="
# A second user joins the session owned by smoketest. They are not a GM until
# added to session_gms, which is exactly what the in-session GM panel does.
COOKIES2=/tmp/smoke-cookies2.txt
curl -s -c "$COOKIES2" \
    -X POST "$(fn_url signup)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'username=coplayer&password=coplaypass123&display_name=Co+Player&email=co%40test.com' \
    >/dev/null
CO_UID=$(sqlite3 "$TMPDB" "SELECT id FROM users WHERE username='coplayer';")
check "second user created (id=$CO_UID)" "$([ -n "$CO_UID" ] && echo 0 || echo 1)"

curl -s -b "$COOKIES2" -o /dev/null \
    -X POST "$(fn_url join_session)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}"

# A non-GM member cannot create a map (create_map is GM-gated).
DENY=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES2" \
    -X POST "$(fn_url create_map)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&name=Nope&width=10&height=10")
check "non-GM member cannot create a map (not 200)" \
    "$([ "$DENY" != "200" ] && echo 0 || echo 1)"

# Grant co-GM, as the in-session owner control does, then the same call works.
sqlite3 "$TMPDB" "INSERT INTO session_gms (session_id, user_id) VALUES ($SESSION_ID, $CO_UID);"
CO_MAP=$(curl -s -b "$COOKIES2" \
    -X POST "$(fn_url create_map)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&name=CoGM+Map&width=10&height=10")
check "co-GM can create a map" \
    "$(echo "$CO_MAP" | grep -q '"id"' && echo 0 || echo 1)"

# Revoking co-GM restores the restriction.
sqlite3 "$TMPDB" "DELETE FROM session_gms WHERE session_id=$SESSION_ID AND user_id=$CO_UID;"
DENY2=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES2" \
    -X POST "$(fn_url create_map)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "session_id=${SESSION_ID}&name=Nope2&width=10&height=10")
check "revoked co-GM cannot create a map (not 200)" \
    "$([ "$DENY2" != "200" ] && echo 0 || echo 1)"

echo ""
echo "=== Server administration ==="

# smoketest is a normal account at this point: admin endpoints must refuse it.
IS_ADMIN=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url current_user_is_admin)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "current_user_is_admin false for normal user" \
    "$([ "$IS_ADMIN" = "false" ] && echo 0 || echo 1)"

DENY_CODE=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$(fn_url admin_list_users)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "admin_list_users refuses non-admin (not 200)" \
    "$([ "$DENY_CODE" != "200" ] && echo 0 || echo 1)"

BK_DENY=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" "$BASE/api/admin/backup")
check "backup refuses non-admin (403)" "$([ "$BK_DENY" = "403" ] && echo 0 || echo 1)"

# A small body: the handler rejects a non-admin before draining the upload, so a
# large one would race the connection close and fail curl under set -e.
RS_DENY=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$BASE/api/admin/restore" -F "file=@/dev/null")
check "restore refuses non-admin (not 200)" "$([ "$RS_DENY" != "200" ] && echo 0 || echo 1)"

RST_DENY=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$(fn_url admin_restart_server)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "admin_restart_server refuses non-admin (not 200)" \
    "$([ "$RST_DENY" != "200" ] && echo 0 || echo 1)"

# Promote the account directly, as the ADMIN_LIST startup step would.
sqlite3 "$TMPDB" "UPDATE users SET access_level = 100 WHERE id = $SMOKE_UID;"

IS_ADMIN=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url current_user_is_admin)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "current_user_is_admin true after promotion" \
    "$([ "$IS_ADMIN" = "true" ] && echo 0 || echo 1)"

ADMIN_USERS=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url admin_list_users)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "admin_list_users returns the account" \
    "$(echo "$ADMIN_USERS" | grep -q '"username":"smoketest"' && echo 0 || echo 1)"

# Add, then remove, a throwaway user (not a GM, so removal is allowed).
curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url admin_add_user)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'username=tempuser&display_name=Temp&email=temp%40test.com&password=temppass123'
TEMP_UID=$(sqlite3 "$TMPDB" "SELECT id FROM users WHERE username = 'tempuser';")
check "admin_add_user created an account" "$([ -n "$TEMP_UID" ] && echo 0 || echo 1)"

curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url admin_remove_user)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "id=${TEMP_UID}"
GONE=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM users WHERE id = $TEMP_UID;")
check "admin_remove_user deleted the account" "$([ "$GONE" -eq 0 ] && echo 0 || echo 1)"

# An admin cannot remove their own account.
SELF_CODE=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$(fn_url admin_remove_user)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "id=${SMOKE_UID}")
check "admin_remove_user refuses self-removal (not 200)" \
    "$([ "$SELF_CODE" != "200" ] && echo 0 || echo 1)"

ADMIN_SESSIONS=$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url admin_list_sessions)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "admin_list_sessions returns an array" \
    "$(echo "$ADMIN_SESSIONS" | grep -q '^\[' && echo 0 || echo 1)"

# Make a throwaway session with a child row to test rename, archive, and the
# cascade delete (foreign keys are not enforced, so the server deletes manually).
ADMIN_SID=$(echo "$(curl -s -b "$COOKIES" \
    -X POST "$(fn_url create_session)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'name=Admin+Throwaway')" | grep -oP '"id":(\d+)' | grep -oP '\d+')
sqlite3 "$TMPDB" "INSERT INTO maps (session_id, name, width, height, cell_size) VALUES ($ADMIN_SID, 'Doomed Map', 10, 10, 40);"

curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url admin_rename_session)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "id=${ADMIN_SID}&name=Renamed+Session"
RENAMED=$(sqlite3 "$TMPDB" "SELECT name FROM sessions WHERE id = $ADMIN_SID;")
check "admin_rename_session renamed it" \
    "$([ "$RENAMED" = "Renamed Session" ] && echo 0 || echo 1)"

curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url admin_set_session_active)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "id=${ADMIN_SID}&active=false"
ARCHIVED=$(sqlite3 "$TMPDB" "SELECT active FROM sessions WHERE id = $ADMIN_SID;")
check "admin_set_session_active archived it (active=0)" \
    "$([ "$ARCHIVED" = "0" ] && echo 0 || echo 1)"

curl -s -b "$COOKIES" -o /dev/null \
    -X POST "$(fn_url admin_remove_session)" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data "id=${ADMIN_SID}"
SESSION_GONE=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM sessions WHERE id = $ADMIN_SID;")
MAP_GONE=$(sqlite3 "$TMPDB" "SELECT COUNT(*) FROM maps WHERE session_id = $ADMIN_SID;")
check "admin_remove_session deleted the session" "$([ "$SESSION_GONE" -eq 0 ] && echo 0 || echo 1)"
check "admin_remove_session cascaded to its maps" "$([ "$MAP_GONE" -eq 0 ] && echo 0 || echo 1)"

# Backup download: a consistent SQLite snapshot. (Restore and restart are not
# exercised here: a successful restore or restart would terminate this server.)
BK_FILE=$(mktemp /tmp/webrpg-smoke-bk-XXXXXX.db)
BK_CODE=$(curl -s -b "$COOKIES" -o "$BK_FILE" -w "%{http_code}" "$BASE/api/admin/backup")
check "admin backup returns 200" "$([ "$BK_CODE" = "200" ] && echo 0 || echo 1)"
check "admin backup is a SQLite file" \
    "$([ "$(head -c 15 "$BK_FILE")" = "SQLite format 3" ] && echo 0 || echo 1)"
BK_USERS=$(sqlite3 "$BK_FILE" "SELECT COUNT(*) FROM users;" 2>/dev/null || echo 0)
check "admin backup is a readable database ($BK_USERS users)" \
    "$([ "$BK_USERS" -gt 0 ] && echo 0 || echo 1)"
rm -f "$BK_FILE"

# Restore validates the upload before doing anything destructive.
BAD_FILE=$(mktemp /tmp/webrpg-smoke-bad-XXXXXX.txt)
echo "this is not a database" > "$BAD_FILE"
BAD_RESTORE=$(curl -s -o /dev/null -w "%{http_code}" -b "$COOKIES" \
    -X POST "$BASE/api/admin/restore" -F "file=@$BAD_FILE")
check "admin restore rejects a non-SQLite upload (415)" \
    "$([ "$BAD_RESTORE" = "415" ] && echo 0 || echo 1)"
rm -f "$BAD_FILE"
ALIVE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE/")
check "server still running after a rejected restore" \
    "$([ "$ALIVE" = "200" ] && echo 0 || echo 1)"

echo ""
echo "========================================"
echo "Results: $PASS passed, $FAIL failed"
echo "========================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
