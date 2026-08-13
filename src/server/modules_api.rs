//! Server functions for browsing and installing game modules.
//!
//! Reading a module is open to any session member, since players need the
//! roll model, the pregens, and the item cards. Installing one changes the
//! session, so it is GM only.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::CharacterInfo;
use crate::modules::{AdventureModule, InstallReport, ModuleSummary, RollRules, SystemModule};

/// What a session is currently running.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionModules {
    pub system: Option<ModuleSummary>,
    pub adventure: Option<ModuleSummary>,
    /// Roll model from the system module, so the client can build its roller.
    pub rules: Option<RollRules>,
}

/// Confirm the caller is the GM of this session.
#[cfg(feature = "ssr")]
fn require_gm(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
    user_id: i32,
) -> Result<(), ServerFnError> {
    use crate::schema::sessions;
    use diesel::prelude::*;

    let gm_id: i32 = sessions::table
        .find(session_id)
        .select(sessions::gm_user_id)
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if gm_id != user_id {
        return Err(ServerFnError::new("Only the GM can install modules"));
    }
    Ok(())
}

/// Confirm the caller belongs to this session.
#[cfg(feature = "ssr")]
fn require_member(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
    user_id: i32,
) -> Result<(), ServerFnError> {
    use crate::schema::session_players;
    use diesel::prelude::*;

    let count: i64 = session_players::table
        .filter(session_players::session_id.eq(session_id))
        .filter(session_players::user_id.eq(user_id))
        .count()
        .get_result(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if count == 0 {
        return Err(ServerFnError::new("Not a member of this session"));
    }
    Ok(())
}

/// Every module the server can see.
#[server]
pub async fn list_modules() -> Result<Vec<ModuleSummary>, ServerFnError> {
    use crate::modules::loader;
    Ok(loader::list_summaries())
}

/// Load a system module in full.
#[server]
pub async fn get_system_module(module_id: String) -> Result<SystemModule, ServerFnError> {
    use crate::modules::loader;
    loader::load_system(&module_id).map_err(ServerFnError::new)
}

/// Load an adventure module in full.
///
/// This includes the room key and other GM material, so it is refused to
/// anyone but the session's GM. Players reach the player-facing parts through
/// the pregen and item lists below.
#[server]
pub async fn get_adventure_module(
    session_id: i32,
    module_id: String,
) -> Result<AdventureModule, ServerFnError> {
    use crate::db;
    use crate::modules::loader;
    use crate::server::api::get_current_user;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();
    require_gm(conn, session_id, user.id)?;

    loader::load_adventure(&module_id).map_err(ServerFnError::new)
}

/// The player-facing half of an adventure module: pregens and item cards.
#[server]
pub async fn get_adventure_handouts(
    session_id: i32,
    module_id: String,
) -> Result<AdventureModule, ServerFnError> {
    use crate::db;
    use crate::modules::loader;
    use crate::server::api::get_current_user;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();
    require_member(conn, session_id, user.id)?;

    let mut adventure = loader::load_adventure(&module_id).map_err(ServerFnError::new)?;
    // Strip everything a player should not read. The GM gets the full module
    // through get_adventure_module.
    adventure.rooms.clear();
    adventure.bestiary.clear();
    adventure.tables.clear();
    adventure.maps.clear();
    Ok(adventure)
}

/// Every reference module, in full.
///
/// Reference modules are lookup material with no GM secrets in them, so they
/// need no install and are open to any member of the session.
#[server]
pub async fn list_reference_modules(
    session_id: i32,
) -> Result<Vec<AdventureModule>, ServerFnError> {
    use crate::db;
    use crate::modules::{ModuleKind, loader};
    use crate::server::api::get_current_user;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();
    require_member(conn, session_id, user.id)?;

    let mut out = Vec::new();
    for summary in loader::list_summaries() {
        if summary.kind != ModuleKind::Reference {
            continue;
        }
        match loader::load_adventure(&summary.id) {
            Ok(module) => out.push(module),
            Err(e) => log::warn!("Skipping reference module {}: {e}", summary.id),
        }
    }
    Ok(out)
}

/// Which modules a session is running, and the roll model that comes with them.
#[server]
pub async fn get_session_modules(session_id: i32) -> Result<SessionModules, ServerFnError> {
    use crate::db;
    use crate::models::db_models::Session;
    use crate::modules::loader;
    use crate::schema::sessions;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    let mut out = SessionModules::default();

    if let Some(id) = session.system_module_id.as_deref() {
        match loader::load_system(id) {
            Ok(system) => {
                out.rules = Some(system.rules);
                out.system = Some(system.summary);
            }
            Err(e) => log::warn!("Session {session_id} names missing system module {id}: {e}"),
        }
    }

    if let Some(id) = session.adventure_module_id.as_deref() {
        match loader::load_summary(id) {
            Ok(summary) => out.adventure = Some(summary),
            Err(e) => log::warn!("Session {session_id} names missing adventure module {id}: {e}"),
        }
    }

    Ok(out)
}

/// Install a module into a session. GM only.
///
/// Installing a system module seeds its sheet schema as a template and points
/// the session at it. Installing an adventure seeds the bestiary as creatures
/// and its maps as maps, skipping anything already there by name, so running
/// it twice does not duplicate content.
#[server]
pub async fn install_module(
    session_id: i32,
    module_id: String,
) -> Result<InstallReport, ServerFnError> {
    use crate::db;
    use crate::modules::{ModuleKind, loader};
    use crate::server::api::get_current_user;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;
    let conn = &mut db::get_conn();
    require_gm(conn, session_id, user.id)?;

    let summary = loader::load_summary(&module_id).map_err(ServerFnError::new)?;

    match summary.kind {
        ModuleKind::System => install_system(conn, session_id, &module_id),
        ModuleKind::Adventure => install_adventure(conn, session_id, &module_id, user.id).await,
        ModuleKind::Reference => Err(ServerFnError::new(
            "Reference modules are always available and are not installed.",
        )),
    }
}

/// Seed a system module's sheet schema and bind the session to it.
#[cfg(feature = "ssr")]
fn install_system(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
    module_id: &str,
) -> Result<InstallReport, ServerFnError> {
    use crate::models::db_models::{NewRpgTemplate, RpgTemplate};
    use crate::modules::loader;
    use crate::schema::{rpg_templates, sessions};
    use diesel::prelude::*;

    let system = loader::load_system(module_id).map_err(ServerFnError::new)?;

    let schema_json = serde_json::to_string(&system.sheet)
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    // One template per system module, keyed by name. Reinstalling refreshes
    // the schema in place so an edited sheet.json reaches existing sessions.
    let existing: Option<RpgTemplate> = rpg_templates::table
        .filter(rpg_templates::name.eq(&system.summary.name))
        .select(RpgTemplate::as_select())
        .first(conn)
        .optional()
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let template_id = match existing {
        Some(t) => {
            diesel::update(rpg_templates::table.find(t.id))
                .set((
                    rpg_templates::description.eq(&system.summary.description),
                    rpg_templates::schema_json.eq(&schema_json),
                ))
                .execute(conn)
                .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;
            t.id
        }
        None => {
            diesel::insert_into(rpg_templates::table)
                .values(&NewRpgTemplate {
                    name: &system.summary.name,
                    description: &system.summary.description,
                    schema_json: &schema_json,
                })
                .execute(conn)
                .map_err(|e| ServerFnError::new(format!("Failed to create template: {e}")))?;
            diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
                "last_insert_rowid()",
            ))
            .get_result(conn)
            .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?
        }
    };

    diesel::update(sessions::table.find(session_id))
        .set((
            sessions::template_id.eq(Some(template_id)),
            sessions::system_module_id.eq(Some(module_id)),
        ))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    Ok(InstallReport {
        module_id: module_id.to_string(),
        module_name: system.summary.name,
        template_id: Some(template_id),
        ..Default::default()
    })
}

