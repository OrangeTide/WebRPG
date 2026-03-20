# Feature 45: Multiple Map Windows

The GM can open multiple map windows simultaneously, each showing a different map. Tokens can be dragged between map windows to move them across maps. Players see only the active (broadcast) map.

## Requirements

- The GM can open more than one map window at the same time, each independently displaying a different map from the session.
- Each map window operates independently: the GM can pan, zoom, and interact with tokens in each window separately.
- The GM can drag a token from one map window and drop it onto another map window, which moves the token from its source map to the destination map at the drop position.
- Players only see the map that the GM has designated as the active (broadcast) map. Opening additional map windows for the GM does not change what players see.
- A clear UI indicator shows which map window is the currently broadcast map.

## Design Notes

### Window management
Each map window is a standard desktop window in the window manager. The GM opens additional map windows via the existing map-open workflow (e.g. double-clicking a map in a list, or a "New Map Window" action). Multiple windows can show the same map or different maps simultaneously.

### Token drag-and-drop across windows
Cross-window drag-and-drop requires coordination at the application level since each window manages its own canvas. Options include:
- A global drag state that is set when a drag begins and checked on drop in any map window.
- Serializing the dragged token info (token ID, source map ID) into the drag event's data transfer, then handling the drop in the receiving map window.

On a successful cross-map drop, the client sends a `MoveTokenToMap` (or equivalent) message to the server specifying the token ID, destination map ID, and destination coordinates. The server removes the token from its source map and places it on the destination map.

### Broadcast map state
The currently broadcast map ID is already server state (or will need to be). Opening a second map window does not alter this state. A visual indicator (e.g. a highlighted title bar or broadcast icon) distinguishes the broadcast window from secondary windows.

### Layer interaction
Each map window has its own layer toolbar state (active layer, layer visibility). Dragging a token between map windows places it on the active layer of the destination window, subject to the usual layer permission rules.

## Dependencies

- **Feature 44: Layer Toolbar** — layer state is per-window; cross-map token drops land on the destination window's active layer

## Status: Not Started

## Plan

(none yet)

## Findings

No multi-map-window support exists:
- The map component renders a single map view; there is no mechanism to open multiple independent map windows.
- No `MoveTokenToMap` or equivalent cross-map token transfer message in `src/ws/messages.rs`.
- No global drag state or cross-window drag-and-drop coordination.
- No broadcast map indicator distinguishing the active map from secondary windows.
- The prerequisite Feature 44 (Layer Toolbar) is itself not started, so this feature remains blocked.
