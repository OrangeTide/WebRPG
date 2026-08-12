-- Bind sessions to the game modules they were set up with, and give inventory
-- items the fields a card-and-slots inventory needs.

ALTER TABLE sessions ADD COLUMN system_module_id TEXT;
ALTER TABLE sessions ADD COLUMN adventure_module_id TEXT;

-- How much of a character's Inventory Score the item eats.
ALTER TABLE inventory_items ADD COLUMN slots INTEGER NOT NULL DEFAULT 1;
-- gear, weapon, armour, treasure, or consumable.
ALTER TABLE inventory_items ADD COLUMN kind TEXT NOT NULL DEFAULT '';
-- What the item gives, in the language of the roll.
ALTER TABLE inventory_items ADD COLUMN bonus TEXT NOT NULL DEFAULT '';
-- Tick boxes for a consumable. NULL means the item is not consumable.
ALTER TABLE inventory_items ADD COLUMN uses_max INTEGER;
ALTER TABLE inventory_items ADD COLUMN uses_left INTEGER;
