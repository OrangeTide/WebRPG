# COMMAND.COM-Style Shell: Design Notes

Made by a machine. PUBLIC DOMAIN (CC0-1.0)

This document describes the design of the WebRPG in-app shell (the "Terminal"
window, branded `COMMAND.COM`) so the approach can be reused elsewhere, including
in a C project. It is written to be language-neutral. The reference
implementation is Leptos/Rust/WebAssembly over a virtual file system, but the
parsing, dispatch, drive model, and line-editing ideas port directly to C.

Reference source: `src/components/terminal.rs` (shell + UI) and `src/vfs.rs`
(the file system it drives).

## 1. What it is

A single-line interactive command prompt rendered inside a draggable window. It
looks and behaves like a DOS `COMMAND.COM`: a prompt showing the current drive
and directory, a set of terse commands, command history on the arrow keys, and
Tab completion. It operates on a virtual file system (VFS) with DOS-style drive
letters (`A:`, `B:`, `C:`, `U:`) rather than the host OS.

It is deliberately small: around twenty commands, no scripting, no pipes. The
value is in the *shape* of the thing, a familiar, self-contained command surface
that is cheap to implement and easy for users to understand.

## 2. Command lineage: MSX-DOS, not MS-DOS

The command set was lifted from **MSX-DOS**, not MS-DOS, then tweaked. This
matters for anyone reusing it, because the choices are intentional:

- **MSX-DOS command names.** `DIR`, `TYPE`, `COPY`, `DEL`/`ERASE`, `MD`/`MKDIR`,
  `RD`/`RMDIR`, `CD`/`CHDIR`, `ATTRIB`, `VER`, `CLS`. These are the MSX-DOS 2
  external and internal commands. MSX-DOS 2 is where hierarchical
  subdirectories, drive letters, and `ATTRIB` arrived on the platform, which is
  exactly the feature set a directory-based VFS needs.
- **Drive letters as first-class.** Like MSX-DOS, a bare drive letter followed by
  a colon (`C:`) is itself a command that switches the current drive. The prompt
  always shows `drive:/path>`.
- **Two-letter aliases preserved.** `MD`/`MKDIR`, `RD`/`RMDIR`, `CD`/`CHDIR`,
  `DEL`/`ERASE`, matching the MSX-DOS convention of a short internal name plus a
  spelled-out form.

### Deviations we made on purpose

- **Unix aliases added** where they cost nothing and help modern users: `CAT` as
  an alias for `TYPE`, `CHMOD` as an alias for `ATTRIB`.
- **`ATTRIB`/`CHMOD` reworked around Unix permission bits** (`U`/`G`/`O` scopes,
  `R`/`W`/`X` bits) instead of the DOS `+R +H +S +A` attribute flags, because the
  underlying VFS uses a Unix-style permission mode and an owner/GM authorization
  model. Example: `ATTRIB O-W C:/maps/dungeon.png`.
- **`GET` and `PUT` added** for the browser environment: `GET` downloads a VFS
  file to the user's real machine, `PUT` opens a file picker to upload into the
  VFS. These are the bridge between the sandboxed VFS and the host, and have no
  DOS equivalent.
- **`EXIT`** minimizes the shell window and resets shell state rather than
  terminating a process.
- **Case-insensitive commands, uppercased on parse**, matching DOS feel, but path
  arguments preserve case because the VFS is case-sensitive.
- **Dropped** the MSX-DOS commands that make no sense here (`FORMAT`, `FDISK`,
  `REN`/`RENAME` was simply never added, `BASIC`, batch files, etc.).

## 3. Architecture

The shell is a straight pipeline with a flat dispatch. There is no AST, no
tokenizer beyond quote-aware splitting, and no execution graph.

```
raw input line
   -> parse_command_line   -> (COMMAND, args_string)
   -> [special-case dispatch: drive-switch, CLS, EXIT, CD, VER, HELP]
   -> parse_args           -> (switches[], positional[])
   -> execute_command      -> match COMMAND { ... cmd_dir/cmd_type/... }
   -> Vec<String> output lines appended to the scrollback
```

