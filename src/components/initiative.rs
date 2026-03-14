use leptos::prelude::*;

use crate::models::InitiativeEntryInfo;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

#[component]
pub fn InitiativeTracker() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let initiative = ctx.initiative;
    let send = ctx.send;
    let initiative_locked = ctx.initiative_locked;

    let show_add_form = RwSignal::new(false);
    let (new_label, set_new_label) = signal(String::new());
    let (new_value, set_new_value) = signal(String::new());

    let do_add_entry = move || {
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
            portrait_url: None,
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
        show_add_form.set(false);
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

    let toggle_lock = move |_| {
        let new_locked = !initiative_locked.get();
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::SetInitiativeLock { locked: new_locked });
            }
        });
    };

    view! {
        <div class="initiative-tracker">
            <div class="panel-header">
                <h3>"Initiative"</h3>
                <div class="initiative-header-btns">
                    <button
                        class="init-lock-btn"
                        class:locked=move || initiative_locked.get()
                        on:click=toggle_lock
                        title=move || if initiative_locked.get() { "Unlock initiative (allow character rolls)" } else { "Lock initiative (prevent character rolls)" }
                    >
                        {move || if initiative_locked.get() { "Locked" } else { "Unlocked" }}
                    </button>
                    <button
                        class="btn-add"
                        title="Add entry manually"
                        on:click=move |_| show_add_form.set(!show_add_form.get())
                    >"+"</button>
                </div>
            </div>

            // Inline add form (shown when + is clicked)
            {move || show_add_form.get().then(|| view! {
                <div class="init-add-form">
                    <input
                        type="text"
                        placeholder="Name"
                        prop:value=new_label
                        on:input=move |ev| set_new_label.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                do_add_entry();
                            }
                        }
                    />
                    <input
                        type="number"
                        placeholder="Init"
                        prop:value=new_value
                        on:input=move |ev| set_new_value.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                do_add_entry();
                            }
                        }
                        style="width: 55px;"
                    />
                    <button on:click=move |_| do_add_entry()>"Add"</button>
                </div>
            })}

            <div class="initiative-list">
                <For
                    each=move || {
                        initiative.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|(_, e)| (e.id, e.label.clone(), e.initiative_value as i32)
                    let:item
                >
                    {
                        let (idx, entry) = item;
                        let portrait = entry.portrait_url.clone();
                        view! {
                            <div class=move || {
                                if entry.is_current_turn {
                                    "init-entry current-turn"
                                } else {
                                    "init-entry"
                                }
                            }>
                                <span class="init-value">{entry.initiative_value as i32}</span>
                                <div class="init-portrait">
                                    {if let Some(url) = portrait.clone() {
                                        view! { <img src=url alt="icon" class="init-portrait-img" /> }.into_any()
                                    } else {
                                        view! { <div class="init-portrait-placeholder"></div> }.into_any()
                                    }}
                                </div>
                                <span class="init-label">{entry.label.clone()}</span>
                                <button
                                    class="init-remove"
                                    on:click=move |_| remove_entry(idx)
                                >"x"</button>
                            </div>
                        }
                    }
                </For>
            </div>

            {move || (!initiative.get().is_empty()).then(|| view! {
                <div class="initiative-controls">
                    <button class="btn-next-turn" on:click=advance_turn>"Next Turn"</button>
                </div>
            })}
        </div>
    }
}
