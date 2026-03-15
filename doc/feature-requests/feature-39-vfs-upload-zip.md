# Feature 39: VFS Upload & ZIP

Multi-file upload, ZIP download, and ZIP extraction support for the Virtual File System. Provides the backend server endpoints and client-side WASM support that COMMAND.COM (Feature 36) and Finder (Feature 37) use for bulk file transfer operations.

Shared UI components (gas gauge progress bar, drag-and-drop handling) are defined in Feature 37 (VFS File Browser) and reused by COMMAND.COM.

### Multi-file Upload

- Single multipart POST with multiple file parts and a `destination` field (the target directory path)
- Server creates parent directories as needed, writes each file respecting quota
- Returns a list of created file paths and sizes
- Folder upload via `webkitdirectory` attribute preserves relative directory structure under the destination path

### ZIP Download

- Server endpoint accepts a directory path, collects all files under it
- Builds ZIP archive to a temp file (`tempfile::NamedTempFile` in `/tmp`) — disk-backed, not in-memory
- Uses **store method** (no compression) to keep implementation simple and fast. Most VTT assets (PNG, JPG, etc.) are already compressed; text files on scratch drives are small enough that the bandwidth cost is negligible.
- Streams the temp file as the response with `Content-Disposition: attachment`
- Max size = drive quota (100 MB for C:, 20 MB for U:)
- Temp file cleaned up after response completes

### ZIP Extraction

- Server endpoint accepts an uploaded ZIP file and a destination directory
- Extracts contents respecting quota and permissions
- Validates against path traversal attacks (no `../` escapes)
- Uses the `zip` crate for reading

### File Count Limits

- **Hard limit: 250 files** — ZIP creation and extraction both refuse to proceed beyond 250 files
- **Soft limit: 25 files** — if more than 25 files are involved, prompt the user for confirmation before continuing (in both COMMAND.COM and Finder)
- These limits apply to both server-side (C:/U:) and client-side (A:/B:) operations

### Client-side ZIP (Scratch Drives)

- The `zip` crate compiles to WASM for A:/B: scratch drive support
- ZIP download builds the archive client-side from IndexedDB (store method), creates a blob URL, triggers browser download
- ZIP extraction runs in WASM, writes to IndexedDB
- Same file count limits apply (hard 250, soft 25)

## Dependencies

- **Feature 34: Virtual File System** — provides the core VFS backend and server functions
- **Feature 37: VFS File Browser** — provides shared UI components (progress bar, drag-and-drop)

## Status: Not Started

## Plan

(none yet)

## Findings

(none yet)
