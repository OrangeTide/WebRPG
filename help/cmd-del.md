# DEL: Delete Files

Delete one or more files.

## Command Usage

```
DEL filespec
```

- `filespec`  Path or wildcard pattern of files to delete.

## Details

DEL removes files that match the given path or pattern. Patterns use `*` and
`?`, so `DEL *.tmp` deletes every file ending in `.tmp` in the current
directory. DEL removes files, not directories; use [RMDIR](help:cmd-rmdir) for
an empty directory. ERASE is an alias for DEL.

Deletion is immediate and cannot be undone, so check the pattern first with
[DIR](help:cmd-dir).

## See Also

- [DIR](help:cmd-dir) list directory contents
- [RMDIR](help:cmd-rmdir) remove a directory
- [COMMAND.COM](help:command-com) shell overview
