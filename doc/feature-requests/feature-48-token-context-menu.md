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

## Status: Not Started

## Plan

(none yet)

## Findings

- Right-click handler exists in `src/components/map.rs` but only does rotation
- Token HP popup exists (click on token shows HP edit) — context menu could subsume or complement this
- Token size field exists in DB and rendering (`token.size`)
- Token color, image_url fields exist
