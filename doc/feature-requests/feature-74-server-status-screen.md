# Feature 74: Text-Based Server Status Screen

A full-screen, interactive TUI status screen displayed on the server's terminal,
inspired by the Renegade BBS "Waiting for Caller" (WFC) screen. Uses a blue
background with cyan, white, and yellow text — the classic BBS sysop aesthetic.

The screen is the server's primary console view while running. It responds to
terminal resize (SIGWINCH) and supports arrow-key navigation for scrollable
panels, similar to `top` or `htop`.

## Visual Design

Inspired by the Renegade BBS WFC screen layout:

- **Background:** Deep blue (ANSI blue / `#0000AA`)
- **Panel borders:** Cyan box-drawing characters
- **Labels:** Cyan or white
- **Values:** Bright white or yellow for emphasis
- **Section headers:** Yellow or bright cyan

### Panel Layout

```
┌─ Server Status ─────────────────────────────────────────────────────────┐
│  3:10 pm       WebRPG vX.Y.Z Server                       2026-03-25  │
├─ Live Stats ──────────┬─ Averages ────────────┬─ Server Info ──────────┤
│  Users Online     3   │  Req/min (1m)   12.4  │  Uptime    2d 04:17   │
│  Peak (24h)       7   │  Req/min (5m)    9.8  │  Version   0.2.1      │
│  Sessions Active  2   │  Req/min (15m)   8.1  │  Port      3000       │
│  WebSocket Conns  5   │  WS Msgs/min    22.3  │  DB Size   4.2 MB     │
├─ Active Sessions ─────┴───────────────────────┴────────────────────────┤
│  # │ Session Name         │ GM           │ Players │ Created          │
│  1 │ Dragon's Lair        │ alice        │ 3       │ 2026-03-24 19:00 │
│  2 │ Tomb of Horrors       │ bob          │ 2       │ 2026-03-25 14:30 │
├─ Recent Sessions (last 5) ─────────────────────────────────────────────┤
│  # │ Session Name         │ GM           │ Last Active              │
│  1 │ Starter Adventure    │ carol        │ 2026-03-23 21:15         │
│  2 │ Test Campaign        │ alice        │ 2026-03-22 10:00         │
├─ Status ───────────────────────────────────────────────────────────────┤
│  Server running...                                                     │
└────────────────────────────────────────────────────────────────────────┘
```

## Interaction

- **Arrow keys (Up/Down):** Scroll through Active Sessions and Recent Sessions
  lists when they overflow their panel
- **`q`:** Quit / shut down server (with confirmation prompt)
- **Terminal resize:** Redraws to fill available terminal size

## Metrics

### Live Stats Panel
- Users currently online (connected WebSocket clients)
- Peak users in the last 24 hours (rolling window)
- Number of active game sessions
- Number of open WebSocket connections

### Averages Panel
- HTTP requests per minute — 1-minute, 5-minute, and 15-minute exponentially
  weighted moving averages (modeled after Unix load averages)
- WebSocket messages per minute (1-minute EWMA)

### Server Info Panel
- Server uptime
- Application version
- Listening port
- Database file size

### Active Sessions Panel
- Scrollable list of currently active game sessions
- Shows session name, GM username, player count, creation time

### Recent Sessions Panel
- Last 5 most recently active sessions (not currently active)
- Shows session name, GM username, last activity time

## Implementation Notes

- Use a TUI library such as `ratatui` + `crossterm` for terminal rendering
- Run the TUI on the main server thread's terminal; the HTTP/WS server runs
  on async tasks in the background
- Metrics collection should be lightweight — atomic counters and a small
  ring buffer, not a full metrics framework
- The 24-hour peak is a rolling max over a circular buffer of per-minute samples
- EWMA smoothing factor: alpha = 2/(N+1) where N is the window in minutes
- Session data for the TUI is served from an in-memory round-robin cache, not
  from DB queries. The server pushes session info (name, GM, player count,
  timestamps) into the cache alongside normal DB writes. The cache is purely
  ephemeral and informational — it may be briefly out-of-sync with the DB and
  that is acceptable for sysop display purposes. This keeps the TUI render path
  completely DB-free.

## Dependencies

(none)

## Status: Done

