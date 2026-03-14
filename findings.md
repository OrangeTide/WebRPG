# Findings & Decisions

## Requirements
- Character sheet window must show all template fields (ability scores, combat stats, info, skills, etc.)
- HP resource bar with red minus (-), green plus (+), number field, and undo button
- Character Selection window must update in real-time when any character data changes
- Any session member can adjust HP (not just character owner)

## Research Findings
- `Resource::new` in Leptos 0.8 causes hydration mismatch when read outside `<Suspense/>` in hydrate mode — console warning confirms
- Sessions had `template_id = NULL` because `seed_default_template` was never called on startup
- Characters created before template assignment had `data_json = '{}'` and no `character_resources` records
- `update_character_resource` server function ran via HTTP, not WebSocket — needed explicit `SESSION_MANAGER.broadcast()` call
- `SESSION_MANAGER` is a static `Lazy<SessionManager>` shared between Axum HTTP handlers and WS handler — broadcast from server functions works

## Technical Decisions
| Decision | Rationale |
|----------|-----------|
| Effect+signal pattern over Resource | Avoids hydration mismatch; consistent with existing patterns in codebase |
| `get_session_template` auto-seeds default template | Ensures all sessions have a template without requiring manual DB fix |
| `ensure_character_defaults` as idempotent server function | Safely backfills legacy characters on first editor open |
| `character_revision` counter in GameContext | Minimal coupling — CharacterSelection just tracks a number, refetches when it changes |
| Broadcast from server functions (not just WS handler) | Resource updates go through HTTP server functions, not WS messages |

## Issues Encountered
| Issue | Resolution |
|-------|------------|
| Character sheet window empty (no fields, no HP) | Hydration mismatch on template Resource + no template assigned to session |
| `data_json` was `{}` for existing characters | `ensure_character_defaults` backfills from template on first editor open |
| No `character_resources` for existing characters | `ensure_character_defaults` creates HP resource if none exists |
| Character Selection not updating after HP change | Added `CharacterResourceUpdated` WS message + `character_revision` signal |
| `update_character_resource` silently failing | User wasn't in `session_players` table AND ownership check was too strict |
| Server function broadcast not reaching client | `character.session_id` was correct but user wasn't a session member — relaxed to membership check |

## Resources
- Leptos 0.8 docs on Resources and hydration: Resource should be read inside `<Suspense/>` or use `LocalResource`
- `SESSION_MANAGER` static in `src/ws/session.rs` — accessible from both WS handler and server functions

## Visual/Browser Findings
- Character sheet renders: header (portrait + name), HP resource bar, ability scores grid (3-col), combat stat badges, info fields, skills/equipment/spells textareas
- HP bar: red minus/green plus buttons, number input for amount, visual fill bar, undo button appears after change
- Character Selection cards show: portrait, name, HP current/max, AC, Level

---
*Updated 2026-03-13 — character sheet rendering, resource bars, real-time updates*
