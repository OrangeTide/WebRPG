use crate::models::InventoryItemInfo;

/// The card-shaped fields an item carries beyond its name and count.
///
/// Modules supply these from their item cards; hand-added items fall back to
/// one slot of ordinary gear.
#[derive(Debug, Clone, Default)]
pub struct ItemCard<'a> {
    pub kind: &'a str,
    pub bonus: &'a str,
    pub slots: i32,
    pub uses: Option<i32>,
    pub owner_character_id: Option<i32>,
    /// Comma-separated gear tags, which is how the column stores them.
    pub tags: &'a str,
}

pub fn add_inventory_item(
    session_id: i32,
    name: &str,
    description: &str,
    quantity: i32,
    is_party_item: bool,
    card: ItemCard<'_>,
) -> Result<Vec<InventoryItemInfo>, String> {
    use crate::db;
    use crate::models::db_models::NewInventoryItem;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let new_item = NewInventoryItem {
        session_id,
        name,
        description,
        quantity,
        is_party_item,
        owner_character_id: card.owner_character_id,
        slots: card.slots.max(0),
        kind: card.kind,
        bonus: card.bonus,
        uses_max: card.uses,
        uses_left: card.uses,
        tags: card.tags,
    };

    diesel::insert_into(inventory_items::table)
        .values(&new_item)
        .execute(conn)
        .map_err(|e| format!("Failed to add inventory item: {e}"))?;

    Ok(load_inventory(session_id))
}

pub fn remove_inventory_item(item_id: i32) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::delete(inventory_items::table.find(item_id)).execute(conn);
}

pub fn update_inventory_item(
    item_id: i32,
    name: Option<&str>,
    description: Option<&str>,
    quantity: Option<i32>,
) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    if let Some(name) = name {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::name.eq(name))
            .execute(conn);
    }
    if let Some(description) = description {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::description.eq(description))
            .execute(conn);
    }
    if let Some(quantity) = quantity {
        let _ = diesel::update(inventory_items::table.find(item_id))
            .set(inventory_items::quantity.eq(quantity))
            .execute(conn);
    }
}

/// Spend or restore uses on a consumable, clamped to what the item has.
pub fn set_inventory_item_uses(item_id: i32, uses_left: i32) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let uses_max: Option<i32> = inventory_items::table
        .find(item_id)
        .select(inventory_items::uses_max)
        .first(conn)
        .optional()
        .ok()
        .flatten()
        .flatten();

    // An item with no use boxes has nothing to tick.
    let Some(uses_max) = uses_max else {
        return;
    };

    let clamped = uses_left.clamp(0, uses_max);
    let _ = diesel::update(inventory_items::table.find(item_id))
        .set(inventory_items::uses_left.eq(Some(clamped)))
        .execute(conn);
}

/// Hand an item to a character, or back to the party with `None`.
pub fn assign_inventory_item(item_id: i32, character_id: Option<i32>) {
    use crate::db;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();
    let _ = diesel::update(inventory_items::table.find(item_id))
        .set((
            inventory_items::owner_character_id.eq(character_id),
            inventory_items::is_party_item.eq(character_id.is_none()),
        ))
        .execute(conn);
}

pub fn load_inventory(session_id: i32) -> Vec<InventoryItemInfo> {
    use crate::db;
    use crate::models::db_models::InventoryItem;
    use crate::schema::inventory_items;
    use diesel::prelude::*;

    let conn = &mut db::get_conn();

    let items: Vec<InventoryItem> = inventory_items::table
        .filter(inventory_items::session_id.eq(session_id))
        .select(InventoryItem::as_select())
        .load(conn)
        .unwrap_or_default();

    items
        .into_iter()
        .map(|item| InventoryItemInfo {
            id: item.id,
            name: item.name,
            description: item.description,
            quantity: item.quantity,
            is_party_item: item.is_party_item,
            owner_character_id: item.owner_character_id,
            slots: item.slots,
            kind: item.kind,
            bonus: item.bonus,
            uses_max: item.uses_max,
            uses_left: item.uses_left,
            tags: item
                .tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect(),
        })
        .collect()
}
