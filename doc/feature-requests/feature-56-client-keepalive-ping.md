# Feature 56: Client Keepalive Ping

Client sends a periodic heartbeat to the server with idle activity status.

## Status: Done

### Done

- The client sends `ClientMessage::Keepalive { idle }` once a minute over the
  session WebSocket (`src/pages/game.rs`), routed through the same sender as all
  other messages, so it always targets the current socket after a reconnect.
- `idle` is derived from a last-activity timestamp updated by `pointerdown`,
  `keydown`, and `wheel` listeners (idle when there has been no input for over a
  minute). The flag is carried for future presence/idle use and is not acted on
  yet.
- The server handles `Keepalive` by refreshing the session's last-active time
  (`push_session_to_cache`) and ignoring the idle flag for now
  (`src/server/ws_handler/mod.rs`).

### Verification

WS test: a client sends `Keepalive` (idle false and true); the server accepts it
with no error, the socket stays open, and the session keeps working (chat still
round-trips) afterward.

### Notes

- Reconnection itself (feature 55) is driven by the socket's close event, which
  fires promptly on a server restart; the keepalive complements it and provides
  the last-active signal for the server status view.
