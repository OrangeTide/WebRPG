# DIR: List Directory

List the files and subdirectories in a directory.

## Command Usage

```
DIR [-w] [filespec]
```

- `-w`  Wide format, names only in multiple columns.
- `filespec`  Optional path or wildcard pattern to match.

## Details

With no argument, DIR lists the current directory. Pass a path to list a
different directory, or a wildcard pattern to filter the listing. Patterns use
`*` to match any run of characters and `?` to match a single character, for
example `DIR *.png`.

The default listing shows one entry per line with its size and modification
time. The `-w` switch prints names only, packed into columns.

## See Also

- [CD](help:cmd-cd) change directory
- [TYPE](help:cmd-type) display a file
- [COMMAND.COM](help:command-com) shell overview
