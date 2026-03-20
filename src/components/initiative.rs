use leptos::prelude::*;

use crate::models::InitiativeEntryInfo;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Compute the insertion index from a Y coordinate within the initiative list.
/// Iterates `.init-entry` children and returns the gap index (0..=entry_count)
/// where the dragged item should land.
#[cfg(feature = "hydrate")]
fn insertion_index_from_y(list_el: &web_sys::HtmlElement, y: f64) -> usize {
    let children = list_el.children();
    let total = children.length();
    let mut entry_idx = 0usize;
    for i in 0..total {
        if let Some(child) = children.item(i) {
            if child.class_list().contains("init-entry") {
                let rect = child.get_bounding_client_rect();
                let mid = rect.top() + rect.height() / 2.0;
                if y < mid {
                    return entry_idx;
                }
                entry_idx += 1;
            }
        }
    }
    entry_idx
}

/// Apply a reorder: move entry at `from` to insertion gap `to` (0..=len).
/// Returns the adjusted insertion index after removal, or None if it's a no-op.
pub(crate) fn reorder_index(from: usize, to: usize, len: usize) -> Option<usize> {
    if from >= len || to > len {
        return None;
    }
    // Dropping right before or right after the source position is a no-op
    if to == from || to == from + 1 {
        return None;
    }
    Some(if to > from { to - 1 } else { to })
}