### 3.1 Parsing, in three tiny stages

1. **`parse_command_line(line)`** splits off the first whitespace-delimited token
   as the command, uppercases it, and returns the remainder verbatim as the
   argument string. That is the whole command/args split.
2. **`shell_split(input)`** tokenizes an argument string respecting double
   quotes (no escape characters). Simple state machine: toggle `in_quotes` on
   `"`, break tokens on unquoted whitespace.
3. **`parse_args(args_str)`** classifies tokens into **switches** (start with `-`,
   e.g. `-w`) and **positional** args, honoring a `--` sentinel that stops switch
   parsing. This gives every command a uniform `(switches, positional)` view.

Keeping these as three separate pure functions (rather than one clever parser)
made each trivially testable and easy to reason about. None of them allocate
beyond the output vectors.

### 3.2 Dispatch

Dispatch is a single `match` on the uppercased command name, with aliases folded
into the same arm (`"TYPE" | "CAT" => cmd_type(...)`). A handful of commands are
handled *before* the main dispatch because they are special:

- **Drive switch** (`C:`) — detected by "two chars, second is `:`, first is a
  letter."
- **`CLS`** — clears the scrollback buffer (a UI action, not file I/O).
- **`EXIT`** — resets shell state and minimizes the window.
- **`CD`, `VER`, `HELP`** — synchronous; they need no file-system round trip
  (`CD` only validates and updates the current directory).

Everything else (`DIR`, `TYPE`, `COPY`, `DEL`, `MKDIR`, `RMDIR`, `ATTRIB`, `GET`,
`PUT`) is asynchronous because it touches the VFS, which may be a network or
IndexedDB call. See 3.4.

### 3.3 Uniform command handler signature

Every file-touching handler has essentially the same shape:

```
async fn cmd_x(shell: &ShellState,
               switches: &[String],      // some commands omit this
               positional: &[String],
               session_id, scratch) -> Vec<String>
```

It takes the shell context and parsed args, and returns a vector of output lines.
Handlers never write to the screen directly; they *return text*. The caller owns
the scrollback. This separation is worth keeping: it makes handlers pure-ish and
testable, and it means the same handler could feed a GUI list, a log, or a pipe
later.

### 3.4 Synchronous vs asynchronous commands

Because the VFS backends are async (server DB for `C:`/`U:`, browser IndexedDB for
`A:`/`B:`), file commands are dispatched onto an async task. The pattern is:

1. Snapshot the parts of shell state the task needs (the current working
   directory is cloned so the borrow does not outlive the call).
2. Spawn the async command.
3. When it resolves, append its returned lines to the output and refresh the
   prompt.

In C this maps to whatever your I/O model is: if the VFS is synchronous
(local disk, in-memory), drop the async entirely and call the handler inline. If
it is not (network), the equivalent is a callback/continuation or a worker thread
that posts result lines back to the UI thread. The key design point is that *the
handler's contract is the same either way*: args in, lines out.

## 4. The drive and path model

The shell is thin; most of the substance is the VFS it sits on. The path model is
the part most worth stealing.

- **Drives are an enum**, not strings: `A`, `B`, `C`, `U`. Each drive carries its
  own *scope* and *backend*:
  - `A:`, `B:` — per-tab **scratch** drives, client-side only (browser
    IndexedDB), ephemeral.
  - `C:` — per-game-session drive, server-backed, persistent.
  - `U:` — per-user drive, server-backed.
  - `D:`..`T:` reserved.
- **`VfsPath { drive, path }`** is the canonical absolute location: a drive plus
  an absolute POSIX-style path string. Everything normalizes to this.
- **Path parsing** (`VfsPath::parse` / `resolve`) handles: absolute-with-drive
  (`C:/maps/x.png`), drive-relative, and cwd-relative inputs, resolving `.` and
  `..` against the shell's current directory.
