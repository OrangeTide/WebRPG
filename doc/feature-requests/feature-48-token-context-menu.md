# Feature 48: Token Context Menu

Right-click context menu on map tokens for quick actions.

## Description

Currently right-click on a selected token only rotates it. Add a proper context menu with:

- Edit label (inline rename)
- Change color
- Change image (open media browser)
- Set size (1x1, 2x2, 3x3, etc.)
- Toggle visibility (GM only — hide from players)
- Toggle conditions (quick condition picker)
- Remove token
- Rotate submenu (or keep right-click-to-rotate as default, with context menu on long-press or Ctrl+right-click)

## Dependencies

- **Feature 47: Fog of War UI** — token visibility toggle depends on the visibility system

## Status: In Progress

Basic context menu implemented (commit 422a536). Right-click on selected token opens menu with:
- Edit label, color, image, size, conditions
- Remove token
- Rotation removed from right-click

### Remaining work
- Toggle visibility (GM only — depends on Feature 47)
- GM option: "Roll Initiative" for that token's character/creature (triggers server-side roll)
- Inline rename (currently opens edit fields)

## Plan

(none yet)

## Findings

- Context menu implemented in `src/components/map.rs`
- Token HP popup coexists with context menu (click vs right-click)
- Token instance model now supports both character and creature tokens
- Unique creature labeling exists server-side for initiative rolls
