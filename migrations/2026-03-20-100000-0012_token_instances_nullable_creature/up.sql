-- Make creature_id nullable so token_instances can also track character tokens.
-- Add character_id column for character token instances.
-- SQLite doesn't support ALTER COLUMN, so we recreate the table.

CREATE TABLE token_instances_new (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    token_id INTEGER NOT NULL REFERENCES tokens(id) UNIQUE,
    creature_id INTEGER REFERENCES creatures(id),
    character_id INTEGER REFERENCES characters(id),
    current_hp INTEGER NOT NULL,
    max_hp INTEGER NOT NULL,
    conditions_json TEXT NOT NULL DEFAULT '[]'
);

INSERT INTO token_instances_new (id, token_id, creature_id, current_hp, max_hp, conditions_json)
SELECT id, token_id, creature_id, current_hp, max_hp, conditions_json
FROM token_instances;

DROP TABLE token_instances;
ALTER TABLE token_instances_new RENAME TO token_instances;
