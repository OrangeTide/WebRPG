# Feature 43: Consistent Hot Keys Across All Apps

Use a consistent set of hot keys in all apps. Default to native keys to match what input elements already support. ctrl-A for select all, ctrl-X for cut, ctrl-C for copy, ctrl-V for paste. Finder would need the most work so that its file/directories are selected and pasted appropriately.

## Dependencies

- **Feature 37: VFS File Browser** — Finder needs file selection model before cut/copy/paste can operate on files

## Status: Not Started

## Plan

- Accept both `event.metaKey` (Cmd on Mac) and `event.ctrlKey` (Ctrl on Windows/Linux) for all shortcut bindings — this is the pragmatic cross-platform approach most web apps use.
- For native input elements (text fields, textareas), the browser already handles Cmd/Ctrl-A/C/X/V — no work needed there.
- Custom UI elements (Finder file list, window manager, etc.) need explicit keyboard event handlers that check `metaKey || ctrlKey`.
- **Terminal/command.com is a special case**: Ctrl+C conventionally means SIGINT in terminals. The terminal app should handle Ctrl+C differently from Cmd+C — Ctrl+C sends interrupt, Cmd+C (or a different binding) copies selected text. Consider following the convention used by modern terminals (e.g., Ctrl+Shift+C for copy).

## Findings

(none yet)
