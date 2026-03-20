use leptos::prelude::*;

use crate::models::{
    CharacterInfo, CreatureInfo, MediaInfo, SessionInfo, TemplateInfo, UserInfo, VfsEntryInfo,
    VfsFileData,
};

/// Set a JWT authentication cookie on the current response.
#[cfg(feature = "ssr")]
fn set_jwt_cookie(token: &str) -> Result<(), ServerFnError> {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&format!(
            "token={token}; HttpOnly; Path=/; Max-Age=86400; SameSite=Strict"
        ))
        .map_err(|e| ServerFnError::new(e.to_string()))?,
    );
    Ok(())
}

/// Clear the JWT authentication cookie (logout).
#[cfg(feature = "ssr")]
fn clear_jwt_cookie() {
    let response = expect_context::<leptos_axum::ResponseOptions>();
    response.insert_header(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_static(
            "token=; HttpOnly; Path=/; Max-Age=0; SameSite=Strict",
        ),
    );
}

#[server]
pub async fn login(username: String, password: String) -> Result<UserInfo, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::User;
    use crate::schema::users;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let user = users::table
        .filter(users::username.eq(&username))
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        .ok_or_else(|| ServerFnError::new("Invalid username or password"))?;

    if user.locked {
        return Err(ServerFnError::new("Account is locked"));
    }

    let passcrypt = user
        .passcrypt
        .as_deref()
        .ok_or_else(|| ServerFnError::new("Account has no password set"))?;

    let valid = auth::verify_password(passcrypt, &password)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    if !valid {
        return Err(ServerFnError::new("Invalid username or password"));
    }

    let token = auth::generate_jwt(user.id, &user.username)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    set_jwt_cookie(&token)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
    })
}

#[server]
pub async fn signup(
    username: String,
    password: String,
    display_name: String,
    email: String,
) -> Result<UserInfo, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::{NewUser, User};
    use crate::schema::users;
    use diesel::prelude::*;

    if username.len() < 3 || username.len() > 50 {
        return Err(ServerFnError::new(
            "Username must be between 3 and 50 characters",
        ));
    }
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }

    let conn = &mut db::get_conn();

    // Check if username already exists
    let exists: bool = users::table
        .filter(users::username.eq(&username))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if exists {
        return Err(ServerFnError::new("Username already taken"));
    }

    let hash = auth::hash_password(&password).map_err(|e| ServerFnError::new(e.to_string()))?;

    let new_user = NewUser {
        username: &username,
        display_name: &display_name,
        email: &email,
        passcrypt: &hash,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {e}")))?;

    let user = users::table
        .filter(users::username.eq(&username))
        .select(User::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let token = auth::generate_jwt(user.id, &user.username)
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    set_jwt_cookie(&token)?;

    Ok(UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
    })
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    clear_jwt_cookie();
    Ok(())
}

#[server]
pub async fn get_current_user() -> Result<Option<UserInfo>, ServerFnError> {
    use crate::auth;
    use crate::db;
    use crate::models::db_models::User;
    use crate::schema::users;
    use diesel::prelude::*;

    let req_parts: axum::http::request::Parts = leptos_axum::extract().await?;

    let cookie_header = match req_parts.headers.get(axum::http::header::COOKIE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Ok(None),
    };

    let token = cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie.strip_prefix("token=")
    });

    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };

    let claims = match auth::verify_jwt(token) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };

    let (user_id, _username) = match auth::parse_claims_sub(&claims.sub) {
        Some(parsed) => parsed,
        None => return Ok(None),
    };

    let conn = &mut db::get_conn();
    let user = users::table
        .find(user_id)
        .select(User::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(user.map(|u| UserInfo {
        id: u.id,
        username: u.username,
        display_name: u.display_name,
    }))
}

#[server]
pub async fn list_sessions() -> Result<Vec<SessionInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::Session;
    use crate::schema::{sessions, users};
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let results: Vec<(Session, String)> = sessions::table
        .inner_join(users::table.on(users::id.eq(sessions::gm_user_id)))
        .filter(sessions::active.eq(true))
        .select((Session::as_select(), users::username))
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(results
        .into_iter()
        .map(|(session, gm_username)| SessionInfo {
            id: session.id,
            name: session.name,
            gm_username,
            active: session.active,
        })
        .collect())
}

