pub fn update_character_field(
    character_id: i32,
    user_id: i32,
    field_path: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    use crate::db;
    use crate::models::db_models::Character;
    use crate::schema::characters;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let character: Character = characters::table
        .find(character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| "Character not found".to_string())?;

    // Players can only edit their own characters
    if character.user_id != user_id {
        return Err("You can only edit your own character".to_string());
    }

    let mut data: serde_json::Value =
        serde_json::from_str(&character.data_json).unwrap_or(serde_json::json!({}));

    // Set the field at the given path (supports dot-separated paths like "stats.strength")
    let parts: Vec<&str> = field_path.split('.').collect();
    let mut current = &mut data;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            current[part] = value.clone();
        } else {
            if !current.get(part).is_some_and(|v| v.is_object()) {
                current[part] = serde_json::json!({});
            }
            current = &mut current[part];
        }
    }

    let json_str = serde_json::to_string(&data).map_err(|e| format!("Serialization error: {e}"))?;

    diesel::update(characters::table.find(character_id))
        .set(characters::data_json.eq(json_str))
        .execute(conn)
        .map_err(|e| format!("Failed to update character: {e}"))?;

    Ok(())
}
