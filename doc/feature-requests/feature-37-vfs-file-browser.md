# Feature 37: VFS File Browser

A graphical file browser window for the Virtual File System (Feature 34), inspired by the NeXTSTEP File Viewer. Provides a visual alternative to the Terminal Shell (Feature 36) for users who prefer point-and-click interaction.

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

- Gray gradient title bar with "File Viewer" label
- Light gray panel backgrounds with subtle inset borders
- Large icons (folders, documents, images) with filename labels centered below
- Grid layout within each panel with consistent spacing
- Horizontal scrollbar on each panel if contents overflow
- Consistent with the existing dock style (already in the project)
- Toolbar buttons with emoji icon glyphs (not yet re-themed to NeXTSTEP raised buttons)
- Location bar styled as an inset text field

### Planned Features

- Large icon grid view with labels (primary view, matching NeXT style)
- Single-panel navigation with back/up buttons and location bar
- Status line showing drive quota (used/total/free)
- Context menu (right-click) for copy, move, rename, delete
- Multi-select with shift/ctrl click
- Double-click to preview text files and images (see ZIP behavior below for `.zip` files)
- Unicode icon differentiation by file type using the shared icon mapping from Feature 34 (folder, text, image, audio, video, archive, script, map, generic)
- Multiple Finder windows open simultaneously (dynamic WindowId like CharacterEditor)
- Drag-and-drop files/directories between Finder windows to copy/move
- Re-theme toolbar buttons to NeXTSTEP raised button style (asymmetric 3D borders)
- Closer visual alignment with the NeXTSTEP File Viewer (reference screenshot in Findings)

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

## Status: Done

### Completed
- WindowId, component, root view, directory view, toolbar, location bar, navigation
- Scratch drive (A:/B:) browsing via IndexedDB
- Upload button with file picker (small inline + large via CAS)
- Delete and rename buttons with confirmation dialogs
- Single-click selection with visual highlight
- Status line with drive quota display
- Download button and download functionality (inline + CAS files)
- Right-click context menu with Download, Rename, Delete
- Double-click file preview for text and images (inline + CAS URL)
- Preview overlay with close button (text as monospace, images scaled to fit)
- Multi-select with Ctrl+click (toggle) and Shift+click (range)
- Multi-delete and multi-download support
- Help manual page (`help/file-viewer.md`)
- Multiple Finder windows (dynamic `FileBrowserExtra(u32)` WindowId with close button)
- Gas gauge progress bar (OpenStep-style) in status line for multi-file operations
- Drag-and-drop file upload from desktop onto browser content area
- Folder upload via `webkitdirectory` preserving directory structure
- File count confirmation dialog (soft limit 25, hard limit 250)

## Plan

### Implementation Steps

1. Add `FileBrowser` to `WindowId` enum (title "File Viewer", dock icon, min size)
2. Create `src/components/file_browser.rs` with the Finder component
3. Root view: drive icon grid (A:, B:, C:, U:) with usage stats
4. Directory view: icon grid with file type icons, labels, sizes
5. Toolbar: back, up, new folder, upload buttons
6. Location bar: editable path with Enter-to-navigate
7. Navigation: double-click folders, back/up history stack
8. Status line: drive quota display (used/total/free)
9. Add CSS for file browser
10. Wire into `game.rs` with `GameWindow`

## Findings

- Reference screenshot: https://eshop.macsales.com/blog/wp-content/uploads/2025/02/NeXT_screenshot.jpg
- The NeXT File Viewer uses vertically stacked panels (not column browser), each panel showing one directory level as an icon grid — deferred to a future enhancement; v1 uses single-panel navigation for simplicity
- The project already has a NeXTSTEP-style dock, so the visual language is established