/// Seed an adventure module's creatures and maps into a session.
#[cfg(feature = "ssr")]
async fn install_adventure(
    conn: &mut diesel::SqliteConnection,
    session_id: i32,
    module_id: &str,
    user_id: i32,
) -> Result<InstallReport, ServerFnError> {
    use crate::models::db_models::{NewCreature, NewMap, Session};
    use crate::modules::loader;
    use crate::schema::{creatures, maps, sessions};
    use diesel::prelude::*;

    let adventure = loader::load_adventure(module_id).map_err(ServerFnError::new)?;

    let mut report = InstallReport {
        module_id: module_id.to_string(),
        module_name: adventure.summary.name.clone(),
        ..Default::default()
    };

    // An adventure written for one system installed over another still works,
    // but the numbers on its stat blocks will not mean what they should.
    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    if let Some(required) = adventure.summary.requires.as_deref() {
        match session.system_module_id.as_deref() {
            Some(installed) if installed == required => {}
            Some(installed) => report.warnings.push(format!(
                "This adventure is written for {required}, but the session is running {installed}."
            )),
            None => report.warnings.push(format!(
                "This adventure is written for {required}, which is not installed in this session yet."
            )),
        }
    }

    // Creatures, skipping any this session already has by name.
    let existing_creatures: Vec<String> = creatures::table
        .filter(creatures::session_id.eq(session_id))
        .select(creatures::name)
        .load(conn)
        .unwrap_or_default();

    for block in &adventure.bestiary {
        if existing_creatures.iter().any(|n| n == &block.name) {
            continue;
        }
        let stats = serde_json::json!({
            "ds": block.ds,
            "attack": "",
            "armour_absorbs": 0,
            "notes": block.notes,
        });
        let stat_json = serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string());

        let inserted = diesel::insert_into(creatures::table)
            .values(&NewCreature {
                session_id,
                template_id: session.template_id,
                name: &block.name,
                stat_data_json: &stat_json,
            })
            .execute(conn);

        match inserted {
            Ok(_) => report.creatures_added += 1,
            Err(e) => report
                .warnings
                .push(format!("Could not add creature {}: {e}", block.name)),
        }
    }

    // Maps, skipping any this session already has by name.
    let existing_maps: Vec<String> = maps::table
        .filter(maps::session_id.eq(session_id))
        .select(maps::name)
        .load(conn)
        .unwrap_or_default();

    for map_def in &adventure.maps {
        if existing_maps.iter().any(|n| n == &map_def.name) {
            continue;
        }

        // Art is optional: modules under a non-commercial licence ship without
        // it, and the GM attaches a background afterwards.
        let background_url = match map_def.asset.as_deref() {
            Some(asset) => match ingest_asset(conn, module_id, asset, user_id).await {
                Ok(url) => Some(url),
                Err(e) => {
                    log::info!("Module {module_id} asset {asset} not installed: {e}");
                    report.maps_missing_art.push(map_def.name.clone());
                    None
                }
            },
            None => None,
        };

        if diesel::insert_into(maps::table)
            .values(&NewMap {
                session_id,
                name: &map_def.name,
                width: map_def.width,
                height: map_def.height,
            })
            .execute(conn)
            .is_err()
        {
            report
                .warnings
                .push(format!("Could not add map {}", map_def.name));
            continue;
        }

        let map_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
            "last_insert_rowid()",
        ))
        .get_result(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

        let _ = diesel::update(maps::table.find(map_id))
            .set((
                maps::cell_size.eq(map_def.cell_size.clamp(10, 200)),
                maps::background_url.eq(&background_url),
            ))
            .execute(conn);

        report.maps_added += 1;
    }

    diesel::update(sessions::table.find(session_id))
        .set(sessions::adventure_module_id.eq(Some(module_id)))
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    if !report.maps_missing_art.is_empty() {
        report.warnings.push(
            "Some maps came up without art. See the module's assets/README.md for where to get it."
                .to_string(),
        );
    }

    Ok(report)
}

