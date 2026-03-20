// @generated automatically by Diesel CLI.

diesel::table! {
    character_resources (id) {
        id -> Integer,
        character_id -> Integer,
        name -> Text,
        current_value -> Integer,
        max_value -> Integer,
    }
}

diesel::table! {
    characters (id) {
        id -> Integer,
        session_id -> Integer,
        user_id -> Integer,
        name -> Text,
        data_json -> Text,
        created_at -> Timestamp,
        portrait_url -> Nullable<Text>,
    }
}

diesel::table! {
    chat_messages (id) {
        id -> Integer,
        session_id -> Integer,
        user_id -> Integer,
        message -> Text,
        is_dice_roll -> Bool,
        dice_result -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    creatures (id) {
        id -> Integer,
        session_id -> Integer,
        template_id -> Nullable<Integer>,
        name -> Text,
        stat_data_json -> Text,
        image_url -> Nullable<Text>,
    }
}

diesel::table! {
    fog_of_war (id) {
        id -> Integer,
        map_id -> Integer,
        x -> Integer,
        y -> Integer,
    }
}

diesel::table! {
    initiative (id) {
        id -> Integer,
        session_id -> Integer,
        label -> Text,
        initiative_value -> Float,
        is_current_turn -> Bool,
        token_id -> Nullable<Integer>,
        character_id -> Nullable<Integer>,
        sort_order -> Integer,
    }
}

diesel::table! {
    inventory_items (id) {
        id -> Integer,
        session_id -> Integer,
        name -> Text,
        description -> Text,
        quantity -> Integer,
        owner_character_id -> Nullable<Integer>,
        is_party_item -> Bool,
    }
}

diesel::table! {
    maps (id) {
        id -> Integer,
        session_id -> Integer,
        name -> Text,
        width -> Integer,
        height -> Integer,
        cell_size -> Integer,
        background_url -> Nullable<Text>,
        default_token_color -> Text,
    }
}

diesel::table! {
    media (id) {
        id -> Integer,
        hash -> Text,
        content_type -> Text,
        media_type -> Text,
        size_bytes -> BigInt,
        uploaded_by -> Integer,
        created_at -> Timestamp,
    }
}

diesel::table! {
    media_tags (id) {
        id -> Integer,
        media_id -> Integer,
        tag -> Text,
    }
}

diesel::table! {
    rpg_templates (id) {
        id -> Integer,
        name -> Text,
        description -> Text,
        schema_json -> Text,
    }
}

diesel::table! {
    session_players (id) {
        id -> Integer,
        session_id -> Integer,
        user_id -> Integer,
        role -> Text,
    }
}

diesel::table! {
    sessions (id) {
        id -> Integer,
        name -> Text,
        gm_user_id -> Integer,
        template_id -> Nullable<Integer>,
        active -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    token_instances (id) {
        id -> Integer,
        token_id -> Integer,
        creature_id -> Nullable<Integer>,
        character_id -> Nullable<Integer>,
        current_hp -> Integer,
        max_hp -> Integer,
        conditions_json -> Text,
    }
}

diesel::table! {
    tokens (id) {
        id -> Integer,
        map_id -> Integer,
        label -> Text,
        x -> Float,
        y -> Float,
        color -> Text,
        size -> Integer,
        visible -> Bool,
        character_id -> Nullable<Integer>,
        creature_id -> Nullable<Integer>,
        image_url -> Nullable<Text>,
        rotation -> Float,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        username -> Text,
        display_name -> Text,
        email -> Text,
        access_level -> Integer,
        locked -> Bool,
        passcrypt -> Nullable<Text>,
        ping_color -> Text,
        suppress_tooltips -> Integer,
    }
}

diesel::table! {
    vfs_files (id) {
        id -> Integer,
        drive -> Text,
        session_id -> Nullable<Integer>,
        user_id -> Nullable<Integer>,
        path -> Text,
        is_directory -> Bool,
        size_bytes -> BigInt,
        content_type -> Nullable<Text>,
        inline_data -> Nullable<Binary>,
        media_hash -> Nullable<Text>,
        modified_by -> Nullable<Integer>,
        created_at -> Integer,
        updated_at -> Integer,
        mode -> Integer,
    }
}

diesel::joinable!(character_resources -> characters (character_id));
diesel::joinable!(characters -> sessions (session_id));
diesel::joinable!(characters -> users (user_id));
diesel::joinable!(chat_messages -> sessions (session_id));
diesel::joinable!(chat_messages -> users (user_id));
diesel::joinable!(creatures -> rpg_templates (template_id));
diesel::joinable!(creatures -> sessions (session_id));
diesel::joinable!(fog_of_war -> maps (map_id));
diesel::joinable!(initiative -> characters (character_id));
diesel::joinable!(initiative -> sessions (session_id));
diesel::joinable!(initiative -> tokens (token_id));
diesel::joinable!(inventory_items -> characters (owner_character_id));
diesel::joinable!(inventory_items -> sessions (session_id));
diesel::joinable!(maps -> sessions (session_id));
diesel::joinable!(media -> users (uploaded_by));
diesel::joinable!(media_tags -> media (media_id));
diesel::joinable!(session_players -> sessions (session_id));
diesel::joinable!(session_players -> users (user_id));
diesel::joinable!(sessions -> rpg_templates (template_id));
diesel::joinable!(sessions -> users (gm_user_id));
diesel::joinable!(token_instances -> characters (character_id));
diesel::joinable!(token_instances -> creatures (creature_id));
diesel::joinable!(token_instances -> tokens (token_id));
diesel::joinable!(tokens -> characters (character_id));
diesel::joinable!(tokens -> creatures (creature_id));
diesel::joinable!(tokens -> maps (map_id));
diesel::joinable!(vfs_files -> sessions (session_id));

diesel::allow_tables_to_appear_in_same_query!(
    character_resources,
    characters,
    chat_messages,
    creatures,
    fog_of_war,
    initiative,
    inventory_items,
    maps,
    media,
    media_tags,
    rpg_templates,
    session_players,
    sessions,
    token_instances,
    tokens,
    users,
    vfs_files,
);
