# ATTRIB: File Permissions

Display or change the permissions on a file. Changing permissions is limited to
the game master.

## Command Usage

```
ATTRIB [+|-attr ...] filespec
```

- With only a `filespec`, show the file's current permissions.
- `+attr` / `-attr`  Grant or revoke a permission.
- Scopes: `U` owner, `G` group, `O` other.
- Bits: `R` read, `W` write, `X` execute.

## Details

Permissions follow the Unix model with three scopes and three bits each. An
attribute combines a scope and a bit, so `O-W` revokes write from others and
`G+R` grants read to the group. For example:

```
ATTRIB O-W C:/maps/dungeon.png
```

removes write access for other players from that map. See [Virtual File
System](help:vfs) for how permissions keep files hidden from players. CHMOD is
an alias for ATTRIB.

## See Also

- [Virtual File System](help:vfs) drives and permissions
- [DIR](help:cmd-dir) list directory contents
- [COMMAND.COM](help:command-com) shell overview
