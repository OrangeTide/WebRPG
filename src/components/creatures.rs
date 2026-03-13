use leptos::prelude::*;

use crate::models::{CreatureInfo, FieldType, TemplateField, TemplateInfo};
use crate::pages::game::GameContext;

#[component]
pub fn CreaturePanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;

    let template = Resource::new(
        move || session_id.get(),
        |sid| crate::server::api::get_session_template(sid),
    );

    let creatures = Resource::new(
        move || session_id.get(),
        |sid| crate::server::api::list_creatures(sid),
    );

    let (editing, set_editing) = signal(Option::<i32>::None);
    let (new_name, set_new_name) = signal(String::new());

    let create_creature = move |_| {
        let name = new_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let sid = session_id.get();
        leptos::task::spawn_local(async move {
            match crate::server::api::create_creature(
                sid,
                name,
                serde_json::json!({"hp_max": 10}),
            )
            .await
            {
                Ok(c) => {
                    set_editing.set(Some(c.id));
                    creatures.refetch();
                }
                Err(e) => log::error!("Failed to create creature: {e}"),
            }
        });
        set_new_name.set(String::new());
    };

    let delete_creature = move |creature_id: i32| {
        leptos::task::spawn_local(async move {
            let _ = crate::server::api::delete_creature(creature_id).await;
            creatures.refetch();
        });
    };

    view! {
        <div class="creature-panel">
            <h3>"Creatures"</h3>

            <div class="creature-create">
                <input
                    type="text"
                    placeholder="New creature name"
                    prop:value=new_name
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                />
                <button on:click=create_creature>"Create"</button>
            </div>

            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || {
                    let tmpl = template.get().unwrap_or(Ok(None)).unwrap_or(None);
                    let creature_list = creatures.get().unwrap_or(Ok(vec![])).unwrap_or_default();
                    let editing_id = editing.get();

                    view! {
                        <div class="creature-list">
                            <For
                                each=move || creature_list.clone()
                                key=|c| c.id
                                let:creature
                            >
                                {
                                    let cid = creature.id;
                                    let is_editing = editing_id == Some(cid);
                                    let tmpl_clone = tmpl.clone();
                                    view! {
                                        <div class="creature-entry">
                                            <div class="creature-header">
                                                <strong>{creature.name.clone()}</strong>
                                                <button on:click=move |_| {
                                                    if editing.get() == Some(cid) {
                                                        set_editing.set(None);
                                                    } else {
                                                        set_editing.set(Some(cid));
                                                    }
                                                }>
                                                    {if is_editing { "Close" } else { "Edit" }}
                                                </button>
                                                <button on:click=move |_| delete_creature(cid)>"Delete"</button>
                                            </div>
                                            {is_editing.then(|| {
                                                view! {
                                                    <CreatureEditor
                                                        creature=creature.clone()
                                                        template=tmpl_clone.clone()
                                                        on_saved=move || creatures.refetch()
                                                    />
                                                }
                                            })}
                                        </div>
                                    }
                                }
                            </For>
                        </div>
                    }
                }}
            </Suspense>
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

    // Get combat-relevant fields from the template (for stat blocks)
    let stat_fields: Vec<TemplateField> = template
        .as_ref()
        .map(|t| {
            t.fields
                .iter()
                .filter(|f| {
                    matches!(
                        f.category.as_str(),
                        "Ability Scores" | "Combat"
                    )
                })
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
            <div class="field-row">
                <label>"Name"</label>
                <input
                    type="text"
                    prop:value=name
                    on:input=move |ev| set_name.set(event_target_value(&ev))
                />
            </div>
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
