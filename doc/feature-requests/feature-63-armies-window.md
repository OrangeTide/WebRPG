# Feature 63: Armies Window

GM-only window listing creature instances on a map with drag-to-reorder and grouping.

## Description

All creature instances of the selected map are listed in this GM-only window.

- **Map selector**: Pull-down to select a map (defaults to current)
- **Drag handles**: Reorder creature instances by dragging
- **Groups**: Button to create group categories; drag tokens under groups
  - Groups work like subdirectories in a file manager
  - Click group to collapse/uncollapse
  - Drag groups to reorder relative to other groups
  - Cannot drag groups above ungrouped items
- **Ungrouped**: Non-grouped items always at top under a virtual "Ungrouped" header (gray, collapsible)

## Dependencies

- Token instance refactoring (2026-03-20) — universal token_instances, unique creature labels
- Feature 61 (Architecture Consolidation) — may inform grouping schema

## Status: Not Started

## Plan

Schema extension: a `group_id` column on `tokens` or a separate `token_groups`/`token_group_members` table.

## Findings

(none yet)
