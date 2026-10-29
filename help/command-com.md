# COMMAND.COM

COMMAND.COM is a DOS-style command-line shell for the [Virtual File
System](help:vfs). It offers a keyboard-driven alternative to the graphical
[File Viewer](help:file-viewer) for browsing drives and moving files around.

Open it from the Terminal tile on the dock. Type a command and press Enter.

## Prompt and Drives

The prompt shows the current drive and directory, for example `C:/maps/>`.
Switch drives by typing a drive letter and colon, such as `C:` or `U:`. Change
directory with [CD](help:cmd-cd). See [Virtual File System](help:vfs) for what
each drive is.

## Getting Help

Type `HELP` on its own to open the graphical help browser at the index. Type
`HELP` followed by a command name to print that command's usage inline in the
terminal, for example `HELP DIR`. Type `HELP` followed by any other topic slug
to open the help browser to that topic.

## Commands

| Command | Action |
|---------|--------|
| [DIR](help:cmd-dir) | List directory contents. |
| [CD](help:cmd-cd) | Change or show the working directory. |
| [TYPE](help:cmd-type) | Display a text file. |
| [COPY](help:cmd-copy) | Copy a file. |
| [DEL](help:cmd-del) | Delete files. |
| [MKDIR](help:cmd-mkdir) | Create a directory. |
| [RMDIR](help:cmd-rmdir) | Remove an empty directory. |
| [ATTRIB](help:cmd-attrib) | Show or change file permissions. |
| [GET](help:cmd-get) | Download a file to your computer. |
| [PUT](help:cmd-put) | Upload files from your computer. |
| [CLS](help:cmd-cls) | Clear the screen. |
| [VER](help:cmd-ver) | Show version information. |
| [EXIT](help:cmd-exit) | Close the terminal. |
| [HELP](help:cmd-help) | Show help. |

## History and Completion

Press the Up and Down arrows to recall previous commands. Press Tab to complete
command names and file paths.

## See Also

- [Virtual File System](help:vfs) drives, quotas, and permissions
- [File Viewer](help:file-viewer) graphical file browser
- [Getting Started](help:getting-started) a tour of the interface
