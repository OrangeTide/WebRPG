# Map Viewer

The Map Viewer displays the session's battle map with tokens, grid, and fog of war. It supports panning, zooming, token selection, measurement, and more.

## Getting Started

When a session has no map, the GM will see a **Create Map** form where they can set the map name, width, and height in grid cells. Once created, the map appears with a grid overlay.

## Navigation

| Action | Control |
|--------|---------|
| **Pan** | Middle-click drag, or hold **Space** and drag |
| **Zoom** | Mouse wheel (zooms toward cursor) |

Zoom is clamped between 0.25x and 4.0x. The viewport starts at the top-left corner of the map at 1.0x zoom.

## Tool Palette

The floating palette in the top-left corner provides quick access to map tools. Each tool can also be activated with a keyboard shortcut.

| Tool | Hotkey | Description |
|------|--------|-------------|
| **Select** | **V** | Click tokens to select, drag to move, Shift+click for multi-select, drag empty space for box select |
| **Pan** | **H** | Click and drag to pan the viewport |
| **Measure** | **M** | Click two points to measure distance in grid squares and feet |
| **Ping** | **P** | Click to ping a location visible to all players |
| **Rotate** | **R** | Click and drag on a selected token to rotate it freely |
| **Grid Snap** | **G** | Toggle grid snapping for token placement and measurement |
| **Facing Arrows** | **F** | Toggle directional arrows on tokens showing their facing |
| **Token List** | **T** | Toggle a dropdown listing all visible tokens; click an entry to center the viewport on that token |

Additional toolbar controls:

- **Ping Color** -- color picker to set your ping and facing arrow color
- **Sync Viewport** (GM only) -- broadcasts your current viewport to all players

## Keyboard Shortcuts

All shortcuts work when the map window has focus (click the map area first).

| Key | Action |
|-----|--------|
| **V** | Switch to Select tool |
| **H** | Switch to Pan tool |
| **M** | Switch to Measure tool |
| **P** | Switch to Ping tool |
| **R** | Switch to Rotate tool |
| **G** | Toggle grid snap on/off |
| **F** | Toggle facing arrows on/off |
| **T** | Toggle token list dropdown |
| **Space** (hold) | Pan mode -- drag to pan the viewport |
| **Escape** | Cancel current measurement, deselect all tokens, return to Select tool |

## Select Tool

The default tool for interacting with tokens.

- **Click a token** to select it (deselects others).
- **Shift+click a token** to add or remove it from the selection.
- **Click empty space and drag** to draw a selection rectangle. All tokens inside the rectangle are selected on release.
- **Drag a selected token** to move all selected tokens together. If grid snap is on, tokens snap to grid cell positions.

When a token is selected, a popup appears below the map showing:

- **Token name** with a link icon if the token is tied to a character or creature (click to open their editor)
- **HP** (current/max) with adjustment buttons: **-1**, **+1**, **-5**, **+5**
- **Condition toggles** for 10 standard conditions: Bloodied, Poisoned, Prone, Stunned, Blinded, Frightened, Paralyzed, Restrained, Invisible, Concentrating. Active conditions are highlighted and shown as icons above the token on the map.

## Measure Tool

Click to set the start point, then move the mouse to see a dashed line with a distance label. Click again to set the end point and lock the measurement. Click a third time to start a new measurement.

Distance is shown in grid squares and feet (assuming 5 ft per square), e.g. "5.0 sq / 25 ft".

If grid snap is on, measurement endpoints snap to cell centers.

Press **Escape** to cancel the current measurement and return to the Select tool.

## Token Rotation

- **Rotate tool (R)**: Click and drag on a selected token to rotate it freely. When multiple tokens are selected, they orbit around the group centroid.
- Single-token rotation works with grid snap enabled. Multi-token group rotation is disabled when grid snap is on, since orbiting positions cannot snap cleanly to the grid.

## Token Context Menu

**Right-click** on a token to open a context menu. If the token is not already selected, it is selected first. The menu shows the token's name (or a count for multi-select) and provides:

- **Delete** (GM only) -- removes the selected token(s) from the map

## Facing Arrows

Toggle with the **F** hotkey or the arrow button in the toolbar. When enabled, a small directional triangle appears outside each token's circle, pointing in the token's facing direction.

Arrow colors reflect token ownership:

- **Player characters** use the player's ping color
- **NPC characters** (GM-owned) use the token's own color
- **Creatures** use the GM's ping color
- **Generic tokens** use the map's default token color (set in Map Settings)

## Token Display

Tokens show the following visual information:

- **Colored circle** or **image** clipped to a circle
- **Label** text centered on the token
- **HP bar** below the token (green > 50%, yellow > 25%, red below)
- **Condition icons** displayed above the token as emoji
- **Facing arrow** (when enabled) as a small triangle outside the token border
- **Selection highlight** as a yellow ring around selected tokens
- **Size** -- tokens can span multiple grid cells (size 2 = 2x2 cells)

## GM Controls

These controls are only available to the Game Master:

### Map Management Bar

The management bar in the top-right corner provides:

- **Map switcher** dropdown to switch between maps in the session
- **New Map (+)** button to create a new map
- **Map Settings** (gear icon) -- opens a dropdown panel with:
  - **Set Background** -- opens the media browser to select a background image
  - **Default Token Color** -- color picker for facing arrows on generic tokens
  - **Delete Map** -- removes the current map (with confirmation)

### Token Management

- **Place tokens** -- use the "Place on Map" button in the Characters or Creatures panel. The button is grayed out if the character already has a token on the current map.
- **Place All Player Tokens** -- places all player characters that don't already have tokens on the map
- **Delete tokens** -- right-click a token and choose Delete from the context menu
- **Sync Viewport** -- broadcasts the GM's current viewport to all players

## Zoom Toolbar

The bottom-right corner shows a zoom toolbar with:

- **+** / **−** -- zoom in or out (centered on the viewport)
- **Fit** -- zoom and pan to fit the entire map in the viewport
- **1:1** -- reset zoom to 100% and scroll to the top-left corner
- **Percentage** -- displays the current zoom level

## Tips

- Click the map area to give it keyboard focus before using hotkeys.
- Use the Token List dropdown (**T**) to quickly find and center on a specific token.
- Middle-click drag is the fastest way to pan around the map.
- The grid snap toggle affects both token dragging and measurement endpoints.
