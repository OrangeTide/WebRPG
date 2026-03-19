# Feature 19: Light/Dark Mode Switch

Add a light mode / dark mode switch. Use the current dark blue color scheme for
dark mode. Add a new sandy brown color scheme for light mode.

## Status: Not Started

## Plan

TBD

## Findings

### Current state (2026-03-16)

- Single CSS file: `style/main.css` (~2,139 lines, 37KB)
- **49 unique hex colors** used across **293 usages** — all hardcoded, zero CSS custom properties
- No theme system: no `var()` references, no `prefers-color-scheme` handling, no class-based theme selectors
- Most-used colors: `#e94560` (accent pink, 35×), `#0f3460` (dark blue, 35×), `#0d1b30` (dark bg, 16×), `#fff`/`#ddd`/`#aaa`/`#888`/`#555` (grays, ~80× combined)
- Win95-style UI section (~100 lines) uses its own grayscale palette, isolated from the main theme
- localStorage already used for window layouts — can reuse for theme preference

### Recommendation

Before implementing light/dark toggle, develop a **theme system** first:

1. **Consolidate the color palette.** 49 unique colors is too many and inconsistent. Reduce to a smaller, intentional set of semantic color tokens (e.g. `--bg-primary`, `--bg-surface`, `--text-primary`, `--text-muted`, `--accent`, `--border`, `--error`, `--success`, `--warn`). Many of the current colors are near-duplicates that can be unified.
2. **Convert to CSS custom properties.** Replace all hardcoded hex values with `var()` references to the consolidated palette. This is the bulk of the mechanical work.
3. **Then** adding light mode becomes straightforward — just define an alternate set of variable values under a `.light-mode` class or `prefers-color-scheme` media query.

Doing it in this order avoids converting 49 colors to variables only to realize half of them should have been the same color. The palette consolidation pays for itself in long-term maintainability regardless of whether light mode ships.
