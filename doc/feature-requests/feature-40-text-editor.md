# Feature 40: Text Editor

An interactive text editor window that can edit a temporary buffer or any file accessible from the VFS. Designed as a general-purpose editor that will eventually also serve as the Markdown/wiki editor for the help system (Feature 38).

### Core Functionality

- Edit a temporary (unsaved) buffer or open any file from the VFS (A:/B:/C:/U: drives)
- Plain text editing with syntax-agnostic behavior
- Undo / redo with reasonable history depth
- Line numbers

### Toolbar / Menu Actions

- **New** — create a new empty buffer
- **Open** — open a file from the VFS (path input or browse via Finder)
- **Save** — save the current buffer to its VFS path (prompt for path if unsaved)
- **Undo / Redo** — step through edit history
- **Quit** — if unsaved changes exist, show a modal dialog: "Abandon Changes" or "Save Changes"

### File Locking

- When a user opens a file for editing on a shared drive (C:), the file is locked to that user
- Other users who attempt to open the same file see a message: "File is being edited by [username]"
- Lock is released when the editor is closed (save or abandon)
- Stale locks are released if the user's WebSocket connection drops
- Collaborative editing with multiple cursors is a future enhancement (see Feature 41)

### Future Extensions (not in this phase)

- **Rich text / Markdown toolbar**: formatting buttons (bold, italic, headings, links) for Markdown editing when used as the help system editor (Feature 38)
- **Version history toolbar**: buttons for viewing revision history, diffing, and reverting — for wiki-style editing
- Rich text is desirable long-term but not required for this phase unless it turns out to be easy to add

### Visual Design

- NeXTSTEP-styled window panel consistent with the existing UI
- Monospace font in the editing area
- Toolbar with NeXTSTEP raised buttons
- Modal quit dialog matching the project's dialog style

## Dependencies

- **Feature 34: Virtual File System** — file open/save operations

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
