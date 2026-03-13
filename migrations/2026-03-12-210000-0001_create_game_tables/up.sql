CREATE TABLE rpg_templates (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    schema_json TEXT NOT NULL
);

CREATE TABLE sessions (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name VARCHAR(255) NOT NULL,
    gm_user_id INTEGER NOT NULL REFERENCES users(id),
    template_id INTEGER REFERENCES rpg_templates(id),
    active BOOLEAN NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE session_players (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    role VARCHAR(20) NOT NULL DEFAULT 'player',
    UNIQUE(session_id, user_id)
);

CREATE TABLE characters (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    name VARCHAR(255) NOT NULL,
    data_json TEXT NOT NULL DEFAULT '{}',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE character_resources (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL REFERENCES characters(id),
    name VARCHAR(100) NOT NULL,
    current_value INTEGER NOT NULL,
    max_value INTEGER NOT NULL
);

CREATE TABLE maps (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    name VARCHAR(255) NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    cell_size INTEGER NOT NULL DEFAULT 40,
    background_url TEXT
);

CREATE TABLE fog_of_war (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    map_id INTEGER NOT NULL REFERENCES maps(id),
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    UNIQUE(map_id, x, y)
);

CREATE TABLE creatures (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    template_id INTEGER REFERENCES rpg_templates(id),
    name VARCHAR(255) NOT NULL,
    stat_data_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE tokens (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    map_id INTEGER NOT NULL REFERENCES maps(id),
    label VARCHAR(100) NOT NULL,
    x REAL NOT NULL DEFAULT 0.0,
    y REAL NOT NULL DEFAULT 0.0,
    color VARCHAR(20) NOT NULL DEFAULT '#ff0000',
    size INTEGER NOT NULL DEFAULT 1,
    visible BOOLEAN NOT NULL DEFAULT 1,
    character_id INTEGER REFERENCES characters(id),
    creature_id INTEGER REFERENCES creatures(id)
);

CREATE TABLE token_instances (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    token_id INTEGER NOT NULL REFERENCES tokens(id) UNIQUE,
    creature_id INTEGER NOT NULL REFERENCES creatures(id),
    current_hp INTEGER NOT NULL,
    max_hp INTEGER NOT NULL,
    conditions_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE chat_messages (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    is_dice_roll BOOLEAN NOT NULL DEFAULT 0,
    dice_result TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE inventory_items (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    quantity INTEGER NOT NULL DEFAULT 1,
    owner_character_id INTEGER REFERENCES characters(id),
    is_party_item BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE initiative (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL REFERENCES sessions(id),
    label VARCHAR(255) NOT NULL,
    initiative_value REAL NOT NULL,
    is_current_turn BOOLEAN NOT NULL DEFAULT 0,
    token_id INTEGER REFERENCES tokens(id),
    character_id INTEGER REFERENCES characters(id),
    sort_order INTEGER NOT NULL DEFAULT 0
);
