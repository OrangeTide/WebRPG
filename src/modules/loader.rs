//! Reading module packs off disk.
//!
//! Modules live under `MODULES_DIR` (default `modules/`), one directory per
//! module. Files are read on demand rather than cached, so editing a module's
//! JSON shows up on the next request without restarting the server. The packs
//! are small, and content authoring is the common case.
//!
//! Nothing here writes to disk. A module that fails to parse is skipped with a
//! log line, so one bad pack cannot stop the server or hide the others.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::models::TemplateField;
use crate::modules::{
    AdventureModule, HelpTopic, ItemDef, MapDef, ModuleKind, ModuleSummary, Pregen, RandomTable,
    RollRules, RoomEntry, StatBlock, SystemModule, is_valid_module_id,
};

/// The files a `module.json` points at. Every entry is optional; a module only
/// names the parts it has.
#[derive(Debug, Clone, Default, Deserialize)]
struct ModuleFiles {
    // System modules
    rules: Option<String>,
    sheet: Option<String>,
    creature_sheet: Option<String>,
    // Adventure modules
    bestiary: Option<String>,
    pregens: Option<String>,
    items: Option<String>,
    rooms: Option<String>,
    maps: Option<String>,
    // Either kind
    tables: Option<String>,
}

/// A parsed `module.json`.
#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    #[serde(flatten)]
    summary: ModuleSummary,
    #[serde(default)]
    overview: String,
    #[serde(default)]
    files: ModuleFiles,
    #[serde(default)]
    help: Vec<HelpTopic>,
}

/// Root directory holding module packs.
pub fn modules_root() -> PathBuf {
    PathBuf::from(std::env::var("MODULES_DIR").unwrap_or_else(|_| "modules".to_string()))
}

/// Directory of one module, if the id is safe and the directory exists.
fn module_dir(id: &str) -> Result<PathBuf, String> {
    if !is_valid_module_id(id) {
        return Err(format!("Invalid module id: {id}"));
    }
    let dir = modules_root().join(id);
    if !dir.is_dir() {
        return Err(format!("No such module: {id}"));
    }
    Ok(dir)
}

/// Read and parse one JSON file from a module directory.
fn read_json<T: serde::de::DeserializeOwned>(dir: &Path, name: &str) -> Result<T, String> {
    // Names come from module.json rather than from a request, but a module
    // author should not be able to reach outside the pack either.
    if name.contains("..") || name.starts_with('/') {
        return Err(format!("Unsafe file reference: {name}"));
    }
    let path = dir.join(name);
    let text =
        fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("Cannot parse {}: {e}", path.display()))
}

/// Read a module's manifest.
fn load_manifest(id: &str) -> Result<Manifest, String> {
    let dir = module_dir(id)?;
    let mut manifest: Manifest = read_json(&dir, "module.json")?;
    // The directory name is the id, whatever the file claims.
    manifest.summary.id = id.to_string();
    Ok(manifest)
}

/// Every module that parses, sorted by name.
pub fn list_summaries() -> Vec<ModuleSummary> {
    let root = modules_root();
    let entries = match fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("Modules directory {} unreadable: {e}", root.display());
            return Vec::new();
        }
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let Some(id) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !is_valid_module_id(&id) {
            continue;
        }
        match load_manifest(&id) {
            Ok(m) => out.push(m.summary),
            Err(e) => log::warn!("Skipping module {id}: {e}"),
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Summary of one module.
pub fn load_summary(id: &str) -> Result<ModuleSummary, String> {
    Ok(load_manifest(id)?.summary)
}

/// Load a system module and the files it names.
pub fn load_system(id: &str) -> Result<SystemModule, String> {
    let dir = module_dir(id)?;
    let manifest = load_manifest(id)?;
    if manifest.summary.kind != ModuleKind::System {
        return Err(format!("Module {id} is not a system module"));
    }

    let rules_file = manifest
        .files
        .rules
        .as_deref()
        .ok_or_else(|| format!("System module {id} names no rules file"))?;
    let sheet_file = manifest
        .files
        .sheet
        .as_deref()
        .ok_or_else(|| format!("System module {id} names no sheet file"))?;

    let rules: RollRules = read_json(&dir, rules_file)?;
    let sheet: Vec<TemplateField> = read_json(&dir, sheet_file)?;
    let creature_sheet: Vec<TemplateField> = match manifest.files.creature_sheet.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let tables: Vec<RandomTable> = match manifest.files.tables.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };

    Ok(SystemModule {
        summary: manifest.summary,
        rules,
        sheet,
        creature_sheet,
        tables,
        help: manifest.help,
    })
}

/// Load an adventure or reference module and the files it names.
///
/// Both kinds carry the same shape of content. They differ in who may read
/// them: an adventure is GM material, a reference is open to the table.
pub fn load_adventure(id: &str) -> Result<AdventureModule, String> {
    let dir = module_dir(id)?;
    let manifest = load_manifest(id)?;
    if !matches!(
        manifest.summary.kind,
        ModuleKind::Adventure | ModuleKind::Reference
    ) {
        return Err(format!("Module {id} carries no adventure content"));
    }

    let bestiary: Vec<StatBlock> = match manifest.files.bestiary.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let pregens: Vec<Pregen> = match manifest.files.pregens.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let items: Vec<ItemDef> = match manifest.files.items.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let rooms: Vec<RoomEntry> = match manifest.files.rooms.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let tables: Vec<RandomTable> = match manifest.files.tables.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };
    let maps: Vec<MapDef> = match manifest.files.maps.as_deref() {
        Some(f) => read_json(&dir, f)?,
        None => Vec::new(),
    };

    Ok(AdventureModule {
        summary: manifest.summary,
        overview: manifest.overview,
        bestiary,
        pregens,
        items,
        rooms,
        tables,
        maps,
        help: manifest.help,
    })
}

/// Read a module's help page by slug, as markdown.
pub fn load_help(module_id: &str, slug: &str) -> Result<String, String> {
    let dir = module_dir(module_id)?;
    let manifest = load_manifest(module_id)?;
    let topic = manifest
        .help
        .iter()
        .find(|t| t.slug == slug)
        .ok_or_else(|| format!("No help topic {slug} in module {module_id}"))?;
    if topic.file.contains("..") || topic.file.starts_with('/') {
        return Err("Unsafe help file reference".to_string());
    }
    let path = dir.join("help").join(&topic.file);
    fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))
}

/// Every help topic contributed by every module, for the help viewer's index.
pub fn all_help_topics() -> Vec<(String, HelpTopic)> {
    let mut out = Vec::new();
    for summary in list_summaries() {
        if let Ok(manifest) = load_manifest(&summary.id) {
            for topic in manifest.help {
                out.push((summary.id.clone(), topic));
            }
        }
    }
    out
}

/// Path to a module asset, if the module ships one by that name.
///
/// `name` may name a subdirectory, such as `cards/torch.png`, but may not climb
/// out of the module's `assets/` directory.
pub fn asset_path(module_id: &str, name: &str) -> Option<PathBuf> {
    let dir = module_dir(module_id).ok()?;
    if name.is_empty() || name.contains("..") || name.starts_with('/') || name.contains('\\') {
        return None;
    }
    // Every component has to be an ordinary name: no absolutes, no symlink
    // tricks through a crafted component.
    if !name
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != ".." && !part.starts_with('.'))
    {
        return None;
    }
    let path = dir.join("assets").join(name);
    path.is_file().then_some(path)
}
