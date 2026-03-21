use crate::models::TokenInfo;

/// Resolve the facing-arrow color for a token.
///
/// - Player character → that player's ping_color
/// - NPC character (owned by GM) → the token's own color
/// - Creature → GM's ping_color
/// - Generic token → map default_token_color
pub fn resolve_facing_color(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
    character_id: Option<i32>,
    creature_id: Option<i32>,
    token_color: &str,
    map_default_color: &str,
) -> String {
    use crate::schema::{characters, sessions, users};
    use diesel::prelude::*;

    let gm_user_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .unwrap_or(0);

    if let Some(cid) = character_id {
        // Look up the character's owning user
        if let Ok(char_user_id) = characters::table
            .find(cid)
            .select(characters::user_id)
            .first::<i32>(conn)
        {
            if char_user_id != gm_user_id {
                // Player-owned character → player's ping color
                return users::table
                    .find(char_user_id)
                    .select(users::ping_color)
                    .first::<String>(conn)
                    .unwrap_or_else(|_| token_color.to_string());
            } else {
                // NPC (GM-owned character) → token's own color
                return token_color.to_string();
            }
        }
    }

    if creature_id.is_some() {
        // Creature → GM's ping color
        return users::table
            .find(gm_user_id)
            .select(users::ping_color)
            .first::<String>(conn)
            .unwrap_or_else(|_| map_default_color.to_string());
    }

    // Generic token → map default color
    map_default_color.to_string()
}

pub fn persist_token_position(token_id: i32, x: f32, y: f32) {
    use crate::db;
    use crate::schema::tokens;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::update(tokens::table.find(token_id))
        .set((tokens::x.eq(x), tokens::y.eq(y)))
        .execute(conn);
}

pub fn place_token(
    session_id: i32,
    label: &str,
    x: f32,
    y: f32,
    color: &str,
    size: i32,
    character_id: Option<i32>,
    creature_id: Option<i32>,
    image_url: Option<&str>,
) -> Result<TokenInfo, String> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    // Get active map for this session
    let (map_id, map_default_color): (i32, String) = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.desc())
        .select((maps::id, maps::default_token_color))
        .first(conn)
        .map_err(|_| "No active map for this session".to_string())?;

    // For creature tokens, generate a unique label ("Wolf", "Wolf 2", ...)
    let unique_label = if creature_id.is_some() {
        make_unique_creature_label(conn, map_id, label)
    } else {
        label.to_string()
    };

    // Find a non-overlapping position for the token
    let (final_x, final_y) = find_open_position(conn, map_id, x, y, size);

    let new_token = NewToken {
        map_id,
        label: &unique_label,
        x: final_x,
        y: final_y,
        color,
        size,
        visible: true,
        character_id,
        creature_id,
        image_url,
    };

    diesel::insert_into(tokens::table)
        .values(&new_token)
        .execute(conn)
        .map_err(|e| format!("Failed to place token: {e}"))?;

    let token_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| format!("Failed to get token id: {e}"))?;

    // Create a token instance for creature or character tokens
    let (current_hp, max_hp) = if let Some(cid) = creature_id {
        // Creature token: seed HP from stat block
        let creature: Creature = creatures::table
            .find(cid)
            .select(Creature::as_select())
            .first(conn)
            .map_err(|_| "Creature not found".to_string())?;

        let stat_data: serde_json::Value =
            serde_json::from_str(&creature.stat_data_json).unwrap_or_default();
        let hp = stat_data
            .get("hp_max")
            .and_then(|v| v.as_i64())
            .unwrap_or(10) as i32;

        let new_instance = NewTokenInstance {
            token_id,
            creature_id: Some(cid),
            character_id: None,
            current_hp: hp,
            max_hp: hp,
            conditions_json: "[]".to_string(),
        };

        diesel::insert_into(token_instances::table)
            .values(&new_instance)
            .execute(conn)
            .map_err(|e| format!("Failed to create token instance: {e}"))?;

        (Some(hp), Some(hp))
    } else if let Some(char_id) = character_id {
        // Character token: seed HP from character_resources named "HP"
        let hp_resource: Option<CharacterResource> = character_resources::table
            .filter(character_resources::character_id.eq(char_id))
            .filter(diesel::dsl::sql::<diesel::sql_types::Bool>(
                "LOWER(name) = 'hp'",
            ))
            .select(CharacterResource::as_select())
            .first(conn)
            .optional()
            .unwrap_or(None);

        let (cur, max) = hp_resource
            .map(|r| (r.current_value, r.max_value))
            .unwrap_or((0, 0));

        let new_instance = NewTokenInstance {
            token_id,
            creature_id: None,
            character_id: Some(char_id),
            current_hp: cur,
            max_hp: max,
            conditions_json: "[]".to_string(),
        };

        diesel::insert_into(token_instances::table)
            .values(&new_instance)
            .execute(conn)
            .map_err(|e| format!("Failed to create token instance: {e}"))?;

        (Some(cur), Some(max))
    } else {
        (None, None)
    };

    let facing_color = Some(resolve_facing_color(
        conn,
        session_id,
        character_id,
        creature_id,
        color,
        &map_default_color,
    ));

    Ok(TokenInfo {
        id: token_id,
        label: unique_label,
        x: final_x,
        y: final_y,
        color: color.to_string(),
        size,
        visible: true,
        current_hp,
        max_hp,
        image_url: image_url.map(|s| s.to_string()),
        rotation: 0.0,
        conditions: vec![],
        character_id,
        creature_id,
        facing_color,
    })
}

