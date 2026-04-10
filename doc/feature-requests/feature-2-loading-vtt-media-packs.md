# Feature 2: loading VTT media packs

Support loading of VTT media packs by copying them into a media directory
before the server starts, or by uploading them from the web client.

## Status: Not Started

## Plan

TBD

## Findings

Universal VTT (FR75) does not replace this feature. FR2 covers loading asset
packs (collections of maps and tokens with metadata), while FR75 covers
importing individual maps in the Universal VTT interchange format. Both are
needed; they address different layers of the pipeline:

- FR75: import a single map from another VTT tool (Dungeondraft, FoundryVTT,
  Fantasy Grounds, etc.)
- FR2: load a curated pack of many maps/tokens distributed as a ZIP with a
  manifest
