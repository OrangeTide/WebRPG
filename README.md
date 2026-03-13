# WebRPG

A virtual tabletop for roleplaying games, built with Rust.

WebRPG hosts multiplayer RPG sessions with real-time synchronization over
WebSockets. Features include dice rolling, grid maps with fog of war, token
placement, character sheets, chat, party inventory, initiative tracking, and
creature stat blocks.

## Prerequisites

- Rust (stable, 1.85+)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- `cargo-leptos`: `cargo install cargo-leptos`
- SQLite3 development libraries (e.g. `libsqlite3-dev` on Debian/Ubuntu)
- Diesel CLI: `cargo install diesel_cli --no-default-features --features sqlite`

## Setup

```sh
# Clone and enter the project
cd webrpg

# Create .env if it doesn't exist
echo 'DATABASE_URL=database.db' >> .env
echo 'SECRET_KEY=change-me-in-production' >> .env

# Run database migrations
diesel migration run
```

## Running

```sh
# Development server with hot-reload
cargo leptos serve

# Then open http://localhost:3000
```

For a release build:

```sh
cargo leptos build --release
```

The output is a server binary at `target/server/release/webrpg` and static
assets in `target/site/`. To deploy, copy both and set these environment
variables:

```sh
LEPTOS_OUTPUT_NAME=webrpg
LEPTOS_SITE_ROOT=site
LEPTOS_SITE_PKG_DIR=pkg
LEPTOS_SITE_ADDR=0.0.0.0:3000
DATABASE_URL=database.db
SECRET_KEY=your-secret-key
```

See [DEV.md](DEV.md) for architecture, project structure, and testing details.
