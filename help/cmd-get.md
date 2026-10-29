# GET: Download a File

Download a file from the Virtual File System to your computer.

## Command Usage

```
GET filespec
```

- `filespec`  Path to the file to download.

## Details

GET sends the file to your browser, which saves it to your computer like any
other download. This is the counterpart to [PUT](help:cmd-put), which uploads
files into the VFS. Downloading a whole directory as a ZIP is not yet
supported.

## See Also

- [PUT](help:cmd-put) upload files
- [TYPE](help:cmd-type) display a file in the terminal
- [COMMAND.COM](help:command-com) shell overview
