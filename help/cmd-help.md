# HELP: Show Help

Show help for a command, or open the graphical help browser.

## Command Usage

```
HELP [topic]
```

- With no argument, open the help browser at the index.
- `HELP command`  Print that command's usage inline, for example `HELP DIR`.
- `HELP topic`  Open the help browser to any other topic.

## Details

HELP bridges the terminal and the graphical help browser. For a known
COMMAND.COM command it prints the Command Usage section right in the terminal so
you do not lose your place. For anything else, such as `HELP getting-started`,
it opens the help browser to that page.

## See Also

- [COMMAND.COM](help:command-com) shell overview and command list
- [Getting Started](help:getting-started) a tour of the interface
