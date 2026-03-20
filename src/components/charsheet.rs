use leptos::prelude::*;

use crate::components::window_manager::WindowManagerContext;
use crate::models::{CharacterInfo, FieldType, ResourceInfo, TemplateField, TemplateInfo};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Character selection list — shows all characters for the session.
/// Clicking a character opens a separate Character Sheet window.
#[component]
pub fn CharacterSelection() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;

    // Use signal + effect instead of Resource so the list loads correctly
    // after hydration (session_id starts at 0 and gets set in an Effect).
    let characters = RwSignal::new(Vec::<CharacterInfo>::new());
    #[allow(unused_variables)]
    let loading = RwSignal::new(true);
    let refetch = RwSignal::new(0u32);

    #[allow(unused_variables)]
    let character_revision = ctx.character_revision;

    // Only fetch on client — SSR must render the same empty initial state
    // that the client sees during hydration.
    #[cfg(feature = "hydrate")]
    {
        let fetch_characters = move || {
            let sid = session_id.get();
            refetch.track();
            character_revision.track();
            if sid == 0 {
                return;
            }
            loading.set(true);
            leptos::task::spawn_local(async move {
                match crate::server::api::list_characters(sid).await {
                    Ok(chars) => characters.set(chars),
                    Err(e) => log::error!("Failed to load characters: {e}"),
                }
                loading.set(false);
            });
        };
        Effect::new(move |_| fetch_characters());
    }

    let trigger_refetch = move || refetch.update(|n| *n += 1);

    let (show_create_form, set_show_create_form) = signal(false);
    let (creating, set_creating) = signal(false);
    let (new_name, set_new_name) = signal(String::new());

    let do_create = move |name: String| {
        if name.is_empty() {
            return;
        }
        let sid = session_id.get();
        set_creating.set(true);
        leptos::task::spawn_local(async move {
            match crate::server::api::create_character(sid, name.clone()).await {
                Ok(c) => {
                    set_show_create_form.set(false);
                    trigger_refetch();
                    // Open the new character in its own window
                    if let Some(wm) = use_context::<WindowManagerContext>() {
                        wm.open_character_editor(c.id, &c.name);
                    }
                }
                Err(e) => log::error!("Failed to create character: {e}"),
            }
            set_creating.set(false);
            set_new_name.set(String::new());
        });
    };

    let create_char = move |_| {
        do_create(new_name.get().trim().to_string());
    };

    let cancel_create = move |_| {
        set_show_create_form.set(false);
        set_new_name.set(String::new());
    };

    let delete_char = move |char_id: i32| {
        // Close the editor window if open
        if let Some(wm) = use_context::<WindowManagerContext>() {
            wm.close_window(crate::components::window_manager::WindowId::CharacterEditor(char_id));
        }
        leptos::task::spawn_local(async move {
            match crate::server::api::delete_character(char_id).await {
                Ok(()) => trigger_refetch(),
                Err(e) => log::error!("Failed to delete character: {e}"),
            }
        });
    };

    let open_character = move |char_id: i32, char_name: String| {
        if let Some(wm) = use_context::<WindowManagerContext>() {
            wm.open_character_editor(char_id, &char_name);
        }
    };

    view! {
        <div class="character-sheet-panel">
            <div class="panel-header">
                <h3>"Characters"</h3>
                <button
                    class="btn-add"
                    data-tooltip="New Character"
                    on:click=move |_| {
                        set_show_create_form.set(true);
                    }
                >"+"</button>
            </div>

            // Create character form
            {move || show_create_form.get().then(|| view! {
                <div class="create-form">
                    <h4>"New Character"</h4>
                    <div class="field-row">
                        <label>"Name"</label>
                        <input
                            type="text"
                            placeholder="Character name"
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
                        <button on:click=create_char disabled=creating>"Create"</button>
                        <button class="btn-cancel" on:click=cancel_create>"Cancel"</button>
                    </div>
                </div>
            })}

            // Character list — always render <For> to avoid SSR/hydration mismatch
            // (SSR resolves server functions synchronously, producing a filled list,
            // while the client starts with an empty signal).
            <div class="item-list">
                <For
                    each=move || characters.get()
                    key=|c| (c.id, c.portrait_url.clone(), c.resources.iter().map(|r| (r.id, r.current_value)).collect::<Vec<_>>(), c.data.to_string())
                    let:c
                >
                    {
                        let cid = c.id;
                        let name = c.name.clone();
                        let name_for_click = c.name.clone();
                        let portrait = c.portrait_url.clone();
                        let resources = c.resources.clone();
                        let stats_summary = {
                            let d = &c.data;
                            let mut parts = Vec::new();
                            if let Some(ac) = d.get("armor_class").and_then(|v| v.as_f64()) {
                                parts.push(format!("AC {}", ac as i32));
                            }
                            if let Some(lvl) = d.get("level").and_then(|v| v.as_f64()) {
                                parts.push(format!("Lv {}", lvl as i32));
                            }
                            parts.join(" | ")
                        };
                        view! {
                            <div class="item-card">
                                <div
                                    class="item-card-clickable"
                                    on:click=move |_| open_character(cid, name_for_click.clone())
                                >
                                    <div class="item-card-portrait">
                                        {if let Some(url) = portrait.clone() {
                                            view! { <img src=url alt="portrait" /> }.into_any()
                                        } else {
                                            view! { <div class="item-card-icon">"&#x1f9d9;"</div> }.into_any()
                                        }}
                                    </div>
                                    <div class="item-card-info">
                                        <strong>{name}</strong>
                                        {(!resources.is_empty()).then(|| {
                                            let hp = resources.iter().find(|r| r.name.to_lowercase().contains("hp") || r.name.to_lowercase().contains("hit points"));
                                            hp.map(|r| view! {
                                                <span class="item-card-stat">"HP " {r.current_value} "/" {r.max_value}</span>
                                            })
                                        })}
                                        {(!stats_summary.is_empty()).then(|| view! {
                                            <span class="item-card-stat">{stats_summary}</span>
                                        })}
                                    </div>
                                </div>
                                <button
                                    class="btn-delete"
                                    data-tooltip="Delete character"
                                    on:click=move |_| delete_char(cid)
                                >"x"</button>
                            </div>
                        }
                    }
                </For>
            </div>
        </div>
    }
}

