use leptos::prelude::*;

use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

#[component]
pub fn InventoryPanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let inventory = ctx.inventory;
    let send = ctx.send;

    let (new_name, set_new_name) = signal(String::new());
    let (new_qty, set_new_qty) = signal(String::new());

    let add_item = move |_| {
        let name = new_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let quantity: i32 = new_qty.get().trim().parse().unwrap_or(1);

        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::AddInventoryItem {
                    name,
                    description: String::new(),
                    quantity,
                    is_party_item: true,
                });
            }
        });

        set_new_name.set(String::new());
        set_new_qty.set(String::new());
    };

    let remove_item = move |item_id: i32| {
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::RemoveInventoryItem { item_id });
            }
        });
    };

    let change_qty = move |item_id: i32, delta: i32| {
        let items = inventory.get();
        if let Some(item) = items.iter().find(|i| i.id == item_id) {
            let new_quantity = (item.quantity + delta).max(0);
            send.with_value(|f| {
                if let Some(f) = f {
                    f(ClientMessage::UpdateInventoryItem {
                        item_id,
                        name: None,
                        description: None,
                        quantity: Some(new_quantity),
                    });
                }
            });
        }
    };

    view! {
        <div class="inventory-panel">
            <h3>"Inventory"</h3>
            <div class="inventory-list">
                <For
                    each=move || inventory.get()
                    key=|item| item.id
                    let:item
                >
                    {
                        let item_id = item.id;
                        view! {
                            <div class="inv-item">
                                <span class="inv-name">{item.name.clone()}</span>
                                <span class="inv-qty">
                                    <button on:click=move |_| change_qty(item_id, -1)>"-"</button>
                                    {item.quantity}
                                    <button on:click=move |_| change_qty(item_id, 1)>"+"</button>
                                </span>
                                <button
                                    class="inv-remove"
                                    on:click=move |_| remove_item(item_id)
                                >
                                    <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                        <polyline points="3 6 5 6 21 6"/>
                                        <path d="M19 6l-1 14H6L5 6"/>
                                        <path d="M10 11v6"/><path d="M14 11v6"/>
                                        <path d="M9 6V4h6v2"/>
                                    </svg>
                                </button>
                            </div>
                        }
                    }
                </For>
            </div>
            <div class="inventory-add">
                <input
                    type="text"
                    placeholder="Item name"
                    prop:value=new_name
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                />
                <input
                    type="number"
                    placeholder="Qty"
                    prop:value=new_qty
                    on:input=move |ev| set_new_qty.set(event_target_value(&ev))
                    style="width: 50px;"
                />
                <button on:click=add_item>"Add"</button>
            </div>
        </div>
    }
}
