# Feature 72: Map Viewport Undo Button

Add a button near the map zoom controls to return to the previous viewport location and zoom. This allows undoing viewport changes caused by initiative auto-center (FR52) or the GM's sync viewport broadcast.

## Dependencies

- **Feature 52: Initiative Map Integration** — Done. Its auto-center is one trigger for needing undo.

## Status: Not Started

## Plan

(none yet)

## Findings

- Map viewport state is `view_offset: RwSignal<(f64, f64)>` and `view_zoom: RwSignal<f64>` in `map.rs`
- `center_on_token_id` and `ViewportSynced` both modify these signals
- Need to push the old (offset, zoom) onto a stack before any programmatic viewport change
- A single "back" button near the zoom +/- buttons would pop the stack
