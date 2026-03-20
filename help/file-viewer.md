# File Viewer

The File Viewer is a graphical file browser for managing files on the Virtual File System. It provides a point-and-click alternative to [COMMAND.COM](help:command-com) for browsing drives, uploading and downloading files, and previewing content.

The File Viewer's design is inspired by the NeXTSTEP File Viewer, with an icon grid layout and inset content panels. Toolbar buttons use emoji glyphs and have not yet been re-themed to match the full NeXTSTEP raised button aesthetic.

## Opening the File Viewer

Click the File Viewer tile on the dock to open it. The File Viewer starts minimized by default on small and medium screens.

## Drives

The root view shows the available drives as large icons:

| Icon | Drive | Label | Description |
|------|-------|-------|-------------|
| 💾 | A: | Scratch | Temporary client-side storage (IndexedDB). Private to this browser tab. |
| 💾 | B: | Scratch | Second scratch drive, same behavior as A:. |
| 💿 | C: | Session | Server-side storage shared with all players in the session. |
| 🔓 | U: | Personal | Server-side storage private to your user account. |

Double-click a drive icon to browse its contents.

Scratch drives (A: and B:) are ephemeral — their data is lost when you close the browser tab. Use C: or U: for files you want to keep.

## Navigation

### Browsing

- **Double-click a folder** to open it and view its contents.
- **Double-click a file** to preview it (text and images) or download it (other types).

### Toolbar Buttons

| Button | Action |
|--------|--------|
| 🔙 Back | Navigate to the previously viewed directory (browser-style history). |
| ➡️ Forward | Undo the last Back action. |
| ⤴️ Up | Navigate to the parent directory. At a drive root, returns to the drive list. |

### Location Bar

Below the toolbar is an editable text field showing the current path (e.g., `C:/maps/dungeon/`). You can:

- Read the current location at a glance.
- Select and copy the path for pasting into COMMAND.COM or chat.
- Type or paste a path and press **Enter** to navigate directly.

Type just a drive letter and colon (e.g., `C:`) to jump to that drive's root.

## Selecting Files and Folders

- **Click** an item to select it. The selected item is highlighted with a white background.
- **Ctrl+Click** (Cmd+Click on Mac) to toggle an item in or out of the selection, allowing you to select multiple items that aren't next to each other.
- **Shift+Click** to select a range of items from the last selected item to the clicked item.
- **Click empty space** in the content area to deselect all items.

The status bar at the bottom shows how many items are selected when more than one is highlighted.

## Toolbar Actions

These buttons appear when browsing a directory (not the drive list):

| Button | Action |
|--------|--------|
| 📁+ New Folder | Create a new folder in the current directory. Prompts for a name. |
| 📤 Upload | Open a file picker to upload files to the current directory. |
| ✏️ Rename | Rename the selected item. Prompts for a new name. Requires exactly one item selected. |
| 📥 Download | Download the selected file(s). Disabled when only folders are selected. |
| 🗑️ Delete | Delete the selected item(s). Prompts for confirmation. When multiple items are selected, shows the count. |

## Context Menu

Right-click on a file or folder to open a context menu with:

- **Download** (files only) — download the file to your computer.
- **Rename** — rename the item.
- **Delete** — delete the item after confirmation.

Right-clicking an item that is not already selected will select it first.

## File Preview

Double-clicking a file opens a preview overlay:

- **Text files** (.txt, .md, .json, .csv, .log, etc.) display in a scrollable monospace text panel.
- **Image files** (.png, .jpg, .gif, .webp, .svg) display scaled to fit the overlay.
- **Other file types** trigger a download instead of a preview.

Click the close button or click outside the preview panel to dismiss it.

## Status Bar

The bottom of the File Viewer shows:

- **In a directory**: The number of items (e.g., "5 items") and drive quota usage (e.g., "1.2 MB / 10 MB used, 8.8 MB free").
- **With multiple items selected**: Appends the selection count (e.g., "5 items — 3 selected").
- **On the drive list**: "Select a drive".

## File Type Icons

Files are displayed with icons based on their type:

| Icon | Type |
|------|------|
| 📁 | Directories |
| 📜 | Text files (.txt, .md, .log, .csv) |
| 📝 | Code/script files (.js, .ts, .rs, .py, .pas, .sh) |
| 📊 | Data files (.json, .xml, .yaml, .toml) |
| 🖼️ | Images (.png, .jpg, .gif, .svg, .webp) |
| 🎵 | Audio files (.mp3, .wav, .ogg, .flac) |
| 🎬 | Video files (.mp4, .webm, .avi) |
| 📦 | Archives (.zip, .tar, .gz, .7z) |
| 🗺️ | Map files (.vtt) |
| 📃 | All other files |

## Keyboard Shortcuts

Currently, all interaction is mouse-driven. Keyboard shortcuts are a planned enhancement.

## See Also

- [COMMAND.COM](help:command-com) — command-line terminal for the VFS
- [Virtual File System](help:vfs) — overview of drives, quotas, and permissions
