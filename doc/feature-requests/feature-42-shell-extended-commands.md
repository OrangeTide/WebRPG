# Feature 42: COMMAND.COM Extended Commands

Additional commands for the VFS Terminal Shell (Feature 36), deferred from the initial release to reduce scope.

### Commands

| Command | Syntax | Description |
|---------|--------|-------------|
| `CONCAT` | `CONCAT source1 source2 ... dest` | Concatenate files |
| `ECHO` | `ECHO [text]` | Output text to terminal |
| `MORE` | `MORE filespec` | Display file contents one page at a time; press any key to continue, Q to quit |
| `MOVE` | `MOVE source [dest]` | Move files within same drive |
| `REN` / `RENAME` | `REN filespec newname` | Rename files; supports wildcards |
| `SET` | `SET [name[=value]]` | Display or set environment variables |
| `UNZIP` | `UNZIP archive.zip [dest]` | Extract ZIP into directory |
| `VOL` | `VOL [d:]` | Show drive label, quota used/limit |
| `XDIR` | `XDIR [filespec]` | Recursive directory listing |

### Notes

- `MOVE` and `REN` can be approximated via `COPY` + `DEL` in the meantime
- `XDIR` is a recursive variant of `DIR`
- `UNZIP` depends on Feature 39 (VFS Upload & ZIP)
- `SET` enables environment variables for future batch file support

## Dependencies

- **Feature 36: VFS Terminal Shell** — provides the shell framework and parser

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