/// Copy a module asset into content-addressable media storage, returning its URL.
#[cfg(feature = "ssr")]
async fn ingest_asset(
    conn: &mut diesel::SqliteConnection,
    module_id: &str,
    asset: &str,
    user_id: i32,
) -> Result<String, String> {
    use crate::models::db_models::{Media, NewMedia, NewMediaTag};
    use crate::modules::loader;
    use crate::schema::{media, media_tags};
    use diesel::prelude::*;
    use sha2::{Digest, Sha256};

    let path =
        loader::asset_path(module_id, asset).ok_or_else(|| format!("no such asset {asset}"))?;

    let content_type = match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        other => return Err(format!("unsupported asset type: {other}")),
    };

    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    let dir = crate::server::media_handler::media_dir().join(&hash[..2]);
    let file_path = dir.join(&hash);
    if !file_path.exists() {
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        tokio::fs::write(&file_path, &bytes)
            .await
            .map_err(|e| format!("cannot write {}: {e}", file_path.display()))?;
    }

    let existing: Option<Media> = media::table
        .filter(media::hash.eq(&hash))
        .select(Media::as_select())
        .first(conn)
        .optional()
        .map_err(|e| format!("database error: {e}"))?;

    let media_id = match existing {
        Some(m) => m.id,
        None => {
            diesel::insert_into(media::table)
                .values(&NewMedia {
                    hash: &hash,
                    content_type,
                    media_type: "image",
                    size_bytes: bytes.len() as i64,
                    uploaded_by: user_id,
                })
                .execute(conn)
                .map_err(|e| format!("database error: {e}"))?;
            diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
                "last_insert_rowid()",
            ))
            .get_result(conn)
            .map_err(|e| format!("database error: {e}"))?
        }
    };

    for tag in [module_id, asset] {
        let _ = diesel::insert_or_ignore_into(media_tags::table)
            .values(&NewMediaTag { media_id, tag })
            .execute(conn);
    }

    Ok(format!("/api/media/{hash}"))
}

