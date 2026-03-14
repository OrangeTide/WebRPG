# Task Plan: Character Sheet Fixes & Real-Time Updates

## Goal
Fix character sheet rendering, add HP resource bars with +/- controls, and make the Character Selection window update in real-time when character state changes.

## Current Phase
Phase 6 — Roll Initiative (not started)

## Phases

### Phase 1: Fix Character Sheet Rendering
- [x] Diagnose why character sheet window shows empty content
- [x] Fix hydration mismatch: replace `Resource::new` with Effect+signal pattern in `CharacterEditorPanel`
- [x] Fix same issue in `CreaturePanel`
- [x] Auto-assign default D&D 5e template to sessions without one (`get_session_template` fallback)
- [x] Use template field defaults as fallback values in ability scores and combat stats
- **Status:** complete

### Phase 2: Add Resource Bars (HP) to Character Sheet
- [x] Add `ensure_character_defaults` server function to backfill empty data_json and create HP resource
- [x] Call `ensure_character_defaults` from `CharacterEditorPanel` on load
- [x] Verify ResourceBar renders with red minus, green plus, number field, undo button
- **Status:** complete

### Phase 3: Real-Time Character Selection Updates
- [x] Add `CharacterResourceUpdated` variant to `ServerMessage`
- [x] Add `character_revision: RwSignal<u32>` to `GameContext`
- [x] Handle `CharacterUpdated` and `CharacterResourceUpdated` in `apply_server_message`
- [x] Broadcast `CharacterResourceUpdated` from `update_character_resource` server function
- [x] Broadcast `CharacterUpdated` from `update_character_portrait` server function
- [x] Make `CharacterSelection` track `character_revision` to trigger refetch
- **Status:** complete

### Phase 4: Fix Ownership / Access Control
- [x] Relax `update_character_resource` ownership check: any session member can adjust HP (not just owner)
- **Status:** complete

### Phase 5: Testing & Verification
- [x] cargo check --features ssr — PASS
- [x] cargo check --features hydrate --target wasm32-unknown-unknown — PASS
- [x] cargo test — PASS
- [x] Browser test: character sheet renders ability scores, combat stats, info fields, skills
- [x] Browser test: HP bar shows with +/- controls, undo button appears after change
- [x] Browser test: Character Selection updates HP in real-time after resource change
- **Status:** complete

### Phase 6: Roll Initiative from Character Sheet & Creature Page
- [ ] Add "Roll Initiative" button to character sheet editor (combat stats section)
- [ ] Add "Roll Initiative" button to creature panel
- [ ] Implement initiative roll calculation: d20 + dexterity modifier + initiative modifier (D&D 5e)
- [ ] Server function `roll_initiative` that rolls, calculates, and adds entry to initiative tracker
- [ ] Broadcast `InitiativeUpdated` to all clients after adding entry
- [ ] Add initiative lock/unlock toggle button to Initiative window (GM control)
- [ ] Add `InitiativeLockChanged { locked: bool }` variant to `ServerMessage`
- [ ] Store initiative lock state in `ActiveSession` (in-memory) and broadcast on change
- [ ] Add `initiative_locked: RwSignal<bool>` to `GameContext`, handle lock messages in `apply_server_message`
- [ ] Character sheet "Roll Initiative" button greyed out + disabled when initiative is locked
- [ ] Creature page "Roll Initiative" button always enabled (ignores lock)
- [ ] Real-time lock state sync: all clients see lock/unlock changes immediately
- [ ] Include initiative lock state in `GameStateSnapshot` so new joiners get current state
- **Status:** not started

### Phase 7: Testing & Verification (Initiative Roll)
- [ ] cargo check --features ssr
- [ ] cargo check --features hydrate --target wasm32-unknown-unknown
- [ ] cargo test
- [ ] Browser test: Roll Initiative from character sheet adds entry to initiative window
- [ ] Browser test: Roll Initiative from creature page adds entry to initiative window
- [ ] Browser test: Lock initiative disables character sheet roll button in real-time
- [ ] Browser test: Creature roll still works when initiative is locked
- [ ] Browser test: Lock state syncs across multiple clients
- **Status:** not started

## Decisions Made
| Decision | Rationale |
|----------|-----------|
| Effect+signal instead of Resource for template loading | Avoids hydration mismatch when reading Resource outside Suspense |
| Auto-assign default template to sessions without one | Characters need template fields to render; existing sessions had NULL template_id |
| `ensure_character_defaults` backfills on first editor open | Handles characters created before template was assigned |
| Session member check instead of owner check for resources | VTT standard: GM/any player can adjust HP (apply damage, healing) |
| `character_revision` counter signal for reactive refetch | Lightweight way to notify CharacterSelection without putting full character list in GameContext |
| Don't track revision in CharacterEditorPanel | Would reset local ResourceBar state (undo button) on every change |
