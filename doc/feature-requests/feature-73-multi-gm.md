# Feature 73: Multiple GMs per session

Allow a session to have multiple GMs, so more than one user can have GM
privileges (placing tokens, managing creatures, fog of war, etc.).

## Status: Done

A session owner can grant and revoke co-GM status to other members from an
in-session Members window, and co-GMs get full GM privileges.

### Done

- `session_gms` join table (migration 0015). A user is a GM of a session if they
  are the owner (`sessions.gm_user_id`) or appear in `session_gms`.
- `crate::server::gm` centralizes the rule (`is_session_gm`, `is_session_owner`,
  `session_owner_id`, `additional_gm_ids`). Every GM check routes through it: the
  WebSocket `is_gm`, and the inline checks that were scattered across
  `characters.rs`, `session.rs`, `modules_api.rs`, `tokens.rs`, and `vfs.rs`.
- In-session management (owner only): a GM-only Members window
  (`components/members_panel.rs`, `WindowId::Members`) lists members with their
  role and lets the owner grant/revoke co-GM. Backed by the `ListMembers` /
  `SetMemberGm` client messages and `MemberList` / `GmStatusChanged` /
  `MembersChanged` server messages.
- Live propagation: a granted or revoked user receives `GmStatusChanged` and
  their `is_gm` flips immediately (GM windows appear/disappear) without
  reconnecting; other GM clients refetch the member list on `MembersChanged`.
- The owner is always a GM and cannot be demoted or removed from the GM list.
- Admin cascades clear `session_gms` when a session or user is removed.
- `ci/smoke-test-server.sh` covers the determination end to end: a non-GM member
  is refused `create_map`, gains access once added to `session_gms`, and loses it
  again when removed. `ci/check-migrations.sh` includes `session_gms`.

### Notes

- `sessions.gm_user_id` is kept as the owner/primary GM: it drives the session
  list display, the creature token owner color, and who may manage the GM list.
- The client already consumed `is_gm` as a boolean, so no structural client
  change was needed beyond the Members window and the live-update handling.

## Future enhancements

- Let co-GMs (not just the owner) manage the GM list, if desired.
- Surface co-GMs in the session list / lobby, not just inside the session.
