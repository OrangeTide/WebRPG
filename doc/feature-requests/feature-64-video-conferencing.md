# Feature 64: Video Conferencing

Investigate architecture for embedding player video conferencing.

## Description

Investigate an architecture for embedding player video conferencing in a meaningful way.

### References
- https://github.com/security-union/videocall-rs
- https://github.com/BiagioFesta/wtransport

### Architecture notes
- Likely a separate wtransport+websocket server to handle video calls
- VTT server could fetch occasional low-resolution snapshot images to update small avatar icon overlays in the initiative bar
- **Completely optional feature** — significant additional setup for administrators
- Most groups will use their own voice/video software outside of WebRPG
- Investigate whether a third-party video conference package could achieve the goals with less development work

## Dependencies

None (standalone investigation).

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