#[server]
pub async fn create_session(name: String) -> Result<SessionInfo, ServerFnError> {
    use crate::db;
    use crate::models::db_models::NewSession;
    use crate::schema::{session_players, sessions};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let new_session = NewSession {
        name: &name,
        gm_user_id: user.id,
        template_id: None,
    };

    diesel::insert_into(sessions::table)
        .values(&new_session)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create session: {e}")))?;

    let session_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    // Add GM as session player
    diesel::insert_into(session_players::table)
        .values(&crate::models::db_models::NewSessionPlayer {
            session_id,
            user_id: user.id,
            role: "gm".to_string(),
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to add GM to session: {e}")))?;

    Ok(SessionInfo {
        id: session_id,
        name,
        gm_username: user.username,
        active: true,
    })
}

#[server]
pub async fn create_map(
    session_id: i32,
    name: String,
    width: i32,
    height: i32,
    cell_size: Option<i32>,
    background_url: Option<String>,
) -> Result<crate::models::MapInfo, ServerFnError> {
    use crate::db;
    use crate::schema::{maps, sessions};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    let gm_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if gm_id != user.id {
        return Err(ServerFnError::new("Only the GM can create maps"));
    }

    let cell = cell_size.unwrap_or(40).max(10).min(200);

    let new_map = crate::models::db_models::NewMap {
        session_id,
        name: &name,
        width,
        height,
    };

    diesel::insert_into(maps::table)
        .values(&new_map)
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create map: {e}")))?;

    let map_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Failed to get map id: {e}")))?;

    // Update cell_size and background if non-default
    let _ = diesel::update(maps::table.find(map_id))
        .set((
            maps::cell_size.eq(cell),
            maps::background_url.eq(&background_url),
        ))
        .execute(conn);

    let map = maps::table
        .find(map_id)
        .select(crate::models::db_models::Map::as_select())
        .first(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(crate::models::MapInfo {
        id: map.id,
        name: map.name,
        width: map.width,
        height: map.height,
        cell_size: map.cell_size,
        background_url: map.background_url,
        default_token_color: map.default_token_color,
    })
}

#[server]
pub async fn list_maps(session_id: i32) -> Result<Vec<crate::models::MapInfo>, ServerFnError> {
    use crate::db;
    use crate::schema::maps;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    let rows = maps::table
        .filter(maps::session_id.eq(session_id))
        .order(maps::id.asc())
        .select(crate::models::db_models::Map::as_select())
        .load::<crate::models::db_models::Map>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|m| crate::models::MapInfo {
            id: m.id,
            name: m.name,
            width: m.width,
            height: m.height,
            cell_size: m.cell_size,
            background_url: m.background_url,
            default_token_color: m.default_token_color,
        })
        .collect())
}

#[server]
pub async fn delete_map(map_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{fog_of_war, maps, sessions, token_instances, tokens};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();

    // Verify GM
    let session_id: i32 = maps::table
        .find(map_id)
        .select(maps::session_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Map not found"))?;

    let gm_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if gm_id != user.id {
        return Err(ServerFnError::new("Only the GM can delete maps"));
    }

    // Delete token instances for tokens on this map
    let token_ids: Vec<i32> = tokens::table
        .filter(tokens::map_id.eq(map_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();

    for tid in &token_ids {
        let _ = diesel::delete(token_instances::table.filter(token_instances::token_id.eq(tid)))
            .execute(conn);
    }

    // Delete tokens, fog, and the map
    let _ = diesel::delete(tokens::table.filter(tokens::map_id.eq(map_id))).execute(conn);
    let _ = diesel::delete(fog_of_war::table.filter(fog_of_war::map_id.eq(map_id))).execute(conn);
    let _ = diesel::delete(maps::table.find(map_id)).execute(conn);

    Ok(())
}

#[server]
pub async fn get_ws_token() -> Result<String, ServerFnError> {
    let req_parts: axum::http::request::Parts = leptos_axum::extract().await?;

    let cookie_header = match req_parts.headers.get(axum::http::header::COOKIE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => return Err(ServerFnError::new("Not logged in")),
    };

    let token = cookie_header.split(';').find_map(|cookie| {
        let cookie = cookie.trim();
        cookie.strip_prefix("token=")
    });

    match token {
        Some(t) if !t.is_empty() => Ok(t.to_string()),
        _ => Err(ServerFnError::new("Not logged in")),
    }
}

#[server]
pub async fn join_session(session_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::{session_players, sessions};
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    // Check session exists and is active
    let exists = sessions::table
        .find(session_id)
        .filter(sessions::active.eq(true))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if !exists {
        return Err(ServerFnError::new("Session not found or inactive"));
    }

    // Check if already a member
    let already_member = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user.id))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;

    if !already_member {
        diesel::insert_into(session_players::table)
            .values(&crate::models::db_models::NewSessionPlayer {
                session_id,
                user_id: user.id,
                role: "player".to_string(),
            })
            .execute(conn)
            .map_err(|e| ServerFnError::new(format!("Failed to join session: {e}")))?;
    }

    Ok(())
}

// ===== Template functions =====

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

    Ok(())
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

        if session.gm_user_id != user.id {
            return Err(ServerFnError::new("Not your character"));
        }
    }

    // Delete associated resources first
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

    if session.gm_user_id != user.id {
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

    if session.gm_user_id != user.id {
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

    if session.gm_user_id != user.id {
        return Err(ServerFnError::new("Only the GM can delete creatures"));
    }

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

    if session.gm_user_id != user.id {
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

#[server]
pub async fn list_media(
    media_type: Option<String>,
    search: Option<String>,
    tag: Option<String>,
) -> Result<Vec<MediaInfo>, ServerFnError> {
    use crate::db;
    use crate::models::db_models::*;
    use crate::schema::*;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let mut query = media::table.into_boxed();

    if let Some(ref mt) = media_type {
        query = query.filter(media::media_type.eq(mt));
    }

    let media_rows: Vec<Media> = query
        .order(media::id.desc())
        .select(Media::as_select())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let mut results = Vec::new();
    for m in media_rows {
        let tags: Vec<String> = media_tags::table
            .filter(media_tags::media_id.eq(m.id))
            .select(media_tags::tag)
            .load(conn)
            .unwrap_or_default();

        // Filter by tag if specified
        if let Some(ref filter_tag) = tag {
            if !tags.iter().any(|t| t == filter_tag) {
                continue;
            }
        }

        // Filter by search term (matches against tags)
        if let Some(ref search_term) = search {
            let term = search_term.to_lowercase();
            if !tags.iter().any(|t| t.to_lowercase().contains(&term)) {
                continue;
            }
        }

        results.push(MediaInfo {
            id: m.id,
            hash: m.hash.clone(),
            url: format!("/api/media/{}", m.hash),
            content_type: m.content_type,
            media_type: m.media_type,
            size_bytes: m.size_bytes,
            tags,
        });
    }

    Ok(results)
}

#[server]
pub async fn list_media_tags(prefix: Option<String>) -> Result<Vec<String>, ServerFnError> {
    use crate::db;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let mut query = media_tags::table
        .select(media_tags::tag)
        .distinct()
        .into_boxed();

    if let Some(ref p) = prefix {
        query = query.filter(media_tags::tag.like(format!("{p}%")));
    }

    let tags: Vec<String> = query
        .order(media_tags::tag.asc())
        .load(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(tags)
}

#[server]
pub async fn add_media_tag(media_id: i32, tag: String) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::NewMediaTag;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let _ = diesel::insert_or_ignore_into(media_tags::table)
        .values(&NewMediaTag {
            media_id,
            tag: &tag,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to add tag: {e}")))?;

    Ok(())
}

#[server]
pub async fn remove_media_tag(media_id: i32, tag: String) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::schema::media_tags;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    diesel::delete(
        media_tags::table
            .filter(media_tags::media_id.eq(media_id))
            .filter(media_tags::tag.eq(&tag)),
    )
    .execute(conn)
    .map_err(|e| ServerFnError::new(format!("Failed to remove tag: {e}")))?;

    Ok(())
}

#[server]
pub async fn delete_media(media_id: i32) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::Media;
    use crate::schema::*;
    use diesel::prelude::*;

    let _user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();

    let m: Media = media::table
        .find(media_id)
        .select(Media::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Media not found"))?;

    // Delete tags first, then media record
    diesel::delete(media_tags::table.filter(media_tags::media_id.eq(media_id)))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete tags: {e}")))?;

    diesel::delete(media::table.find(media_id))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to delete media: {e}")))?;

    // Delete file from disk
    let media_dir = std::path::PathBuf::from(
        std::env::var("MEDIA_DIR").unwrap_or_else(|_| "uploads/media".to_string()),
    );
    let file_path = media_dir.join(&m.hash[..2]).join(&m.hash);
    let _ = std::fs::remove_file(&file_path);

    Ok(())
}

#[server]
pub async fn update_character_portrait(
    character_id: i32,
    portrait_url: Option<String>,
) -> Result<(), ServerFnError> {
    use crate::db;
    use crate::models::db_models::Character;
    use crate::schema::characters;
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
        return Err(ServerFnError::new("Not your character"));
    }

    diesel::update(characters::table.find(character_id))
        .set(characters::portrait_url.eq(&portrait_url))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to update portrait: {e}")))?;

    // Update image on any tokens linked to this character
    use crate::schema::tokens;
    let updated_token_ids: Vec<i32> = tokens::table
        .filter(tokens::character_id.eq(character_id))
        .select(tokens::id)
        .load(conn)
        .unwrap_or_default();
    if !updated_token_ids.is_empty() {
        diesel::update(tokens::table.filter(tokens::character_id.eq(character_id)))
            .set(tokens::image_url.eq(&portrait_url))
            .execute(conn)
            .ok();
    }

    // Broadcast character change to all clients in the session
    use crate::ws::messages::ServerMessage;
    use crate::ws::session::SESSION_MANAGER;
    SESSION_MANAGER.broadcast(
        character.session_id,
        &ServerMessage::CharacterUpdated {
            character_id,
            field_path: "portrait_url".to_string(),
            value: serde_json::json!(portrait_url),
        },
        None,
    );

    // Broadcast token image updates
    for tid in updated_token_ids {
        SESSION_MANAGER.broadcast(
            character.session_id,
            &ServerMessage::TokenImageUpdated {
                token_id: tid,
                image_url: portrait_url.clone(),
            },
            None,
        );
    }

    Ok(())
}

// ===== VFS (Virtual File System) =====

/// Build a VfsScope for C: drive operations (session-scoped, shared).
#[cfg(feature = "ssr")]
fn vfs_session_scope(
    session_id: i32,
    user_id: i32,
    conn: &mut diesel::SqliteConnection,
) -> Result<crate::vfs::VfsScope, ServerFnError> {
    use crate::models::db_models::Session;
    use crate::schema::{session_players, sessions};
    use diesel::prelude::*;

    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let is_player: bool = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user_id))
        .count()
        .get_result::<i64>(conn)
        .unwrap_or(0)
        > 0;

    Ok(crate::vfs::VfsScope {
        session_id: Some(session_id),
        user_id: Some(user_id),
        is_gm: session.gm_user_id == user_id,
        is_player,
        umask: crate::vfs::DEFAULT_UMASK,
    })
}

/// Build a VfsScope for U: drive operations (user-scoped, private).
#[cfg(feature = "ssr")]
fn vfs_user_scope(user_id: i32) -> crate::vfs::VfsScope {
    crate::vfs::VfsScope {
        session_id: None,
        user_id: Some(user_id),
        is_gm: false,
        is_player: false,
        umask: crate::vfs::DEFAULT_UMASK,
    }
}

/// Parse a drive letter string ("C" or "U") into a Drive enum.
#[cfg(feature = "ssr")]
fn parse_drive(drive: &str) -> Result<crate::vfs::Drive, ServerFnError> {
    let c = drive
        .chars()
        .next()
        .ok_or_else(|| ServerFnError::new("Empty drive letter"))?;
    match c.to_ascii_uppercase() {
        'C' | 'U' => {}
        'A' | 'B' => {
            return Err(ServerFnError::new(
                "Scratch drives (A:, B:) are client-side only",
            ));
        }
        _ => return Err(ServerFnError::new("Invalid drive letter")),
    }
    crate::vfs::Drive::from_letter(c).ok_or_else(|| ServerFnError::new("Invalid drive letter"))
}

/// Build a VfsScope for a given drive, validating auth context.
#[cfg(feature = "ssr")]
fn vfs_scope_for(
    drive: crate::vfs::Drive,
    session_id: Option<i32>,
    user_id: i32,
    conn: &mut diesel::SqliteConnection,
) -> Result<crate::vfs::VfsScope, ServerFnError> {
    match drive {
        crate::vfs::Drive::C => {
            let sid =
                session_id.ok_or_else(|| ServerFnError::new("session_id required for C: drive"))?;
            vfs_session_scope(sid, user_id, conn)
        }
        crate::vfs::Drive::U => Ok(vfs_user_scope(user_id)),
        _ => Err(ServerFnError::new(
            "Only C: and U: drives are supported server-side",
        )),
    }
}

/// Common VFS preamble: authenticate, get DB connection, parse drive, build scope.
/// Returns (connection, drive, scope, user_id) or an error.
#[cfg(feature = "ssr")]
async fn vfs_auth_scope(
    drive_str: &str,
    session_id: Option<i32>,
) -> Result<
    (
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        crate::vfs::Drive,
        crate::vfs::VfsScope,
        i32,
    ),
    ServerFnError,
> {
    use crate::db;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let mut conn = db::get_conn();
    let drive = parse_drive(drive_str)?;
    let scope = vfs_scope_for(drive, session_id, user.id, &mut conn)?;
    Ok((conn, drive, scope, user.id))
}

/// Broadcast a VfsChanged notification for C: drive modifications.
#[cfg(feature = "ssr")]
fn vfs_broadcast(session_id: Option<i32>, path: &str, action: &str) {
    if let Some(sid) = session_id {
        use crate::ws::messages::ServerMessage;
        use crate::ws::session::SESSION_MANAGER;
        SESSION_MANAGER.broadcast(
            sid,
            &ServerMessage::VfsChanged {
                path: path.to_string(),
                action: action.to_string(),
            },
            None,
        );
    }
}

/// List directory contents.
#[server]
pub async fn vfs_list_dir(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<Vec<VfsEntryInfo>, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let entries = vfs::vfs_list(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(entries
        .into_iter()
        .map(|e| VfsEntryInfo {
            path: e.path,
            is_directory: e.is_directory,
            size_bytes: e.size_bytes,
            content_type: e.content_type,
            modified_by: e.modified_by,
            created_at: e.created_at,
            updated_at: e.updated_at,
            mode: e.mode,
        })
        .collect())
}

/// Get file/directory metadata.
#[server]
pub async fn vfs_stat_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<VfsEntryInfo, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let entry = vfs::vfs_stat(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(VfsEntryInfo {
        path: entry.path,
        is_directory: entry.is_directory,
        size_bytes: entry.size_bytes,
        content_type: entry.content_type,
        modified_by: entry.modified_by,
        created_at: entry.created_at,
        updated_at: entry.updated_at,
        mode: entry.mode,
    })
}

/// Read file contents. Returns inline data for small files, or a CAS URL
/// for large files that the client can fetch directly.
#[server]
pub async fn vfs_read_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<VfsFileData, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let content = vfs::vfs_read(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    match content {
        vfs::VfsFileContent::Inline { data, content_type } => {
            Ok(VfsFileData::Inline { data, content_type })
        }
        vfs::VfsFileContent::CasReference {
            hash,
            content_type,
            size_bytes,
        } => Ok(VfsFileData::CasUrl {
            url: format!("/api/media/{}", hash),
            content_type,
            size_bytes,
        }),
    }
}

/// Write a small file (inline, up to 8 KB). For larger files, use the
/// media upload endpoint and then call `vfs_write_cas` with the hash.
#[server]
pub async fn vfs_write_file(
    drive: String,
    path: String,
    data: Vec<u8>,
    content_type: Option<String>,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_write(
        conn,
        &scope,
        drive,
        &parsed.path,
        &data,
        content_type.as_deref(),
        None,
        user_id,
        true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "write");
    Ok(())
}

/// Write a CAS-referenced file (large file already uploaded via media endpoint).
#[server]
pub async fn vfs_write_cas(
    drive: String,
    path: String,
    media_hash: String,
    size_bytes: i64,
    content_type: Option<String>,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Verify the media hash exists in CAS
    use crate::schema::media;
    use diesel::prelude::*;
    let exists: bool = media::table
        .filter(media::hash.eq(&media_hash))
        .count()
        .get_result::<i64>(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        > 0;
    if !exists {
        return Err(ServerFnError::new("Media hash not found in storage"));
    }

    // Use vfs_write with the media_hash for CAS reference
    let dummy = vec![0u8; size_bytes as usize];
    vfs::vfs_write(
        conn,
        &scope,
        drive,
        &parsed.path,
        &dummy,
        content_type.as_deref(),
        Some(&media_hash),
        user_id,
        true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "write");
    Ok(())
}

/// Create a directory.
#[server]
pub async fn vfs_mkdir_dir(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_mkdir(conn, &scope, drive, &parsed.path, user_id, true)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "mkdir");
    Ok(())
}

/// Delete a file or empty directory.
#[server]
pub async fn vfs_delete_file(
    drive: String,
    path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_delete(conn, &scope, drive, &parsed.path)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "delete");
    Ok(())
}

/// Rename or move a file/directory within the same drive.
#[server]
pub async fn vfs_rename_file(
    drive: String,
    old_path: String,
    new_path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let old = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), old_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let new = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), new_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_rename(conn, &scope, drive, &old.path, &new.path, user_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &old.path, "rename");
    Ok(())
}

