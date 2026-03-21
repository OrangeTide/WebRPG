use crate::models::InitiativeEntryInfo;
use crate::ws::messages::ServerMessage;
use crate::ws::session::SESSION_MANAGER;

use super::chat::save_chat_message;

/// Initiative roll result with breakdown for chat logging.
pub struct InitiativeRollResult {
    pub label: String,
    pub d20: i32,
    pub modifier: i32,
    pub total: i32,
    pub entries: Vec<InitiativeEntryInfo>,
}

pub fn save_initiative(session_id: i32, entries: &[InitiativeEntryInfo]) {
    use crate::db;
    use crate::models::db_models::NewInitiativeEntry;
    use crate::schema::initiative;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    // Replace all initiative entries for this session
    let _ = diesel::delete(initiative::table.filter(initiative::session_id.eq(session_id)))
        .execute(conn);

    for (i, entry) in entries.iter().enumerate() {
        let new_entry = NewInitiativeEntry {
            session_id,
            label: &entry.label,
            initiative_value: entry.initiative_value,
            is_current_turn: entry.is_current_turn,
            sort_order: i as i32,
            token_id: entry.token_id,
            character_id: entry.character_id,
        };
        let _ = diesel::insert_into(initiative::table)
            .values(&new_entry)
            .execute(conn);
    }
}

/// Save initiative, update cache, broadcast result + chat message.
pub fn broadcast_initiative_roll(
    session_id: i32,
    user_id: i32,
    username: &str,
    result: InitiativeRollResult,
) {
    save_initiative(session_id, &result.entries);
    if let Some(mut session) = SESSION_MANAGER.sessions.get_mut(&session_id) {
        session.initiative_order = result.entries.clone();
    }
    SESSION_MANAGER.broadcast(
        session_id,
        &ServerMessage::InitiativeUpdated {
            entries: result.entries,
        },
        None,
    );
    let mod_str = if result.modifier >= 0 {
        format!("+{}", result.modifier)
    } else {
        format!("{}", result.modifier)
    };
    let _ = save_chat_message(
        session_id,
        user_id,
        username,
        &format!(
            "{} rolled initiative: [{}]{} = {}",
            result.label, result.d20, mod_str, result.total
        ),
        true,
        Some(&format!(
            "{{\"rolls\":[{}],\"total\":{}}}",
            result.d20, result.total
        )),
    );
    SESSION_MANAGER.broadcast(
        session_id,
        &ServerMessage::DiceResult {
            username: username.to_string(),
            expression: format!("d20{mod_str} (initiative)"),
            rolls: vec![result.d20],
            total: result.total,
        },
        None,
    );
}

/// D&D 5e ability modifier: floor((score - 10) / 2)
fn ability_modifier(score: f64) -> i32 {
    ((score - 10.0) / 2.0).floor() as i32
}

/// Get the total initiative modifier for a character (dex mod + initiative_bonus).
fn get_initiative_modifier(session_id: i32, character_id: i32) -> i32 {
    use crate::db;
    use crate::schema::characters;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let data_json: String = characters::table
        .find(character_id)
        .filter(characters::session_id.eq(session_id))
        .select(characters::data_json)
        .first(conn)
        .unwrap_or_default();

    let data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&data_json).unwrap_or_default();

    let dex = data
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    let init_bonus = data
        .get("initiative_bonus")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i32;
    ability_modifier(dex) + init_bonus
}

/// Get the total initiative modifier for a creature (dex mod + initiative_bonus from stat_data).
fn get_creature_initiative_modifier(_session_id: i32, creature_id: i32) -> i32 {
    use crate::db;
    use crate::schema::creatures;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let stat_json: String = creatures::table
        .find(creature_id)
        .select(creatures::stat_data_json)
        .first(conn)
        .unwrap_or_default();

    let data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&stat_json).unwrap_or_default();

    let dex = data
        .get("dexterity")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    let init_bonus = data
        .get("initiative_bonus")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as i32;
    ability_modifier(dex) + init_bonus
}

