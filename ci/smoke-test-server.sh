#!/usr/bin/env bash
# Smoke test: start the server, verify key endpoints, then shut down.
# Requires: cargo-leptos, sqlite3, curl, a test image in testing/
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
    rm -f "$TMPDB" /tmp/smoke-cookies.txt
}
trap cleanup EXIT

echo "=== Setting up test database ==="
diesel migration run

echo ""
echo "=== Building server ==="
cargo build --features ssr 2>&1

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

# Leptos hashes server function URLs per function, so a function's suffix is
# not always the one signup got. Look each one up in the WASM.
#
# A missing name means the bundle is older than the function, and the checks
# that follow would test nothing: an unknown /api/ path falls through to the
# SPA handler, which answers 200 with HTML. Fail loudly instead.
fn_url() {
    local name="$1" found
    found=$(strings target/site/pkg/webrpg.wasm 2>/dev/null \
        | grep -oP "(?<=/api/${name})\d+" | head -1)
    if [ -z "$found" ]; then
        echo "FAIL: server fn '$name' not found in the WASM bundle; rebuild with cargo leptos build" >&2
        exit 1
    fi
    printf '%s/api/%s%s' "$BASE" "$name" "$found"
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
    -X POST "$BASE/api/signup${SUFFIX}" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'username=smoketest&password=smokepass123&display_name=Smoke+Tester&email=smoke%40test.com')
check "Signup returns user" "$(echo "$SIGNUP_RESP" | grep -q '"username":"smoketest"' && echo 0 || echo 1)"

HAS_TOKEN=$(grep -c "token" "$COOKIES" 2>/dev/null || echo 0)
check "JWT cookie set" "$([ "$HAS_TOKEN" -gt 0 ] && echo 0 || echo 1)"

echo ""
echo "=== Session CRUD ==="

CREATE_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$BASE/api/create_session${SUFFIX}" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' \
    --data 'name=Smoke+Test+Session')
SESSION_ID=$(echo "$CREATE_RESP" | grep -oP '"id":(\d+)' | grep -oP '\d+')
check "Create session (id=$SESSION_ID)" "$([ -n "$SESSION_ID" ] && echo 0 || echo 1)"

LIST_RESP=$(curl -s -b "$COOKIES" \
    -X POST "$BASE/api/list_sessions${SUFFIX}" \
    -H 'Content-Type: application/x-www-form-urlencoded' \
    -H 'Accept: application/json' --data '')
check "List sessions returns array" "$(echo "$LIST_RESP" | grep -q '^\[' && echo 0 || echo 1)"

echo ""
echo "=== WebSocket endpoint ==="

WS_TOKEN=$(curl -s -b "$COOKIES" \
    -X POST "$BASE/api/get_ws_token${SUFFIX}" \
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

# Find a test image
TEST_IMAGE=$(find testing/ -name '*.jpg' | head -1)
if [ -z "$TEST_IMAGE" ]; then
    echo "SKIP: no test images in testing/"
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
        -X POST "$BASE/api/list_media${SUFFIX}" \
        -H 'Content-Type: application/x-www-form-urlencoded' \
        -H 'Accept: application/json' \
        --data 'media_type=image')
    check "list_media returns images" "$(echo "$LIST_MEDIA" | grep -q '"media_type":"image"' && echo 0 || echo 1)"

    # list_media_tags
    LIST_TAGS=$(curl -s -b "$COOKIES" \
        -X POST "$BASE/api/list_media_tags${SUFFIX}" \
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
echo "========================================"
echo "Results: $PASS passed, $FAIL failed"
echo "========================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