/// Create a character for the calling player from one of a module's pregens.
///
/// The pregen's sheet values are laid over the session template's defaults, so
/// a pregen that omits a field still gets a sensible one. Its listed gear
/// becomes inventory items owned by the new character, with slot costs taken
/// from the module's item cards where they match by name.
#[server]
pub async fn create_character_from_pregen(
    session_id: i32,
    module_id: String,
    pregen_id: String,
    name: Option<String>,
) -> Result<CharacterInfo, ServerFnError> {
    use crate::db;
    use crate::models::TemplateField;
    use crate::models::db_models::*;
    use crate::modules::loader;
    use crate::schema::*;
    use crate::server::api::get_current_user;
    use diesel::prelude::*;

    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not logged in"))?;

    let conn = &mut db::get_conn();
    require_member(conn, session_id, user.id)?;

    let adventure = loader::load_adventure(&module_id).map_err(ServerFnError::new)?;
    let pregen = adventure
        .pregens
        .iter()
        .find(|p| p.id == pregen_id)
        .ok_or_else(|| ServerFnError::new(format!("No pregen {pregen_id} in {module_id}")))?;

    let session: Session = sessions::table
        .find(session_id)
        .select(Session::as_select())
        .first(conn)
        .map_err(|_| ServerFnError::new("Session not found"))?;

    // Template defaults first, then the pregen's own values on top.
    let mut data = serde_json::Map::new();
    if let Some(tid) = session.template_id {
        let template: RpgTemplate = rpg_templates::table
            .find(tid)
            .select(RpgTemplate::as_select())
            .first(conn)
            .map_err(|_| ServerFnError::new("Template not found"))?;
        let fields: Vec<TemplateField> =
            serde_json::from_str(&template.schema_json).unwrap_or_default();
        for field in &fields {
            data.insert(field.name.clone(), field.default.clone());
        }
    }
    if let Some(sheet) = pregen.sheet.as_object() {
        for (k, v) in sheet {
            data.insert(k.clone(), v.clone());
        }
    }
    let data_value = serde_json::Value::Object(data.clone());
    let data_str = serde_json::to_string(&data_value)
        .map_err(|e| ServerFnError::new(format!("Serialization error: {e}")))?;

    let char_name = name.unwrap_or_else(|| pregen.name.clone());

    diesel::insert_into(characters::table)
        .values(&NewCharacter {
            session_id,
            user_id: user.id,
            name: &char_name,
            data_json: &data_str,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create character: {e}")))?;

    let char_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>(
        "last_insert_rowid()",
    ))
    .get_result(conn)
    .map_err(|e| ServerFnError::new(format!("Database error: {e}")))?;

    let hp_max = data.get("hp_max").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
    diesel::insert_into(character_resources::table)
        .values(&NewCharacterResource {
            character_id: char_id,
            name: "HP",
            current_value: hp_max,
            max_value: hp_max,
        })
        .execute(conn)
        .map_err(|e| ServerFnError::new(format!("Failed to create resource: {e}")))?;

    for item in &pregen.items {
        let card = adventure
            .items
            .iter()
            .find(|i| i.name.eq_ignore_ascii_case(&item.name));
        let slots = card.map(|c| c.slots).unwrap_or(1);
        let kind = card.map(|c| c.kind.as_str()).unwrap_or("gear");
        let uses = card.and_then(|c| c.uses);
        let bonus = if item.bonus.is_empty() {
            card.map(|c| c.bonus.as_str()).unwrap_or("")
        } else {
            item.bonus.as_str()
        };

        let _ = diesel::insert_into(inventory_items::table)
            .values(&NewInventoryItem {
                session_id,
                name: &item.name,
                description: card.map(|c| c.note.as_str()).unwrap_or(""),
                quantity: 1,
                is_party_item: false,
                owner_character_id: Some(char_id),
                slots,
                kind,
                bonus,
                uses_max: uses,
                uses_left: uses,
            })
            .execute(conn);
    }

    Ok(CharacterInfo {
        id: char_id,
        session_id,
        user_id: user.id,
        name: char_name,
        data: data_value,
        resources: vec![],
        portrait_url: None,
    })
}
