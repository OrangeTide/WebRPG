# COPY: Copy a File

Copy a file from one path to another, including across drives.

## Command Usage

```
COPY source dest
```

- `source`  Path to the file to copy.
- `dest`  Destination path or directory.

## Details

COPY reads the source file and writes it to the destination. Source and
destination can be on different drives, so COPY is how you move a file between
a scratch drive and a server drive. For example, `COPY A:/token.png C:/maps/`
copies a token from your local scratch drive to the shared session drive.

## See Also

- [DEL](help:cmd-del) delete files
- [Virtual File System](help:vfs) drives and paths
- [COMMAND.COM](help:command-com) shell overview