#[component]
pub fn InitiativeTracker() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let initiative = ctx.initiative;
    let send = ctx.send;
    let initiative_locked = ctx.initiative_locked;

    let show_add_form = RwSignal::new(false);
    let (new_label, set_new_label) = signal(String::new());
    let (new_value, set_new_value) = signal(String::new());

    // Drag state: which entry index is being dragged, and which gap to insert at
    let drag_from = RwSignal::new(Option::<usize>::None);
    let drag_insert = RwSignal::new(Option::<usize>::None);
    let list_ref = NodeRef::<leptos::html::Div>::new();

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

        // Stable sort descending by initiative value
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

    let set_current_turn = move |idx: usize| {
        let mut entries = initiative.get();
        if idx < entries.len() {
            for e in entries.iter_mut() {
                e.is_current_turn = false;
            }
            entries[idx].is_current_turn = true;
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

    let do_drop = move || {
        if let (Some(from), Some(to)) = (drag_from.get(), drag_insert.get()) {
            drag_from.set(None);
            drag_insert.set(None);
            let mut entries = initiative.get();
            if let Some(target) = reorder_index(from, to, entries.len()) {
                let item = entries.remove(from);
                entries.insert(target, item);
                send.with_value(|f| {
                    if let Some(f) = f {
                        f(ClientMessage::UpdateInitiative {
                            entries: entries.clone(),
                        });
                    }
                });
            }
        } else {
            drag_from.set(None);
            drag_insert.set(None);
        }
    };

    // Determine which entry should show a drop-above or drop-below border.
    // Returns (entry_index, is_above) or None.
    let drop_highlight = move || -> Option<(usize, bool)> {
        let from = drag_from.get()?;
        let to = drag_insert.get()?;
        let len = initiative.get().len();
        // Check it's not a no-op
        reorder_index(from, to, len)?;
        if to == len {
            // Dropping after the last entry: highlight last entry below
            Some((len - 1, false))
        } else {
            // Dropping before entry `to`: highlight entry `to` above
            Some((to, true))
        }
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
                        data-tooltip="Add entry manually"
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

            <div
                class="initiative-list"
                node_ref=list_ref
                on:dragover=move |ev| {
                    ev.prevent_default();
                    #[cfg(feature = "hydrate")]
                    {
                        if let Some(el) = list_ref.get() {
                            let html_el: &web_sys::HtmlElement = el.as_ref();
                            let insert = insertion_index_from_y(html_el, ev.client_y() as f64);
                            drag_insert.set(Some(insert));
                        }
                    }
                }
                on:drop=move |ev| {
                    ev.prevent_default();
                    do_drop();
                }
            >
                <For
                    each=move || {
                        initiative.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|(idx, e)| (*idx, e.label.clone(), e.initiative_value.to_bits(), e.is_current_turn)
                    let:item
                >
                    {
                        let (idx, entry) = item;
                        let portrait = entry.portrait_url.clone();
                        view! {
                            <div
                                class=move || {
                                    let mut cls = String::from("init-entry");
                                    if entry.is_current_turn {
                                        cls.push_str(" current-turn");
                                    }
                                    if drag_from.get() == Some(idx) {
                                        cls.push_str(" dragging");
                                    }
                                    if let Some((hi, above)) = drop_highlight() {
                                        if hi == idx {
                                            if above {
                                                cls.push_str(" drop-above");
                                            } else {
                                                cls.push_str(" drop-below");
                                            }
                                        }
                                    }
                                    cls
                                }
                                draggable="true"
                                on:dragstart=move |_| {
                                    drag_from.set(Some(idx));
                                }
                                on:dragend=move |_| {
                                    drag_from.set(None);
                                    drag_insert.set(None);
                                }
                            >
                                <span class="init-grab" data-tooltip="Drag to reorder">"⠿"</span>
                                <span class="init-value">{entry.initiative_value as i32}</span>
                                <div class="init-portrait">
                                    {if let Some(url) = portrait.clone() {
                                        view! { <img src=url alt="icon" class="init-portrait-img" /> }.into_any()
                                    } else {
                                        view! { <div class="init-portrait-placeholder"></div> }.into_any()
                                    }}
                                </div>
                                <span
                                    class="init-label"
                                    title="Click to set current turn"
                                    on:click=move |_| set_current_turn(idx)
                                >{entry.label.clone()}</span>
                                <button
                                    class="init-find"
                                    data-tooltip="Find on Map"
                                    on:click={
                                        let label = entry.label.clone();
                                        move |_| ctx.center_on_token_label.set(Some(label.clone()))
                                    }
                                >
                                    <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                                        <circle cx="12" cy="12" r="3"/>
                                    </svg>
                                </button>
                                <button
                                    class="init-remove"
                                    on:click=move |_| remove_entry(idx)
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

            {move || (!initiative.get().is_empty()).then(|| view! {
                <div class="initiative-controls">
                    <button class="btn-next-turn" on:click=advance_turn>"Next Turn"</button>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::reorder_index;

    #[test]
    fn move_forward() {
        // Move item 0 to gap 2 (after item 1) → insert at index 1
        assert_eq!(reorder_index(0, 2, 3), Some(1));
        // Move item 0 to gap 3 (after item 2) → insert at index 2
        assert_eq!(reorder_index(0, 3, 3), Some(2));
    }

    #[test]
    fn move_backward() {
        // Move item 2 to gap 0 (before item 0) → insert at index 0
        assert_eq!(reorder_index(2, 0, 3), Some(0));
        // Move item 2 to gap 1 (before item 1) → insert at index 1
        assert_eq!(reorder_index(2, 1, 3), Some(1));
    }

    #[test]
    fn no_op_same_position() {
        // Dropping right before or right after source is a no-op
        assert_eq!(reorder_index(1, 1, 3), None);
        assert_eq!(reorder_index(1, 2, 3), None);
    }

    #[test]
    fn out_of_bounds() {
        assert_eq!(reorder_index(3, 0, 3), None); // from >= len
        assert_eq!(reorder_index(0, 4, 3), None); // to > len
    }

    #[test]
    fn single_element_list() {
        // Only one element, can't move anywhere meaningful
        assert_eq!(reorder_index(0, 0, 1), None);
        assert_eq!(reorder_index(0, 1, 1), None);
    }

    #[test]
    fn two_element_swap() {
        // Move item 0 after item 1
        assert_eq!(reorder_index(0, 2, 2), Some(1));
        // Move item 1 before item 0
        assert_eq!(reorder_index(1, 0, 2), Some(0));
    }

    #[test]
    fn empty_list() {
        assert_eq!(reorder_index(0, 0, 0), None);
    }
}
