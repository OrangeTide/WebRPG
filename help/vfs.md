# Virtual File System

WebRPG stores files on a Virtual File System (VFS) with lettered drives, much
like classic DOS. Both the [File Viewer](help:file-viewer) and
[COMMAND.COM](help:command-com) read and write the same drives.

## Drives

| Drive | Label | Where it lives | Shared with |
|-------|-------|----------------|-------------|
| A: | Scratch | This browser tab (IndexedDB) | No one, private to this tab |
| B: | Scratch | This browser tab (IndexedDB) | No one, private to this tab |
| C: | Session | The server | Everyone in the session |
| U: | Personal | The server | Your user account only |

Scratch drives (A: and B:) are temporary. Their contents are lost when you
close the browser tab. Use C: or U: for anything you want to keep.

## Paths

Paths use forward slashes and begin with a drive, for example
`C:/maps/dungeon.png`. A path without a drive is resolved against the current
drive and directory in COMMAND.COM.

## Quotas

Server drives have a storage quota. The File Viewer status bar shows how much
space a drive has used and how much remains.

## Permissions

Files on server drives carry Unix-style permissions with three scopes, owner
(U), group (G), and other (O), each with read, write, and execute bits. The
game master can change permissions with [ATTRIB](help:cmd-attrib). This is how
the GM keeps some files hidden from players.

## See Also

- [COMMAND.COM](help:command-com) command-line terminal
- [File Viewer](help:file-viewer) graphical file browser
- [ATTRIB](help:cmd-attrib) show or change file permissions
