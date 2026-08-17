//! The check roller: dice + ability + the bonuses the player picked, against a
//! Difficulty Score they may or may not know.
//!
//! Which items help is a judgement call in these systems, not a lookup, so the
//! player ticks them. The panel only appears when the session runs a system
//! module that defines a roll model.

use leptos::prelude::*;

use crate::models::InventoryItemInfo;
use crate::modules::RollRules;
use crate::pages::game::GameContext;
use crate::ws::messages::{ClientMessage, RollBonus};

/// A character's score in one ability, read off its sheet data.
fn ability_mod(data: &serde_json::Value, ability_name: &str) -> i32 {
    data.get(ability_name).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

/// Slots a stack of items eats.
fn stack_slots(item: &InventoryItemInfo) -> i32 {
    item.slots.max(0) * item.quantity.max(0)
}

/// The penalty for carrying more than you can, when it applies to this ability.
///
/// The roller applies it rather than leaving the player to remember, since the
/// whole point of the rule is that loot should hurt.
fn over_capacity_penalty(
    rules: &RollRules,
    data: &serde_json::Value,
    carried: &[InventoryItemInfo],
    ability_name: &str,
) -> Option<RollBonus> {
    let capacity = data
        .get(&rules.inventory.capacity_field)
        .and_then(|v| v.as_i64())? as i32;
    let used: i32 = carried.iter().map(stack_slots).sum();
    if used <= capacity {
        return None;
    }
    if !rules
        .inventory
        .penalty_applies_to
        .iter()
        .any(|a| a == ability_name)
    {
        return None;
    }
    Some(RollBonus {
        label: "over capacity".to_string(),
        value: rules.inventory.over_capacity_penalty,
    })
}

#[component]
pub fn CheckRoller(character_id: i32, data: serde_json::Value) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let send = ctx.send;
    let modules = ctx.modules;
    let inventory = ctx.inventory;

    let data = StoredValue::new(data);
    let (label, set_label) = signal(String::new());
    let (ability, set_ability) = signal(String::new());
    let (ds, set_ds) = signal(String::new());
    let (dangerous, set_dangerous) = signal(false);
    // Inventory item ids the player says are helping this roll.
    let picked = RwSignal::new(Vec::<i32>::new());

    let rules = move || modules.get().rules;

    // Default to the first ability the system defines.
    Effect::new(move |_| {
        if ability.get().is_empty()
            && let Some(first) = rules().and_then(|r| r.abilities.first().cloned())
        {
            set_ability.set(first.name);
        }
    });

    let carried = move || {
        inventory
            .get()
            .into_iter()
            .filter(|i| i.owner_character_id == Some(character_id))
            .collect::<Vec<_>>()
    };

    let roll = move |_| {
        let Some(rules) = rules() else { return };
        let ability_name = ability.get();
        let ability_label = rules
            .abilities
            .iter()
            .find(|a| a.name == ability_name)
            .map(|a| a.label.clone())
            .unwrap_or_default();

        let items = carried();
        let mut bonuses: Vec<RollBonus> = picked
            .get()
            .iter()
            .filter_map(|id| items.iter().find(|i| i.id == *id))
            .map(|i| RollBonus {
                label: i.name.clone(),
                value: rules.item_bonus,
            })
            .collect();

        let modifier = data.with_value(|d| ability_mod(d, &ability_name));
        if let Some(penalty) =
            data.with_value(|d| over_capacity_penalty(&rules, d, &items, &ability_name))
        {
            bonuses.push(penalty);
        }

        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::RollCheck {
                    label: label.get().trim().to_string(),
                    ability: ability_label.clone(),
                    ability_mod: modifier,
                    bonuses: bonuses.clone(),
                    ds: ds.get().trim().parse::<i32>().ok(),
                    dangerous: dangerous.get(),
                    dice: rules.dice.clone(),
                });
            }
        });

        // The label is per-action; the ability and the ticked items usually
        // carry over to the next roll.
        set_label.set(String::new());
    };

    view! {
        <div class="check-roller">
            {move || {
                let Some(rules) = rules() else {
                    return ().into_any();
                };

                view! {
                    <div class="check-head">
                        <h5>"Roll a check"</h5>
                        <span class="check-summary">{rules.summary.clone()}</span>
                    </div>

                    <input
                        class="check-label"
                        type="text"
                        placeholder="What are you attempting?"
                        prop:value=label
                        on:input=move |ev| set_label.set(event_target_value(&ev))
                    />

                    <div class="check-abilities">
                        {rules
                            .abilities
                            .iter()
                            .map(|a| {
                                let name = a.name.clone();
                                let active_name = name.clone();
                                let modifier = data.with_value(|d| ability_mod(d, &name));
                                view! {
                                    <button
                                        class="check-ability"
                                        class:active=move || ability.get() == active_name
                                        title=a.help.clone()
                                        on:click=move |_| set_ability.set(name.clone())
                                    >
                                        <span class="check-ability-name">{a.label.clone()}</span>
                                        <span class="check-ability-mod">{format!("{modifier:+}")}</span>
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>

                    <div class="check-items">
                        {move || {
                            let items = carried()
                                .into_iter()
                                .filter(|i| !i.bonus.is_empty())
                                .collect::<Vec<_>>();
                            if items.is_empty() {
                                return view! {
                                    <span class="check-no-items">
                                        "No carried item says what it helps with."
                                    </span>
                                }
                                    .into_any();
                            }
                            items
                                .into_iter()
                                .map(|item| {
                                    let id = item.id;
                                    view! {
                                        <label class="check-item" title=item.bonus.clone()>
                                            <input
                                                type="checkbox"
                                                prop:checked=move || picked.get().contains(&id)
                                                on:change=move |_| {
                                                    picked
                                                        .update(|p| {
                                                            if let Some(pos) = p.iter().position(|x| *x == id) {
                                                                p.remove(pos);
                                                            } else {
                                                                p.push(id);
                                                            }
                                                        });
                                                }
                                            />
                                            <span>{item.name.clone()}</span>
                                        </label>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>

                    <div class="check-ds">
                        <span class="check-ds-label">"DS"</span>
                        {rules
                            .difficulties
                            .iter()
                            .map(|step| {
                                let value = step.value.to_string();
                                let active_value = value.clone();
                                view! {
                                    <button
                                        class="check-ds-preset"
                                        class:active=move || ds.get() == active_value
                                        on:click=move |_| set_ds.set(value.clone())
                                    >
                                        {format!("{} {}", step.label, step.value)}
                                    </button>
                                }
                            })
                            .collect_view()}
                        <input
                            class="check-ds-input"
                            type="number"
                            placeholder="unknown"
                            prop:value=ds
                            on:input=move |ev| set_ds.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="check-actions">
                        <label
                            class="check-dangerous"
                            title="The gap between your total and the DS is damage"
                        >
                            <input
                                type="checkbox"
                                prop:checked=dangerous
                                on:change=move |ev| set_dangerous.set(event_target_checked(&ev))
                            />
                            <span>"Dangerous"</span>
                        </label>
                        <button class="btn-roll-check" on:click=roll>
                            {format!("Roll {}", rules.dice)}
                        </button>
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}
