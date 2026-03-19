# Feature 36: VFS Terminal Shell

A COMMAND.COM terminal window for interacting with the Virtual File System (Feature 34). Command set and behavior modeled after MSX-DOS 2 COMMAND2.COM v2.20 (internal reference only — the user-facing name is simply "COMMAND.COM").

The terminal is a standalone window panel (separate from game chat) with a text input, scrollback output, and a working directory prompt (e.g., `A:/>`).

### Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `ATTRIB` / `CHMOD` | `ATTRIB [+\|-attr ...] filespec` | Display or change file permissions (GM-only to modify) |
| `CD` / `CHDIR` | `CD [d:] [path]` | Change or display working directory |
| `CLS` | `CLS` | Clear terminal output |
| `COPY` | `COPY source dest` | Copy files between drives/paths |
| `DEL` / `ERASE` | `DEL [-p] filespec` | Delete files; -p prompts per file |
| `DIR` | `DIR [-w] [-p] [filespec]` | List directory; -w wide, -p paged |
| `HELP` | `HELP [command]` | Show help for a command or list all |
| `MD` / `MKDIR` | `MKDIR [d:] path` | Create directory |
| `RD` / `RMDIR` | `RMDIR path` | Remove empty directory |
| `TYPE` / `CAT` | `TYPE filespec` | Display file contents |
| `VER` | `VER` | Show version info |

Additional commands deferred to Feature 42: CONCAT, ECHO, MORE, MOVE, REN/RENAME, SET, UNZIP, VOL, XDIR.

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

Not recursive — when used on a directory, it changes only that directory's permissions. A `-r` flag for recursive operation may be added in the future.

Examples:
```
A:/> attrib C:/maps/dungeon.png
rw-rw-rw-  45,312  2026-03-14 16:00  image/png  C:/maps/dungeon.png

A:/> attrib o-w C:/maps/dungeon.png
rw-rw-r--  C:/maps/dungeon.png

A:/> chmod u+x o-r o-w C:/scripts/run.txt
rwxrw----  C:/scripts/run.txt
```

### Command Switches

Switches use `-` (hyphen) instead of the DOS `/` convention, since `/` is the path separator. Use `--` to stop switch parsing (everything after `--` is treated as arguments, not switches). This follows Unix convention.

Examples:
- `dir -w C:/maps` — wide directory listing
- `del -p C:/temp/*` — delete with per-file prompt
- `del -- -weird-filename.txt` — delete a file whose name starts with `-`

Note: ATTRIB/CHMOD attribute flags (`u+r`, `o-w`, etc.) are not command switches — they are positional arguments and are not affected by `--`.

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
- **Tab completion**: Press Tab to complete command names and file/directory paths. If the cursor is at the first token position, completes against command names. Otherwise, completes against file and directory names in the relevant directory (resolved from the working directory). If exactly one match, inserts it. If multiple matches, shows the list in the terminal output and completes the common prefix. Completion is case-insensitive (matching DOS convention). Directories are completed with a trailing `/`.
- Output scrollback buffer
- Pairs with Pascal compiler (Feature 31) for compile-and-run workflows
- **Drag-and-drop upload**: Drop files or folders onto the terminal to upload to the current working directory. Folder structure is preserved. Uses shared drag-and-drop handler from Feature 37.
- **Gas gauge progress bar**: Shared progress bar component from Feature 37, rendered inline in terminal output for uploads, downloads, and ZIP operations.

### Example Session

```
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

U:/my-maps> help dir
DIR [-w] [-p] [filespec]
  List directory contents.
  -w  Wide format
  -p  Paged output
```

## Dependencies

- **Feature 34: Virtual File System** — provides the backend VFS operations

## Status: Done

## Plan

### Implementation Steps

1. ~~Add `Terminal` variant to `WindowId` enum~~ ✓
2. ~~Create `src/components/terminal.rs` with shell component~~ ✓
3. ~~Add terminal CSS~~ ✓
4. ~~Wire into `game.rs` with `GameWindow`~~ ✓
5. ~~Implement core commands: VER, CLS, CD, DIR, TYPE, HELP, MKDIR, RMDIR, DEL, COPY, ATTRIB/CHMOD~~ ✓
6. ~~Command history (up/down arrow)~~ ✓
7. ~~Tab completion (command names + file paths)~~ ✓
8. ~~FTP-style commands: GET, PUT~~ ✓
9. ~~Client-side scratch drive support (A:, B: via IndexedDB)~~ ✓

## Findings

- Component is `src/components/terminal.rs`, ~1500 lines
- Scratch drive module is `src/scratch_drive.rs`, ~330 lines (IndexedDB-backed)
- MSX-DOS 2 COMMAND2.COM source used as reference: https://www.msxarchive.nl/pub/msx/mirrors/msx2.com/sources/command.txt
- Hardware-specific commands skipped: ASSIGN, BASIC, BUFFERS, CHKDSK, DISKCOPY, FIXDISK, FORMAT, MODE, PAUSE, RAMDISK, UNDEL, VERIFY
- Batch file support (REM, PAUSE, IF, GOTO) deferred — can add later, especially once Pascal compiler exists
- Shell is client-side WASM — server functions enforce security, the shell is just a UI. Same architecture as the File Browser (Feature 37). Shell state (cwd, env vars, history) is naturally per-tab.
