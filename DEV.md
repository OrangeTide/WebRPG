# Developer Information

## Architecture

- **Server:** Rust with [Leptos](https://leptos.dev/) SSR and
  [Axum](https://github.com/tokio-rs/axum)
- **Client:** WASM via Leptos hydration
- **Database:** SQLite via [Diesel](https://diesel.rs/) ORM
- **Real-time:** WebSocket with JSON messages (snapshot + event model)
- **Auth:** JWT tokens in HttpOnly cookies, Argon2 password hashing

The project uses `ssr` and `hydrate` feature flags. `cargo-leptos` builds the
server binary with `--features ssr` and the WASM client with
`--features hydrate --target wasm32-unknown-unknown`. Server-only code (auth,
database, WebSocket handler) is gated behind `#[cfg(feature = "ssr")]`. Shared
types (DTOs, WebSocket message enums) compile for both targets. Server functions
use Leptos `#[server]` which generates client-side RPC stubs automatically.

## Project Structure

```
src/
├── main.rs           # Axum server entry point (SSR)
├── lib.rs            # Module declarations + WASM hydrate entry
├── app.rs            # Root App component, router, SSR shell
├── auth.rs           # JWT + Argon2 auth (server only)
├── db.rs             # Diesel/SQLite connection pool (server only)
├── schema.rs         # Diesel table definitions (auto-generated)
├── models.rs         # Shared DTOs + Diesel models
├── pages/
│   ├── mod.rs            # Module declarations
│   ├── landing.rs        # Landing/intro page
│   ├── login.rs          # Login + signup page with JWT auth
│   ├── sessions.rs       # Session list, create/join
│   └── game.rs           # Main game view, GameContext, WebSocket setup
├── components/
│   ├── mod.rs              # Module declarations
│   ├── window_manager/     # Draggable/resizable window system + dock
│   │   ├── mod.rs          #   WindowManager component + GameWindow
│   │   ├── dock.rs         #   NeXTSTEP-style dock (minimize tiles)
│   │   ├── persistence.rs  #   localStorage save/restore of layout
│   │   └── settings.rs     #   Settings dialog (hotkeys, preferences)
│   ├── map.rs              # HTML5 Canvas map with viewport, tools, tokens
│   ├── chat.rs             # Chat panel with dice rolling
│   ├── charsheet.rs        # Template-driven character sheet editor
│   ├── creatures.rs        # GM creature stat block CRUD
│   ├── inventory.rs        # Party/character inventory with slot cards
│   ├── initiative.rs       # Initiative tracker with turn order
│   ├── check_roller.rs     # Module-driven check roller (dice + ability vs DS)
│   ├── modules_panel.rs    # Module browser, pregens, item cards, room key
│   ├── media_browser.rs    # Media upload/browse/search modal
│   ├── file_browser.rs     # NeXTSTEP-style graphical file browser (Finder)
│   ├── terminal.rs         # DOS-style COMMAND.COM terminal emulator
│   ├── help_viewer.rs      # Online help viewer (Markdown-based)
│   └── browser_helpers.rs  # Browser utility functions
├── server/
│   ├── mod.rs            # Module declarations
│   ├── api.rs            # Leptos server functions (sessions, characters, templates)
│   ├── modules_api.rs    # Game module browse/install server functions
│   ├── media_handler.rs  # Media upload/serve endpoints (CAS)
│   ├── module_assets.rs  # Serves art a game module ships
│   └── ws_handler.rs     # WebSocket upgrade + authentication
├── modules/
│   ├── mod.rs            # Game module types (shared by both targets)
│   └── loader.rs         # Reads module packs off disk (server only)
├── vfs.rs            # Virtual file system abstraction (drive dispatch)
├── scratch_drive.rs  # Client-side IndexedDB scratch drives (A:/B:)
└── ws/
    ├── mod.rs            # Module declarations
    ├── messages.rs       # WebSocket message type definitions
    └── session.rs        # Server-side session state manager
migrations/               # Diesel SQL migrations
modules/                  # Game module packs (data, not code)
```

## Build Prerequisites

For general development:

  - Rust (stable, 1.85+)
  - SQLite3 development libraries (e.g. `libsqlite3-dev` on Debian/Ubuntu)
    ```bash
    sudo apt install libsqlite3-dev
    ```
  - install tools needed for static linking:
    ```bash
    sudo apt install musl-tools
    rustup target add x86_64-unknown-linux-musl
    ```
  - Add `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
  - Install cargo leptos: `cargo install --locked cargo-leptos`
  - Install Diesel CLI tools: `cargo install diesel_cli --no-default-features --features sqlite`

Use `--locked` when installing cargo-leptos. Resolving its dependencies afresh
can pull crates that need a newer rustc than the toolchain this project builds
with.

### Always build the server through cargo-leptos

`cargo leptos serve` and `cargo leptos build` set `LEPTOS_OUTPUT_NAME` while
compiling, and Leptos reads it **at compile time** to write the hydration
script that every page carries. A server built with a plain
`cargo build --features ssr` bakes in the wrong wasm file name: the page asks
the browser for `/pkg/webrpg_bg.wasm`, which cargo-leptos never produces, so
the fetch 404s, `hydrate()` never runs, and nothing on the page responds to a
click.

This is worth recognising because of how it presents. Pages render correctly,
the server answers every request, `curl` against the API succeeds, and logging
in even sets its cookie, because the form posts without any JavaScript. Only
the parts that need the client are dead, which reads as broken UI code rather
than a build problem.

`cargo check --features ssr` and `cargo build --features ssr` are fine for
compiling and for CI. Just do not serve from a binary they produced. If you
need to run `./target/debug/webrpg` directly, build it with `cargo leptos build`
first, or set the variable yourself:

```sh
LEPTOS_OUTPUT_NAME=webrpg cargo build --features ssr
```

`ci/smoke-test-server.sh` sets it for the same reason: it builds the server
itself and would otherwise leave a binary behind that cannot hydrate.

### For AI (Claude, Gemini, etc) and MCP tools

Commands to check and install requirements (idempotent)

Debian/ubuntu commands: #TODO: setup environment: "WEBDRIVER_PREFERRED_DRIVER": "chrome", "WEBDRIVER_HEADLESS": "true"
  ```bash
  geckodriver --version || sudo apt install firefox                   # Install Firefox (for geckodriver)
  chromedriver --version || sudo apt install chromium-chromedriver    # Install Chromium's chromedriver
  # build rust-browser-mcp from source:
  git clone https://github.com/EmilLindfors/rust-browser-mcp.git
  cd rust-browser-mcp
  cargo build --release
  # add the MCP to the config
  claude mcp add rust-browser-mcp -- rust-browser-mcp --transport stdio
  ```

## Testing

```sh
cargo test --features ssr
```

Unit tests are co-located in their source files using `#[cfg(test)]` modules.
Current coverage:

- **`auth.rs`** — JWT claims subject parsing (`parse_claims_sub`)
- **`components/initiative.rs`** — drag-and-drop reorder index calculation (`reorder_index`)
- **`vfs.rs`** — the bulk of them: path parsing and normalisation, permissions,
  quota, and the database operations against an in-memory SQLite, including
  that delete refuses a non-empty directory while the recursive form does not,
  and that a name containing `_` is not treated as a SQL wildcard
- **`modules/mod.rs`** — module id validation and asset URL resolution
- **`server/ws_handler/chat.rs`** — dice parsing, including `d66`/`d666`
- **`server/module_assets.rs`** — which file types a module may serve

Endpoints are covered separately by `ci/smoke-test-server.sh`, which exercises
the server functions over HTTP against a throwaway database.

See [TODO.md](TODO.md) for planned additional tests.

### Supported Platforms

  * [Firefox 115.32.0 ESR](https://ftp.mozilla.org/pub/firefox/releases/115.32.0esr/)
  * Chrome 138.0 (LTS-138) [Long Term Support](https://support.google.com/chrome/a/answer/11333726)

## Design Details

### Pages

- **Landing page** — introduction to the site with links to log in or sign up.
- **Login page** — handles user authentication via JWT. Supports login and signup with a toggle.
- **Sessions page** — lists active game sessions; lets users create or join sessions.
- **Game page** — main game view with map, chat, initiative tracker, inventory, character sheets, and creature stat blocks.

### Authentication

JWT tokens are stored in HttpOnly cookies set by the login/signup server
functions. The `get_current_user` server function extracts and validates the
token from the cookie header. WebSocket connections authenticate via a `token`
query parameter.

The login page requires HTTPS. This is enforced in two ways depending on
deployment:

- **Built-in TLS:** Set `TLS_CERT_PATH` and `TLS_KEY_PATH` env vars. The
  server runs HTTPS on port 3443 (or `TLS_PORT`) and an HTTP redirect server
  on the normal port that sends all traffic to HTTPS.
- **Reverse proxy:** When behind a proxy that sets `X-Forwarded-Proto`, the
  server redirects `/login` requests that arrive over plain HTTP to HTTPS.
- **Development:** With no TLS config and no proxy headers, the server runs
  plain HTTP with no redirects.

### Session creation limit

To curb abuse and denial of service, a user may be GM of at most
`MAX_ACTIVE_SESSIONS_PER_GM` (3) active sessions at once. `create_session` in
`server/api/session.rs` counts the user's active sessions and rejects the
request when the limit is reached. Archived (inactive) sessions do not count, so
an admin archiving or removing a session frees a slot.

### Server Administration

Administrators reach a console at `/admin` for managing accounts, sessions, and
media storage across the whole server.

- **Who is an admin:** the `users.access_level` column. `ACCESS_LEVEL_ADMIN`
  (100) and above grant access; `ACCESS_LEVEL_USER` (0) is a normal account
  (`models.rs`). The `ADMIN_LIST` environment variable, a comma-separated list of
  usernames, promotes matching accounts to admin at startup
  (`promote_admins_from_env` in `server/api/admin.rs`, called from `main.rs`).
  Promotion is additive: it never demotes an account, so admins granted in-app
  survive a restart, and removing a name from `ADMIN_LIST` does not revoke that
  account. Revoke in-app instead.
- **Enforcement:** every admin server function calls `require_admin` first, which
  loads the request's user and rejects non-admins. The `/admin` page also gates
  its UI on `current_user_is_admin`, but that is only cosmetic; the server
  functions are the real boundary.
- **Deleting sessions and users:** the database does not enable SQLite foreign
  keys, so most `sessions(id)` references are not enforced and `ON DELETE
  CASCADE` does not fire. `admin_remove_session` therefore deletes every
  dependent row (tokens, maps, characters, chat, and so on) in one transaction.
  `admin_remove_user` refuses to delete an account that still GMs a session (to
  avoid orphaning it) and otherwise removes the user plus their personal rows.
- **Server control** (the admin page's "Server" tab, `server/admin_handler.rs`):
  - *Restart* (`admin_restart_server`) exits the process after flushing the
    response. It relies on a supervisor (systemd, docker restart policy) to start
    the server again; run bare, it simply stops. The exit is a hard
    `process::exit`, which skips destructors so the connection pool cannot
    checkpoint stale WAL pages during shutdown.
  - *Backup* (`GET /api/admin/backup`) streams a `VACUUM INTO` snapshot of the
    database as a SQLite file download, consistent even with a live WAL.
  - *Restore* (`POST /api/admin/restore`, multipart) validates the upload is a
    SQLite file, copies the current database aside as a timestamped `.bak`,
    writes the upload over the configured database path, clears stale WAL
    sidecars, and restarts onto the restored database. It overwrites all current
    data, so like restart it needs a supervisor to come back up.

  Backup and restore are plain Axum routes rather than server functions because
  they move raw file bytes; both authenticate the cookie and require admin.

### Server status screen (TUI)

When the release binary's stdout is a TTY, it runs a full-screen ratatui status
screen (`server/tui.rs`), a Renegade BBS "Waiting for Caller" style console:
live stats (users online, 24h peak, active sessions, WebSocket connections),
request/message rate averages (EWMA, like Unix load), server info (uptime,
version, port, DB size), and scrollable active and recent session lists. Arrow
keys scroll, `q` quits (with a confirmation), and it redraws each tick so it
adapts to terminal resize. A panic hook restores the terminal.

The HTTP/WS server runs on tokio tasks while the TUI runs on a `spawn_blocking`
thread (`main.rs`). When stdout is not a TTY (for example under `cargo leptos
watch`), it falls back to plain logging and no TUI. The render path is DB-free:
it reads atomic counters and EWMA ring buffers plus an ephemeral session cache
in `server/metrics.rs` (`SERVER_METRICS`), which the hot paths update alongside
normal work (request middleware, WS connect/disconnect/message, and the
keepalive heartbeat refresh the session's last-active time). The cache may be
briefly out of sync with the database, which is acceptable for a sysop display.

### HTTPS / TLS Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `TLS_CERT_PATH` | (none) | Path to PEM certificate file. Enables built-in TLS when set with `TLS_KEY_PATH`. |
| `TLS_KEY_PATH` | (none) | Path to PEM private key file. |
| `TLS_PORT` | `3443` | Port for the HTTPS listener. |

To generate a self-signed certificate for development:

```sh
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj '/CN=localhost'
```

Then run with:

```sh
TLS_CERT_PATH=cert.pem TLS_KEY_PATH=key.pem cargo leptos serve
```

### WebSocket Protocol

On connect, the client calls the `get_ws_token` server function to obtain the
JWT token, then opens a WebSocket to `/api/ws?token=<jwt>`. It sends a `JoinSession`
message and receives a `SessionJoined` response with a full `GameStateSnapshot`.
After that, incremental events are broadcast to all connected clients. The server
is the single source of truth — clients send requests, the server validates and
broadcasts results.

All message types are fully implemented:

- **Chat:** messages and dice rolls (`NdN+M` notation, rolled server-side)
- **Tokens:** place, move, remove, HP updates (creature-linked tokens auto-init HP)
- **Bulk token move:** `MoveTokens`/`TokensMoved` — move multiple selected tokens
  in a single message
- **Token rotation:** `RotateTokens`/`TokensRotated` — rotate selected tokens
- **Token conditions:** `UpdateTokenConditions`/`TokenConditionsUpdated` — set
  status condition icons on tokens
- **Character placement:** `PlaceToken` with optional `character_id`/`creature_id`;
  `PlaceAllPlayerTokens` for GM bulk placement of all player characters
- **Fog of war:** `SetFogEnabled` turns fog on/off per map; `RevealFog`/`HideFog`
  edit revealed cells (all GM only). When fog is on, unrevealed cells are covered;
  players see an opaque overlay, the GM sees it faint and can hide it in their own
  view while editing. Revealed cells persist in the `fog_of_war` table (reveal
  inserts, hide deletes); `maps.fog_enabled` stores the per-map toggle. The GM
  paints with the Reveal and Cover brush tools, or Reveal/Cover the entire map.
- **Map:** switch active map (`SetMap`), set background (`SetMapBackground`).
  Map create/delete/list via server functions (not WebSocket).
- **Map settings:** `UpdateMapSettings`/`MapSettingsUpdated` (GM only) edit the
  active map's name, grid dimensions, cell size, grid origin offset, and default
  token color in one go, broadcasting the full updated map. The grid offset
  (`maps.grid_offset_x`/`grid_offset_y`) shifts the drawn grid, tokens, and fog
  together so the overlay grid can be aligned to a background image. The New Map
  and Map Settings forms share the same fields.
- **Ping:** `Ping`/`PingBroadcast` — collaborative map pings with per-user
  color (`SetPingColor`)
- **GM viewport sync:** `SyncViewport`/`ViewportSynced` — GM broadcasts viewport
  to all players
- **Initiative:** add/remove entries, advance turn (GM only), roll initiative
  from character sheet or creature panel, lock/unlock initiative rolls
- **Character sheets:** update fields via dot-path (e.g. `stats.strength`),
  real-time resource updates via `CharacterResourceUpdated`. When the HP resource
  changes, the server syncs the character's tokens' HP (`TokenHpUpdated`) and
  auto-toggles the "bloodied" condition (`TokenConditionsUpdated`) when HP crosses
  below/at half of max. This applies only to player-character tokens; creature
  bloodied status is left to the GM.
- **Temporary hit points:** `set_character_temp_hp` sets a character's temp HP
  (stored on `characters.temp_hp`, no maximum). The server mirrors it onto the
  character's tokens and broadcasts `CharacterTempHpUpdated` plus `TokenHpUpdated`
  (which now carries `temp_hp`). Damage spends temp HP before normal HP; the HP
  bars draw current HP in red and temp HP in blue, scaled to max HP + temp HP.
- **Inventory:** add, remove, update items
- **Progress clocks:** `CreateProgressClock`, `SetProgressClockFilled` (the +/-
  buttons), `EditProgressClock` (label and description), `SetProgressClockLock`,
  and `DeleteProgressClock` (all GM only). Any change reloads the session's clocks
  and broadcasts the full list as `ProgressClocksUpdated`. Clocks persist in the
  `progress_clocks` table and are included in `GameStateSnapshot`. A locked clock
  rejects advance and edit until unlocked. See Progress Clocks under the UI
  sections for the hub and per-clock windows.
- **VFS notifications:** `VfsChanged` — server broadcasts file changes on C: drive
- **Preferences:** `SetSuppressTooltips` — persist tooltip suppression preference

GM role is enforced server-side for token placement/removal, fog, map,
initiative list updates, initiative lock/unlock, and all progress-clock changes.

#### Game masters

A session has one owner (`sessions.gm_user_id`) plus any number of additional
GMs listed in the `session_gms` table. A user has GM privileges if they are the
owner or an additional GM. Every GM check routes through
`crate::server::gm::is_session_gm`, so the rule lives in one place; server
functions and WebSocket handlers both call it.

The owner manages co-GMs from the in-session Members window (GM-only; the
grant/revoke controls show only for the owner). It uses the `ListMembers` and
`SetMemberGm` client messages. When the owner grants or revokes GM, the affected
user receives a `GmStatusChanged` event and their UI flips live without
reconnecting, and GM clients get a `MembersChanged` prompt to refetch the list.
The owner is always a GM and cannot be demoted.

#### Connection lifecycle

The client WebSocket (`src/pages/game.rs`) reconnects automatically. `ws_connect`
wires the socket; on an unexpected close `schedule_reconnect` retries with
exponential backoff (1s to a 15s cap), fetching a fresh token each time, and a
successful open re-sends `JoinSession` so the snapshot restores all state. A
per-attempt `generation` guard keeps a superseded socket from reconnecting or
racing the current one. The loading modal shows `RECONNECTING`, then
`DISCONNECTED` (with a manual "Reconnect now") after several tries, clearing once
reconnected. The client also sends a `Keepalive { idle }` heartbeat once a
minute; the server refreshes the session's last-active time and ignores `idle`
for now (carried for future presence tracking).

### Server function URLs

Non-realtime calls go through Leptos `#[server]` functions in `server/api/`. Each
one gets a URL like `/api/<fn_name><suffix>`, where `<suffix>` is a 64-bit
integer. That suffix is generated at compile time by the `server_fn` macro as:

```text
xxh64( CARGO_MANIFEST_DIR + ":" + module_path!(), seed = 0 )
```

Two things determine it, and build profile (debug vs release) is not one of them:

- **`CARGO_MANIFEST_DIR`** — the absolute path of the crate when it was built. A
  local build in `/home/you/webrpg` and the Docker builder in `/app` produce
  different suffixes for the same function.
- **`module_path!()`** — the module the function lives in. Every function in one
  module shares a suffix, and moving a function to another module changes it. For
  example, splitting `api.rs` into `server/api/{auth,session,...}.rs` moved these
  from `webrpg::server::api` into submodules and rewrote all their suffixes.

This has one practical consequence and one non-consequence:

- **Not a deployment risk.** `cargo leptos build` compiles the WASM client and the
  server binary from the same source in the same directory, so they always agree
  on the suffix. A tarball or Docker image is internally consistent. The only way
  to get a mismatch is to pair a client from one build with a server from another
  build directory, which no build or deploy path does.
- **URLs are not reproducible across checkout paths.** The same commit built in a
  different directory yields different `/api/...` URLs. Nothing in this project
  depends on a fixed URL, but if that were ever needed, `server_fn` reads three
  compile-time env vars: `SERVER_FN_OVERRIDE_KEY` replaces `CARGO_MANIFEST_DIR`
  with a fixed string (path-independent suffixes), `SERVER_FN_MOD_PATH` puts the
  module path in the URL instead of a hash, and `DISABLE_SERVER_FN_HASH` drops the
  suffix entirely.

`ci/smoke-test-server.sh` never hard-codes a suffix: it extracts each one from the
freshly built `webrpg.wasm`, so it always matches the server it just built.

### Game Page Architecture

The game page (`pages/game.rs`) creates a `GameContext` provided via Leptos
context to all child components. `GameContext` holds `RwSignal`s for each piece
of game state (map, tokens, fog, chat, initiative, inventory) plus:

- `characters: RwSignal<Vec<CharacterInfo>>` — the single client-side cache of
  the session's characters, loaded once when the session id is known. The server
  pushes granular `CharacterUpdated` / `CharacterResourceUpdated` deltas, which
  are applied to this signal in place; windows (character selection, inventory,
  modules) read it reactively instead of each refetching the whole list.
- `character_revision: RwSignal<u32>` — bumped only to force a full reload of
  `characters` from the server for changes that are not broadcast as deltas
  (character create/delete). Field and resource edits apply in place and do not
  bump it.
- `initiative_locked: RwSignal<bool>` — whether character sheet initiative
  rolls are locked (GM toggle).
- `pings: RwSignal<Vec<(f64, f64, String, f64)>>` — active map pings
  (x, y, color, timestamp_ms). Auto-expired after 3 seconds.
- `viewport_override: RwSignal<Option<(f64, f64, f64)>>` — when set by a
  GM viewport sync, the map canvas jumps to the given (x, y, zoom).
- `center_on: RwSignal<Option<CenterTarget>>` — a request for the map to
  recenter on a token. `CenterTarget` is `Character(id)` / `TokenLabel(label)` /
  `TokenId(id)`; the map resolves whichever key it carries, centers, and clears
  the signal. One command in place of a separate signal per lookup key.
- `loading_status` / `loading_error: RwSignal<Option<String>>` — startup
  modal state.
- `send: Sender` — WebSocket send handle. The send closure captures the socket
  (a non-`Send` JS value), so on the client it lives in a `LocalStorage`
  `StoredValue`; on the server it holds nothing and `send()` is a no-op. See
  "SSR and non-`Send` reactive values" below.

Components read state reactively and send messages through the context:

- **MapCanvas** — HTML5 Canvas rendering with a full viewport pan/zoom system
  (`screen_to_world`/`world_to_screen` transforms). Features include:
  - **Tool palette:** floating toolbar with Select (V), Pan (H), Measure (M),
    and Ping (P) tools, plus Grid Snap toggle (G) and Token List toggle (T).
    Space held for temporary pan.
  - **Tokens:** colored circles or images clipped to circles, HP bars, fog of
    war overlay. Background images and token images loaded from the media system.
  - **Multi-select:** Shift+click to extend selection; rubber-band selection
    rectangle on empty canvas.
  - **Multi-drag:** drag multiple selected tokens with snap-to-grid.
  - **Token rotation:** right-click to rotate selected tokens.
  - **Token conditions:** emoji status icons displayed above tokens.
  - **Token list dropdown:** lists all tokens with click-to-center-on-token.
  - **Measurement tool:** click-and-drag line showing distance in grid squares
    and feet.
  - **Ping tool:** click to ping a map location, broadcast to all players with
    per-user color. Pings auto-expire after 3 seconds.
  - **Map management:** create maps (with image picker and DPI-based auto-sizing
    of grid dimensions), switch active map, delete maps. GM can set map
    background via the media browser.
  - **Character placement:** place individual characters from character sheet,
    or GM bulk-place all player characters.
  - **ResizeObserver** for proper canvas resize handling on window resize.
  - **Firefox ESR compatibility:** image decode retry logic for async-decode
    race conditions.
  - **Escape** clears measurement and selection.
- **ChatPanel** — message list with auto-scroll, input field that auto-detects
  dice notation (`NdN+M`). Last 100 messages loaded from DB on connect. Dice
  results persisted with structured JSON data. Messages styled with username in
  accent color and dice rolls in golden italic.
- **InitiativeTracker** — sorted initiative list showing value, portrait icon,
  and name per entry. "+" button for manual entry, "Next Turn" to advance.
  Lock/unlock toggle (GM) prevents character sheet initiative rolls when locked.
  Characters and creatures can roll initiative from their respective panels
  (d20 + DEX modifier + initiative bonus for D&D 5e), which automatically adds
  them to the tracker sorted by value. Rolls also appear in the chat log.
- **InventoryPanel** — item list with quantity controls, add/remove.
- **CharacterSheet** — template-driven character sheet editor. Fields are
  grouped by category and rendered by type (number, text, checkbox, textarea).
  Includes resource tracking bars (HP, spell slots) with +/- buttons and undo.
  "Roll Initiative" button between resource bars and ability scores (disabled
  when initiative is locked). Character field edits are sent as
  `UpdateCharacterField` WebSocket messages for real-time sync. The Character
  Selection list iterates the shared `characters` cache with a composite `<For>`
  key (id, portrait, data, resources), so an in-place delta update to a character
  re-renders just that row. Supports character portraits via the media browser.
- **CreaturePanel** — GM-only creature stat block CRUD. Create, edit, and
  delete creature stat blocks with template-driven stat fields. "Roll
  Initiative" button on each creature card (always enabled, ignores lock).
  Creature stat blocks are linked to tokens for HP auto-initialization.

- **MediaBrowser** — modal dialog for browsing, searching, and uploading media
  files. Supports image thumbnails, tag-based filtering with autocomplete, and
  text search. Used by map background picker, token image picker, and character
  portrait picker.

### SSR and non-`Send` reactive values

`cargo-leptos` renders every page on the server first (SSR), then hydrates it on
the client. The SSR render runs on the Axum/Tokio runtime, which is
multi-threaded: Leptos streams the response and can drop a page's reactive owner
on a different worker thread than the one that built it.

A reactive value created with a `new_local` constructor (`StoredValue::new_local`,
`RwSignal::new_local`, anything typed with `LocalStorage`) is **not `Send`** and
is wrapped in a `SendWrapper`. If such a value is created during the SSR render
and then dropped on another worker thread, `SendWrapper`'s `Drop` panics:

```
Dropped SendWrapper<T> variable from a thread different to the one it has been
created with.
```

The panic aborts the response mid-stream, so the browser receives a truncated
document (`ERR_INCOMPLETE_CHUNKED_ENCODING`), hydration never completes, and the
page is stuck "Connecting..." with no data. Because it depends on how Tokio
schedules the stream, it is a race: it can pass on one load and fail on the next,
and a change elsewhere (restructuring resources or `<Suspense>` boundaries,
which is what first exposed this) can start triggering it on most loads.

**Rule: never create a non-`Send` reactive value during the SSR render.** Either

- gate its creation behind `#[cfg(feature = "hydrate")]` when it is only useful
  on the client, e.g. the WebSocket `Sender` (holds nothing on the server) and
  the per-tab scratch-drive signal; or
- if the value is actually `Send + Sync` (holds only plain data), use the plain
  `StoredValue::new` / `RwSignal::new` instead of the `new_local` variant, e.g.
  the terminal's `ShellState` (just a `VfsPath`).

To check for a regression, load an authenticated `/game/:id` repeatedly rather
than once (it is a race) and confirm the server log has no `SendWrapper` panic
and every response is the full document:

```sh
for i in $(seq 1 24); do curl -s -b cookies.txt "$BASE/game/1" | wc -c; done
```

### Theming (light/dark)

All colors live as CSS custom properties on `:root` in `style/main.css`. The
default values are the dark blue/red game theme. A sandy brown light palette
redefines the same `--color-*` tokens under `:root[data-theme="light"]`, and a
`@media (prefers-color-scheme: light)` block carries the same values for
visitors with no saved choice and JavaScript disabled. Every rule in the
stylesheet reads tokens, so switching the `data-theme` attribute on the root
element reskins the whole app.

Three subsystems keep fixed palettes on purpose and do not flip: the NeXTSTEP
gray chrome (`--chrome-*` tokens: dock, file browser), the COMMAND.COM terminal
(a dark console), and the help viewer's white "paper" view. `--color-on-accent`
is also fixed (text drawn on an accent-filled element stays light in both
themes).

The effective theme is resolved before first paint by a small inline script in
the document head (`app::shell`, `THEME_INIT_SCRIPT`): it reads the stored
preference from `localStorage` (`webrpg_theme`), falling back to the OS color
scheme, and sets `data-theme` on the root element. This avoids any flash of the
wrong theme. The `ThemeToggle` component (`components/theme_toggle.rs`) is a
floating button rendered on every route; clicking it flips the attribute live
and persists the choice. With no stored choice the app follows the OS setting
across reloads; an explicit toggle pins the theme until changed again.

### Multi-Window UI

The game page uses a windowed interface where each feature lives in its own
draggable, resizable window (`components/window_manager.rs`). The
`WindowManager` component wraps the game viewport and renders `GameWindow`
children. Each window has a title bar (drag handle, minimize/close buttons),
resizable edges/corners, and z-index stacking (click to front). Window layout
is persisted to `localStorage`.

Minimized windows appear as 64×64 tiles in a NeXTSTEP-style dock in the
upper-left corner. The dock has a fixed system icon anchor at (0,0) and tiles
snap to a 2D grid adjacent to existing tiles. Clicking a dock tile restores the
window; long-pressing and dragging repositions the tile within the dock grid
(with a snap preview ghost tile). Non-minimized windows are pushed away from
the dock area when new tiles appear. Dock tile layout is persisted to
`localStorage`.

Default windows: Map (large, center), Chat (right), Character Sheet, Initiative,
Inventory (minimized), Creatures (GM only).

See [doc/dock-design.md](doc/dock-design.md) for a language-neutral write-up of
the dock's design, grid/snapping algorithm, lessons learned, and limitations,
intended for reusing the approach in other projects.

### Progress Clocks

Progress clocks are segmented circles persistent to the session, used to track
countdowns and mounting threats. A clock has a fixed segment count (set at
creation), a filled count that advances from the top clockwise, configurable
filled and empty colors, a label, a description, and a lock.

The UI follows the Character Selection / Character Editor split. A static
"Progress Clocks" hub window (`components/progress_clock.rs`, `ProgressClockHub`)
lists every clock in the session; clicking "Open" launches that clock in its own
dynamic window (`WindowId::ProgressClock(clock_id)`, rendered by
`DynamicProgressClockWindows` in `pages/game.rs`). Which clock windows are open
is per-client window-manager state; the clock data itself is shared.

GMs create, advance (+/-), edit, lock, and delete clocks; players see the hub
and can open clocks but get a read-only view with no controls. The lock guards a
clock against accidental changes: while locked, advance and edit are rejected
server-side, and only the GM can unlock it. The clock face is rendered as an
inline SVG of pie wedges. See the Progress clocks entry under WebSocket Protocol
for the message set.

### Media Storage

Media files (images and audio) use content-addressable storage (CAS). Files are
stored on disk by SHA-256 hash under `uploads/media/` (configurable via
`MEDIA_DIR` env var), sharded by first two hex characters. Upload via multipart
POST to `/api/media/upload` (JWT auth from cookie, 20 MB limit). Serve via
`GET /api/media/:hash` with immutable cache headers. Supported types: PNG, JPG,
GIF, WebP (images), WAV, MP3 (audio). Tags are stored in the `media_tags` table;
the original filename is automatically added as a tag on upload.

### Virtual File System

The VFS provides a unified file system abstraction across multiple storage
backends, exposed through both a command-line terminal (COMMAND.COM) and a
graphical file browser (File Viewer).

**Drive letters:**

| Drive | Scope | Storage | Description |
|-------|-------|---------|-------------|
| A:, B: | Per-tab | IndexedDB (browser) | Scratch drives — ephemeral, client-side only |
| C: | Per-session | SQLite (server) | Session-scoped shared storage |
| U: | Per-user | SQLite (server) | User-scoped persistent storage |

**Scratch drive limitations (A: and B:):**

- Data is stored in the browser's IndexedDB and is not shared between tabs or
  users. Each tab gets its own isolated scratch drives.
- IndexedDB does not support native rename. Renaming a file reads the content,
  writes it to the new path, and deletes the original. Renaming a directory
  recursively moves all children.
- Directory rename is limited to **64 levels of nesting**. Operations on deeper
  directory trees will fail with an error. This limit prevents stack overflow in
  the browser's WASM runtime.
- Scratch drive data is lost when the tab is closed.

**File Viewer (Finder):**

The File Viewer is a graphical file browser (`src/components/file_browser.rs`)
inspired by the NeXTSTEP File Viewer. It uses icon grids and inset panels but
toolbar buttons have not yet been re-themed to full NeXTSTEP style. It provides:

- Drive list root view with A:/B:/C:/U: icons
- Icon grid directory view with file type icons and labels
- Toolbar: back, forward, up, new folder, upload, rename, download, delete
- Editable location bar with Enter-to-navigate
- Multi-select with Ctrl+click (toggle) and Shift+click (range)
- Right-click context menu (download, rename, delete)
- Double-click file preview (text as monospace, images scaled to fit)
- Status bar with item count, selection count, and drive quota

**COMMAND.COM:**

The terminal emulator (`src/components/terminal.rs`) provides a DOS-style
command-line interface to the VFS. Commands: ATTRIB, CD, CLS, COPY, DEL, DIR,
EXIT, GET, HELP, MKDIR, PUT, RMDIR, TYPE, VER. Bare drive letters (e.g. `C:`)
switch drives. EXIT minimizes the terminal to the dock. The command set is
lifted from MSX-DOS (not MS-DOS) with some tweaks.

See [doc/shell-design.md](doc/shell-design.md) for a language-neutral write-up
of the shell's design, MSX-DOS lineage and deviations, parse/dispatch
architecture, lessons learned, limitations, and extension ideas, intended for
reusing the approach in other projects.

**Help documentation:**

Help pages for the online help system (Feature 38) are authored as Markdown
files in the `help/` directory. Each file covers one topic with a slug-based
filename (e.g. `file-viewer.md`, `command-com.md`). Cross-references use
`[link text](help:topic-slug)` syntax.

Installed game modules contribute help pages too, resolved by slug after the
`help/` directory is checked, so a module's `help:tunnel-goons` link works
without the server shipping that file.

### Game Modules

A game module is a directory of JSON under `MODULES_DIR` (default `modules/`)
that supplies either an RPG system or an adventure. Nothing about a module is
compiled in. See [modules/README.md](modules/README.md) for the pack format.

- **System modules** carry the character sheet schema, the creature schema, and
  a roll model (`rules.json`). Installing one seeds its sheet as an
  `rpg_templates` row and points the session at it.
- **Adventure modules** carry a bestiary, pregens, item cards, a room key,
  tables, and maps, and name the system they were written for. Installing one
  seeds creatures and maps.
- **Reference modules** carry tables and item cards only. They are never
  installed and every session sees them, so nothing secret belongs in one.

Three modules ship with the repo: `tunnel-goons` (system), `sky-blind-spire`
(adventure), and `cairn-spellbooks` (reference: 216 Cairn spells as a d666 table
and one spellbook card per spell, generated from `spells.txt` by
`scripts/build-cairn-spells.sh`).

`d66` and `d666` roll as table dice: two or three d6 read as digits in order, so
3, 1, 5 is entry 315 rather than one 666-sided die.

Sessions record what they run in `sessions.system_module_id` and
`sessions.adventure_module_id`. `GameContext::modules` holds the result, so the
client knows which roll model to present.

Packs are read per request rather than cached, so editing a module's JSON takes
effect without a restart. A pack that fails to parse is skipped with a log line.

**Who sees what.** `get_adventure_module` returns the room key, bestiary, and
tables, and refuses anyone but the session's GM. Players call
`get_adventure_handouts`, which returns only pregens and item cards. Anything
secret belongs in a room's `gm` field, never in its `card`.

**Licensing.** The repository is MIT-0; `modules/` is not. Bundled modules carry
third-party content whose licences impose conditions MIT-0 disclaims, including
one non-commercial and one share-alike work, so a module cannot be assumed to
inherit the repository's terms. Each `module.json` states its own licence and
what was changed, and [modules/LICENSING.md](modules/LICENSING.md) collects the
detail. A new module that reproduces someone else's wording needs the same:
author, licence, link, and a note of what was altered.

**Module art.** Files in a module's `assets/` are served by
`GET /api/modules/{module_id}/assets/{path}` straight off disk, for image types
only, with traversal refused in `modules::loader::asset_path`. An item card
names its art as `cards/torch.png`, or `other-module:cards/torch.png` to borrow
another module's deck. Map art named in `maps.json` is instead ingested into
media storage at install time, since maps carry a stored background URL.

The Tunnel Goons card deck is generated from ASCII sources:

```sh
scripts/ascii-cards-to-png.sh     # modules/tunnel-goons/cards/*.card -> assets/cards/*.png
```

### Check Roller

Systems built on "roll dice, add an ability, add what is helping, compare to a
difficulty" cannot be expressed by the `NdN+M` chat parser, which knows nothing
about abilities, items, or the margin. `ClientMessage::RollCheck` carries the
ability, the bonuses the player ticked, an optional DS, and whether the action
was dangerous; the server rolls, computes the margin, and broadcasts the result
as a chat message with structured JSON in `dice_result`.

Which items help is a judgement call, not a lookup, so the player picks them.
The DS is optional because players usually have not been told it.

The roller appears on the character sheet only when the session runs a system
module. It comes from `rules.json`, so a system with different abilities or a
different die needs no code change.

### Inventory Slots

`inventory_items` carries `slots`, `kind`, `bonus`, `uses_max`, and `uses_left`,
and items can be owned by a character or held by the party. The inventory window
totals slots per character against the capacity field the system module names,
and warns when a character is over it.

The rules stay data: the penalty for being over capacity, and which abilities it
applies to, come from `rules.inventory`, and the check roller applies it.

Movement and positioning are deliberately not enforced anywhere. Tunnel Goons
has no skirmish system, and the map is a shared pointing device rather than a
rules engine.

## CI / CD

### Pull Requests

Every PR targeting `main` runs four parallel jobs:

| Job | What it does |
|-----|-------------|
| **Compile & Test** | Diesel migrations on a fresh DB, `cargo check` for both SSR and hydrate targets, `cargo test` |
| **Formatting** | `cargo fmt --check` |
| **Clippy** | `cargo clippy` for both SSR and hydrate targets with `-D warnings` |
| **Smoke Test** | Full cargo-leptos build, then `ci/smoke-test-server.sh` (starts server, exercises all endpoints) |

All four jobs must pass before a PR can be merged.

### Releases

Pushing a tag matching `v*` (e.g. `v0.2.0`) to `main` triggers the release
workflow:

1. **Test** — same checks as the PR workflow (migrations, compile, unit tests)
2. **Build** — `cargo leptos build --release`, packages server binary + site
   assets + migrations into a tarball
3. **Smoke Test** — runs the smoke test suite against a debug build
4. **Publish** — creates a GitHub Release with auto-generated release notes and
   attaches the tarball

### CI Scripts

The `ci/` directory contains the test scripts used by both workflows:

- `ci/check-compile.sh` — checks both SSR and hydrate targets compile without
  warnings, runs `cargo test`
- `ci/check-migrations.sh` — applies migrations to a fresh SQLite database,
  verifies all expected tables and columns exist, tests rollback/redo
- `ci/smoke-test-server.sh` — builds and starts the server, then exercises page
  routes, CSS serving, signup, session CRUD, WebSocket endpoint, media
  upload/serve/dedup/tags, invalid input handling, game page rendering, and
  server administration (admin guard, user/session/media management)

## Deployment

### Building a Release Tarball

```sh
scripts/build-release.sh
```

This builds the server as a fully static musl binary (no glibc dependency) and
packages it with site assets and Diesel migrations into
`target/webrpg-<version>.tar.gz`. The resulting binary runs on any x86_64 Linux
regardless of the host's libc version.

Requires musl toolchain (see [Build Prerequisites](#build-prerequisites)).

### Deploying to a Remote Server

```sh
scripts/deploy.sh user@host [tarball]
```

If no tarball is specified, the script auto-detects the latest one in `target/`.
It uploads the tarball via SCP, unpacks it into `~/webrpg/<release>/`, and
creates a `current` symlink pointing to the new release. The `.env` file,
`database.db`, and `uploads/` directory are kept at `~/webrpg/` and symlinked
into the release, so they persist across deploys.

**First deploy:**

1. Run `scripts/build-release.sh` locally
2. Run `scripts/deploy.sh user@host`
3. SSH in and copy `~/webrpg/current/env.example` to `~/webrpg/.env`, edit it
4. Start the server: `cd ~/webrpg/current && ./webrpg`

The server applies any pending Diesel migrations at startup (migrations are
compiled into the binary), so a fresh `database.db` is created and brought up to
schema on first run. No manual `diesel migration run` is needed. The Diesel CLI
is still useful for rolling a migration back.

**Subsequent deploys:**

1. Run `scripts/build-release.sh`
2. Run `scripts/deploy.sh user@host`
3. Restart the server

### Running with Docker

A multi-stage [Dockerfile](Dockerfile) builds the static musl server and the
WASM client, then copies them into a minimal Alpine runtime image. The database
and uploaded media live under `/app/data`, exposed as a volume, so they survive
image upgrades. Migrations run at startup.

The simplest path is Docker Compose:

```sh
echo "SECRET_KEY=$(openssl rand -hex 32)" >> .env   # signs auth cookies, keep it stable
docker compose up -d --build
```

The app is then on `http://localhost:3000`. To run the image directly instead:

```sh
docker build -t webrpg .
docker run -d -p 3000:3000 \
    -e SECRET_KEY="$(openssl rand -hex 32)" \
    -v webrpg-data:/app/data \
    webrpg
```

`SECRET_KEY` has no default and must be supplied; keep it stable across restarts
or existing login cookies stop validating. For TLS, terminate at a reverse proxy
in front of the container, or set `TLS_CERT_PATH`, `TLS_KEY_PATH`, and `TLS_PORT`
(see [HTTPS / TLS Configuration](#https--tls-configuration)) and publish that
port. Publishing images to a registry is not set up yet.

## Feature Requests

Feature requests are tracked as individual files in `doc/feature-requests/`. Each
file contains the feature description, current progress, and status. This
replaces any global planning document.

**Workflow:**

1. Create a file in `doc/feature-requests/` describing the feature
2. Update progress and status in the file as work proceeds
3. When the feature is complete, include the feature request content in the git
   commit message and delete the feature request file

## Contributing

### Getting Started

1. Fork and clone the repo
2. Follow the setup steps in [README.md](README.md)
3. Create a feature branch from `main`
4. Make your changes, keeping commits focused and well-described
5. Open a PR against `main`

### Code Standards

- **Both targets must compile cleanly.** Run both checks before pushing:
  ```sh
  cargo check --features ssr
  cargo check --features hydrate --target wasm32-unknown-unknown
  ```
- **No warnings.** CI runs with `-D warnings` on clippy. Fix all warnings
  before submitting.
- **Format with rustfmt.** Run `cargo fmt` before committing. CI enforces this.
- **Run tests.** `cargo test` must pass. Add tests for new server-side logic
  when practical.
- **Feature-gate correctly.** Server-only code behind `#[cfg(feature = "ssr")]`,
  client-only code behind `#[cfg(feature = "hydrate")]`. Imports used only
  inside `#[server]` function bodies go inside the function.

### PR Guidelines

- Keep PRs focused on a single change. Large features should be broken into
  reviewable chunks.
- Write a clear description of what the PR does and why.
- If the PR changes the database schema, include a migration with both `up.sql`
  and `down.sql`. The `down.sql` must actually undo the change (not be a no-op).
  Diesel CLI regenerates `src/schema.rs` on every `diesel migration run`, so
  commit the regenerated file. Hand edits to it do not survive: corrections live
  in `src/schema.patch`, which `diesel.toml` applies after generation. That is
  where the `size_bytes -> BigInt` fix lives, since SQLite reports those columns
  as INTEGER while the code uses `i64`.
- If the PR adds new endpoints or server functions, add corresponding checks to
  the smoke test script.
- Update `DEV.md` if the change affects architecture, build steps, or
  configuration.
- All CI checks must pass before merge.

### Commit Messages

Use concise, imperative-mood commit messages that describe the change:

```
Add media upload endpoint with SHA-256 dedup
Fix token HP popup not closing on click outside
Update initiative tracker to highlight current turn
```

### RPG Template System

Templates define the fields available on character sheets and creature stat
blocks via a JSON schema of `TemplateField` entries. Each field has a name,
label, type (`number`, `text`, `boolean`, `textarea`), category, and default
value. A default D&D 5e template is provided via the `seed_default_template`
server function (idempotent). Sessions can be assigned a template; characters
created in that session are initialized with the template's defaults.