- **The prompt is derived** from `ShellState.cwd` (`format!("{cwd}> ")`), so it is
  always correct by construction, never manually kept in sync.

`ShellState` itself is tiny, essentially just the current working directory. All
the heavy state lives in the VFS. That is a deliberate split: the shell is a
*view and a parser*, the VFS is the *model*.

The lesson for a C port: define `typedef enum { DRIVE_A, ... } drive_t;` and a
`struct vfs_path { drive_t drive; char *path; }`, and route every command through
one `vfs_resolve(const char *input, const struct vfs_path *cwd, struct vfs_path
*out)`. Do not let raw path strings leak into command handlers; resolve to a
`vfs_path` at the boundary.

## 5. Line editing and interaction

The interactive layer is independent of the command layer and equally reusable:

- **Command history.** A growable list of submitted lines. Up/Down arrows walk it
  via a `history_pos` cursor (`None` = editing a fresh line). Consecutive
  duplicates are not stored. Submitting resets the cursor to `None`.
- **Prompt refresh.** After every command the prompt is recomputed from the cwd,
  so a `CD` or drive-switch immediately shows the new location.
- **Tab completion** (`tab_complete`) is context-sensitive:
  - At the first token → complete against the **command name table**.
  - Otherwise → resolve the partial last token to a directory + filename prefix,
    list that directory in the VFS, and complete against the entries (directories
    get a trailing `/`).
  - When several candidates share a longer **common prefix**, the input is
    extended to that prefix (`common_prefix(matches)`); the full candidate list
    is also returned so the UI can show options.
- **`CLS`** clears the scrollback; **`EXIT`** resets and hides the window.

The command-name table appears in three places (dispatch, `tab_complete`, and
`HELP`). See Limitations, this is the main duplication.

## 6. Output model

Output is a flat, append-only list of text lines (`Vec<TermLine>` where
`TermLine` is just a `String`). Submitting echoes `prompt + input` as a line,
then appends the command's returned lines. `CLS` empties the vector. There is no
styling per line beyond the terminal-wide theme.

Themes are a small enum (`GrayOnBlack`, `GreenOnBlack`, `AmberOnBlack`,
`WhiteOnBlue`, `HotDogStand`) mapping to CSS color variables, persisted to local
storage. A nice, cheap touch: it costs a handful of lines and makes the terminal
feel like a real one.

## 7. Lessons learned

- **A flat `match` beats a command-table abstraction at this scale.** Twenty
  commands in one dispatch `match` with aliases folded into arms is clearer than
  a registry of function pointers. Reach for a table only when commands become
  dynamic or pluggable.
- **Handlers return lines; they do not print.** This one contract kept the
  command layer free of UI concerns and made everything testable. Keep it even if
  it feels indirect.
- **Normalize to `(drive, absolute_path)` at the edge.** Every path bug we did
  *not* have came from resolving relative/drive-relative/absolute inputs into one
  canonical `VfsPath` immediately, so handlers only ever see resolved paths.
- **Derive the prompt from state.** Formatting the prompt from `cwd` on every
  refresh, rather than mutating a prompt string, removed a whole category of
  "prompt shows the wrong directory" bugs.
- **Split the three parse stages.** Command/args split, quote-aware tokenize, and
  switch/positional classification as separate pure functions were each a few
  lines and each independently correct.
- **Uniform `(switches, positional)` for every command** meant no command
  reinvented argument handling, and `--` worked everywhere for free.
- **Sync/async is a caller concern, not a handler concern.** Because handlers are
  "args in, lines out," making some synchronous and some async was a dispatch
  detail, not a rewrite.
- **The MSX-DOS command set was a good anchor.** Picking an existing, coherent,
  small command vocabulary avoided bikeshedding names and gave users something
  familiar. Borrow a real historical CLI rather than inventing one.

## 8. Limitations

- **No pipes, redirection, or command chaining.** One command per line, output
  goes to the screen only. There is no `|`, `>`, `>>`, `<`, or `&&`.
