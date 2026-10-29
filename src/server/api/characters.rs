use leptos::prelude::*;

use crate::models::{CharacterInfo, CreatureInfo, TemplateInfo};

#[cfg(feature = "ssr")]
use super::get_current_user;

#[server]
pub async fn list_templates() -> Result<Vec<TemplateInfo>, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::RpgTemplate;
    use crate::schema::rpg_templates;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let templates: Vec<RpgTemplate> = rpg_templates::table
        .select(RpgTemplate::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(templates
        .into_iter()
        .map(|t| {
            let fields: Vec<TemplateField> =
                serde_json::from_str(&t.schema_json).unwrap_or_default();
            TemplateInfo {
                id: t.id,
                name: t.name,
                description: t.description,
                fields,
            }
        })
        .collect())
}

#[server]
pub async fn get_template(template_id: i32) -> Result<TemplateInfo, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::RpgTemplate;
    use crate::schema::rpg_templates;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let t: RpgTemplate = rpg_templates::table
        .find(template_id)
        .select(RpgTemplate::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Template not found"))?;

    let fields: Vec<TemplateField> = serde_json::from_str(&t.schema_json).unwrap_or_default();

    Ok(TemplateInfo {
        id: t.id,
        name: t.name,
        description: t.description,
        fields,
    })
}

