# Feature 53: Map Settings Window

Full window for map settings, replacing the current popup panel.

## Description

Change the map settings from a popup to a full window. Each map can have its own settings window open.

- **Parallel with New Map**: Map Settings window mirrors the same settings available on the New Map window, plus a delete button
- **New Map gets default token color**: Add the default token color setting to the New Map window too
- **Consolidate code**: Share common code between New Map and Map Settings for the overlapping fields
- **Grid offset**: Add grid offset (origin) adjustment with +/- for X/Y on both New Map and Map Settings
- **Live updates**: Changes to map settings apply to all clients viewing the map

## Dependencies

- Map Settings popup already exists (gear icon, commit series from facing arrows plan)
- Default token color field exists in DB (`maps.default_token_color`)

## Status: Not Started

## Plan

(none yet)

## Findings

### Audit (2026-10-29)

A minimal settings popup exists; the full-window rework does not.

- A gear popup on the map provides the default token color
  (`SetMapDefaultColor`) and a delete action (`components/map.rs:1814-1865`).
- Missing: promoting the popup to a full, per-map window; grid offset (origin)
  +/- controls on both New Map and Map Settings; shared code between New Map and
  Map Settings for the overlapping fields; and the live-update broadcast for
  setting changes beyond the existing default-color path.
