# Feature 38: Online Help System

Provide an in-game help browser window for accessing internal documentation. The visual design is based on the Windows 3.0 Help system (https://guidebookgallery.org/pics/gui/system/features/help/win30.png), re-themed to match the project's NeXTSTEP visual style.

### Visual Design

A dedicated window panel with:
- **Title bar** — NeXTSTEP gray gradient, showing the current topic title
- **Button bar** — Back, History, Index buttons (styled as NeXTSTEP raised buttons)
- **Content area** — Rendered help content with clickable cross-reference links (green underlined text, Win3.0 style), section headings, and inline formatting

The layout and navigation mirrors Windows 3.0 Help (topic-based with hyperlinks between topics), but all chrome uses the NeXTSTEP visual language already established in the project.

Full-text search is a future enhancement. For now, the Index button shows a list of all topic titles/slugs for navigation.

### Document Backend

- Help documents are authored as **Markdown**
- Each document has a topic slug (e.g., `cmd-dir`, `getting-started`, `dice-rolling`)
- Documents are read-only in-game — edited offline in the `help/` folder
- In-game editing and wiki-style version history are future enhancements

### Help File Distribution & Storage

The distribution ships with a `help/` folder containing all system help documents as Markdown files. This makes them easy to edit and update with each release.

**Storage approach (to be explored further):**
- The simplest initial approach: read help files directly from the `help/` folder at runtime. No database, no ingestion, no migration. The server reads Markdown from disk on each request (with optional caching).
- A database schema (e.g., `help_topics` + `help_revisions`) can be added later when in-game editing and version history are needed.
- **System help files** ship in `help/` and are clobbered on upgrade — just overwrite the folder.
- **User-created help files** can coexist in the same folder or a separate `help/user/` subfolder. Upstream updates won't conflict since end-users are unlikely to edit system topics. Best effort preservation on upgrade.
- The exact storage strategy can be decided during implementation — the important thing is the user-facing behavior, not the underlying mechanism.

### Help Document Structure

Help documents use standard Markdown with a convention for the COMMAND.COM integration:

```markdown
# DIR — List Directory

Brief description of the command.

## Command Usage

```
DIR [/W] [/P] [filespec]
```

Options:
- `/W` — Wide format
- `/P` — Paged output

## Details

Extended documentation, examples, cross-references, etc.

## See Also

- [XDIR](help:cmd-xdir) — Recursive directory listing
- [CD](help:cmd-cd) — Change directory
```

The `## Command Usage` section is the key convention: COMMAND.COM extracts and displays this section inline in the terminal when `HELP <command>` is used.

### COMMAND.COM Integration

The `HELP` command in COMMAND.COM (Feature 36) works as follows:

1. Maintain a **whitelist of known command slugs** (e.g., `dir` → `cmd-dir`, `copy` → `cmd-copy`). This is a simple static map — no fuzzy matching needed.
2. `HELP DIR` — looks up `dir` in the whitelist, finds slug `cmd-dir`, fetches the help document, extracts the `## Command Usage` section, and prints it inline in the terminal.
3. `HELP getting-started` — not in the command whitelist, so it opens the GUI help browser window navigated to that topic.
4. `HELP` (no argument) — opens the GUI help browser to the index/home page.

This keeps the terminal experience fast for command help while providing full documentation access through the GUI window.

### Markdown Rendering

Use `pulldown-cmark` (~25KB in WASM, zero dependencies, already the standard Rust Markdown library). The rendering pipeline:

1. Parse Markdown with `pulldown-cmark` into an event stream
2. Filter events to rewrite `help:topic-slug` URLs into internal navigation calls
3. Render to HTML string
4. Inject into the content area via `innerHTML`
5. Style `help:` links as green underlined text via CSS class
6. Attach a click handler on the content area that intercepts `help:` link clicks for internal navigation (no page reload)

This gives correct Markdown rendering (headings, bold, italic, code blocks, lists, links) with minimal code. The `help:` link rewriting is a ~10-line filter on the parser event stream.

### Cross-References

Help documents link to each other using a `help:` URI scheme: `[link text](help:topic-slug)`. The help browser resolves these as internal navigation. In the rendered content area, these appear as clickable green underlined links (Win3.0 style).

## Dependencies

(none — this is a standalone window. Feature 36's `HELP` command consumes this system, not the other way around.)

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
