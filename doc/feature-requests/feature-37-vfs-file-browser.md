# Feature 37: VFS File Browser

A graphical file browser window for the Virtual File System (Feature 34), styled after the NeXTSTEP File Viewer. Provides a visual alternative to the Terminal Shell (Feature 36) for users who prefer point-and-click interaction.

### Visual Design (NeXTSTEP File Viewer style)

Single-panel view showing one directory at a time as an icon grid. Navigation via double-click (enter folder), toolbar buttons (back, up), or the location bar.

- At the root level, shows available drives as large icons (A:, B:, C:, U:) with a status line showing total/used/free space for the selected drive.
- Inside a drive/folder, shows contents as a grid of icons with labels below each.
- Double-clicking a folder navigates into it, replacing the current view.
- Stacked multi-panel drill-down navigation is a future enhancement.

### Toolbar

Below the title bar, a toolbar row with:
- **Back button (←)** — navigate to the previously viewed directory (browser-style history)
- **Up button (↑ directory)** — navigate to the parent directory of the current location
- **New Folder button (📁+)** — create a new folder in the current directory (prompts for name)
- **Upload button (↑ file)** — open multi-file browser dialog to upload to the current directory

### Location Bar

Below the toolbar, an editable text field showing the full path of the current directory (e.g., `C:/maps/dungeon/`). Users can:
- Read the current path at a glance
- Select and copy the path for pasting into COMMAND.COM or chat
- Type or paste a path directly and press Enter to navigate there
- The location bar updates as the user navigates via panels

### Visual Styling

- NeXTSTEP gray gradient title bar with "File Viewer" label
- Light gray panel backgrounds with subtle inset borders
- Large icons (folders, documents, images) with filename labels centered below
- Grid layout within each panel with consistent spacing
- Horizontal scrollbar on each panel if contents overflow
- Consistent with the existing NeXTSTEP-style dock (already in the project)
- Toolbar buttons styled as NeXTSTEP raised buttons with icon glyphs
- Location bar styled as an inset text field matching NeXTSTEP input fields

### Planned Features

- Large icon grid view with labels (primary view, matching NeXT style)
- Single-panel navigation with back/up buttons and location bar
- Status line showing drive quota (used/total/free)
- Context menu (right-click) for copy, move, rename, delete
- Multi-select with shift/ctrl click
- Double-click to preview text files and images (see ZIP behavior below for `.zip` files)
- Unicode icon differentiation by file type using the shared icon mapping from Feature 34 (folder, text, image, audio, video, archive, script, map, generic)

### Upload & Download

- **Upload**: Via toolbar button (see above) or drag-and-drop (see Shared UI Components)
- **Folder upload**: Toolbar menu option opens folder picker (`webkitdirectory`). Preserves folder structure under current directory.
- **Download**: Right-click context menu "Download" on a file triggers a browser save. On a folder, triggers ZIP download.

### Shared UI Components

This feature defines reusable UI components that COMMAND.COM (Feature 36) also uses:

- **Gas gauge progress bar**: OpenStep-style progress indicator matching the NeXTSTEP visual theme. Reference: https://guidebookgallery.org/pics/gui/installation/copying/openstep42.png. In the Finder, shown in the status line. In COMMAND.COM, rendered inline in terminal output. Used for uploads, downloads, and ZIP operations.
- **Drag-and-drop handler**: Shared logic for accepting file/folder drops from the desktop. Uses `dragenter`/`dragover`/`drop` events with `DataTransfer.items` and `webkitGetAsEntry()` for folder tree traversal. In the Finder, the drop target panel determines the destination directory. In COMMAND.COM, the drop target is the working directory.
- **File count confirmation dialog**: When an operation involves more than 25 files, prompts the user to confirm before proceeding (see Feature 39 limits).

### ZIP File Handling

Clicking (or double-clicking) a `.zip` file presents a dialog with three options:
- **Extract Here** — extract contents into the current directory
- **Extract to Destination** — prompts for a destination path, then extracts there
- **Cancel** — dismiss the dialog

Right-click on a `.zip` also offers these options in the context menu.

## Dependencies

- **Feature 34: Virtual File System** — provides the backend VFS operations

## Status: Not Started

## Plan

(none yet)

## Findings

- Reference screenshot: https://eshop.macsales.com/blog/wp-content/uploads/2025/02/NeXT_screenshot.jpg
- The NeXT File Viewer uses vertically stacked panels (not column browser), each panel showing one directory level as an icon grid — deferred to a future enhancement; v1 uses single-panel navigation for simplicity
- The project already has a NeXTSTEP-style dock, so the visual language is established