pub fn remove_token(token_id: i32) {
    use crate::db;
    use crate::schema::{initiative, token_instances, tokens};
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    // Delete token instance, initiative entries, then the token itself
    if let Err(e) =
        diesel::delete(token_instances::table.filter(token_instances::token_id.eq(token_id)))
            .execute(conn)
    {
        log::warn!("Failed to delete token instance for token {token_id}: {e}");
    }
    if let Err(e) =
        diesel::delete(initiative::table.filter(initiative::token_id.eq(token_id))).execute(conn)
    {
        log::warn!("Failed to delete initiative entries for token {token_id}: {e}");
    }
    if let Err(e) = diesel::delete(tokens::table.find(token_id)).execute(conn) {
        log::warn!("Failed to delete token {token_id}: {e}");
    }
}

/// Generate a unique label for a creature token on the given map.
/// First instance keeps the base name ("Wolf"), subsequent ones get "Wolf 2", "Wolf 3", etc.
fn make_unique_creature_label(
    conn: &mut diesel::SqliteConnection,
    map_id: i32,
    base_name: &str,
) -> String {
    use crate::schema::tokens as tokens_table;
    use diesel::prelude::*;

    let existing_labels: Vec<String> = tokens_table::table
        .filter(tokens_table::map_id.eq(map_id))
        .select(tokens_table::label)
        .load(conn)
        .unwrap_or_default();

    // Check if base name is free
    if !existing_labels.iter().any(|l| l == base_name) {
        return base_name.to_string();
    }

    // Find the next available number
    for n in 2.. {
        let candidate = format!("{base_name} {n}");
        if !existing_labels.iter().any(|l| l == &candidate) {
            return candidate;
        }
    }
    unreachable!()
}

/// Find an open grid position for a token, avoiding overlap with existing tokens.
/// Starts at the requested position, then scans adjacent cells clockwise from 12:00,
/// expanding outward in concentric rings. Falls back to the original position if
/// no open spot is found within 3 rings.
fn find_open_position(
    conn: &mut diesel::SqliteConnection,
    map_id: i32,
    x: f32,
    y: f32,
    size: i32,
) -> (f32, f32) {
    use crate::schema::tokens as tokens_table;
    use diesel::prelude::*;

    let existing: Vec<(f32, f32, i32)> = tokens_table::table
        .filter(tokens_table::map_id.eq(map_id))
        .select((tokens_table::x, tokens_table::y, tokens_table::size))
        .load(conn)
        .unwrap_or_default();

    let overlaps = |px: f32, py: f32, ps: i32| -> bool {
        existing.iter().any(|&(ex, ey, es)| {
            // Two tokens overlap if their bounding boxes intersect
            let ps_f = ps as f32;
            let es_f = es as f32;
            px < ex + es_f && px + ps_f > ex && py < ey + es_f && py + ps_f > ey
        })
    };

    // Try the requested position first
    if !overlaps(x, y, size) {
        return (x, y);
    }

    let sf = size as f32;

    // Scan clockwise in concentric rings (ring 1 = adjacent, ring 2, ring 3)
    // Clockwise from 12:00: N, NE, E, SE, S, SW, W, NW
    for ring in 1..=3 {
        let r = ring as f32;
        // Generate positions around the ring, starting from 12:00 going clockwise
        // For ring r, we check all positions at distance r in each direction
        let offsets: Vec<(f32, f32)> = {
            let mut pts = Vec::new();
            let ri = ring as i32;
            // Top edge: from (−r+1, −r) to (r, −r) — starts at 12:00
            for dx in (-ri + 1)..=ri {
                pts.push((dx as f32 * sf, -r * sf));
            }
            // Right edge: from (r, −r+1) to (r, r)
            for dy in (-ri + 1)..=ri {
                pts.push((r * sf, dy as f32 * sf));
            }
            // Bottom edge: from (r−1, r) to (−r, r)
            for dx in ((-ri)..ri).rev() {
                pts.push((dx as f32 * sf, r * sf));
            }
            // Left edge: from (−r, r−1) to (−r, −r)
            for dy in ((-ri)..ri).rev() {
                pts.push((-r * sf, dy as f32 * sf));
            }
            pts
        };

        for (dx, dy) in offsets {
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 0.0 && ny >= 0.0 && !overlaps(nx, ny, size) {
                return (nx, ny);
            }
        }
    }

    // Give up — place at the original position
    (x, y)
}
