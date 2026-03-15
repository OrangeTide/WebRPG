-- Add Unix-style permission mode to VFS files.
-- Stored as an integer bitmask (octal 0777 = decimal 511).
-- Owner = GM, other = everyone else in scope.
-- Default 0666 (rw-rw-rw-) — readable and writable by all.
-- For directories, default 0777 (rwxrwxrwx) — listable by all.
ALTER TABLE vfs_files ADD COLUMN mode INTEGER NOT NULL DEFAULT 438;
-- 438 = 0o666 in decimal
