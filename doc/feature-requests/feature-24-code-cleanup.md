# Feature 24: Code Cleanup and Refactoring

Examine the codebase and clean up and simplify the project. Suggest refactor
options.

## Status: In Progress

## Plan

Large file splits, each roughly a half-day refactor:

1. **Split `api.rs`** (2159 lines, ~80 server functions) into `api/auth.rs`,
   `api/session.rs`, `api/characters.rs`, `api/vfs.rs`, `api/media.rs`.
2. **Split `terminal.rs`** (1890 lines) by extracting `terminal/commands.rs`
   for the cmd_* handlers and `terminal/parser.rs`.

Already done (prior sessions):
- Split `ws_handler.rs` into a `server/ws_handler/` module directory
  (`chat.rs`, `initiative.rs`, `inventory.rs`, `maps.rs`, `tokens.rs`,
  `character.rs`, plus `mod.rs`).
- CSS custom properties consolidation (22 variables, 247 var() refs).
- Extract `trigger_browser_download` to shared `browser_helpers.rs`.
- Extract `open_file_picker` and `upload_large_file` to shared `browser_helpers.rs`.
- Add `Drive::is_scratch()` and `Drive::session_id()` helpers to reduce boilerplate.

Also done (review-driven cleanup):
- Broadcast by cloning the message instead of serializing once and then
  deserializing per client; dropped the unreachable error fallback
  (`ws/session.rs`).
- Centralize the client character cache into one `GameContext.characters` signal
  and apply `CharacterUpdated` / `CharacterResourceUpdated` deltas in place, so a
  field or resource edit no longer triggers a full `list_characters` refetch in
  three windows.
- Collapse three `center_on_*` signals into one `center_on: Option<CenterTarget>`
  command, with a single map effect resolving the lookup key.
- Introduce a `Sender` type for the WS send handle, removing the
  `with_value(|f| if let Some(f) = ...)` boilerplate from nine files, and keep
  non-Send `*_local` values off the SSR render path (see DEV.md, "SSR and
  non-Send reactive values").

## Findings

(none yet)