- **No wildcard/glob expansion.** `DEL *.png` does not expand; commands that say
  "filespec" take a single literal path. (`DIR` filtering is the exception.)
- **No `REN`/`RENAME` or `MOVE`.** Rename must be done as `COPY` + `DEL`.
- **No scripting / batch files.** No `.BAT` equivalent, no variables, no control
  flow.
- **No escape characters in quoting.** `shell_split` toggles on `"` but cannot
  represent a literal quote or a quote-inside-a-word.
- **Command list is duplicated in three places** (dispatch match, tab-complete
  table, HELP text). Adding a command means editing all three, and they can
  drift. A single source-of-truth command table would fix this (see 9).
- **Every file command re-snapshots the cwd** and spawns its own task; there is
  no shared command context object.
- **No streaming output.** A command returns all its lines at once, so a long
  `TYPE` or a slow operation cannot show partial/progressive output.
- **No interrupt.** There is no Ctrl-C to cancel a running async command.
- **Help/switch parsing is per-command and ad hoc.** There is no shared
  "usage/options" declaration a command is validated against.

## 9. Extensions worth considering

These are the enhancements we would have made with more time, roughly in order of
value-to-effort:

- **Single command registry.** Replace the three duplicated lists with one table:
  `{ name, aliases[], help_summary, help_detail, handler, is_async }`. Dispatch,
  tab completion, and HELP all read from it. This is the first thing to do in a C
  port, a `static const struct command commands[]` array, iterated for dispatch
  and completion alike.
- **Wildcard/glob expansion** for `DIR`, `DEL`, `COPY`, `TYPE`. Expand `*`/`?`
  against the target directory before the handler runs, so handlers still receive
  concrete paths. This unlocks the DOS muscle memory users expect.
- **`REN`/`RENAME` and `MOVE`** as first-class VFS operations (cheaper and safer
  than copy+delete, and atomic if the VFS supports it).
- **Output redirection** (`> file`, `>> file`) by having the runner capture the
  returned line vector and write it to a VFS file instead of the screen. Because
  handlers already return lines, this is nearly free, intercept at the caller.
- **Pipes** (`cmd | cmd`) by feeding one command's returned lines as another's
  stdin. Requires giving handlers an optional input-lines parameter; the
  "lines in, lines out" shape is already 90% of the way there.
- **Batch files** (`.BAT`): read a VFS file and feed its lines through the same
  submit path. Add simple `%1` argument substitution and a `REM` comment command.
- **Environment variables and `SET`**, plus prompt customization (`PROMPT $p$g`).
- **Streaming/progressive output** for long operations, and a **Ctrl-C**
  cancellation path for async commands.
- **A `MORE`/paged `TYPE`** for long files, and column-aware `DIR` that adapts to
  the window width.
- **Escaped quoting** in `shell_split` (backslash escapes, or doubled `""`).
- **Per-command usage declarations** (name, allowed switches, arity) that the
  runner validates against, producing consistent error messages and auto-generated
  HELP.

## 10. Porting checklist for a C project

1. Define `drive_t` (enum) and `struct vfs_path { drive_t drive; char *path; }`.
   Write `vfs_resolve(input, cwd, out)` and resolve *all* path args at the edge.
2. Write the three parse stages as separate functions: split command from args,
   quote-aware tokenize, classify switches vs positional (honor `--`).
3. Define one command table:
   `struct command { const char *name; const char *aliases; const char *help;
   int (*run)(shell_t*, char **switches, int nsw, char **args, int nargs,
   line_sink_t *out); };`. Dispatch, completion, and help all read it.
4. Make handlers write to a `line_sink` (a callback or a growable string list),
   not to `stdout` directly. The caller decides whether that sink is the screen,
   a file (redirection), or another command (pipe).
5. Keep `shell_t` tiny: current drive + cwd. Put real state in the VFS.
6. Recompute the prompt from the cwd after every command.
7. Implement history (ring buffer or growable list + cursor) and Tab completion
   (command table first token, VFS listing otherwise) in the input layer, kept
   separate from the command layer.
