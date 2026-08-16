-- Gear tags carried by a dealt item card: reach, piercing, heavy, and so on.
-- Stored comma-separated, since they are a short fixed vocabulary and are only
-- ever read as a whole.
ALTER TABLE inventory_items ADD COLUMN tags TEXT NOT NULL DEFAULT '';
