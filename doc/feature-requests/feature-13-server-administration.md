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

## Status: In Progress

Administrator identity, the `/admin` console, and user/session/media management
are implemented. Server restart and database backup/restore are not yet built.

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
- `ci/smoke-test-server.sh` covers the admin guard (non-admin rejection),
  promotion, and the user/session/media operations.
- `ADMIN_LIST` documented in the release `env.example`, `docker-compose.yml`, and
  DEV.md ("Server Administration").

### Remaining

- Restart server from the admin page.
- Download a backup dump of the database.
- Restore the database from an uploaded backup.

## Findings

- `access_level` field exists in users table schema
  (`migrations/2026-03-12-203733-0000_create_users/up.sql`) and the User model.
- The database does not enable SQLite foreign keys, and most `sessions(id)`
  references lack `ON DELETE CASCADE`, so deleting a session or user requires a
  manual cascade rather than relying on the database.
- Server-function URL suffixes are per-module, so the admin endpoints share one
  suffix; the smoke test looks each up in the WASM (see DEV.md, "Server function
  URLs").
