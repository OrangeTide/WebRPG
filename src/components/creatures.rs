use leptos::prelude::*;

use crate::models::{CreatureInfo, FieldType, TemplateField, TemplateInfo};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// One reversible action on the Creatures table. The undo stack holds these so
/// the most recent table-local action (delete, field change, rename) can be
/// reversed. Place on map and roll initiative are not tracked yet.
#[derive(Clone)]
enum CreatureUndo {
    /// A creature was deleted; undo recreates it from this snapshot. The new
    /// row gets a fresh id, so the undo handler remaps later stack entries.
    Deleted(CreatureInfo),
    /// A creature was edited; undo restores these previous name and stats.
    Updated {
        id: i32,
        name: String,
        stat_data: serde_json::Value,
    },
}

#[component]
pub fn CreaturePanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;

    // Use signal + effect (not Resource) to avoid hydration mismatch
    let template = RwSignal::new(Option::<TemplateInfo>::None);
    let creatures = RwSignal::new(Vec::<CreatureInfo>::new());
    #[allow(unused_variables)]
    let loading = RwSignal::new(true);
    let refetch = RwSignal::new(0u32);

    #[cfg(feature = "hydrate")]
    {
        let fetch_creatures = move || {
            let sid = session_id.get();
            refetch.track();
            if sid == 0 {
                return;
            }
            loading.set(true);
            leptos::task::spawn_local(async move {
                match crate::server::api::list_creatures(sid).await {
                    Ok(list) => creatures.set(list),
                    Err(e) => log::error!("Failed to load creatures: {e}"),
                }
                match crate::server::api::get_session_template(sid).await {
                    Ok(tmpl) => template.set(tmpl),
                    Err(e) => log::error!("Failed to load template: {e}"),
                }
                loading.set(false);
            });
        };
        Effect::new(move |_| fetch_creatures());
    }

    let trigger_refetch = move || refetch.update(|n| *n += 1);

    // Undo stack for the Creatures table (feature 58). Capped so it cannot grow
    // without bound over a long session.
    let undo_stack = RwSignal::new(Vec::<CreatureUndo>::new());
    let push_undo = move |action: CreatureUndo| {
        undo_stack.update(|s| {
            s.push(action);
            if s.len() > 50 {
                s.remove(0);
            }
        });
    };

    let (editing, set_editing) = signal(Option::<i32>::None);

    // When focus_creature is set, open the editor for that creature
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let Some(creature_id) = ctx.focus_creature.get() else {
                return;
            };
            ctx.focus_creature.set(None);
            set_editing.set(Some(creature_id));
        });
    }
    let (show_create_form, set_show_create_form) = signal(false);
    let (new_name, set_new_name) = signal(String::new());

    let do_create = move |name: String| {
        if name.is_empty() {
            return;
        }
        let sid = session_id.get();
        leptos::task::spawn_local(async move {
            match crate::server::api::create_creature(sid, name, serde_json::json!({"hp_max": 10}))
                .await
            {
                Ok(c) => {
                    set_editing.set(Some(c.id));
                    set_show_create_form.set(false);
                    trigger_refetch();
                }
                Err(e) => log::error!("Failed to create creature: {e}"),
            }
            set_new_name.set(String::new());
        });
    };

    let create_creature = move |_| {
        do_create(new_name.get().trim().to_string());
    };

    let cancel_create = move |_| {
        set_show_create_form.set(false);
        set_new_name.set(String::new());
    };

    let delete_creature = move |creature: CreatureInfo| {
        let creature_id = creature.id;
        push_undo(CreatureUndo::Deleted(creature));
        leptos::task::spawn_local(async move {
            let _ = crate::server::api::delete_creature(creature_id).await;
            trigger_refetch();
        });
    };

    // Reverse the most recent table-local action.
    let undo_last = move |_| {
        let Some(action) = undo_stack.try_update(|s| s.pop()).flatten() else {
            return;
        };
        let sid = session_id.get();
        leptos::task::spawn_local(async move {
            match action {
                CreatureUndo::Deleted(info) => {
                    let old_id = info.id;
                    let image = info.image_url.clone();
                    match crate::server::api::create_creature(sid, info.name, info.stat_data).await
                    {
                        Ok(c) => {
                            if let Some(url) = image {
                                let _ = crate::server::api::update_creature_image(c.id, Some(url))
                                    .await;
                            }
                            // The recreated creature has a new id. Point any
                            // later undo entries for the old id at the new one.
                            undo_stack.update(|s| {
                                for a in s.iter_mut() {
                                    if let CreatureUndo::Updated { id, .. } = a
                                        && *id == old_id
                                    {
                                        *id = c.id;
                                    }
                                }
                            });
                        }
                        Err(e) => log::error!("Failed to undo creature delete: {e}"),
                    }
                }
                CreatureUndo::Updated {
                    id,
                    name,
                    stat_data,
                } => {
                    if let Err(e) = crate::server::api::update_creature(id, name, stat_data).await {
                        log::error!("Failed to undo creature edit: {e}");
                    }
                }
            }
            trigger_refetch();
        });
    };

    let duplicate_creature = move |name: String,
                                   stat_data: serde_json::Value,
                                   image_url: Option<String>| {
        let sid = session_id.get();
        let dup_name = format!("{name} (copy)");
        leptos::task::spawn_local(async move {
            match crate::server::api::create_creature(sid, dup_name, stat_data).await {
                Ok(c) => {
                    // Copy the image if the original had one
                    if let Some(url) = image_url {
                        let _ = crate::server::api::update_creature_image(c.id, Some(url)).await;
                    }
                    set_editing.set(Some(c.id));
                    trigger_refetch();
                }
                Err(e) => log::error!("Failed to duplicate creature: {e}"),
            }
        });
    };

    view! {
        <div class="creature-panel">
            <div class="panel-header">
                <h3>"Creatures"</h3>
                <div class="panel-header-actions">
                    {move || (!undo_stack.get().is_empty()).then(|| view! {
                        <button
                            class="btn-undo"
                            data-tooltip="Undo last action"
                            on:click=undo_last
                        >
                            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M3 7v6h6"/>
                                <path d="M3 13a9 9 0 103-7l-3 3"/>
                            </svg>
                        </button>
                    })}
                    <button
                        class="btn-add"
                        data-tooltip="New Creature"
                        on:click=move |_| {
                            set_show_create_form.set(true);
                            set_editing.set(None);
                        }
                    >"+"</button>
                </div>
            </div>

            // Create creature form
            {move || show_create_form.get().then(|| view! {
                <div class="create-form">
                    <h4>"New Creature"</h4>
                    <div class="field-row">
                        <label>"Name"</label>
                        <input
                            type="text"
                            placeholder="Creature name"
                            prop:value=new_name
                            on:input=move |ev| set_new_name.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    do_create(new_name.get().trim().to_string());
                                }
                            }
                        />
                    </div>
                    <div class="form-actions">
                        <button on:click=create_creature>"Create"</button>
                        <button class="btn-cancel" on:click=cancel_create>"Cancel"</button>
                    </div>
                </div>
            })}

            // Editor overlay (shown when editing a creature)
            {move || {
                let tmpl = template.get();
                let creature_list = creatures.get();
                let editing_id = editing.get();
                editing_id.and_then(|edit_id| {
                    creature_list.into_iter().find(|c| c.id == edit_id).map(|creature| {
                        let dup_name = creature.name.clone();
                        let dup_stats = creature.stat_data.clone();
                        let dup_image = creature.image_url.clone();
                        let del_info = creature.clone();
                        view! {
                            <div>
                                <div class="editor-toolbar">
                                    <button
                                        class="btn-back"
                                        on:click=move |_| {
                                            trigger_refetch();
                                            set_editing.set(None);
                                        }
                                    >"< Back to list"</button>
                                    <button
                                        class="btn-duplicate"
                                        title="Duplicate creature"
                                        on:click=move |_| {
                                            duplicate_creature(dup_name.clone(), dup_stats.clone(), dup_image.clone());
                                        }
                                    >
                                        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                                            <path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                                        </svg>
                                    </button>
                                    <button
                                        class="btn-delete"
                                        title="Delete creature"
                                        on:click=move |_| {
                                            delete_creature(del_info.clone());
                                            set_editing.set(None);
                                        }
                                    >
                                        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                            <polyline points="3 6 5 6 21 6"/>
                                            <path d="M19 6l-1 14H6L5 6"/>
                                            <path d="M10 11v6"/><path d="M14 11v6"/>
                                            <path d="M9 6V4h6v2"/>
                                        </svg>
                                    </button>
                                </div>
                                <CreatureEditor
                                    creature=creature
                                    template=tmpl
                                    undo_stack=undo_stack
                                    on_saved=move || {
                                        trigger_refetch();
                                    }
                                />
                            </div>
                        }
                    })
                })
            }}

            // Creature list — shown only in list state, hidden while editing a
            // creature (feature 68). The initial render (SSR and hydrate) has no
            // creature being edited, so the list is present at hydration time and
            // this reactive toggle only takes effect afterward.
            {move || editing.get().is_none().then(|| view! {
            <div class="item-list">
                <For
                    each=move || creatures.get()
                    key=|c| (c.id, c.name.clone(), c.image_url.clone(), c.stat_data.to_string())
                    let:creature
                >
                    {
                        let cid = creature.id;
                        let name = creature.name.clone();
                        let image = creature.image_url.clone();
                        let del_info = creature.clone();
                        let stats_summary = {
                            let d = &creature.stat_data;
                            let mut parts = Vec::new();
                            if let Some(hp) = d.get("hp_max").and_then(|v| v.as_f64()) {
                                parts.push(format!("HP {}", hp as i32));
                            }
                            if let Some(ac) = d.get("armor_class").and_then(|v| v.as_f64()) {
                                parts.push(format!("AC {}", ac as i32));
                            }
                            parts.join(" | ")
                        };
                        let roll_name = creature.name.clone();
                        let place_name = creature.name.clone();
                        let place_image = creature.image_url.clone();
                        let send = ctx.send;
                        let roll_init = move |_| {
                            let label = roll_name.clone();
                            send.send(ClientMessage::RollCreatureInitiative {
                                creature_id: cid,
                                label,
                            });
                        };
                        let place_on_map = {
                            let label = place_name;
                            let img = place_image;
                            let view_center = ctx.map_view_center;
                            move |_| {
                                let label = label.clone();
                                let img = img.clone();
                                let (cx, cy) = view_center.get();
                                send.send(ClientMessage::PlaceToken {
                                    label,
                                    x: cx,
                                    y: cy,
                                    color: "#aa4444".to_string(),
                                    size: 1,
                                    character_id: None,
                                    creature_id: Some(cid),
                                    image_url: img,
                                });
                            }
                        };
                        view! {
                            <div class="item-card creature-card-item">
                                <div class="creature-card-top"
                                    on:click=move |_| set_editing.set(Some(cid))
                                >
                                    <div class="item-card-portrait">
                                        {if let Some(url) = image.clone() {
                                            view! { <img src=url alt="icon" /> }.into_any()
                                        } else {
                                            view! { <div class="item-card-icon">"&#x1f47e;"</div> }.into_any()
                                        }}
                                    </div>
                                    <div class="item-card-info">
                                        <strong>{name}</strong>
                                        {(!stats_summary.is_empty()).then(|| view! {
                                            <span class="item-card-stat">{stats_summary}</span>
                                        })}
                                    </div>
                                    <button
                                        class="btn-delete"
                                        data-tooltip="Delete creature"
                                        on:click=move |ev: leptos::ev::MouseEvent| {
                                            ev.stop_propagation();
                                            delete_creature(del_info.clone());
                                        }
                                    >
                                        <svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2">
                                            <polyline points="3 6 5 6 21 6"/>
                                            <path d="M19 6l-1 14H6L5 6"/>
                                            <path d="M10 11v6"/><path d="M14 11v6"/>
                                            <path d="M9 6V4h6v2"/>
                                        </svg>
                                    </button>
                                </div>
                                <div class="creature-card-actions">
                                    <button
                                        class="btn-roll-initiative"
                                        title="Roll Initiative"
                                        on:click=roll_init
                                    >"Roll Init"</button>
                                    <button
                                        class="btn-place-map"
                                        title="Place on Map"
                                        on:click=place_on_map
                                    >"Place on Map"</button>
                                </div>
                            </div>
                        }
                    }
                </For>
            </div>
            })}
        </div>
    }
}

