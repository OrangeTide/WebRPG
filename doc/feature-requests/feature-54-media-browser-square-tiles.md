# Feature 54: Media Browser Square Tiles

Fix media browser image tiles to maintain square aspect ratio.

## Description

The Media Browser (image mode) auto-adjusts the aspect ratio of image icons to suit the widest image. This causes square images (avatars) to get cropped when wider images (16:9 backgrounds) are also loaded.

**Fix:** Keep image tiles in the browser square. Scale non-square images to fit within the square tile with padding, rather than using a zoom-to-fill that clips.

## Dependencies

None.

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
