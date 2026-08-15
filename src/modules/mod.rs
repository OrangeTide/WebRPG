//! Game modules: RPG systems and adventures loaded from disk as data.
//!
//! A module is a directory under `MODULES_DIR` (default `modules/`) holding a
//! `module.json` plus the files it names. System modules describe how
//! characters and rolls work; adventure modules carry content to run and
//! declare the system they need.
//!
//! The types here compile for both targets so the client can render module
//! content directly. Reading modules off disk lives in [`loader`], which is
//! server-only.

use serde::{Deserialize, Serialize};

use crate::models::TemplateField;

#[cfg(feature = "ssr")]
pub mod loader;

/// What a module provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleKind {
    /// Rules: sheet schema, creature schema, roll model.
    System,
    /// Content: bestiary, pregens, items, rooms, tables, maps.
    Adventure,
    /// Lookup material: tables and item cards, with no secrets in it.
    ///
    /// Reference modules are never installed and are visible to every member
    /// of a session, so anything a player should not read does not belong in
    /// one.
    Reference,
}

/// A link the module browser offers, such as where to buy the rulebook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLink {
    pub label: String,
    pub url: String,
}

/// The `module.json` header, and what the module browser lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSummary {
    /// Directory name, and the id used everywhere else.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    pub kind: ModuleKind,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: String,
    /// For adventures: the id of the system module this content is written for.
    #[serde(default)]
    pub requires: Option<String>,
    #[serde(default)]
    pub links: Vec<ModuleLink>,
}

// ===== System modules =====

/// One rollable ability, such as Brute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    /// Field name on the character sheet.
    pub name: String,
    pub label: String,
    #[serde(default)]
    pub help: String,
}

/// A named rung on the difficulty ladder, offered as a preset in the roller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyStep {
    pub label: String,
    pub value: i32,
}

/// How carried items count against a character's capacity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRules {
    /// Sheet field holding the capacity, e.g. "inventory".
    pub capacity_field: String,
    /// Slots an item costs when it does not say otherwise.
    #[serde(default = "one")]
    pub default_slots: i32,
    /// Modifier applied to each affected ability while over capacity.
    #[serde(default)]
    pub over_capacity_penalty: i32,
    /// Ability names the penalty applies to.
    #[serde(default)]
    pub penalty_applies_to: Vec<String>,
    #[serde(default)]
    pub note: String,
}

fn one() -> i32 {
    1
}

/// The roll model: what to roll, what can be added, and how to read the result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollRules {
    /// Dice rolled before modifiers, in `NdN` form.
    pub dice: String,
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub difficulties: Vec<DifficultyStep>,
    /// Bonus one helpful item contributes.
    #[serde(default = "one")]
    pub item_bonus: i32,
    /// Whether the gap between the total and the DS is damage, either way.
    #[serde(default)]
    pub margin_is_damage: bool,
    /// Whether a creature's difficulty score doubles as its hit points.
    #[serde(default)]
    pub monster_ds_is_hp: bool,
    pub inventory: InventoryRules,
    /// One-line summary shown above the roller.
    #[serde(default)]
    pub summary: String,
}

/// A system module and everything it defines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemModule {
    pub summary: ModuleSummary,
    pub rules: RollRules,
    /// Character sheet schema, seeded into `rpg_templates` on install.
    pub sheet: Vec<TemplateField>,
    /// Creature stat block schema.
    #[serde(default)]
    pub creature_sheet: Vec<TemplateField>,
    /// Character-creation tables and any other system-level tables.
    #[serde(default)]
    pub tables: Vec<RandomTable>,
    /// Help topic slugs this module contributes.
    #[serde(default)]
    pub help: Vec<HelpTopic>,
}

// ===== Adventure modules =====

/// A creature as this system states it. In a DS-is-HP system `hp` is left
/// empty and the difficulty score serves as both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatBlock {
    pub id: String,
    pub name: String,
    /// Difficulty score to beat it. Zero marks something that is never fought.
    pub ds: i32,
    /// Hit points, when the system tracks them separately from the DS.
    #[serde(default)]
    pub hp: Option<i32>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// One item a character carries, in the language of the roll.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub name: String,
    /// gear, weapon, armour, treasure, or consumable.
    #[serde(default)]
    pub kind: String,
    #[serde(default = "one")]
    pub slots: i32,
    #[serde(default)]
    pub bonus: String,
    #[serde(default)]
    pub note: String,
    /// Tick boxes for something with a limited number of uses.
    #[serde(default)]
    pub uses: Option<i32>,
    /// Card art, as a path under the module's `assets/`. Prefix it with
    /// `other-module:` to use art another module ships, which is how an
    /// adventure borrows the system module's card deck.
    #[serde(default)]
    pub art: String,
}

/// URL for a module asset reference, resolving `module:path` against the
/// module the reference came from.
///
/// Returns `None` for an empty reference or an id that is not a safe module
/// name, so a bad reference renders nothing rather than a broken request.
pub fn asset_url(default_module: &str, reference: &str) -> Option<String> {
    if reference.is_empty() {
        return None;
    }
    let (module, path) = match reference.split_once(':') {
        Some((m, p)) => (m, p),
        None => (default_module, reference),
    };
    if !is_valid_module_id(module) || path.is_empty() || path.contains("..") {
        return None;
    }
    Some(format!("/api/modules/{module}/assets/{path}"))
}