#[component]
fn CreatureEditor(
    creature: CreatureInfo,
    template: Option<TemplateInfo>,
    undo_stack: RwSignal<Vec<CreatureUndo>>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let creature_id = creature.id;
    // The creature's values when this editor opened, used as the "before" state
    // for the undo stack on the next save.
    let prev_snapshot = RwSignal::new((creature.name.clone(), creature.stat_data.clone()));
    let (name, set_name) = signal(creature.name.clone());
    let show_image_picker = RwSignal::new(false);
    let (image, set_image) = signal(creature.image_url.clone());

    let on_image_select = {
        Callback::new(move |media: crate::models::MediaInfo| {
            let url = media.url.clone();
            set_image.set(Some(url.clone()));
            show_image_picker.set(false);
            leptos::task::spawn_local(async move {
                let _ = crate::server::api::update_creature_image(creature_id, Some(url)).await;
            });
        })
    };

    // Get combat-relevant fields from the template (for stat blocks)
    let stat_fields: Vec<TemplateField> = template
        .as_ref()
        .map(|t| {
            t.fields
                .iter()
                .filter(|f| matches!(f.category.as_str(), "Ability Scores" | "Combat"))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Build initial field values from creature's stat_data
    let initial_data = creature.stat_data.clone();
    let field_signals: Vec<(TemplateField, RwSignal<String>)> = stat_fields
        .iter()
        .map(|f| {
            let val = initial_data
                .get(&f.name)
                .map(|v| {
                    if v.is_number() {
                        v.as_f64().unwrap_or(0.0).to_string()
                    } else {
                        v.as_str().unwrap_or("").to_string()
                    }
                })
                .unwrap_or_else(|| {
                    if f.default.is_number() {
                        f.default.as_f64().unwrap_or(0.0).to_string()
                    } else {
                        f.default.as_str().unwrap_or("").to_string()
                    }
                });
            (f.clone(), RwSignal::new(val))
        })
        .collect();

    let field_signals_save = field_signals.clone();

    let save = move |_| {
        let new_name = name.get().trim().to_string();
        let mut stat_data = serde_json::Map::new();
        for (field, sig) in &field_signals_save {
            let val_str = sig.get();
            let json_val = match field.field_type {
                FieldType::Number => {
                    serde_json::json!(val_str.parse::<f64>().unwrap_or(0.0))
                }
                FieldType::Boolean => {
                    serde_json::json!(val_str.parse::<bool>().unwrap_or(false))
                }
                _ => serde_json::json!(val_str),
            };
            stat_data.insert(field.name.clone(), json_val);
        }
        let new_stats = serde_json::Value::Object(stat_data);

        let (prev_name, prev_stats) = prev_snapshot.get();
        let changed = new_name != prev_name || new_stats != prev_stats;
        let name_for_update = new_name.clone();
        let stats_for_update = new_stats.clone();

        let on_saved = on_saved;
        leptos::task::spawn_local(async move {
            match crate::server::api::update_creature(
                creature_id,
                name_for_update,
                stats_for_update,
            )
            .await
            {
                Ok(()) => {
                    // Record the previous values so this edit can be undone.
                    if changed {
                        undo_stack.update(|s| {
                            s.push(CreatureUndo::Updated {
                                id: creature_id,
                                name: prev_name,
                                stat_data: prev_stats,
                            });
                            if s.len() > 50 {
                                s.remove(0);
                            }
                        });
                        prev_snapshot.set((new_name, new_stats));
                    }
                    on_saved.run(());
                }
                Err(e) => log::error!("Failed to save creature: {e}"),
            }
        });
    };

    view! {
        <div class="creature-editor">
            <div class="creature-editor-header">
                <div
                    class="char-portrait"
                    on:click=move |_| show_image_picker.set(true)
                    style="cursor: pointer;"
                >
                    {move || {
                        if let Some(url) = image.get() {
                            view! { <img src=url alt="icon" class="portrait-img" /> }.into_any()
                        } else {
                            view! { <div class="portrait-placeholder">"Set Icon"</div> }.into_any()
                        }
                    }}
                </div>
                <div class="field-row" style="flex: 1;">
                    <label>"Name"</label>
                    <input
                        type="text"
                        prop:value=name
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                    />
                </div>
            </div>
            <crate::components::media_browser::MediaBrowser
                on_select=on_image_select
                filter_type="image".to_string()
                show=show_image_picker
            />
            <For
                each=move || field_signals.clone()
                key=|(f, _)| f.name.clone()
                let:item
            >
                {
                    let (field, sig) = item;
                    let is_number = field.field_type == FieldType::Number;
                    view! {
                        <div class="field-row">
                            <label>{field.label.clone()}</label>
                            <input
                                type=move || if is_number { "number" } else { "text" }
                                prop:value=sig
                                on:input=move |ev| sig.set(event_target_value(&ev))
                            />
                        </div>
                    }
                }
            </For>
            <button on:click=save>"Save"</button>
        </div>
    }
}
