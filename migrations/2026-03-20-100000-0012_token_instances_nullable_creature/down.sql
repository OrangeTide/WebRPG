-- Revert: restore non-nullable creature_id, drop character_id
CREATE TABLE token_instances_old (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    token_id INTEGER NOT NULL REFERENCES tokens(id) UNIQUE,
    creature_id INTEGER NOT NULL REFERENCES creatures(id),
    current_hp INTEGER NOT NULL,
    max_hp INTEGER NOT NULL,
    conditions_json TEXT NOT NULL DEFAULT '[]'
);

INSERT INTO token_instances_old (id, token_id, creature_id, current_hp, max_hp, conditions_json)
SELECT id, token_id, creature_id, current_hp, max_hp, conditions_json
FROM token_instances
WHERE creature_id IS NOT NULL;

DROP TABLE token_instances;
ALTER TABLE token_instances_old RENAME TO token_instances;
