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
├── pages/            # Page components (landing, login, sessions, game)
├── components/       # UI components (map, chat, inventory, initiative, media browser)
├── server/           # Server functions, media handler + WebSocket upgrade handler
└── ws/               # WebSocket message types + session state manager
migrations/           # Diesel SQL migrations
```

## Build Prerequisites

For general development:

  - Rust (stable, 1.85+)
  - `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
  - `cargo-leptos`: `cargo install cargo-leptos`
  - SQLite3 development libraries (e.g. `libsqlite3-dev` on Debian/Ubuntu)
  - Diesel CLI: `cargo install diesel_cli --no-default-features --features sqlite`

For AI and MCP, commands to check and install requirements (idempotent):

Debian/ubuntu commands: #TODO: setup environment: "WEBDRIVER_PREFERRED_DRIVER": "chrome", "WEBDRIVER_HEADLESS": "true"
  ```bash
  geckodriver --version || sudo apt install firefox                   # Install Firefox (for geckodriver)
  chromedriver --version || sudo apt install chromium-chromedriver    # Install Chromium's chromedriver
  # build from source:
  git clone https://github.com/EmilLindfors/rust-browser-mcp.git
  cd rust-browser-mcp
  cargo build --release
  # add the MCP to the config
  claude mcp add rust-browser-mcp -- rust-browser-mcp --transport stdio
  ```

## Testing

```sh
cargo test
```

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
JWT token, then opens a WebSocket to `/ws?token=<jwt>`. It sends a `JoinSession`
message and receives a `SessionJoined` response with a full `GameStateSnapshot`.
After that, incremental events are broadcast to all connected clients. The server
is the single source of truth — clients send requests, the server validates and
broadcasts results.

All message types are fully implemented:

- **Chat:** messages and dice rolls (`NdN+M` notation, rolled server-side)
- **Tokens:** place, move, remove, HP updates (creature-linked tokens auto-init HP)
- **Fog of war:** reveal/hide cells (GM only)
- **Map:** switch active map (GM only)
- **Initiative:** add/remove entries, advance turn (GM only)
- **Character sheets:** update fields via dot-path (e.g. `stats.strength`)
- **Inventory:** add, remove, update items

GM role is enforced server-side for token placement/removal, fog, map, and
initiative operations.

### Game Page Architecture

The game page (`pages/game.rs`) creates a `GameContext` provided via Leptos
context to all child components. `GameContext` holds `RwSignal`s for each piece
of game state (map, tokens, fog, chat, initiative, inventory) and a
`StoredValue<SendFn, LocalStorage>` for sending WebSocket messages.

Components read state reactively and send messages through the context:

- **MapCanvas** — HTML5 Canvas rendering with grid, tokens (colored circles or
  images clipped to circles), HP bars, fog of war overlay. Supports background
  images and token images loaded from the media system. Drag-and-drop token
  movement. GM can set map background via the media browser.
- **ChatPanel** — message list, input field that auto-detects dice notation.
- **InitiativeTracker** — sorted entry list with current-turn highlight, add/remove/advance.
- **InventoryPanel** — item list with quantity controls, add/remove.
- **CharacterSheet** — template-driven character sheet editor. Fields are
  grouped by category and rendered by type (number, text, checkbox, textarea).
  Includes resource tracking bars (HP, spell slots) with +/- buttons. Character
  field edits are sent as `UpdateCharacterField` WebSocket messages for
  real-time sync. Supports character portraits via the media browser.
- **CreaturePanel** — GM-only creature stat block CRUD. Create, edit, and
  delete creature stat blocks with template-driven stat fields. Creature stat
  blocks are linked to tokens for HP auto-initialization.

- **MediaBrowser** — modal dialog for browsing, searching, and uploading media
  files. Supports image thumbnails, tag-based filtering with autocomplete, and
  text search. Used by map background picker, token image picker, and character
  portrait picker.

### Media Storage

Media files (images and audio) use content-addressable storage (CAS). Files are
stored on disk by SHA-256 hash under `uploads/media/` (configurable via
`MEDIA_DIR` env var), sharded by first two hex characters. Upload via multipart
POST to `/api/media/upload` (JWT auth from cookie, 20 MB limit). Serve via
`GET /api/media/:hash` with immutable cache headers. Supported types: PNG, JPG,
GIF, WebP (images), WAV, MP3 (audio). Tags are stored in the `media_tags` table;
the original filename is automatically added as a tag on upload.

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
