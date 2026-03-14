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

Database has `access_level` field in users table and User model, but no admin
checks, admin routes, or admin UI are implemented yet.

## Plan

TBD

## Findings

- `access_level` field exists in users table schema
  (`migrations/2026-03-12-203733-0000_create_users/up.sql`)
- `access_level` field exists in User model (`models.rs`)
- No `ADMIN_LIST` env var handling, no admin route guards, no admin pages