/// An item as a pregenerated character starts with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PregenItem {
    pub name: String,
    #[serde(default)]
    pub bonus: String,
}

/// A ready-to-play character the GM can hand to a player.
///
/// `sheet` carries the field values verbatim, keyed by the sheet field names
/// the system module defines, so nothing here has to know what a given system
/// calls its abilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pregen {
    pub id: String,
    pub name: String,
    /// One line for the picker, such as the role and a tagline.
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub sheet: serde_json::Value,
    #[serde(default)]
    pub items: Vec<PregenItem>,
}

/// One location in the adventure. `card` is player-facing and safe to show;
/// everything else is GM material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEntry {
    pub number: i32,
    #[serde(default)]
    pub title: String,
    /// What the players are told on entering, dealt as a card.
    #[serde(default)]
    pub card: String,
    /// Boxed text to read or paraphrase.
    #[serde(default)]
    pub read_aloud: String,
    /// GM notes: triggers, treasure, connections.
    #[serde(default)]
    pub gm: String,
    #[serde(default)]
    pub exits: String,
}

/// A row of a random or reference table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    /// Roll or key this row answers to. Reference tables leave it as a label.
    #[serde(default)]
    pub key: String,
    pub text: String,
}

/// A table the GM can read or roll on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomTable {
    pub id: String,
    pub name: String,
    /// Dice to roll, e.g. "d6". Empty means the table is reference only.
    #[serde(default)]
    pub die: String,
    #[serde(default)]
    pub description: String,
    pub entries: Vec<TableEntry>,
}

/// A map the module wants created in the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapDef {
    pub name: String,
    pub width: i32,
    pub height: i32,
    #[serde(default = "default_cell_size")]
    pub cell_size: i32,
    /// File under the module's `assets/`, ingested into media if present.
    #[serde(default)]
    pub asset: Option<String>,
    /// Where to obtain the art when it is not shipped with the module.
    #[serde(default)]
    pub asset_source: String,
    #[serde(default)]
    pub notes: String,
}

fn default_cell_size() -> i32 {
    50
}

/// A markdown help page a module contributes to the online help.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpTopic {
    /// Slug used in `help:` links.
    pub slug: String,
    pub title: String,
    /// File under the module's `help/`.
    pub file: String,
}

/// An adventure module and everything it carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdventureModule {
    pub summary: ModuleSummary,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub bestiary: Vec<StatBlock>,
    #[serde(default)]
    pub pregens: Vec<Pregen>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub rooms: Vec<RoomEntry>,
    #[serde(default)]
    pub tables: Vec<RandomTable>,
    #[serde(default)]
    pub maps: Vec<MapDef>,
    #[serde(default)]
    pub help: Vec<HelpTopic>,
}

/// What installing a module into a session did.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstallReport {
    pub module_id: String,
    pub module_name: String,
    /// Template the system module was seeded into, if any.
    pub template_id: Option<i32>,
    pub creatures_added: usize,
    pub maps_added: usize,
    /// Maps that were already present and have now been given the module's art.
    pub maps_art_attached: usize,
    /// Maps created without art because the asset was not present.
    pub maps_missing_art: Vec<String>,
    /// Anything the GM should know, such as a missing asset directory.
    pub warnings: Vec<String>,
}

/// Whether a module id is safe to use as a directory name.
///
/// Ids index straight into the modules directory, so anything that could climb
/// out of it or name a hidden file is rejected.
pub fn is_valid_module_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::{asset_url, is_valid_module_id};

    #[test]
    fn asset_references_resolve_within_modules() {
        assert_eq!(
            asset_url("sky-blind-spire", "cards/torch.png").as_deref(),
            Some("/api/modules/sky-blind-spire/assets/cards/torch.png")
        );
        // An adventure borrowing the system module's deck.
        assert_eq!(
            asset_url("sky-blind-spire", "tunnel-goons:cards/torch.png").as_deref(),
            Some("/api/modules/tunnel-goons/assets/cards/torch.png")
        );
        assert_eq!(asset_url("tunnel-goons", ""), None);
        assert_eq!(asset_url("tunnel-goons", "../../etc/passwd"), None);
        assert_eq!(asset_url("tunnel-goons", "..:cards/torch.png"), None);
    }

    #[test]
    fn module_ids_reject_traversal() {
        assert!(is_valid_module_id("tunnel-goons"));
        assert!(is_valid_module_id("sky_blind_spire2"));
        assert!(!is_valid_module_id(""));
        assert!(!is_valid_module_id(".."));
        assert!(!is_valid_module_id(".hidden"));
        assert!(!is_valid_module_id("a/b"));
        assert!(!is_valid_module_id("../etc"));
        assert!(!is_valid_module_id("mod ule"));
        assert!(!is_valid_module_id(&"x".repeat(65)));
    }
}
