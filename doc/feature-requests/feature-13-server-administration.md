# Feature 13: Server Administration

Establish administrator accounts from the environment (`ADMIN_LIST`). These
accounts can access a server administration page.

On the administration page the following actions are possible:

- Browse, edit, remove, and add user accounts
- List, archive, remove game sessions
- Rename game sessions
- Restart server
- Add/remove media files from storage
- Browse, preview, and search media files in storage
- Download backup dump of database
- Restore database from an uploaded backup

## Status: Done

Administrator identity, the `/admin` console, and every listed action (user,
session, and media management, server restart, and database backup/restore) are
implemented.

### Done

- Admin identity on the `users.access_level` column: `ACCESS_LEVEL_ADMIN` (100)
  and above are admins, `ACCESS_LEVEL_USER` (0) is a normal account
  (`models.rs`). `ADMIN_LIST` (comma-separated usernames) promotes matching
  accounts at startup via `promote_admins_from_env`, called from `main.rs`.
  Promotion is additive and never demotes, so in-app grants survive restarts.
- `require_admin` guard on every admin server function
  (`src/server/api/admin.rs`); `current_user_is_admin` for cosmetic UI gating.
- `/admin` page (`src/pages/admin.rs`), a tabbed console (Users, Sessions,
  Media), linked from the sessions page for admins. Non-admins see an
  access-denied notice, and every underlying endpoint rejects them regardless.
- Users: list, add, edit (display name, email, lock, admin flag), remove. Guards
  prevent self-lock, self-demotion, and self-removal, and refuse removing an
  account that still GMs a session.
- Sessions: list all (including archived), rename, archive/unarchive (the
  `active` flag), and permanent remove. Because SQLite foreign keys are not
  enforced, remove performs a manual transactional cascade across every
  dependent table.
- Media: browse, search by tag, preview, and remove from storage
  (`admin_delete_media`, which also deletes the file on disk).
- Server control (`src/server/admin_handler.rs`, admin page "Server" tab):
  - Restart (`admin_restart_server`): exits the process after responding, relying
    on a supervisor to restart it. Hard `process::exit` so the pool cannot
    checkpoint stale WAL pages during shutdown.
  - Backup (`GET /api/admin/backup`): streams a `VACUUM INTO` SQLite snapshot as
    a download.
  - Restore (`POST /api/admin/restore`): validates the SQLite upload, saves the
    current database as a timestamped `.bak`, writes the upload over the database
    path, clears stale WAL sidecars, and restarts onto it.
- `ci/smoke-test-server.sh` covers the admin guard (non-admin rejection) across
  all endpoints, promotion, the user/session/media operations, backup download,
  and restore validation. (A successful restore or restart would terminate the
  test server, so those are verified manually, not in the suite.)
- `ADMIN_LIST` documented in the release `env.example`, `docker-compose.yml`, and
  DEV.md ("Server Administration"), which also documents the server-control
  endpoints and the supervisor requirement.

## Future enhancements

- Restart and restore need a process supervisor to bring the server back; a bare
  `./webrpg` just stops. A built-in self-respawn could remove that requirement.
- Restore ties downtime to a restart; a live in-place swap (SQLite backup API)
  would avoid it but is riskier with an active pool.

## Findings

- `access_level` field exists in users table schema
  (`migrations/2026-03-12-203733-0000_create_users/up.sql`) and the User model.
- The database does not enable SQLite foreign keys, and most `sessions(id)`
  references lack `ON DELETE CASCADE`, so deleting a session or user requires a
  manual cascade rather than relying on the database.
- Server-function URL suffixes are per-module, so the admin endpoints share one
  suffix; the smoke test looks each up in the WASM (see DEV.md, "Server function
  URLs").