#[server]
pub async fn seed_default_template() -> Result<TemplateInfo, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::{NewRpgTemplate, RpgTemplate};
    use crate::schema::rpg_templates;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    // Check if default already exists
    let existing = rpg_templates::table
        .filter(rpg_templates::name.eq("D&D 5e"))
        .select(RpgTemplate::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if let Some(t) = existing {
        let fields: Vec<TemplateField> = serde_json::from_str(&t.schema_json).unwrap_or_default();
        return Ok(TemplateInfo {
            id: t.id,
            name: t.name,
            description: t.description,
            fields,
        });
    }

    let schema_json = serde_json::to_string(&default_5e_fields())
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    diesel::insert_into(rpg_templates::table)
        .values(&NewRpgTemplate {
            name: "D&D 5e",
            description: "Dungeons & Dragons 5th Edition",
            schema_json: &schema_json,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create template: {e}")))?;

    let t: RpgTemplate = rpg_templates::table
        .filter(rpg_templates::name.eq("D&D 5e"))
        .select(RpgTemplate::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let fields: Vec<TemplateField> = serde_json::from_str(&t.schema_json).unwrap_or_default();

    Ok(TemplateInfo {
        id: t.id,
        name: t.name,
        description: t.description,
        fields,
    })
}

#[cfg(feature = "ssr")]
fn default_5e_fields() -> Vec<crate::models::TemplateField> {
    use crate::models::{FieldType, TemplateField};

    vec![
        // Ability scores
        TemplateField {
            name: "strength".into(),
            label: "Strength".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "dexterity".into(),
            label: "Dexterity".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "constitution".into(),
            label: "Constitution".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "intelligence".into(),
            label: "Intelligence".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "wisdom".into(),
            label: "Wisdom".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "charisma".into(),
            label: "Charisma".into(),
            field_type: FieldType::Number,
            category: "Ability Scores".into(),
            default: serde_json::json!(10),
        },
        // Core stats
        TemplateField {
            name: "hp_max".into(),
            label: "Max HP".into(),
            field_type: FieldType::Number,
            category: "Combat".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "armor_class".into(),
            label: "Armor Class".into(),
            field_type: FieldType::Number,
            category: "Combat".into(),
            default: serde_json::json!(10),
        },
        TemplateField {
            name: "speed".into(),
            label: "Speed".into(),
            field_type: FieldType::Number,
            category: "Combat".into(),
            default: serde_json::json!(30),
        },
        TemplateField {
            name: "initiative_bonus".into(),
            label: "Initiative Bonus".into(),
            field_type: FieldType::Number,
            category: "Combat".into(),
            default: serde_json::json!(0),
        },
        TemplateField {
            name: "proficiency_bonus".into(),
            label: "Proficiency Bonus".into(),
            field_type: FieldType::Number,
            category: "Combat".into(),
            default: serde_json::json!(2),
        },
        // Character info
        TemplateField {
            name: "race".into(),
            label: "Race".into(),
            field_type: FieldType::Text,
            category: "Info".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "class".into(),
            label: "Class".into(),
            field_type: FieldType::Text,
            category: "Info".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "level".into(),
            label: "Level".into(),
            field_type: FieldType::Number,
            category: "Info".into(),
            default: serde_json::json!(1),
        },
        TemplateField {
            name: "background".into(),
            label: "Background".into(),
            field_type: FieldType::Text,
            category: "Info".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "alignment".into(),
            label: "Alignment".into(),
            field_type: FieldType::Text,
            category: "Info".into(),
            default: serde_json::json!(""),
        },
        // Skills and notes
        TemplateField {
            name: "skills".into(),
            label: "Skills & Proficiencies".into(),
            field_type: FieldType::Textarea,
            category: "Skills".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "features".into(),
            label: "Features & Traits".into(),
            field_type: FieldType::Textarea,
            category: "Skills".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "equipment".into(),
            label: "Equipment".into(),
            field_type: FieldType::Textarea,
            category: "Equipment".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "spells".into(),
            label: "Spells".into(),
            field_type: FieldType::Textarea,
            category: "Spells".into(),
            default: serde_json::json!(""),
        },
        TemplateField {
            name: "notes".into(),
            label: "Notes".into(),
            field_type: FieldType::Textarea,
            category: "Notes".into(),
            default: serde_json::json!(""),
        },
    ]
}

// ===== Character functions =====

#[server]
pub async fn create_character(
    session_id: i32,
    name: String,
) -> Result<CharacterInfo, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    // Check user is a member of the session
    let is_member = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user.id))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if !is_member {
        return Err(ServerFnError::new("Not a member of this session"));
    }

    // Get session template to build default data
    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let default_data = if let Some(tid) = session.template_id {
        let template: RpgTemplate = rpg_templates::table
            .find(tid)
            .select(RpgTemplate::as_select())
            .first(conn)
            .map_err(|_| ServerFnError::new("Template not found"))?;

        let fields: Vec<TemplateField> =
            serde_json::from_str(&template.schema_json).unwrap_or_default();

        let mut data = serde_json::Map::new();
        for field in &fields {
            data.insert(field.name.clone(), field.default.clone());
        }
        serde_json::Value::Object(data)
    } else {
        serde_json::json!({})
    };

    let data_str = serde_json::to_string(&default_data)
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    diesel::insert_into(characters::table)
        .values(&NewCharacter {
            session_id,
            user_id: user.id,
            name: &name,
            data_json: &data_str,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create character: {e}")))?;

    let char_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    // Create default resources (HP)
    if let Some(hp_max) = default_data.get("hp_max").and_then(|v| v.as_i64()) {
        diesel::insert_into(character_resources::table)
            .values(&NewCharacterResource {
                character_id: char_id,
                name: "HP",
                current_value: hp_max as i32,
                max_value: hp_max as i32,
            })
            .execute(conn)
            .map_err(|e| ServerFnError::new(format!("Failed to create resource: {e}")))?;
    }

    Ok(CharacterInfo {
        id: char_id,
        session_id,
        user_id: user.id,
        name,
        data: default_data,
        resources: vec![],
        portrait_url: None,
    })
}

#[server]
pub async fn list_characters(session_id: i32) -> Result<Vec<CharacterInfo>, ServerFnError> {
    use crate::db;
    use crate::models::ResourceInfo;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let chars: Vec<Character> = characters::table
        .filter(characters::session_id.eq(session_id))
        .select(Character::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let mut result = Vec::new();
    for c in chars {
        let resources: Vec<CharacterResource> = character_resources::table
            .filter(character_resources::character_id.eq(c.id))
            .select(CharacterResource::as_select())
            .load(conn)
            .unwrap_or_default();

        result.push(CharacterInfo {
            id: c.id,
            session_id: c.session_id,
            user_id: c.user_id,
            name: c.name,
            data: serde_json::from_str(&c.data_json).unwrap_or(serde_json::json!({})),
            resources: resources
                .into_iter()
                .map(|r| ResourceInfo {
                    id: r.id,
                    name: r.name,
                    current_value: r.current_value,
                    max_value: r.max_value,
                })
                .collect(),
            portrait_url: c.portrait_url,
        });
    }

    Ok(result)
}

/// Ensure a character has default resources (HP) and template defaults in data_json.
/// Called when opening a character sheet to backfill characters created before
/// template assignment. Returns true if any changes were made.
#[server]
pub async fn ensure_character_defaults(character_id: i32) -> Result<bool, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let character: Character = characters::table
        .find(character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Character not found"))?;

    let session: Session = sessions::table
        .find(character.session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let Some(tid) = session.template_id else {
        return Ok(false);
    };

    let template: RpgTemplate = rpg_templates::table
        .find(tid)
        .select(RpgTemplate::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Template not found"))?;

    let fields: Vec<TemplateField> =
        serde_json::from_str(&template.schema_json).unwrap_or_default();

    let mut changed = false;

    // Backfill empty data_json with template defaults
    let mut data: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&character.data_json).unwrap_or_default();
    if data.is_empty() && !fields.is_empty() {
        for field in &fields {
            data.insert(field.name.clone(), field.default.clone());
        }
        let new_json = serde_json::to_string(&serde_json::Value::Object(data.clone()))
            .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;
        diesel::update(characters::table.find(character_id))
            .set(characters::data_json.eq(&new_json))
            .execute(conn)
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
        changed = true;
    }

    // Create HP resource if none exists
    let resource_count: i64 = character_resources::table
        .filter(character_resources::character_id.eq(character_id))
        .count()
        .get_result(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if resource_count == 0 {
        let hp_max = data.get("hp_max").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
        diesel::insert_into(character_resources::table)
            .values(&NewCharacterResource {
                character_id,
                name: "HP",
                current_value: hp_max,
                max_value: hp_max,
            })
            .execute(conn)
            .map_err(|e| ServerFnError::new(format!("Failed to create resource: {e}")))?;
        changed = true;
    }

    Ok(changed)
}

#[server]
pub async fn update_character_resource(
    resource_id: i32,
    current_value: i32,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let resource: CharacterResource = character_resources::table
        .find(resource_id)
        .select(CharacterResource::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Resource not found"))?;

    let character: Character = characters::table
        .find(resource.character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Character not found"))?;

    // Any session member can adjust resources (GM applies damage, etc.)
    let is_member = session_players::table
        .filter(session_players::session_id.eq(character.session_id))
        .filter(session_players::user_id.eq(_user.id))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;
    if !is_member {
        return Err(ServerFnError::new("Not a member of this session"));
    }

    diesel::update(character_resources::table.find(resource_id))
        .set(character_resources::current_value.eq(current_value))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to update resource: {e}")))?;

    // Broadcast to all clients in the session
    use crate::ws::messages::ServerMessage;
    use crate::ws::session::SESSION_MANAGER;
    SESSION_MANAGER.broadcast(
        character.session_id,
        &ServerMessage::CharacterResourceUpdated {
            character_id: resource.character_id,
            resource_id,
            current_value,
            max_value: resource.max_value,
        },
        None,
    );

    // When a character's HP changes, sync its tokens' HP and auto-manage the
    // "bloodied" condition. This applies only to player-character tokens;
    // creature bloodied status is left to the GM.
    if resource.name.eq_ignore_ascii_case("hp") {
        sync_character_token_hp(
            resource.character_id,
            character.session_id,
            current_value,
            resource.max_value,
        );
    }

    Ok(())
}

/// A token is bloodied when its HP is strictly below half of its maximum (so a
/// character sitting at exactly half is not bloodied). Max HP of zero or less is
/// never bloodied.
#[cfg(feature = "ssr")]
fn is_bloodied(current_hp: i32, max_hp: i32) -> bool {
    max_hp > 0 && current_hp * 2 < max_hp
}

/// Given the current conditions and HP, return the updated conditions when the
/// bloodied state should change, or `None` when nothing changes.
#[cfg(feature = "ssr")]
fn bloodied_conditions(conditions: &[String], current_hp: i32, max_hp: i32) -> Option<Vec<String>> {
    let bloodied = is_bloodied(current_hp, max_hp);
    let had = conditions.iter().any(|c| c == "bloodied");
    match (bloodied, had) {
        (true, false) => {
            let mut next = conditions.to_vec();
            next.push("bloodied".to_string());
            Some(next)
        }
        (false, true) => Some(conditions.iter().filter(|c| *c != "bloodied").cloned().collect()),
        _ => None,
    }
}

#[cfg(all(test, feature = "ssr"))]
mod bloodied_tests {
    use super::{bloodied_conditions, is_bloodied};

    #[test]
    fn threshold_is_strictly_below_half() {
        assert!(!is_bloodied(10, 10));
        assert!(!is_bloodied(5, 10)); // exactly half is not bloodied
        assert!(is_bloodied(4, 10));
        assert!(is_bloodied(0, 10));
        assert!(!is_bloodied(3, 0)); // no max -> never bloodied
        assert!(!is_bloodied(0, 0));
        assert!(!is_bloodied(2, 3)); // ceil(3/2)=2 is not below half
        assert!(is_bloodied(1, 3));
    }

    #[test]
    fn adds_and_removes_only_on_change() {
        // Not bloodied, none present -> no change.
        assert_eq!(bloodied_conditions(&[], 10, 10), None);
        // Drops below half -> add.
        assert_eq!(
            bloodied_conditions(&[], 4, 10),
            Some(vec!["bloodied".to_string()])
        );
        // Already bloodied and still hurt -> no change.
        assert_eq!(bloodied_conditions(&["bloodied".to_string()], 4, 10), None);
        // Healed to half -> remove, keeping other conditions.
        assert_eq!(
            bloodied_conditions(&["poisoned".to_string(), "bloodied".to_string()], 5, 10),
            Some(vec!["poisoned".to_string()])
        );
    }
}

/// Sync a character's tokens with its current HP and auto-manage the "bloodied"
/// condition. A token is bloodied when its HP is strictly below half of max.
/// Broadcasts `TokenHpUpdated` for each token, plus `TokenConditionsUpdated`
/// when the bloodied state changed. Only character tokens are touched; creatures
/// are never auto-bloodied.
#[cfg(feature = "ssr")]
fn sync_character_token_hp(character_id: i32, session_id: i32, current_hp: i32, max_hp: i32) {
    use crate::db;
    use crate::models::db_models::TokenInstance;
    use crate::schema::{token_instances, tokens};
    use crate::ws::messages::ServerMessage;
    use crate::ws::session::SESSION_MANAGER;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::character_id.eq(character_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();

    for token_id in token_ids {
        let _ = diesel::update(token_instances::table.filter(token_instances::token_id.eq(token_id)))
            .set((
                token_instances::current_hp.eq(current_hp),
                token_instances::max_hp.eq(max_hp),
            ))
            .execute(conn);

        let instance: Option<TokenInstance> = token_instances::table
            .filter(token_instances::token_id.eq(token_id))
            .select(TokenInstance::as_select())
            .first(conn)
            .optional()
            .ok()
            .flatten();
        let conditions: Vec<String> = instance
            .and_then(|i| serde_json::from_str(&i.conditions_json).ok())
            .unwrap_or_default();

        let updated = bloodied_conditions(&conditions, current_hp, max_hp);
        if let Some(ref new_conditions) = updated {
            let json = serde_json::to_string(new_conditions).unwrap_or_else(|_| "[]".to_string());
            let _ = diesel::update(
                token_instances::table.filter(token_instances::token_id.eq(token_id)),
            )
            .set(token_instances::conditions_json.eq(&json))
            .execute(conn);
        }

        SESSION_MANAGER.broadcast(
            session_id,
            &ServerMessage::TokenHpUpdated {
                token_id,
                current_hp,
                max_hp,
            },
            None,
        );
        if let Some(new_conditions) = updated {
            SESSION_MANAGER.broadcast(
                session_id,
                &ServerMessage::TokenConditionsUpdated {
                    token_id,
                    conditions: new_conditions,
                },
                None,
            );
        }
    }
}

#[server]
pub async fn delete_character(character_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let character: Character = characters::table
        .find(character_id)
        .select(Character::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Character not found"))?;

    if character.user_id != user.id {
        // Also allow session GM to delete
        let session: Session = sessions::table
            .find(character.session_id)
            .select(Session::as_select())
            .first(conn)
            .map_err(|_| ServerFnError::new("Session not found"))?;

        if !crate::server::gm::is_session_gm(conn, session.id, user.id) {
            return Err(ServerFnError::new("Not your character"));
        }
    }

    // Delete cascade: token_instances, tokens, initiative entries, resources
    // First get all token IDs for this character
    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::character_id.eq(character_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();

    // Delete token instances for these tokens
    if !token_ids.is_empty() {
        diesel::delete(token_instances::table.filter(token_instances::token_id.eq_any(&token_ids)))
            .execute(conn)
            .ok();
    }

    // Delete initiative entries referencing this character
    diesel::delete(initiative::table.filter(initiative::character_id.eq(character_id)))
        .execute(conn)
        .ok();

    // Delete tokens
    diesel::delete(tokens::table.filter(tokens::character_id.eq(character_id)))
        .execute(conn)
        .ok();

    // Delete associated resources
    diesel::delete(
        character_resources::table.filter(character_resources::character_id.eq(character_id)),
    )
    .execute(conn)
    .map_err(|e| ServerFnError::new(format!("Failed to delete resources: {e}")))?;

    diesel::delete(characters::table.find(character_id))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete character: {e}")))?;

    Ok(())
}

// ===== Creature functions =====

#[server]
pub async fn list_creatures(session_id: i32) -> Result<Vec<CreatureInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::Creature;
    use crate::schema::creatures;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let rows: Vec<Creature> = creatures::table
        .filter(creatures::session_id.eq(session_id))
        .select(Creature::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|c| CreatureInfo {
            id: c.id,
            name: c.name,
            stat_data: serde_json::from_str(&c.stat_data_json).unwrap_or_default(),
            image_url: c.image_url,
        })
        .collect())
}

#[server]
pub async fn create_creature(
    session_id: i32,
    name: String,
    stat_data: serde_json::Value,
) -> Result<CreatureInfo, ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if !crate::server::gm::is_session_gm(conn, session.id, user.id) {
        return Err(ServerFnError::new("Only the GM can create creatures"));
    }

    let stat_json = serde_json::to_string(&stat_data)
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    diesel::insert_into(creatures::table)
        .values(&NewCreature {
            session_id,
            template_id: session.template_id,
            name: &name,
            stat_data_json: &stat_json,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create creature: {e}")))?;

    let id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(CreatureInfo {
        id,
        name,
        stat_data,
        image_url: None,
    })
}

#[server]
pub async fn update_creature(
    creature_id: i32,
    name: String,
    stat_data: serde_json::Value,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let creature: Creature = creatures::table
        .find(creature_id)
        .select(Creature::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Creature not found"))?;

    let session: Session = sessions::table
        .find(creature.session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if !crate::server::gm::is_session_gm(conn, session.id, user.id) {
        return Err(ServerFnError::new("Only the GM can edit creatures"));
    }

    let stat_json = serde_json::to_string(&stat_data)
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    diesel::update(creatures::table.find(creature_id))
        .set((
            creatures::name.eq(&name),
            creatures::stat_data_json.eq(&stat_json),
        ))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to update creature: {e}")))?;

    Ok(())
}

#[server]
pub async fn delete_creature(creature_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let creature: Creature = creatures::table
        .find(creature_id)
        .select(Creature::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Creature not found"))?;

    let session: Session = sessions::table
        .find(creature.session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if !crate::server::gm::is_session_gm(conn, session.id, user.id) {
        return Err(ServerFnError::new("Only the GM can delete creatures"));
    }

    // Delete cascade: token_instances, tokens for this creature
    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::creature_id.eq(creature_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();

    if !token_ids.is_empty() {
        diesel::delete(token_instances::table.filter(token_instances::token_id.eq_any(&token_ids)))
            .execute(conn)
            .ok();
    }

    // Delete initiative entries referencing these tokens
    if !token_ids.is_empty() {
        diesel::delete(initiative::table.filter(initiative::token_id.eq_any(&token_ids)))
            .execute(conn)
            .ok();
    }

    // Delete tokens
    diesel::delete(tokens::table.filter(tokens::creature_id.eq(creature_id)))
        .execute(conn)
        .ok();

    diesel::delete(creatures::table.find(creature_id))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete creature: {e}")))?;

    Ok(())
}

#[server]
pub async fn update_creature_image(
    creature_id: i32,
    image_url: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let creature: Creature = creatures::table
        .find(creature_id)
        .select(Creature::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Creature not found"))?;

    let session: Session = sessions::table
        .find(creature.session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if !crate::server::gm::is_session_gm(conn, session.id, user.id) {
        return Err(ServerFnError::new("Only the GM can edit creatures"));
    }

    diesel::update(creatures::table.find(creature_id))
        .set(creatures::image_url.eq(&image_url))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to update creature image: {e}")))?;

    // Update image on any tokens linked to this creature
    use crate::schema::tokens;
    let updated_token_ids: Vec<i32> = tokens::table
        .filter(tokens::creature_id.eq(creature_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();
    if !updated_token_ids.is_empty() {
        diesel::update(tokens::table.filter(tokens::creature_id.eq(creature_id)))
            .set(tokens::image_url.eq(&image_url))
            .execute(conn)
            .ok();
    }

    // Broadcast token image updates
    use crate::ws::messages::ServerMessage;
    use crate::ws::session::SESSION_MANAGER;
    for tid in updated_token_ids {
        SESSION_MANAGER.broadcast(
            creature.session_id,
            &ServerMessage::TokenImageUpdated {
                token_id: tid,
                image_url: image_url.clone(),
            },
            None,
        );
    }

    Ok(())
}

/// Get the template for a session (if one is assigned).
#[server]
pub async fn get_session_template(session_id: i32) -> Result<Option<TemplateInfo>, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let tid = match session.template_id {
        Some(tid) => tid,
        None => {
            // Fall back to default template; seed it if it doesn't exist yet,
            // and assign it to this session.
            let tmpl = seed_default_template().await?;
            diesel::update(sessions::table.find(session_id))
                .set(sessions::template_id.eq(Some(tmpl.id)))
                .execute(conn)
                .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
            return Ok(Some(tmpl));
        }
    };

    let t: RpgTemplate = rpg_templates::table
        .find(tid)
        .select(RpgTemplate::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Template not found"))?;

    let fields: Vec<TemplateField> = serde_json::from_str(&t.schema_json).unwrap_or_default();

    Ok(Some(TemplateInfo {
        id: t.id,
        name: t.name,
        description: t.description,
        fields,
    }))
}

// ===== Media functions =====