/// Copy a file (within same drive or across C:/U: drives).
#[server]
pub async fn vfs_copy_file(
    src_drive: String,
    src_path: String,
    dst_drive: String,
    dst_path: String,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, src_drv, src_scope, user_id) = vfs_auth_scope(&src_drive, session_id).await?;
    let conn = &mut conn;
    let dst_drv = parse_drive(&dst_drive)?;
    let dst_scope = vfs_scope_for(dst_drv, session_id, user_id, conn)?;
    let src = vfs::VfsPath::parse(&format!("{}:{}", src_drv.letter(), src_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let dst = vfs::VfsPath::parse(&format!("{}:{}", dst_drv.letter(), dst_path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_copy(
        conn, &src_scope, src_drv, &src.path, &dst_scope, dst_drv, &dst.path, user_id, true,
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Broadcast if destination is on C: drive
    if matches!(dst_drv, vfs::Drive::C) {
        vfs_broadcast(session_id, &dst.path, "write");
    }
    Ok(())
}

/// Change file permissions (GM-only).
#[server]
pub async fn vfs_chmod_file(
    drive: String,
    path: String,
    mode: i32,
    session_id: Option<i32>,
) -> Result<(), ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;
    let parsed = vfs::VfsPath::parse(&format!("{}:{}", drive.letter(), path))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs::vfs_chmod(conn, &scope, drive, &parsed.path, mode, user_id)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    vfs_broadcast(session_id, &parsed.path, "chmod");
    Ok(())
}

/// Get drive usage and quota information.
#[server]
pub async fn vfs_get_drive_info(
    drive: String,
    session_id: Option<i32>,
) -> Result<crate::models::VfsDriveInfo, ServerFnError> {
    use crate::vfs;

    let (mut conn, drive, scope, _user_id) = vfs_auth_scope(&drive, session_id).await?;
    let conn = &mut conn;

    let used_bytes =
        vfs::vfs_drive_usage(conn, &scope, drive).map_err(|e| ServerFnError::new(e.to_string()))?;
    let quota_bytes = drive.quota_bytes(scope.is_gm);

    Ok(crate::models::VfsDriveInfo {
        drive: drive.letter(),
        used_bytes,
        quota_bytes,
    })
}
