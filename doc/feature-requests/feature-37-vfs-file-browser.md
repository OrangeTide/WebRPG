# Feature 37: VFS File Browser

A graphical file browser window for the Virtual File System (Feature 34), styled after the NeXTSTEP File Viewer. Provides a visual alternative to the Terminal Shell (Feature 36) for users who prefer point-and-click interaction.

### Visual Design (NeXTSTEP File Viewer style)

The browser uses **stacked horizontal shelf panels** that drill down vertically:

- **Top shelf** — displays available drives as large icons (A:, B:, C:, U:) with a status line showing total/used/free space for the selected drive.
- **Second panel** — shows the contents of the selected drive as a grid of icons with labels below each.
- **Third panel** — shows the contents of the selected folder from the panel above.
- Panels stack vertically, each representing one level deeper in the directory hierarchy.
- Selecting a folder in any panel opens its contents in the panel below, clearing any deeper panels.

### Visual Styling

- NeXTSTEP gray gradient title bar with "File Viewer" label
- Light gray panel backgrounds with subtle inset borders
- Large icons (folders, documents, images) with filename labels centered below
- Grid layout within each panel with consistent spacing
- Horizontal scrollbar on each panel if contents overflow
- Consistent with the existing NeXTSTEP-style dock (already in the project)

### Planned Features

- Large icon grid view with labels (primary view, matching NeXT style)
- Drill-down navigation via stacked panels
- Status line showing drive quota (used/total/free)
- Drag-and-drop file upload from desktop into any panel
- Context menu (right-click) for copy, move, rename, delete
- Multi-select with shift/ctrl click
- Double-click to preview text files and images
- Icon differentiation by file type (folder, text, image, binary)

## Dependencies

- **Feature 34: Virtual File System** — provides the backend VFS operations

## Status: Not Started

## Plan

(none yet)

## Findings

- Reference screenshot: https://eshop.macsales.com/blog/wp-content/uploads/2025/02/NeXT_screenshot.jpg
- The NeXT File Viewer uses vertically stacked panels (not column browser), each panel showing one directory level as an icon grid
- The project already has a NeXTSTEP-style dock, so the visual language is established
