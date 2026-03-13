use leptos::prelude::*;

use crate::models::InitiativeEntryInfo;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

#[component]
pub fn InitiativeTracker() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let initiative = ctx.initiative;
    let send = ctx.send;

    let (new_label, set_new_label) = signal(String::new());
    let (new_value, set_new_value) = signal(String::new());

    let add_entry = move |_| {
        let label = new_label.get().trim().to_string();
        let value: f32 = new_value.get().trim().parse().unwrap_or(0.0);
        if label.is_empty() {
            return;
        }

        let mut entries = initiative.get();
        entries.push(InitiativeEntryInfo {
            id: 0,
            label,
            initiative_value: value,
            is_current_turn: entries.is_empty(),
        });

        // Sort descending by initiative value
        entries.sort_by(|a, b| b.initiative_value.partial_cmp(&a.initiative_value).unwrap());

        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::UpdateInitiative {
                    entries: entries.clone(),
                });
            }
        });

        set_new_label.set(String::new());
        set_new_value.set(String::new());
    };

    let advance_turn = move |_| {
        let mut entries = initiative.get();
        if entries.is_empty() {
            return;
        }
        let current_idx = entries.iter().position(|e| e.is_current_turn).unwrap_or(0);
        entries[current_idx].is_current_turn = false;
        let next_idx = (current_idx + 1) % entries.len();
        entries[next_idx].is_current_turn = true;

        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::UpdateInitiative {
                    entries: entries.clone(),
                });
            }
        });
    };

    let remove_entry = move |idx: usize| {
        let mut entries = initiative.get();
        if idx < entries.len() {
            let was_current = entries[idx].is_current_turn;
            entries.remove(idx);
            if was_current && !entries.is_empty() {
                let new_idx = idx.min(entries.len() - 1);
                entries[new_idx].is_current_turn = true;
            }
            send.with_value(|f| {
                if let Some(f) = f {
                    f(ClientMessage::UpdateInitiative {
                        entries: entries.clone(),
                    });
                }
            });
        }
    };

    view! {
        <div class="initiative-tracker">
            <h3>"Initiative"</h3>
            <div class="initiative-list">
                <For
                    each=move || {
                        initiative.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|(_, e)| (e.id, e.label.clone())
                    let:item
                >
                    {
                        let (idx, entry) = item;
                        view! {
                            <div class=move || {
                                if entry.is_current_turn {
                                    "init-entry current-turn"
                                } else {
                                    "init-entry"
                                }
                            }>
                                <span class="init-value">{entry.initiative_value}</span>
                                <span class="init-label">{entry.label.clone()}</span>
                                <button
                                    class="init-remove"
                                    on:click=move |_| remove_entry(idx)
                                >"×"</button>
                            </div>
                        }
                    }
                </For>
            </div>
            <div class="initiative-controls">
                <input
                    type="text"
                    placeholder="Name"
                    prop:value=new_label
                    on:input=move |ev| set_new_label.set(event_target_value(&ev))
                />
                <input
                    type="number"
                    placeholder="Init"
                    prop:value=new_value
                    on:input=move |ev| set_new_value.set(event_target_value(&ev))
                    style="width: 50px;"
                />
                <button on:click=add_entry>"Add"</button>
                <button on:click=advance_turn>"Next Turn"</button>
            </div>
        </div>
    }
}
