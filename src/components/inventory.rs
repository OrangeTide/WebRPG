use leptos::prelude::*;

use crate::models::{CharacterInfo, InventoryItemInfo};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Who the panel is currently showing gear for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Party,
    Character(i32),
}

/// Slots a stack of items eats: cost per item times how many there are.
fn stack_slots(item: &InventoryItemInfo) -> i32 {
    item.slots.max(0) * item.quantity.max(0)
}

#[component]
pub fn InventoryPanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let inventory = ctx.inventory;
    let send = ctx.send;
    let modules = ctx.modules;
    let session_id = ctx.session_id;
    let character_revision = ctx.character_revision;

    let (owner, set_owner) = signal(Owner::Party);
    let (new_name, set_new_name) = signal(String::new());
    let (new_slots, set_new_slots) = signal(String::new());
    let (new_bonus, set_new_bonus) = signal(String::new());

    // Characters, for the owner tabs and their carrying capacity.
    let characters = Resource::new(
        move || (session_id.get(), character_revision.get()),
        |(sid, _)| async move {
            if sid == 0 {
                return Vec::new();
            }
            crate::server::api::list_characters(sid)
                .await
                .unwrap_or_default()
        },
    );

    // Carrying capacity comes from the system module: which sheet field holds
    // it, and what going over costs. Without a module there is no limit.
    let capacity_of = move |chars: &[CharacterInfo], character_id: i32| -> Option<i32> {
        let field = modules
            .get()
            .rules
            .as_ref()
            .map(|r| r.inventory.capacity_field.clone())?;
        chars
            .iter()
            .find(|c| c.id == character_id)
            .and_then(|c| c.data.get(&field).and_then(|v| v.as_i64()))
            .map(|v| v as i32)
    };

    let visible_items = move || {
        let items = inventory.get();
        match owner.get() {
            Owner::Party => items
                .into_iter()
                .filter(|i| i.owner_character_id.is_none())
                .collect::<Vec<_>>(),
            Owner::Character(id) => items
                .into_iter()
                .filter(|i| i.owner_character_id == Some(id))
                .collect::<Vec<_>>(),
        }
    };

    let add_item = move |_| {
        let name = new_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let slots = new_slots.get().trim().parse::<i32>().unwrap_or(1).max(0);
        let bonus = new_bonus.get().trim().to_string();
        let owner_character_id = match owner.get() {
            Owner::Party => None,
            Owner::Character(id) => Some(id),
        };

        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::AddInventoryItem {
                    name: name.clone(),
                    description: String::new(),
                    quantity: 1,
                    is_party_item: owner_character_id.is_none(),
                    kind: String::new(),
                    bonus: bonus.clone(),
                    slots: Some(slots),
                    uses: None,
                    tags: Vec::new(),
                    owner_character_id,
                });
            }
        });

        set_new_name.set(String::new());
        set_new_slots.set(String::new());
        set_new_bonus.set(String::new());
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

    // Clicking a use box spends it; clicking a spent one gives it back.
    let toggle_use = move |item_id: i32, index: i32| {
        let items = inventory.get();
        let Some(item) = items.iter().find(|i| i.id == item_id) else {
            return;
        };
        let left = item.uses_left.unwrap_or(0);
        let uses_left = if index < left { index } else { index + 1 };
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::SetInventoryItemUses { item_id, uses_left });
            }
        });
    };

    let assign = move |item_id: i32, character_id: Option<i32>| {
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::AssignInventoryItem {
                    item_id,
                    character_id,
                });
            }
        });
    };

    view! {
        <div class="inventory-panel">
            <div class="inv-owners">
                <button
                    class="inv-owner-tab"
                    class:active=move || owner.get() == Owner::Party
                    on:click=move |_| set_owner.set(Owner::Party)
                >
                    "Party"
                </button>
                <Suspense fallback=|| ()>
                    {move || {
                        characters
                            .get()
                            .map(|chars| {
                                chars
                                    .into_iter()
                                    .map(|c| {
                                        let id = c.id;
                                        view! {
                                            <button
                                                class="inv-owner-tab"
                                                class:active=move || owner.get() == Owner::Character(id)
                                                on:click=move |_| set_owner.set(Owner::Character(id))
                                            >
                                                {c.name.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()
                            })
                    }}
                </Suspense>
            </div>

            // Slot meter. Party gear is not carried by anyone, so it has no limit.
            <Suspense fallback=|| ()>
                {move || {
                    let used: i32 = visible_items().iter().map(stack_slots).sum();
                    let chars = characters.get().unwrap_or_default();
                    let capacity = match owner.get() {
                        Owner::Party => None,
                        Owner::Character(id) => capacity_of(&chars, id),
                    };
                    let over = capacity.map(|cap| used > cap).unwrap_or(false);
                    let penalty_note = modules
                        .get()
                        .rules
                        .as_ref()
                        .map(|r| r.inventory.note.clone())
                        .unwrap_or_default();

                    view! {
                        <div class="inv-slots" class:over=move || over>
                            <span class="inv-slots-count">
                                {match capacity {
                                    Some(cap) => format!("{used} / {cap} slots"),
                                    None => format!("{used} slots"),
                                }}
                            </span>
                            <Show when=move || over>
                                <span class="inv-over-warning">{penalty_note.clone()}</span>
                            </Show>
                        </div>
                    }
                }}
            </Suspense>

            <div class="inventory-list">
                <For each=visible_items key=|item| (item.id, item.quantity, item.uses_left) let:item>
                    {
                        let item_id = item.id;
                        let uses_max = item.uses_max.unwrap_or(0);
                        let uses_left = item.uses_left.unwrap_or(0);
                        let slots = item.slots.max(0);
                        let chars = characters.get().unwrap_or_default();
                        let current_owner = item.owner_character_id;

                        view! {
                            <div class="inv-card" class:kind-treasure=item.kind == "treasure">
                                <div class="inv-card-head">
                                    <span class="inv-name">{item.name.clone()}</span>
                                    <span class="inv-kind">{item.kind.to_uppercase()}</span>
                                </div>

                                {(!item.tags.is_empty())
                                    .then(|| {
                                        let tags = item.tags.clone();
                                        view! {
                                            <div class="inv-tags">
                                                {tags
                                                    .into_iter()
                                                    .map(|t| view! { <span class="gear-tag">{t}</span> })
                                                    .collect_view()}
                                            </div>
                                        }
                                    })}
                                {(!item.bonus.is_empty())
                                    .then(|| view! { <div class="inv-bonus">{item.bonus.clone()}</div> })}
                                {(!item.description.is_empty())
                                    .then(|| {
                                        view! { <div class="inv-note">{item.description.clone()}</div> }
                                    })}

                                <div class="inv-card-foot">
                                    <span class="inv-pips" title="Inventory slots">
                                        {(0..slots.min(8))
                                            .map(|_| view! { <i class="inv-pip"></i> })
                                            .collect_view()}
                                        {(slots > 8).then(|| view! { <span>{format!("x{slots}")}</span> })}
                                    </span>

                                    <Show when=move || { uses_max > 0 }>
                                        <span class="inv-uses" title="Uses left">
                                            {(0..uses_max)
                                                .map(|i| {
                                                    let spent = i >= uses_left;
                                                    view! {
                                                        <button
                                                            class="inv-use"
                                                            class:spent=spent
                                                            on:click=move |_| toggle_use(item_id, i)
                                                        ></button>
                                                    }
                                                })
                                                .collect_view()}
                                        </span>
                                    </Show>

                                    <span class="inv-qty">
                                        <button on:click=move |_| change_qty(item_id, -1)>"-"</button>
                                        {item.quantity}
                                        <button on:click=move |_| change_qty(item_id, 1)>"+"</button>
                                    </span>

                                    <select
                                        class="inv-assign"
                                        title="Who carries it"
                                        on:change=move |ev| {
                                            let v = event_target_value(&ev);
                                            assign(item_id, v.parse::<i32>().ok());
                                        }
                                    >
                                        <option value="" selected=current_owner.is_none()>"Party"</option>
                                        {chars
                                            .iter()
                                            .map(|c| {
                                                view! {
                                                    <option
                                                        value=c.id.to_string()
                                                        selected=current_owner == Some(c.id)
                                                    >
                                                        {c.name.clone()}
                                                    </option>
                                                }
                                            })
                                            .collect_view()}
                                    </select>

                                    <button class="inv-remove" on:click=move |_| remove_item(item_id)>
                                        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                            <polyline points="3 6 5 6 21 6"/>
                                            <path d="M19 6l-1 14H6L5 6"/>
                                            <path d="M10 11v6"/><path d="M14 11v6"/>
                                            <path d="M9 6V4h6v2"/>
                                        </svg>
                                    </button>
                                </div>
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
                    placeholder="Slots"
                    prop:value=new_slots
                    on:input=move |ev| set_new_slots.set(event_target_value(&ev))
                    style="width: 60px;"
                />
                <input
                    type="text"
                    placeholder="What it gives you"
                    prop:value=new_bonus
                    on:input=move |ev| set_new_bonus.set(event_target_value(&ev))
                />
                <button on:click=add_item>"Add"</button>
            </div>
        </div>
    }
}
