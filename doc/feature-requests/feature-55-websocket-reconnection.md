# Feature 55: WebSocket Reconnection

Handle server restarts gracefully — reconnect or show error state.

## Description

"Connecting..." never resolves when the server is restarted. Clients are stuck in the "Connecting..." state indefinitely.

**Expected behavior:**
- If the server comes back, automatically reconnect and transition to "Connected"
- If the server is unreachable after a timeout, show an error/disconnected state
- Related to Feature 56 (keepalive ping) which would help detect connection loss

## Dependencies

- Feature 56 (Client Keepalive Ping) — complementary but not required

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
