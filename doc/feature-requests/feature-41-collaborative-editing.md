# Feature 41: Collaborative Text Editing

Upgrade the text editor (Feature 40) from lock-based single-user editing to real-time collaborative editing with multiple cursors.

- When multiple users open the same file on a shared drive (C:), all users can edit simultaneously
- Each user's cursor is shown in a distinct color with their username label
- Conflict resolution via operational transform (OT) or CRDT — choose whichever is simpler to implement
- WebSocket-based real-time sync, reusing the existing WebSocket infrastructure
- Replaces the lock-based system from Feature 40

## Dependencies

- **Feature 40: Text Editor** — provides the base editor with lock-based editing

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
