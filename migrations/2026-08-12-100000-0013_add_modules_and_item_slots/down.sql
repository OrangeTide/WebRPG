-- Drop the columns added by this migration. Requires SQLite 3.35 or newer,
-- which is what the bundled libsqlite3-sys provides.

ALTER TABLE inventory_items DROP COLUMN uses_left;
ALTER TABLE inventory_items DROP COLUMN uses_max;
ALTER TABLE inventory_items DROP COLUMN bonus;
ALTER TABLE inventory_items DROP COLUMN kind;
ALTER TABLE inventory_items DROP COLUMN slots;

ALTER TABLE sessions DROP COLUMN adventure_module_id;
ALTER TABLE sessions DROP COLUMN system_module_id;