Implemented as described: `server/metrics.rs` (atomic counters, EWMA ring
buffers, 24h peak, ephemeral session cache) and `server/tui.rs` (the ratatui WFC
screen with all panels, arrow-key scrolling, `q`-to-quit with confirmation, a
panic hook that restores the terminal, and redraw-per-tick resize handling).
`main.rs` runs the HTTP/WS server on tokio tasks and the TUI on `spawn_blocking`,
falling back to plain logging when stdout is not a TTY. The hot paths (request
middleware, WS connect/disconnect/message, and the feature-56 keepalive) feed the
metrics, and the render path is DB-free. Documented in DEV.md ("Server status
screen (TUI)").

Verified by running the server under a PTY: the full screen renders (title bar,
Live Stats, Averages, Server Info, Active/Recent Sessions, status line) and `q`
then `y` quits cleanly, restoring the terminal.

## Plan

### Architecture

The TUI runs on a `spawn_blocking` thread while the HTTP/WS server runs on
tokio async tasks. A shared metrics store (atomics + EWMA ring buffers) bridges
them with minimal contention.

```
main() thread
  └── tokio runtime spawns HTTP/WS server
  └── spawn_blocking runs TUI event loop
       reads from: ServerMetrics (atomics + Mutex<RingBuffer>)
       reads from: SESSION_MANAGER (existing DashMap)
       reads from: SessionCache (round-robin, ephemeral, no DB queries)
```

### Phase 1: Dependencies & Metrics Infrastructure

1. Add `ratatui` + `crossterm` as optional SSR-only deps in `Cargo.toml`
2. Create `src/server/metrics.rs` — `ServerMetrics` struct with:
   - `AtomicU64` for HTTP request count, WS message count
   - `AtomicI64` for current WS connection count
   - `AtomicU64` for 24h peak connections
   - `Mutex<MetricsInner>` for EWMA values and ring buffers
   - `RingBuffer<T, N>` — fixed-size circular buffer for per-minute samples
   - `SessionCache` — round-robin buffer of `SessionInfo` structs (name, GM,
     player count, created_at, last_active). Pushed to by the server alongside
     normal DB writes. Purely ephemeral — may be briefly out-of-sync with DB.
   - `tick()` — called once/minute, computes EWMA deltas and updates 24h peak
   - `snapshot()` — returns a `MetricsSnapshot` for the TUI to render
   - EWMA formula: `alpha = 2.0 / (N + 1.0)`, `ewma = alpha * sample + (1 - alpha) * ewma`
3. Register `metrics` module in `src/server/mod.rs`

### Phase 2: Wire Metrics into Hot Paths

4. Instrument `src/server/ws_handler/mod.rs`:
   - `handle_socket()` entry: `inc_ws_connections()`
   - `handle_socket()` exit: `dec_ws_connections()`
   - Message receive loop: `inc_ws_messages()`
5. Add Axum request-counting middleware in `src/main.rs`:
   - Simple `count_requests` middleware function calling `inc_http_requests()`
   - Apply as a layer on the router
6. Push session info to `SessionCache` alongside existing DB writes:
   - On session create: push new entry
   - On session join/leave: update player count
   - On session activity (WS messages): update `last_active` timestamp
   - These are fire-and-forget writes to the cache — no error handling needed,
     and brief inconsistency with the DB is acceptable

### Phase 3: TUI Module

7. Create `src/server/tui.rs`:
   - `TuiApp` struct with scroll offsets, quit state, config (port, db path)
   - `run_tui()` — public entry point:
     - Enable raw mode, enter alternate screen
     - Event loop: poll crossterm events (1s timeout), handle keys, draw
     - Every 60 ticks call `ServerMetrics::tick()`
     - On exit: restore terminal state
   - `draw()` — ratatui layout:
     - Title bar (1 line): time, "WebRPG vX.Y.Z Server", date
     - Three-column row (~6 lines): Live Stats | Averages | Server Info
     - Active Sessions (variable height, scrollable table)
     - Recent Sessions (fixed ~7 lines)
     - Status bar (1 line)
   - Renegade BBS theme constants:
     - `BG: Color::Blue`, `BORDER: Color::Cyan`
     - `LABEL: Color::Cyan`, `VALUE: Color::White`, `EMPHASIS: Color::Yellow`
   - Data sources per frame (all in-memory, no DB queries):
     - `MetricsSnapshot` from `ServerMetrics::global().snapshot()`
     - Active sessions from `SESSION_MANAGER` DashMap (collect to Vec immediately)
     - Recent sessions from `SessionCache` round-robin buffer (last 5)
     - DB file size from `std::fs::metadata()`
     - Version from `env!("CARGO_PKG_VERSION")`
8. Register `tui` module in `src/server/mod.rs`

### Phase 4: Restructure `main()`

9. Refactor `src/main.rs`:
   - Move `axum::serve()` into `tokio::spawn` tasks
   - Run TUI via `tokio::task::spawn_blocking`
   - Detect non-TTY with `std::io::IsTerminal` — fall back to current blocking
     behavior (for `cargo leptos watch` compatibility)

### Phase 5: Graceful Shutdown

10. `q` key with confirmation → `std::process::exit(0)` initially
   - Graceful shutdown via `tokio::sync::watch` channel can be added later

### Risks & Mitigations

- **No TTY** (e.g. `cargo leptos watch`): detect with `std::io::IsTerminal`,
  skip TUI and fall back to current behavior
- **Panic safety**: install a panic hook that calls `disable_raw_mode()` and
  `LeaveAlternateScreen` before printing the panic message
- **DashMap iteration**: collect to Vec immediately, then render — keeps read
  lock time minimal
- **WASM compat**: everything behind `#[cfg(feature = "ssr")]`, no impact on
  hydrate target

### Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` | Modify | Add `ratatui`, `crossterm` (optional, SSR-only) |
| `src/server/mod.rs` | Modify | Register `metrics` and `tui` modules |
| `src/server/metrics.rs` | Create | Atomic counters, EWMA, ring buffers |
| `src/server/tui.rs` | Create | TUI rendering, layout, event loop |
| `src/server/ws_handler/mod.rs` | Modify | Instrument WS connect/disconnect/message |
| `src/main.rs` | Modify | Spawn server, add middleware, run TUI |

## Findings

(none yet)
