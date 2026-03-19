use leptos::prelude::*;

use crate::models::{CreatureInfo, FieldType, TemplateField, TemplateInfo};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

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

    let (editing, set_editing) = signal(Option::<i32>::None);
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

    let delete_creature = move |creature_id: i32| {
        leptos::task::spawn_local(async move {
            let _ = crate::server::api::delete_creature(creature_id).await;
            trigger_refetch();
        });
    };

    view! {
        <div class="creature-panel">
            <div class="panel-header">
                <h3>"Creatures"</h3>
                <button
                    class="btn-add"
                    title="New Creature"
                    on:click=move |_| {
                        set_show_create_form.set(true);
                        set_editing.set(None);
                    }
                >"+"</button>
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
                        let cid = creature.id;
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
                                        class="btn-delete"
                                        title="Delete creature"
                                        on:click=move |_| {
                                            delete_creature(cid);
                                            set_editing.set(None);
                                        }
                                    >"Delete"</button>
                                </div>
                                <CreatureEditor
                                    creature=creature
                                    template=tmpl
                                    on_saved=move || {
                                        trigger_refetch();
                                    }
                                />
                            </div>
                        }
                    })
                })
            }}

            // Creature list — always render <For> to avoid SSR/hydration mismatch
            <div class="item-list">
                <For
                    each=move || creatures.get()
                    key=|c| c.id
                    let:creature
                >
                    {
                        let cid = creature.id;
                        let name = creature.name.clone();
                        let image = creature.image_url.clone();
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
                        let send = ctx.send;
                        let roll_init = move |_| {
                            let label = roll_name.clone();
                            send.with_value(|f| {
                                if let Some(f) = f {
                                    f(ClientMessage::RollCreatureInitiative {
                                        creature_id: cid,
                                        label,
                                    });
                                }
                            });
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
                                        title="Delete creature"
                                        on:click:stopPropagation=move |_: leptos::ev::MouseEvent| delete_creature(cid)
                                    >"x"</button>
                                </div>
                                <button
                                    class="btn-roll-initiative"
                                    title="Roll Initiative"
                                    on:click=roll_init
                                >"Roll Initiative"</button>
                            </div>
                        }
                    }
                </For>
            </div>
        </div>
    }
}

#[component]
fn CreatureEditor(
    creature: CreatureInfo,
    template: Option<TemplateInfo>,
    #[prop(into)] on_saved: Callback<()>,
) -> impl IntoView {
    let creature_id = creature.id;
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

        let on_saved = on_saved.clone();
        leptos::task::spawn_local(async move {
            match crate::server::api::update_creature(
                creature_id,
                new_name,
                serde_json::Value::Object(stat_data),
            )
            .await
            {
                Ok(()) => on_saved.run(()),
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
