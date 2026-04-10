# Feature 1: VTT media pack file format

Document a file format for map and token packs called a VTT media
pack. The file is a ZIP file with a manifest.json that describes each
map and token, including media type, description, copyright license,
and tags for each file. optional screenshots can be included in with
names in the form of "screenshot00.jpg" to "screenshot99.jpg" and a
README.md to describe the media pack. These screenshots are not to be
included in the manifest.json

## Status: Not Started

## Plan

TBD

## Findings

Universal VTT (FR75) is NOT a replacement for this format. The two serve different purposes:

- Universal VTT (.uvtt/.dd2vtt) is a single-map interchange format covering:
  image (base64 PNG), grid resolution, walls/line-of-sight, portals/doors,
  and lights. It has no concept of collections, tokens, copyright, tags, or
  descriptions.

- This format (FR1) is a distribution/packaging format for *collections* of
  maps and tokens with attribution metadata. The capabilities that FR1 covers
  and Universal VTT does not: multiple maps per ZIP, token assets, copyright/
  license, description, tags per file, screenshots, and README.

These two formats are complementary, not competing. A media pack (FR1/FR2)
could contain .uvtt files as its per-map format internally, but the pack
wrapper and metadata layer are still needed.
