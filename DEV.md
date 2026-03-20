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
│   ├── inventory.rs        # Party inventory management
│   ├── initiative.rs       # Initiative tracker with turn order
│   ├── media_browser.rs    # Media upload/browse/search modal
│   ├── file_browser.rs     # NeXTSTEP-style graphical file browser (Finder)
│   ├── terminal.rs         # DOS-style COMMAND.COM terminal emulator
│   ├── help_viewer.rs      # Online help viewer (Markdown-based)
│   └── browser_helpers.rs  # Browser utility functions
├── server/
│   ├── mod.rs            # Module declarations
│   ├── api.rs            # Leptos server functions (sessions, characters, templates)
│   ├── media_handler.rs  # Media upload/serve endpoints (CAS)
│   └── ws_handler.rs     # WebSocket upgrade + authentication
├── vfs.rs            # Virtual file system abstraction (drive dispatch)
├── scratch_drive.rs  # Client-side IndexedDB scratch drives (A:/B:)
└── ws/
    ├── mod.rs            # Module declarations
    ├── messages.rs       # WebSocket message type definitions
    └── session.rs        # Server-side session state manager
migrations/               # Diesel SQL migrations
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
  - Install cargo leptos: `cargo install cargo-leptos`
  - Install Diesel CLI tools: `cargo install diesel_cli --no-default-features --features sqlite`

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
Current test coverage includes:

- **`auth.rs`** — JWT claims subject parsing (`parse_claims_sub`)
- **`components/initiative.rs`** — drag-and-drop reorder index calculation (`reorder_index`)

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
- **Fog of war:** reveal/hide cells (GM only)
- **Map:** switch active map (`SetMap`), set background (`SetMapBackground`).
  Map create/delete/list via server functions (not WebSocket).
- **Ping:** `Ping`/`PingBroadcast` — collaborative map pings with per-user
  color (`SetPingColor`)
- **GM viewport sync:** `SyncViewport`/`ViewportSynced` — GM broadcasts viewport
  to all players
- **Initiative:** add/remove entries, advance turn (GM only), roll initiative
  from character sheet or creature panel, lock/unlock initiative rolls
- **Character sheets:** update fields via dot-path (e.g. `stats.strength`),
  real-time resource updates via `CharacterResourceUpdated`
- **Inventory:** add, remove, update items
- **VFS notifications:** `VfsChanged` — server broadcasts file changes on C: drive
- **Preferences:** `SetSuppressTooltips` — persist tooltip suppression preference

GM role is enforced server-side for token placement/removal, fog, map,
initiative list updates, and initiative lock/unlock.

### Game Page Architecture

The game page (`pages/game.rs`) creates a `GameContext` provided via Leptos
context to all child components. `GameContext` holds `RwSignal`s for each piece
of game state (map, tokens, fog, chat, initiative, inventory) plus:

- `character_revision: RwSignal<u32>` — bumped on any character data/resource
  change; listeners (e.g. character selection list) track this to trigger
  refetches.
- `initiative_locked: RwSignal<bool>` — whether character sheet initiative
  rolls are locked (GM toggle).
- `pings: RwSignal<Vec<(f64, f64, String, f64)>>` — active map pings
  (x, y, color, timestamp_ms). Auto-expired after 3 seconds.
- `viewport_override: RwSignal<Option<(f64, f64, f64)>>` — when set by a
  GM viewport sync, the map canvas jumps to the given (x, y, zoom).
- `loading_status` / `loading_error: RwSignal<Option<String>>` — startup
  modal state.
- `send: StoredValue<Option<SendFn>, LocalStorage>` — WebSocket send
  function (non-`Send` JS type, stored with `LocalStorage`).

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
  Selection list uses a composite `<For>` key that includes portrait, data,
  and resources so that any change (portrait, HP, stats) triggers a re-render.
  Supports character portraits via the media browser.
- **CreaturePanel** — GM-only creature stat block CRUD. Create, edit, and
  delete creature stat blocks with template-driven stat fields. "Roll
  Initiative" button on each creature card (always enabled, ignores lock).
  Creature stat blocks are linked to tokens for HP auto-initialization.

- **MediaBrowser** — modal dialog for browsing, searching, and uploading media
  files. Supports image thumbnails, tag-based filtering with autocomplete, and
  text search. Used by map background picker, token image picker, and character
  portrait picker.

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
switch drives. EXIT minimizes the terminal to the dock.

**Help documentation:**

Help pages for the online help system (Feature 38) are authored as Markdown
files in the `help/` directory. Each file covers one topic with a slug-based
filename (e.g. `file-viewer.md`, `command-com.md`). Cross-references use
`[link text](help:topic-slug)` syntax.

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
  upload/serve/dedup/tags, invalid input handling, and game page rendering (26
  checks)

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
4. Install Diesel CLI on the server and run `cd ~/webrpg/current && diesel migration run`
5. Start the server: `cd ~/webrpg/current && ./webrpg`

**Subsequent deploys:**

1. Run `scripts/build-release.sh`
2. Run `scripts/deploy.sh user@host`
3. Restart the server

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
