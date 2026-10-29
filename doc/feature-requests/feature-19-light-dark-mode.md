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

### Audit (2026-10-29)

Step 1 and most of step 2 of the recommendation above have since landed. The
"zero CSS custom properties" finding from 2026-03-16 is now stale.

- `style/main.css` defines a semantic token palette on `:root` (around
  `style/main.css:10-37`): `--color-*`, `--chrome-*`, and related tokens.
- Roughly 387 `var()` references now consume those tokens across the stylesheet.
- Still missing for this feature: the toggle control, a second (sandy brown
  light) palette, a `data-theme` selector or `prefers-color-scheme` media query,
  and persistence of the choice.

Remaining work is small relative to the original scope: add the light palette
values and a switch that flips `data-theme`, reusing the existing token layer.
This is one of the cheaper features to finish.