/// Roll initiative for a character: d20 + dex mod + initiative bonus.
pub fn roll_character_initiative(
    session_id: i32,
    character_id: i32,
) -> Result<InitiativeRollResult, String> {
    use crate::db;
    use crate::schema::characters;
    use diesel::prelude::*;
    use rand::Rng;

    let conn = &mut db::get_conn();

    let (char_name, portrait_url): (String, Option<String>) = characters::table
        .find(character_id)
        .filter(characters::session_id.eq(session_id))
        .select((characters::name, characters::portrait_url))
        .first(conn)
        .map_err(|_| "Character not found in this session".to_string())?;

    let modifier = get_initiative_modifier(session_id, character_id);
    let mut rng = rand::thread_rng();
    let d20: i32 = rng.gen_range(1..=20);
    let total = d20 + modifier;

    let mut entries = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_order.clone())
        .unwrap_or_default();

    // Look up token_id for this character on the active map
    let token_id: Option<i32> = {
        use crate::schema::{maps, tokens as tokens_table};
        let map_id: Option<i32> = maps::table
            .filter(maps::session_id.eq(session_id))
            .order(maps::id.desc())
            .select(maps::id)
            .first(conn)
            .optional()
            .unwrap_or(None);
        map_id.and_then(|mid| {
            tokens_table::table
                .filter(tokens_table::map_id.eq(mid))
                .filter(tokens_table::character_id.eq(character_id))
                .select(tokens_table::id)
                .first(conn)
                .optional()
                .unwrap_or(None)
        })
    };

    // Remove existing entry for this character (by character_id or label match)
    entries.retain(|e| e.character_id != Some(character_id) && e.label != char_name);

    let is_first = entries.is_empty();
    entries.push(InitiativeEntryInfo {
        id: 0,
        label: char_name.clone(),
        initiative_value: total as f32,
        is_current_turn: is_first,
        portrait_url,
        token_id,
        character_id: Some(character_id),
    });

    entries.sort_by(|a, b| b.initiative_value.partial_cmp(&a.initiative_value).unwrap());

    Ok(InitiativeRollResult {
        label: char_name,
        d20,
        modifier,
        total,
        entries,
    })
}

/// Roll initiative for a creature: d20 + dex mod + initiative bonus.
pub fn roll_creature_initiative(
    session_id: i32,
    creature_id: i32,
    label: &str,
) -> Result<InitiativeRollResult, String> {
    use crate::db;
    use crate::schema::creatures;
    use diesel::prelude::*;
    use rand::Rng;

    let conn = &mut db::get_conn();
    let image_url: Option<String> = creatures::table
        .find(creature_id)
        .select(creatures::image_url)
        .first::<Option<String>>(conn)
        .ok()
        .flatten();

    let modifier = get_creature_initiative_modifier(session_id, creature_id);
    let mut rng = rand::thread_rng();
    let d20: i32 = rng.gen_range(1..=20);
    let total = d20 + modifier;

    let mut entries = SESSION_MANAGER
        .sessions
        .get(&session_id)
        .map(|s| s.initiative_order.clone())
        .unwrap_or_default();

    // Generate unique initiative label: "Wolf", "Wolf 2", "Wolf 3", ...
    // based on existing initiative entries
    let unique_label = {
        let base = label;
        if !entries.iter().any(|e| e.label == base) {
            base.to_string()
        } else {
            let mut n = 2;
            loop {
                let candidate = format!("{base} {n}");
                if !entries.iter().any(|e| e.label == candidate) {
                    break candidate;
                }
                n += 1;
            }
        }
    };

    // Look up token_id by matching label on the active map
    let token_id: Option<i32> = {
        use crate::schema::{maps, tokens as tokens_table};
        let map_id: Option<i32> = maps::table
            .filter(maps::session_id.eq(session_id))
            .order(maps::id.desc())
            .select(maps::id)
            .first(conn)
            .optional()
            .unwrap_or(None);
        map_id.and_then(|mid| {
            tokens_table::table
                .filter(tokens_table::map_id.eq(mid))
                .filter(tokens_table::label.eq(&unique_label))
                .select(tokens_table::id)
                .first(conn)
                .optional()
                .unwrap_or(None)
        })
    };

    let is_first = entries.is_empty();
    entries.push(InitiativeEntryInfo {
        id: 0,
        label: unique_label.clone(),
        initiative_value: total as f32,
        is_current_turn: is_first,
        portrait_url: image_url,
        token_id,
        character_id: None,
    });

    entries.sort_by(|a, b| b.initiative_value.partial_cmp(&a.initiative_value).unwrap());

    Ok(InitiativeRollResult {
        label: unique_label,
        d20,
        modifier,
        total,
        entries,
    })
}
