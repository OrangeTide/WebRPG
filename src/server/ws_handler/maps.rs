use crate::models::{MapInfo, TokenInfo};
use crate::ws::messages::GameStateSnapshot;
use crate::ws::session::SESSION_MANAGER;

use super::inventory::load_inventory;
use super::tokens::resolve_facing_color;

pub fn load_map_with_tokens(
    map_id: i32,
) -> Result<(MapInfo, Vec<TokenInfo>, Vec<(i32, i32)>), String> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let m: Map = maps::table
        .find(map_id)
        .select(Map::as_select())
        .first(conn)
        .map_err(|_| "Map not found".to_string())?;

    let session_id = m.session_id;
    let default_token_color = m.default_token_color.clone();
    let map_info = MapInfo {
        id: m.id,
        name: m.name,
        width: m.width,
        height: m.height,
        cell_size: m.cell_size,
        background_url: m.background_url,
        default_token_color: m.default_token_color,
    };

    let db_tokens: Vec<Token> = tokens::table
        .filter(tokens::map_id.eq(map_id))
        .select(Token::as_select())
        .load(conn)
        .unwrap_or_default();

    let token_list: Vec<TokenInfo> = db_tokens
        .into_iter()
        .map(|t| {
            let instance: Option<TokenInstance> = token_instances::table
                .filter(token_instances::token_id.eq(t.id))
                .select(TokenInstance::as_select())
                .first(conn)
                .optional()
                .unwrap_or(None);

            let conditions: Vec<String> = instance
                .as_ref()
                .and_then(|i| serde_json::from_str(&i.conditions_json).ok())
                .unwrap_or_default();

            let facing_color = Some(resolve_facing_color(
                conn,
                session_id,
                t.character_id,
                t.creature_id,
                &t.color,
                &default_token_color,
            ));

            TokenInfo {
                id: t.id,
                label: t.label,
                x: t.x,
                y: t.y,
                color: t.color,
                size: t.size,
                visible: t.visible,
                current_hp: instance.as_ref().map(|i| i.current_hp),
                max_hp: instance.as_ref().map(|i| i.max_hp),
                image_url: t.image_url,
                rotation: t.rotation,
                conditions,
                character_id: t.character_id,
                creature_id: t.creature_id,
                facing_color,
            }
        })
        .collect();

    let fog_cells: Vec<(i32, i32)> = fog_of_war::table
        .filter(fog_of_war::map_id.eq(map_id))
        .select((fog_of_war::x, fog_of_war::y))
        .load(conn)
        .unwrap_or_default();

    Ok((map_info, token_list, fog_cells))
}

pub fn build_snapshot(session_id: i32, user_id: i32) -> GameStateSnapshot {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let session_name = sessions::table
        .find(session_id)
        .select(sessions::name)
        .first::<String>(conn)
        .unwrap_or_default();

    let players: Vec<String> = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.clients.iter().map(|e| e.key().clone()).collect())
        .unwrap_or_default();

    // Load active map (most recent map for this session)
    let map_row: Option<Map> = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.desc())
        .select(Map::as_select())
        .first(conn)
        .optional()
        .unwrap_or(None);

    let (map_info, token_list, fog_cells) = if let Some(m) = map_row {
        let map_id = m.id;

        // Ensure active_map_id is set so subsequent operations (e.g. SetMapBackground) work
        if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
            session.active_map_id = Some(map_id);
        }

        let default_token_color = m.default_token_color.clone();
        let map_info = MapInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            cell_size: m.cell_size,
            background_url: m.background_url,
            default_token_color: m.default_token_color,
        };

        let db_tokens: Vec<Token> = tokens::table
            .filter(tokens::map_id.eq(map_id))
            .select(Token::as_select())
            .load(conn)
            .unwrap_or_default();

        let token_list: Vec<TokenInfo> = db_tokens
            .into_iter()
            .map(|t| {
                let instance: Option<TokenInstance> = token_instances::table
                    .filter(token_instances::token_id.eq(t.id))
                    .select(TokenInstance::as_select())
                    .first(conn)
                    .optional()
                    .unwrap_or(None);

                let conditions: Vec<String> = instance
                    .as_ref()
                    .and_then(|i| serde_json::from_str(&i.conditions_json).ok())
                    .unwrap_or_default();

                let facing_color = Some(resolve_facing_color(
                    conn,
                    session_id,
                    t.character_id,
                    t.creature_id,
                    &t.color,
                    &default_token_color,
                ));

                TokenInfo {
                    id: t.id,
                    label: t.label,
                    x: t.x,
                    y: t.y,
                    color: t.color,
                    size: t.size,
                    visible: t.visible,
                    current_hp: instance.as_ref().map(|i| i.current_hp),
                    max_hp: instance.as_ref().map(|i| i.max_hp),
                    image_url: t.image_url,
                    rotation: t.rotation,
                    conditions,
                    character_id: t.character_id,
                    creature_id: t.creature_id,
                    facing_color,
                }
            })
            .collect();

        let fog_cells: Vec<(i32, i32)> = fog_of_war::table
            .filter(fog_of_war::map_id.eq(map_id))
            .select((fog_of_war::x, fog_of_war::y))
            .load(conn)
            .unwrap_or_default();

        (Some(map_info), token_list, fog_cells)
    } else {
        (None, vec![], vec![])
    };

    // Load initiative
    let init_entries: Vec<InitiativeEntry> = initiative::table
        .filter(initiative::session_id.eq(session_id))
        .order(initiative::sort_order.asc())
        .select(InitiativeEntry::as_select())
        .load(conn)
        .unwrap_or_default();

    let initiative_list: Vec<crate::models::InitiativeEntryInfo> = init_entries
        .into_iter()
        .map(|e| crate::models::InitiativeEntryInfo {
            id: e.id,
            label: e.label,
            initiative_value: e.initiative_value,
            is_current_turn: e.is_current_turn,
            portrait_url: None,
            token_id: e.token_id,
            character_id: e.character_id,
        })
        .collect();

    // Load recent chat (last 100 messages)
    let chat_rows: Vec<(ChatMessage, String)> = chat_messages::table
        .inner_join(users::table.on(users::id.eq(chat_messages::user_id)))
        .filter(chat_messages::session_id.eq(session_id))
        .order(chat_messages::id.desc())
        .limit(100)
        .select((ChatMessage::as_select(), users::username))
        .load(conn)
        .unwrap_or_default();

    let recent_chat: Vec<crate::models::ChatMessageInfo> = chat_rows
        .into_iter()
        .rev()
        .map(|(msg, uname)| crate::models::ChatMessageInfo {
            id: msg.id,
            username: uname,
            message: msg.message,
            is_dice_roll: msg.is_dice_roll,
            dice_result: msg.dice_result,
            created_at: msg.created_at,
        })
        .collect();

    // Load inventory
    let inventory = load_inventory(session_id);

    let initiative_locked = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_locked)
        .unwrap_or(false);

    GameStateSnapshot {
        session_id,
        session_name,
        players,
        map: map_info,
        tokens: token_list,
        fog: fog_cells,
        initiative: initiative_list,
        recent_chat,
        inventory,
        initiative_locked,
        is_gm: super::is_gm(session_id, user_id),
        ping_color: {
            users::table
                .find(user_id)
                .select(users::ping_color)
                .first::<String>(conn)
                .unwrap_or_else(|_| "#ffcc00".to_string())
        },
        suppress_tooltips: {
            users::table
                .find(user_id)
                .select(users::suppress_tooltips)
                .first::<i32>(conn)
                .unwrap_or(0)
                != 0
        },
    }
}
