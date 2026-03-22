# WebRPG TODO

## Current Version: 0.2.1

## Recently Completed Features
- FR5: GM role check — gate Creatures window to GM-only
- FR9, FR25, FR27, FR54, FR57: Tier 1 quick wins
- FR37: VFS File Browser — multi-window, drag-drop, refactors
- FR48: Token context menu — inline rename, visibility toggle, roll initiative
- FR52, FR59: Initiative map integration and creature duplicate button
- FR66: Turn start notification — title flash, map star, status bar

## Feature Requests — In Progress
- FR11: Dockerfile
- FR13: Server Administration
- FR24: Code Cleanup and Refactoring
- FR38: Online Help System

## Feature Requests — Not Started (by category)

### Core Gameplay
- FR4: Macro Bar
- FR6: Progress Clocks
- FR7: Dice Roll Enhancement — Expressions, Types and Emoji
- FR20: GM Character Assignment Window
- FR58: Creatures Undo Button
- FR63: Armies Window
- FR65: Character Sheet Edit Lock
- FR68: Creature Window List/Edit States
- FR69: Auto-Bloodied Status for Player Characters
- FR70: Temporary Hit Points
- FR71: Multiple RPG System Support
- FR73: Multiple GMs per Session

### Map & Visualization
- FR30: Modular Map Making Tool
- FR44: Layer Toolbar
- FR45: Multiple Map Windows
- FR46: Visibility Classes
- FR47: Fog of War UI & Visibility
- FR49: Drawing & Annotation Tools
- FR50: Line of Sight
- FR51: Scenes
- FR53: Map Settings Window
- FR72: Map Viewport Undo Button

### VFS & Editing
- FR1: VTT Media Pack File Format
- FR2: Loading VTT Media Packs
- FR39: VFS Upload & ZIP
- FR40: Text Editor
- FR41: Collaborative Text Editing

### UI & UX
- FR8: Rename Project
- FR16: Jukebox Music Player
- FR18: Tips Window
- FR19: Light/Dark Mode Switch
- FR21: Login Modal Popup
- FR23: GitHub Link on Landing Page
- FR26: Session Settings Window
- FR28: Lobby Page Redesign
- FR42: COMMAND.COM Extended Commands
- FR43: Consistent Hot Keys Across All Apps

### Infrastructure & Backend
- FR14: Email Validation
- FR15: Server Invite Links
- FR22: Authentication Hardening
- FR29: Session Creation Rate Limit
- FR55: WebSocket Reconnection
- FR56: Client Keepalive Ping
- FR60: Database V1 Rewrite
- FR61: Architecture Consolidation
- FR67: Reverse Proxy Subdirectory Support

### External Integrations
- FR12: User Profile Avatars
- FR17: Discord Bot
- FR31: Pascal to JavaScript Compiler
- FR33: Split Window Manager Into A Library
- FR62: Firefox ESR Automated Testing
- FR64: Video Conferencing

## Unit Tests TODO

### High Priority

- [ ] `parse_and_roll` — `src/server/ws_handler/chat.rs` — Dice notation parser (`2d6`, `1d20+5`, `d6-2`)
  - Valid notation: `2d6`, `1d20+5`, `d6-2`, `d20`, `100d6`
  - Edge cases: `0d6`, `1d0`, negative modifiers
  - Invalid input: `abc`, empty string, `dd6`
  - Result bounds: verify rolls fall within expected min/max

- [ ] `ability_modifier` — `src/server/ws_handler/initiative.rs` — D&D ability score → modifier
  - Standard: 10→0, 12→+1, 8→-1, 20→+5, 1→-5
  - Edge values: 0, very high scores

- [ ] `is_dice_roll` — `src/components/chat.rs` — Validate dice notation strings
  - Valid: `2d6`, `d20`, `1d6+3`, `1d6-2`
  - Invalid: `hello`, `2d`, `d`, empty string

### Medium Priority

- [ ] `hash_password` / `verify_password` — `src/auth.rs` — Argon2 password hashing
  - Round-trip: hash then verify same password succeeds
  - Wrong password fails verification

- [ ] `generate_jwt` / `verify_jwt` — `src/auth.rs` — JWT token handling
  - Round-trip token generation and validation
  - Expired token rejection
  - Requires `SECRET_KEY` env var in test setup
