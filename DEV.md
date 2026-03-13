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
├── components/       # UI components (map, chat, inventory, initiative)
├── server/           # Server functions + WebSocket upgrade handler
└── ws/               # WebSocket message types + session state manager
migrations/           # Diesel SQL migrations
```

## Build Prerequisites

- Rust (stable, 1.85+)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- `cargo-leptos`: `cargo install cargo-leptos`
- SQLite3 development libraries (e.g. `libsqlite3-dev` on Debian/Ubuntu)
- Diesel CLI: `cargo install diesel_cli --no-default-features --features sqlite`

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
- **Game page** — main game view with map, chat, initiative tracker, and inventory.

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

- **MapCanvas** — HTML5 Canvas rendering with grid, tokens (colored circles with
  labels), HP bars, fog of war overlay. Drag-and-drop token movement.
- **ChatPanel** — message list, input field that auto-detects dice notation.
- **InitiativeTracker** — sorted entry list with current-turn highlight, add/remove/advance.
- **InventoryPanel** — item list with quantity controls, add/remove.
