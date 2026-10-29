# Feature 55: WebSocket Reconnection

Handle server restarts gracefully, reconnect or show an error state.

## Status: Done

The client no longer gets stuck in "Connecting..." when the server restarts. It
reconnects automatically when the server returns, and shows a disconnected error
state (with a manual retry) after several failed attempts while continuing to
retry in the background.

### Done

- The game-page WebSocket setup (`src/pages/game.rs`) was refactored into a
  reusable `ws_connect` plus `schedule_reconnect`. On an unexpected close it
  reconnects with exponential backoff (1s, 2s, 4s, 8s, capped at 15s); a
  successful open resets the backoff. Each attempt fetches a fresh WS token.
- A per-attempt `generation` guard means a superseded socket's handlers never
  reconnect or fight the current one, so manual retries and overlapping attempts
  cannot produce duplicate connections.
- Loading states: `RECONNECTING` (spinner) for the first few attempts, then
  `DISCONNECTED` (error modal) with a "Reconnect now" button; the modal clears
  itself once a reconnect succeeds (the fresh snapshot restores state).
- `on_cleanup` stops reconnection and closes the socket when leaving the page.

### Verification

Live browser test: load the game page (Connected), kill the server (drops to
"Connecting..."), restart it, and the client auto-reconnects to Connected.
