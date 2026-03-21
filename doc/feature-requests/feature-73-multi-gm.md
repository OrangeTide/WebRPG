# Feature 73: Multiple GMs per session

Allow a session to have multiple GMs, so more than one user can have GM
privileges (placing tokens, managing creatures, fog of war, etc.).

## Status: Open

## Notes

- Currently `sessions.gm_user_id` is a single integer column
- `is_gm()` in `ws_handler/mod.rs` does a simple `gm_id == user_id` check
- Would need a join table (e.g., `session_gms`) or similar mechanism
- Client already receives `is_gm` as a boolean in `GameStateSnapshot`, so the
  client side needs no structural changes — only the server determination logic
