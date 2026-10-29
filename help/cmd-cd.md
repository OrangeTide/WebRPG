# CD: Change Directory

Change or display the current working directory.

## Command Usage

```
CD [d:][path]
```

- With no argument, print the current directory.
- `d:`  Switch to the root of drive `d:`.
- `path`  Move to the given directory, absolute or relative.

## Details

CD with no argument shows where you are. `CD ..` moves to the parent directory.
`CD C:` jumps to the root of the C: drive. A bare drive letter typed on its own,
such as `C:`, also switches drives. CHDIR is an alias for CD.

## See Also

- [DIR](help:cmd-dir) list directory contents
- [Virtual File System](help:vfs) drives and paths
- [COMMAND.COM](help:command-com) shell overview
