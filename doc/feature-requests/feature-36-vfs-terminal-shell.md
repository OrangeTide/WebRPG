# Feature 36: VFS Terminal Shell

A COMMAND.COM terminal window for interacting with the Virtual File System (Feature 34). Command set and behavior modeled after MSX-DOS 2 COMMAND2.COM v2.20 (internal reference only — the user-facing name is simply "COMMAND.COM").

The terminal is a standalone window panel (separate from game chat) with a text input, scrollback output, and a working directory prompt (e.g., `A:/>`).

### Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `ATTRIB` / `CHMOD` | `ATTRIB [+\|-attr ...] filespec` | Display or change file permissions (GM-only to modify) |
| `CD` / `CHDIR` | `CD [d:] [path]` | Change or display working directory |
| `CLS` | `CLS` | Clear terminal output |
| `CONCAT` | `CONCAT source1 source2 ... dest` | Concatenate files |
| `COPY` | `COPY source dest` | Copy files between drives/paths |
| `DEL` / `ERASE` | `DEL [/P] filespec` | Delete files; /P prompts per file |
| `DIR` | `DIR [/W] [/P] [filespec]` | List directory; /W wide, /P paged |
| `ECHO` | `ECHO [text]` | Output text to terminal |
| `HELP` | `HELP [command]` | Show help for a command or list all |
| `MD` / `MKDIR` | `MKDIR [d:] path` | Create directory |
| `MOVE` | `MOVE source [dest]` | Move files within same drive |
| `RD` / `RMDIR` | `RMDIR path` | Remove empty directory |
| `REN` / `RENAME` | `REN filespec newname` | Rename files; supports wildcards |
| `SET` | `SET [name[=value]]` | Display or set environment variables |
| `TYPE` | `TYPE [/P] filespec` | Display file contents; /P paged |
| `UNZIP` | `UNZIP archive.zip [dest]` | Extract ZIP into directory |
| `VER` | `VER` | Show version info |
| `VOL` | `VOL [d:]` | Show drive label, quota used/limit |
| `XDIR` | `XDIR [filespec]` | Recursive directory listing |

#### FTP-style File Transfer Commands

These commands use FTP syntax for transferring files between the VFS and the user's local machine (browser).

| Command | Syntax | Description |
|---------|--------|-------------|
| `GET` | `GET filespec` | Download file to browser; if filespec is a directory, downloads as ZIP |
| `PUT` | `PUT [dest]` | Open browser file picker to upload file(s) to dest (default: cwd) |

#### ATTRIB / CHMOD

`ATTRIB` and `CHMOD` are aliases for the same command. Without attribute arguments, it displays the current permissions. With `+` or `-` arguments, it sets or clears permission bits (GM-only).

Attribute flags use the format `{scope}{+|-}{bit}`:
- **Scopes**: `U` (user/owner), `G` (group, reserved), `O` (other)
- **Bits**: `R` (read), `W` (write), `X` (execute/list)
- All flags are case-insensitive

Not recursive — when used on a directory, it changes only that directory's permissions. A `/R` flag for recursive operation may be added in the future.

Examples:
```
A:/> attrib C:/maps/dungeon.png
rw-rw-rw-  45,312  2026-03-14 16:00  image/png  C:/maps/dungeon.png

A:/> attrib o-w C:/maps/dungeon.png
rw-rw-r--  C:/maps/dungeon.png

A:/> chmod u+x o-r o-w C:/scripts/run.txt
rwxrw----  C:/scripts/run.txt
```

### Case Sensitivity

All filenames in COMMAND.COM are **case-preserving but case-insensitive**, matching modern Windows behavior. Commands, paths, and attribute flags are all case-insensitive. File names retain the casing they were created with for display purposes.

### Architecture

The shell runs entirely client-side in WASM (`#[cfg(feature = "hydrate")]`). The server never sees command strings — only VFS server function calls for C: and U: drives. Scratch drive operations (A:, B:) go directly to browser IndexedDB.

The command parser, working directory, environment variables, and output formatting are all client-side concerns. The path parser (`VfsPath`), pattern matcher (`vfs_fnmatch`), and permission logic compile to WASM and are shared with the server.

### Features

- Working directory with drive letter, persisted per-tab
- `*` and `?` wildcard support in file specifications (via `vfs_fnmatch`)
- Case-insensitive command names (matching DOS convention)
- Command history (up/down arrow keys)
- Output scrollback buffer
- Pairs with Pascal compiler (Feature 31) for compile-and-run workflows
- **Drag-and-drop upload**: Drop files or folders onto the terminal to upload to the current working directory. Folder structure is preserved. Uses shared drag-and-drop handler from Feature 37.
- **Gas gauge progress bar**: Shared progress bar component from Feature 37, rendered inline in terminal output for uploads, downloads, and ZIP operations.

### Example Session

```
A:/> vol C:
 Volume in drive C: is GAME-SESSION
 100 MB total, 45 MB used, 55 MB free

A:/> dir C:/maps
 Directory of C:/maps

🖼️ dungeon.png        45,312  2026-03-14 16:00
🖼️ forest.jpg        122,880  2026-03-14 16:05
        2 file(s)    168,192 bytes

A:/> copy C:/maps/dungeon.png U:/my-maps/
        1 file(s) copied.

A:/> cd U:/my-maps
U:/my-maps> type readme.txt
Welcome to my maps collection.

U:/my-maps> put
[browser file picker opens]
Uploaded castle.png (89,400 bytes) to U:/my-maps/castle.png

U:/my-maps> get C:/maps/
Downloading C:/maps/ as maps.zip...
[============████████████        ] 67%
Downloaded maps.zip (168,192 bytes)

U:/my-maps> unzip C:/backup.zip C:/restored/
Extracting backup.zip to C:/restored/...
        8 file(s) extracted.
```

## Dependencies

- **Feature 34: Virtual File System** — provides the backend VFS operations

## Status: Not Started

## Plan

(none yet)

## Findings

- MSX-DOS 2 COMMAND2.COM source used as reference: https://www.msxarchive.nl/pub/msx/mirrors/msx2.com/sources/command.txt
- Hardware-specific commands skipped: ASSIGN, BASIC, BUFFERS, CHKDSK, DISKCOPY, FIXDISK, FORMAT, MODE, PAUSE, RAMDISK, UNDEL, VERIFY
- Batch file support (REM, PAUSE, IF, GOTO) deferred — can add later, especially once Pascal compiler exists
- Shell is client-side WASM — server functions enforce security, the shell is just a UI. Same architecture as the File Browser (Feature 37). Shell state (cwd, env vars, history) is naturally per-tab.
