# Feature 24: Code Cleanup and Refactoring

Examine the codebase and clean up and simplify the project. Suggest refactor
options.

## Status: In Progress

## Plan

Large file splits — each is a half-day refactor:

1. **Split `api.rs`** (1897 lines, 78 server functions) → `api/auth.rs`, `api/session.rs`, `api/characters.rs`, `api/vfs.rs`, `api/media.rs`
2. **Split `terminal.rs`** (1741 lines) → extract `terminal/commands.rs` for cmd_* handlers, `terminal/parser.rs`
3. **Split `ws_handler.rs`** (1440 lines) → `ws/inventory.rs`, `ws/initiative.rs`, `ws/chat.rs`

Already done (this session and prior):
- CSS custom properties consolidation (22 variables, 247 var() refs)
- Extract `trigger_browser_download` to shared `browser_helpers.rs`
- Extract `open_file_picker` and `upload_large_file` to shared `browser_helpers.rs`
- Add `Drive::is_scratch()` and `Drive::session_id()` helpers to reduce boilerplate

## Findings

(none yet)
