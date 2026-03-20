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

| Tool | Button | Hotkey | Description |
|------|--------|--------|-------------|
| **Select** | Arrow icon (highlighted when active) | **V** | Click tokens to select, drag to move, Shift+click for multi-select, drag empty space for box select |
| **Measure** | Ruler icon | **M** | Click two points to measure distance in grid squares and feet |
| **Grid Snap** | Grid icon (highlighted when on) | **G** | Toggle grid snapping for token placement and measurement |
| **Token List** | List icon | **T** | Toggle a dropdown listing all visible tokens; click an entry to center the viewport on that token |

## Keyboard Shortcuts

All shortcuts work when the map window has focus (click the map area first).

| Key | Action |
|-----|--------|
| **V** | Switch to Select tool |
| **M** | Switch to Measure tool |
| **G** | Toggle grid snap on/off |
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

- **Token name** and **HP** (current/max) with adjustment buttons: **-1**, **+1**, **-5**, **+5**
- **Condition toggles** for 10 standard conditions: Bloodied, Poisoned, Prone, Stunned, Blinded, Frightened, Paralyzed, Restrained, Invisible, Concentrating. Active conditions are highlighted and shown as icons above the token on the map.

## Measure Tool

Click to set the start point, then move the mouse to see a dashed line with a distance label. Click again to set the end point and lock the measurement. Click a third time to start a new measurement.

Distance is shown in grid squares and feet (assuming 5 ft per square), e.g. "5.0 sq / 25 ft".

If grid snap is on, measurement endpoints snap to cell centers.

Press **Escape** to cancel the current measurement and return to the Select tool.

## Token Rotation

Right-click on selected tokens to rotate them 15 degrees clockwise. Hold **Shift** and right-click to rotate counterclockwise. Rotation is visible when tokens have images.

## Token Display

Tokens show the following visual information:

- **Colored circle** or **image** clipped to a circle
- **Label** text centered on the token
- **HP bar** below the token (green > 50%, yellow > 25%, red below)
- **Condition icons** displayed above the token as emoji
- **Selection highlight** as a yellow ring around selected tokens
- **Size** -- tokens can span multiple grid cells (size 2 = 2x2 cells)

## GM Controls

These controls are only available to the Game Master:

- **Set Background** button (top-right) -- Opens the media browser to select an image. The image fills the entire map behind the grid. Only visible when a map exists.
- **Create Map** form -- Shown when the session has no map. Set the name, width (cells), and height (cells).
- **Place tokens** -- Use the "Place on Map" button in the Beasts panel to add creature tokens to the map.
- **Remove tokens** -- (via WebSocket commands; UI for this is planned)

## Tips

- Click the map area to give it keyboard focus before using hotkeys.
- Use the Token List dropdown (**T**) to quickly find and center on a specific token.
- Middle-click drag is the fastest way to pan around the map.
- The grid snap toggle affects both token dragging and measurement endpoints.
