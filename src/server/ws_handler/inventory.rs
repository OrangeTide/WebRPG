use crate::models::InventoryItemInfo;

pub fn add_inventory_item(
    session_id: i32,
    name: &str,
    description: &str,
    quantity: i32,
    is_party_item: bool,
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
        })
        .collect()
}
