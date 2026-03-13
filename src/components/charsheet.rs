use leptos::prelude::*;

use crate::models::{CharacterInfo, FieldType, ResourceInfo, TemplateField, TemplateInfo};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

#[component]
pub fn CharacterSheet() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;

    let template = Resource::new(
        move || session_id.get(),
        |sid| crate::server::api::get_session_template(sid),
    );

    let characters = Resource::new(
        move || session_id.get(),
        |sid| crate::server::api::list_characters(sid),
    );

    let (selected_char, set_selected_char) = signal(Option::<i32>::None);
    let (creating, set_creating) = signal(false);
    let (new_name, set_new_name) = signal(String::new());

    let create_char = move |_| {
        let name = new_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let sid = session_id.get();
        set_creating.set(true);
        leptos::task::spawn_local(async move {
            match crate::server::api::create_character(sid, name).await {
                Ok(c) => {
                    set_selected_char.set(Some(c.id));
                    characters.refetch();
                }
                Err(e) => log::error!("Failed to create character: {e}"),
            }
            set_creating.set(false);
            set_new_name.set(String::new());
        });
    };

    view! {
        <div class="character-sheet-panel">
            <h3>"Characters"</h3>

            // Character selector
            <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                {move || {
                    let chars = characters.get().unwrap_or(Ok(vec![])).unwrap_or_default();
                    view! {
                        <div class="char-selector">
                            <For
                                each=move || chars.clone()
                                key=|c| c.id
                                let:c
                            >
                                {
                                    let cid = c.id;
                                    view! {
                                        <button
                                            class=move || {
                                                if selected_char.get() == Some(cid) {
                                                    "char-tab selected"
                                                } else {
                                                    "char-tab"
                                                }
                                            }
                                            on:click=move |_| set_selected_char.set(Some(cid))
                                        >
                                            {c.name.clone()}
                                        </button>
                                    }
                                }
                            </For>
                        </div>
                    }
                }}
            </Suspense>

            // Create character
            <div class="char-create">
                <input
                    type="text"
                    placeholder="New character name"
                    prop:value=new_name
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                />
                <button
                    on:click=create_char
                    disabled=creating
                >"Create"</button>
            </div>

            // Character editor
            <Suspense fallback=|| ()>
                {move || {
                    let tmpl = template.get().unwrap_or(Ok(None)).unwrap_or(None);
                    let chars = characters.get().unwrap_or(Ok(vec![])).unwrap_or_default();
                    let sel = selected_char.get();

                    sel.and_then(|sel_id| {
                        let character = chars.into_iter().find(|c| c.id == sel_id)?;
                        Some(view! {
                            <CharacterEditor
                                character=character
                                template=tmpl
                            />
                        })
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn CharacterEditor(
    character: CharacterInfo,
    template: Option<TemplateInfo>,
) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let send = ctx.send;
    let char_id = character.id;

    let fields = template
        .as_ref()
        .map(|t| t.fields.clone())
        .unwrap_or_default();

    // Group fields by category
    let mut categories: Vec<(String, Vec<TemplateField>)> = Vec::new();
    for field in &fields {
        if let Some(cat) = categories.iter_mut().find(|(c, _)| c == &field.category) {
            cat.1.push(field.clone());
        } else {
            categories.push((field.category.clone(), vec![field.clone()]));
        }
    }

    let data = character.data.clone();
    let resources = character.resources.clone();

    view! {
        <div class="char-editor">
            <h4>{character.name.clone()}</h4>

            // Resource bars
            <div class="char-resources">
                <For
                    each=move || resources.clone()
                    key=|r| r.id
                    let:resource
                >
                    <ResourceBar resource=resource />
                </For>
            </div>

            // Template fields by category
            <For
                each=move || categories.clone()
                key=|(cat, _)| cat.clone()
                let:item
            >
                {
                    let (cat_name, cat_fields) = item;
                    let data = data.clone();
                    view! {
                        <div class="char-category">
                            <h5>{cat_name}</h5>
                            <div class="char-fields">
                                <For
                                    each=move || cat_fields.clone()
                                    key=|f| f.name.clone()
                                    let:field
                                >
                                    <FieldEditor
                                        character_id=char_id
                                        field=field
                                        data=data.clone()
                                        send=send
                                    />
                                </For>
                            </div>
                        </div>
                    }
                }
            </For>
        </div>
    }
}

#[component]
fn ResourceBar(resource: ResourceInfo) -> impl IntoView {
    let rid = resource.id;
    let (current, set_current) = signal(resource.current_value);
    let max = resource.max_value;

    let update_resource = move |new_val: i32| {
        let clamped = new_val.max(0).min(max);
        set_current.set(clamped);
        leptos::task::spawn_local(async move {
            let _ = crate::server::api::update_character_resource(rid, clamped).await;
        });
    };

    view! {
        <div class="resource-bar">
            <span class="resource-name">{resource.name.clone()}</span>
            <button on:click=move |_| update_resource(current.get() - 1)>"-"</button>
            <span class="resource-value">{move || current.get()} "/" {max}</span>
            <button on:click=move |_| update_resource(current.get() + 1)>"+"</button>
            <div class="resource-bar-visual">
                <div
                    class="resource-bar-fill"
                    style=move || {
                        let ratio = if max > 0 {
                            (current.get() as f64 / max as f64 * 100.0).clamp(0.0, 100.0)
                        } else {
                            0.0
                        };
                        format!("width: {ratio}%")
                    }
                />
            </div>
        </div>
    }
}

type SendHandle = StoredValue<Option<Box<dyn Fn(ClientMessage)>>, leptos::reactive::owner::LocalStorage>;

#[component]
fn FieldEditor(
    character_id: i32,
    field: TemplateField,
    data: serde_json::Value,
    send: SendHandle,
) -> impl IntoView {
    let current_value = data
        .get(&field.name)
        .cloned()
        .unwrap_or(field.default.clone());

    let field_name = field.name.clone();

    let on_change = move |new_value: serde_json::Value| {
        let path = field_name.clone();
        let val = new_value.clone();
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::UpdateCharacterField {
                    character_id,
                    field_path: path,
                    value: val,
                });
            }
        });
    };

    match field.field_type {
        FieldType::Number => {
            let num_val = current_value.as_f64().unwrap_or(0.0);
            let (val, set_val) = signal(num_val.to_string());
            let on_change = on_change.clone();
            view! {
                <div class="field-row">
                    <label>{field.label.clone()}</label>
                    <input
                        type="number"
                        prop:value=val
                        on:change=move |ev| {
                            let s = event_target_value(&ev);
                            set_val.set(s.clone());
                            if let Ok(n) = s.parse::<f64>() {
                                on_change(serde_json::json!(n));
                            }
                        }
                    />
                </div>
            }
            .into_any()
        }
        FieldType::Text => {
            let text_val = current_value.as_str().unwrap_or("").to_string();
            let (val, set_val) = signal(text_val);
            let on_change = on_change.clone();
            view! {
                <div class="field-row">
                    <label>{field.label.clone()}</label>
                    <input
                        type="text"
                        prop:value=val
                        on:change=move |ev| {
                            let s = event_target_value(&ev);
                            set_val.set(s.clone());
                            on_change(serde_json::json!(s));
                        }
                    />
                </div>
            }
            .into_any()
        }
        FieldType::Boolean => {
            let bool_val = current_value.as_bool().unwrap_or(false);
            let (val, set_val) = signal(bool_val);
            let on_change = on_change.clone();
            view! {
                <div class="field-row">
                    <label>
                        <input
                            type="checkbox"
                            prop:checked=val
                            on:change=move |_| {
                                let new_val = !val.get();
                                set_val.set(new_val);
                                on_change(serde_json::json!(new_val));
                            }
                        />
                        {field.label.clone()}
                    </label>
                </div>
            }
            .into_any()
        }
        FieldType::Textarea => {
            let text_val = current_value.as_str().unwrap_or("").to_string();
            let (val, set_val) = signal(text_val);
            let on_change = on_change.clone();
            view! {
                <div class="field-row field-textarea">
                    <label>{field.label.clone()}</label>
                    <textarea
                        prop:value=val
                        on:change=move |ev| {
                            let s = event_target_value(&ev);
                            set_val.set(s.clone());
                            on_change(serde_json::json!(s));
                        }
                        rows="3"
                    />
                </div>
            }
            .into_any()
        }
    }
}
