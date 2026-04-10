# Feature 75: Support Universal VTT Import/Export

Support importing and exporting maps in the Universal VTT file format
(.uvtt / .dd2vtt), as defined by Arkenforge at
https://arkenforge.com/universal-vtt-files/

The Universal VTT format (.uvtt, .dd2vtt, .df2vtt — all identical JSON) is a
single-map interchange standard supported by Dungeondraft, FoundryVTT, Fantasy
Grounds, Arkenforge, and others. The complete JSON schema is:

- `resolution` — `map_origin` (x/y), `map_size` (x/y in grid units),
  `pixels_per_grid`
- `image` — base64-encoded PNG of the map
- `line_of_sight` — array of wall/occluder geometry segments
- `objects_line_of_sight` — array of object occluder geometry (optional)
- `portals` — doors/windows: `bounds`, `rotation`, `closed` (true=door,
  false=window)
- `lights` — light sources: `position` (x/y), `color` (hex), `range`

**Scope:** This format is explicitly NOT a replacement for the FR1/FR2 media
pack format. It covers a single map with geometry; it has no concept of token
assets, collections, copyright/license, tags, descriptions, screenshots, or
README. FR1/FR2 and FR75 are complementary.

Import should parse all fields above into the internal map representation.
Export should produce a valid .uvtt from an existing map.

## Dependencies

- **Feature 30: Modular Map Maker** — map internals need to be
  sufficiently structured to hold wall/portal data for round-tripping

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