/// Standalone character editor panel — fetches its own data by character_id.
/// Used inside a dynamic GameWindow.
#[component]
#[allow(unused_variables)]
pub fn CharacterEditorPanel(character_id: i32) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;

    let template = RwSignal::new(Option::<TemplateInfo>::None);
    let character = RwSignal::new(Option::<CharacterInfo>::None);
    #[allow(unused_variables)]
    let loading = RwSignal::new(true);

    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let sid = session_id.get();
            if sid == 0 {
                return;
            }
            loading.set(true);
            leptos::task::spawn_local(async move {
                match crate::server::api::get_session_template(sid).await {
                    Ok(tmpl) => template.set(tmpl),
                    Err(e) => log::error!("Failed to load template: {e}"),
                }
                let _ = crate::server::api::ensure_character_defaults(character_id).await;
                match crate::server::api::list_characters(sid).await {
                    Ok(chars) => {
                        let found = chars.into_iter().find(|c| c.id == character_id);
                        character.set(found);
                    }
                    Err(e) => log::error!("Failed to load character: {e}"),
                }
                loading.set(false);
            });
        });
    }

    view! {
        <div class="character-sheet-panel">
            {move || {
                if loading.get() && character.get().is_none() {
                    view! { <p class="loading-text">"Loading..."</p> }.into_any()
                } else if let Some(char_data) = character.get() {
                    let tmpl = template.get();
                    view! {
                        <CharacterEditor
                            character=char_data
                            template=tmpl
                        />
                    }.into_any()
                } else {
                    view! { <p>"Character not found"</p> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn CharacterEditor(character: CharacterInfo, template: Option<TemplateInfo>) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let send = ctx.send;
    let char_id = character.id;

    let fields = template
        .as_ref()
        .map(|t| t.fields.clone())
        .unwrap_or_default();

    // Separate fields into display groups
    let ability_scores: Vec<TemplateField> = fields
        .iter()
        .filter(|f| f.category == "Ability Scores")
        .cloned()
        .collect();
    let combat_fields: Vec<TemplateField> = fields
        .iter()
        .filter(|f| f.category == "Combat")
        .cloned()
        .collect();
    let info_fields: Vec<TemplateField> = fields
        .iter()
        .filter(|f| f.category == "Info")
        .cloned()
        .collect();
    // All other categories
    let other_categories: Vec<(String, Vec<TemplateField>)> = {
        let mut cats: Vec<(String, Vec<TemplateField>)> = Vec::new();
        for field in &fields {
            if matches!(
                field.category.as_str(),
                "Ability Scores" | "Combat" | "Info"
            ) {
                continue;
            }
            if let Some(cat) = cats.iter_mut().find(|(c, _)| c == &field.category) {
                cat.1.push(field.clone());
            } else {
                cats.push((field.category.clone(), vec![field.clone()]));
            }
        }
        cats
    };

    let data = character.data.clone();
    let resources = character.resources.clone();
    let show_portrait_picker = RwSignal::new(false);
    let (portrait, set_portrait) = signal(character.portrait_url.clone());

    let on_portrait_select = {
        let char_id_for_portrait = char_id;
        Callback::new(move |media: crate::models::MediaInfo| {
            let url = media.url.clone();
            set_portrait.set(Some(url.clone()));
            show_portrait_picker.set(false);
            leptos::task::spawn_local(async move {
                let _ =
                    crate::server::api::update_character_portrait(char_id_for_portrait, Some(url))
                        .await;
            });
        })
    };

    // Extract info fields for the header subtitle
    let race = data
        .get("race")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let class = data
        .get("class")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let level = data.get("level").and_then(|v| v.as_f64()).map(|v| v as i32);
    let subtitle = {
        let mut parts = Vec::new();
        if !race.is_empty() {
            parts.push(race.clone());
        }
        if !class.is_empty() {
            if let Some(lvl) = level {
                parts.push(format!("{} {}", class, lvl));
            } else {
                parts.push(class.clone());
            }
        }
        parts.join(" ")
    };

    let data_for_abilities = data.clone();
    let data_for_combat = data.clone();
    let data_for_info = data.clone();
    let data_for_other = data.clone();

    view! {
        <div class="char-editor">
            // Header: portrait + name + subtitle
            <div class="char-header">
                <div
                    class="char-portrait"
                    on:click=move |_| show_portrait_picker.set(true)
                    style="cursor: pointer;"
                >
                    {move || {
                        if let Some(url) = portrait.get() {
                            view! { <img src=url alt="portrait" class="portrait-img" /> }.into_any()
                        } else {
                            view! { <div class="portrait-placeholder">"Set Portrait"</div> }.into_any()
                        }
                    }}
                </div>
                <div class="char-header-text">
                    <h4>{character.name.clone()}</h4>
                    {(!subtitle.is_empty()).then(|| view! {
                        <span class="char-subtitle">{subtitle}</span>
                    })}
                </div>
            </div>
            <crate::components::media_browser::MediaBrowser
                on_select=on_portrait_select
                filter_type="image".to_string()
                show=show_portrait_picker
            />

            // HP / Resource bars (prominent, with +/- controls)
            <div class="char-resources">
                <For
                    each=move || resources.clone()
                    key=|r| r.id
                    let:resource
                >
                    <ResourceBar resource=resource />
                </For>
            </div>

            // Roll Initiative button (between resources and ability scores)
            {
                let initiative_locked = ctx.initiative_locked;
                let roll_char_id = char_id;
                let roll_init = move |_| {
                    send.with_value(|f| {
                        if let Some(f) = f {
                            f(ClientMessage::RollCharacterInitiative {
                                character_id: roll_char_id,
                            });
                        }
                    });
                };
                view! {
                    <button
                        class="btn-roll-initiative"
                        class:disabled=move || initiative_locked.get()
                        disabled=move || initiative_locked.get()
                        on:click=roll_init
                        title=move || if initiative_locked.get() { "Initiative is locked".to_string() } else { "Roll d20 + DEX mod + Initiative bonus".to_string() }
                    >
                        {move || if initiative_locked.get() { "Initiative Locked" } else { "Roll Initiative" }}
                    </button>
                }
            }

            // Ability Scores (compact grid)
            {(!ability_scores.is_empty()).then(|| {
                let scores = ability_scores.clone();
                let data = data_for_abilities.clone();
                view! {
                    <div class="char-category">
                        <h5>"Ability Scores"</h5>
                        <div class="ability-grid">
                            <For
                                each=move || scores.clone()
                                key=|f| f.name.clone()
                                let:field
                            >
                                {
                                    let val = data
                                        .get(&field.name)
                                        .or(Some(&field.default))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(10.0) as i32;
                                    let modifier = (val - 10) / 2;
                                    let mod_str = if modifier >= 0 {
                                        format!("+{modifier}")
                                    } else {
                                        format!("{modifier}")
                                    };
                                    // Short label: first 3 chars
                                    let short = field.label.chars().take(3).collect::<String>().to_uppercase();
                                    view! {
                                        <div class="ability-score">
                                            <span class="ability-label">{short}</span>
                                            <span class="ability-value">{val}</span>
                                            <span class="ability-mod">{mod_str}</span>
                                        </div>
                                    }
                                }
                            </For>
                        </div>
                    </div>
                }
            })}

            // Combat stats (compact row)
            {(!combat_fields.is_empty()).then(|| {
                let fields = combat_fields.clone();
                let data = data_for_combat.clone();
                view! {
                    <div class="char-category">
                        <h5>"Combat"</h5>
                        <div class="combat-stats">
                            <For
                                each=move || fields.clone()
                                key=|f| f.name.clone()
                                let:field
                            >
                                {
                                    let val = data
                                        .get(&field.name)
                                        .or(Some(&field.default))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0) as i32;
                                    view! {
                                        <div class="combat-stat">
                                            <span class="combat-stat-value">{val}</span>
                                            <span class="combat-stat-label">{field.label.clone()}</span>
                                        </div>
                                    }
                                }
                            </For>
                        </div>
                    </div>
                }
            })}

            // Info fields (editable)
            {(!info_fields.is_empty()).then(|| {
                let fields = info_fields.clone();
                let data = data_for_info.clone();
                view! {
                    <div class="char-category">
                        <h5>"Info"</h5>
                        <div class="char-fields">
                            <For
                                each=move || fields.clone()
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
            })}

            // Other categories (Skills, Equipment, Spells, Notes, etc.)
            <For
                each=move || other_categories.clone()
                key=|(cat, _)| cat.clone()
                let:item
            >
                {
                    let (cat_name, cat_fields) = item;
                    let data = data_for_other.clone();
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
    let (amount, set_amount) = signal(1i32);
    let (prev_value, set_prev_value) = signal(Option::<i32>::None);

    let apply_change = move |delta: i32| {
        let old = current.get();
        let amt = amount.get().max(1);
        let new_val = (old + delta * amt).max(0).min(max);
        if new_val != old {
            set_prev_value.set(Some(old));
            set_current.set(new_val);
            leptos::task::spawn_local(async move {
                let _ = crate::server::api::update_character_resource(rid, new_val).await;
            });
        }
    };

    let undo = move |_| {
        if let Some(old) = prev_value.get() {
            let restore = old.max(0).min(max);
            set_current.set(restore);
            set_prev_value.set(None);
            leptos::task::spawn_local(async move {
                let _ = crate::server::api::update_character_resource(rid, restore).await;
            });
        }
    };

    let is_hp = {
        let name_lower = resource.name.to_lowercase();
        name_lower.contains("hp") || name_lower.contains("hit points")
    };

    view! {
        <div class="resource-bar" class:resource-hp=is_hp>
            <div class="resource-header">
                <span class="resource-name">{resource.name.clone()}</span>
                <span class="resource-value">{move || current.get()} " / " {max}</span>
            </div>
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
            <div class="resource-controls">
                <button
                    class="resource-btn resource-btn-minus"
                    on:click=move |_| apply_change(-1)
                    data-tooltip="Subtract"
                >"-"</button>
                <input
                    type="number"
                    class="resource-amount"
                    prop:value=move || amount.get().to_string()
                    on:input=move |ev| {
                        if let Ok(n) = event_target_value(&ev).parse::<i32>() {
                            set_amount.set(n.max(1));
                        }
                    }
                    min="1"
                />
                <button
                    class="resource-btn resource-btn-plus"
                    on:click=move |_| apply_change(1)
                    data-tooltip="Add"
                >"+"</button>
            </div>
            {move || prev_value.get().map(|old| view! {
                <button
                    class="resource-undo"
                    on:click=undo
                    title=format!("Undo (restore to {})", old)
                >
                    "undo (" {old} ")"
                </button>
            })}
        </div>
    }
}

type SendHandle =
    StoredValue<Option<Box<dyn Fn(ClientMessage)>>, leptos::reactive::owner::LocalStorage>;

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
