//! The Modules window: browse installed module packs, install them into the
//! session, hand out pregens and item cards, and run the adventure.
//!
//! Everything the GM should not share lives behind the GM check: the room key,
//! the bestiary, and the tables come from `get_adventure_module`, which the
//! server refuses to anyone else. Players get the handouts instead.

use leptos::prelude::*;

use crate::modules::{AdventureModule, ModuleKind, ModuleSummary};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Which part of the window is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Library,
    Pregens,
    Cards,
    Rooms,
    Bestiary,
    Tables,
}

#[component]
pub fn ModulesPanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let session_id = ctx.session_id;
    let is_gm = ctx.is_gm;
    let modules = ctx.modules;
    let send = ctx.send;

    let (tab, set_tab) = signal(Tab::Library);
    let (status, set_status) = signal(String::new());
    // Bumped after an install so the content resources refetch.
    let installed_revision = RwSignal::new(0u32);
    // Who receives dealt cards and pregens.
    let (deal_to, set_deal_to) = signal(Option::<i32>::None);

    let available = Resource::new(
        move || installed_revision.get(),
        |_| async move {
            crate::server::modules_api::list_modules()
                .await
                .unwrap_or_default()
        },
    );

    let characters = Resource::new(
        move || (session_id.get(), ctx.character_revision.get()),
        |(sid, _)| async move {
            if sid == 0 {
                return Vec::new();
            }
            crate::server::api::list_characters(sid)
                .await
                .unwrap_or_default()
        },
    );

    // The adventure content, as much of it as this user is allowed to see.
    let adventure = Resource::new(
        move || {
            (
                session_id.get(),
                modules.get().adventure.map(|a| a.id),
                is_gm.get(),
                installed_revision.get(),
            )
        },
        |(sid, module_id, gm, _)| async move {
            let module_id = module_id?;
            if sid == 0 {
                return None;
            }
            let result = if gm {
                crate::server::modules_api::get_adventure_module(sid, module_id).await
            } else {
                crate::server::modules_api::get_adventure_handouts(sid, module_id).await
            };
            match result {
                Ok(a) => Some(a),
                Err(e) => {
                    log::warn!("Failed to load adventure module: {e}");
                    None
                }
            }
        },
    );

    // System-level tables, such as character creation.
    let system_tables = Resource::new(
        move || (modules.get().system.map(|s| s.id), installed_revision.get()),
        |(module_id, _)| async move {
            let module_id = module_id?;
            crate::server::modules_api::get_system_module(module_id)
                .await
                .ok()
                .map(|s| s.tables)
        },
    );

    let install = move |module_id: String| {
        let sid = session_id.get();
        set_status.set(format!("Installing {module_id}..."));
        leptos::task::spawn_local(async move {
            match crate::server::modules_api::install_module(sid, module_id).await {
                Ok(report) => {
                    let mut parts = vec![format!("Installed {}", report.module_name)];
                    if report.creatures_added > 0 {
                        parts.push(format!("{} creatures", report.creatures_added));
                    }
                    if report.maps_added > 0 {
                        parts.push(format!("{} maps", report.maps_added));
                    }
                    parts.extend(report.warnings.iter().cloned());
                    set_status.set(parts.join(". "));

                    if let Ok(m) = crate::server::modules_api::get_session_modules(sid).await {
                        modules.set(m);
                    }
                    installed_revision.update(|r| *r += 1);
                }
                Err(e) => set_status.set(format!("Install failed: {e}")),
            }
        });
    };

    let post_to_chat = move |message: String| {
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::ChatMessage { message });
            }
        });
    };

    let roll_die = move |die: String| {
        let expression = if die.starts_with('d') {
            format!("1{die}")
        } else {
            die
        };
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::RollDice { expression });
            }
        });
    };

    view! {
        <div class="modules-panel">
            <div class="modules-tabs">
                {[
                    (Tab::Library, "Library"),
                    (Tab::Pregens, "Characters"),
                    (Tab::Cards, "Item cards"),
                    (Tab::Rooms, "Rooms"),
                    (Tab::Bestiary, "Bestiary"),
                    (Tab::Tables, "Tables"),
                ]
                    .into_iter()
                    .map(|(t, label)| {
                        // The room key, bestiary, and tables are GM material.
                        let gm_only = matches!(t, Tab::Rooms | Tab::Bestiary);
                        view! {
                            <Show when=move || !gm_only || is_gm.get()>
                                <button
                                    class="modules-tab"
                                    class:active=move || tab.get() == t
                                    on:click=move |_| set_tab.set(t)
                                >
                                    {label}
                                </button>
                            </Show>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="modules-status">
                {move || {
                    let m = modules.get();
                    let system = m.system.map(|s| s.name).unwrap_or("no system".to_string());
                    let adventure = m
                        .adventure
                        .map(|a| a.name)
                        .unwrap_or("no adventure".to_string());
                    format!("{system} - {adventure}")
                }}
            </div>
            {move || {
                let text = status.get();
                (!text.is_empty()).then(|| view! { <div class="modules-message">{text}</div> })
            }}

            // Who gets pregens and dealt cards
            <Show when=move || matches!(tab.get(), Tab::Pregens | Tab::Cards)>
                <div class="modules-dealto">
                    <span>"Deal to"</span>
                    <select on:change=move |ev| {
                        set_deal_to.set(event_target_value(&ev).parse::<i32>().ok())
                    }>
                        <option value="">"Party"</option>
                        <Suspense fallback=|| ()>
                            {move || {
                                characters
                                    .get()
                                    .map(|chars| {
                                        chars
                                            .into_iter()
                                            .map(|c| {
                                                view! { <option value=c.id.to_string()>{c.name}</option> }
                                            })
                                            .collect_view()
                                    })
                            }}
                        </Suspense>
                    </select>
                </div>
            </Show>

            <div class="modules-body">
                {move || match tab.get() {
                    Tab::Library => {
                        view! {
                            <Suspense fallback=move || {
                                view! { <p class="loading-text">"Loading modules..."</p> }
                            }>
                                {move || {
                                    available
                                        .get()
                                        .map(|list| {
                                            if list.is_empty() {
                                                return view! {
                                                    <p class="modules-empty">
                                                        "No modules found. Packs live in the server's modules directory."
                                                    </p>
                                                }
                                                    .into_any();
                                            }
                                            list.into_iter()
                                                .map(|summary| {
                                                    view! {
                                                        <ModuleCard
                                                            summary=summary
                                                            is_gm=is_gm
                                                            on_install=Callback::new(move |id: String| install(id))
                                                        />
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        })
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                    Tab::Pregens => {
                        view! {
                            <Suspense fallback=|| ()>
                                {move || {
                                    let Some(Some(adv)) = adventure.get() else {
                                        return view! {
                                            <p class="modules-empty">"No adventure installed."</p>
                                        }
                                            .into_any();
                                    };
                                    let module_id = adv.summary.id.clone();
                                    adv.pregens
                                        .iter()
                                        .map(|p| {
                                            let module_id = module_id.clone();
                                            let pregen_id = p.id.clone();
                                            let items = p
                                                .items
                                                .iter()
                                                .map(|i| i.name.clone())
                                                .collect::<Vec<_>>()
                                                .join(", ");
                                            let take = move |_| {
                                                let sid = session_id.get();
                                                let module_id = module_id.clone();
                                                let pregen_id = pregen_id.clone();
                                                let revision = ctx.character_revision;
                                                set_status.set("Creating character...".to_string());
                                                leptos::task::spawn_local(async move {
                                                    match crate::server::modules_api::create_character_from_pregen(
                                                            sid,
                                                            module_id,
                                                            pregen_id,
                                                            None,
                                                        )
                                                        .await
                                                    {
                                                        Ok(c) => {
                                                            set_status.set(format!("Created {}", c.name));
                                                            revision.update(|r| *r += 1);
                                                        }
                                                        Err(e) => set_status.set(format!("Failed: {e}")),
                                                    }
                                                });
                                            };
                                            view! {
                                                <div class="module-entry">
                                                    <div class="module-entry-head">
                                                        <strong>{p.name.clone()}</strong>
                                                        <button class="btn-add" on:click=take>"Play this one"</button>
                                                    </div>
                                                    <div class="module-entry-body">{p.summary.clone()}</div>
                                                    {(!items.is_empty())
                                                        .then(|| view! { <div class="module-entry-note">{items}</div> })}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                    Tab::Cards => {
                        view! {
                            <Suspense fallback=|| ()>
                                {move || {
                                    let Some(Some(adv)) = adventure.get() else {
                                        return view! {
                                            <p class="modules-empty">"No adventure installed."</p>
                                        }
                                            .into_any();
                                    };
                                    let module_id = adv.summary.id.clone();
                                    adv.items
                                        .iter()
                                        .map(|item| {
                                            let art = crate::modules::asset_url(&module_id, &item.art);
                                            let name = item.name.clone();
                                            let description = item.note.clone();
                                            let kind = item.kind.clone();
                                            let bonus = item.bonus.clone();
                                            let slots = item.slots;
                                            let uses = item.uses;
                                            let deal = move |_| {
                                                let owner = deal_to.get();
                                                send.with_value(|f| {
                                                    if let Some(f) = f {
                                                        f(ClientMessage::AddInventoryItem {
                                                            name: name.clone(),
                                                            description: description.clone(),
                                                            quantity: 1,
                                                            is_party_item: owner.is_none(),
                                                            kind: kind.clone(),
                                                            bonus: bonus.clone(),
                                                            slots: Some(slots),
                                                            uses,
                                                            owner_character_id: owner,
                                                        });
                                                    }
                                                });
                                            };
                                            view! {
                                                <div class="module-entry">
                                                    <div class="module-entry-head">
                                                        <strong>{item.name.clone()}</strong>
                                                        <span class="module-entry-tag">
                                                            {format!("{} - {} slot(s)", item.kind, item.slots)}
                                                        </span>
                                                        <button class="btn-add" on:click=deal>"Deal"</button>
                                                    </div>
                                                    {art
                                                        .map(|url| {
                                                            view! {
                                                                <img class="module-card-art" src=url alt=item.name.clone() />
                                                            }
                                                        })}
                                                    <div class="module-entry-body">{item.bonus.clone()}</div>
                                                    {(!item.note.is_empty())
                                                        .then({
                                                            let note = item.note.clone();
                                                            move || view! { <div class="module-entry-note">{note}</div> }
                                                        })}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                    Tab::Rooms => {
                        view! {
                            <Suspense fallback=|| ()>
                                {move || {
                                    let Some(Some(adv)) = adventure.get() else {
                                        return view! {
                                            <p class="modules-empty">"No adventure installed."</p>
                                        }
                                            .into_any();
                                    };
                                    adv.rooms
                                        .iter()
                                        .map(|room| {
                                            let card = format!("Room {}. {}", room.number, room.card);
                                            let read_aloud = room.read_aloud.clone();
                                            let deal_card = move |_| post_to_chat(card.clone());
                                            let read = move |_| post_to_chat(read_aloud.clone());
                                            let has_read_aloud = !room.read_aloud.is_empty();
                                            view! {
                                                <div class="module-entry room-entry">
                                                    <div class="module-entry-head">
                                                        <strong>{format!("{}. {}", room.number, room.title)}</strong>
                                                        <button class="btn-add" on:click=deal_card>"Deal card"</button>
                                                        {has_read_aloud
                                                            .then(|| {
                                                                view! {
                                                                    <button class="btn-add" on:click=read>"Read aloud"</button>
                                                                }
                                                            })}
                                                    </div>
                                                    <div class="module-entry-body">{room.card.clone()}</div>
                                                    <div class="module-entry-gm">{room.gm.clone()}</div>
                                                    {(!room.exits.is_empty())
                                                        .then({
                                                            let exits = room.exits.clone();
                                                            move || {
                                                                view! {
                                                                    <div class="module-entry-note">
                                                                        {format!("Exits: {exits}")}
                                                                    </div>
                                                                }
                                                            }
                                                        })}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                    Tab::Bestiary => {
                        view! {
                            <Suspense fallback=|| ()>
                                {move || {
                                    let Some(Some(adv)) = adventure.get() else {
                                        return view! {
                                            <p class="modules-empty">"No adventure installed."</p>
                                        }
                                            .into_any();
                                    };
                                    adv.bestiary
                                        .iter()
                                        .map(|block| {
                                            view! {
                                                <div class="module-entry">
                                                    <div class="module-entry-head">
                                                        <strong>{block.name.clone()}</strong>
                                                        <span class="module-entry-tag">
                                                            {if block.ds > 0 {
                                                                format!("DS {} (HP {})", block.ds, block.hp.unwrap_or(block.ds))
                                                            } else {
                                                                "never fought".to_string()
                                                            }}
                                                        </span>
                                                    </div>
                                                    <div class="module-entry-body">{block.notes.clone()}</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                    Tab::Tables => {
                        view! {
                            <Suspense fallback=|| ()>
                                {move || {
                                    let mut tables = system_tables.get().flatten().unwrap_or_default();
                                    if is_gm.get()
                                        && let Some(Some(adv)) = adventure.get()
                                    {
                                        tables.extend(adv.tables.iter().cloned());
                                    }
                                    if tables.is_empty() {
                                        return view! {
                                            <p class="modules-empty">"No tables in the installed modules."</p>
                                        }
                                            .into_any();
                                    }
                                    tables
                                        .into_iter()
                                        .map(|table| {
                                            let die = table.die.clone();
                                            let has_die = !die.is_empty();
                                            let roll = move |_| roll_die(die.clone());
                                            view! {
                                                <div class="module-table">
                                                    <div class="module-entry-head">
                                                        <strong>{table.name.clone()}</strong>
                                                        {has_die
                                                            .then({
                                                                let label = table.die.clone();
                                                                move || {
                                                                    view! {
                                                                        <button class="btn-add" on:click=roll>
                                                                            {format!("Roll {label}")}
                                                                        </button>
                                                                    }
                                                                }
                                                            })}
                                                    </div>
                                                    {(!table.description.is_empty())
                                                        .then({
                                                            let description = table.description.clone();
                                                            move || {
                                                                view! { <div class="module-entry-note">{description}</div> }
                                                            }
                                                        })}
                                                    {table
                                                        .entries
                                                        .iter()
                                                        .map(|entry| {
                                                            let line = if entry.key.is_empty() {
                                                                entry.text.clone()
                                                            } else {
                                                                format!("{}: {}", entry.key, entry.text)
                                                            };
                                                            let post = line.clone();
                                                            let share = move |_| post_to_chat(post.clone());
                                                            view! {
                                                                <div class="module-table-row">
                                                                    <span class="module-table-key">{entry.key.clone()}</span>
                                                                    <span class="module-table-text">{entry.text.clone()}</span>
                                                                    <button
                                                                        class="btn-add"
                                                                        title="Post this line to chat"
                                                                        on:click=share
                                                                    >
                                                                        "Share"
                                                                    </button>
                                                                </div>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                        .into_any()
                                }}
                            </Suspense>
                        }
                            .into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// One module in the library list, with what it is and who wrote it.
#[component]
fn ModuleCard(
    summary: ModuleSummary,
    is_gm: RwSignal<bool>,
    on_install: Callback<String>,
) -> impl IntoView {
    let id = summary.id.clone();
    let kind = match summary.kind {
        ModuleKind::System => "SYSTEM",
        ModuleKind::Adventure => "ADVENTURE",
    };

    view! {
        <div class="module-entry module-card">
            <div class="module-entry-head">
                <strong>{summary.name.clone()}</strong>
                <span class="module-entry-tag">{kind}</span>
                <Show when=move || is_gm.get()>
                    {
                        let id = id.clone();
                        view! {
                            <button class="btn-add" on:click=move |_| on_install.run(id.clone())>
                                "Install"
                            </button>
                        }
                    }
                </Show>
            </div>
            <div class="module-entry-body">{summary.description.clone()}</div>
            {(!summary.authors.is_empty())
                .then({
                    let authors = summary.authors.join(", ");
                    move || view! { <div class="module-entry-note">{authors}</div> }
                })}
            {(!summary.license.is_empty())
                .then({
                    let license = summary.license.clone();
                    move || view! { <div class="module-entry-note">{license}</div> }
                })}
            {summary
                .links
                .iter()
                .map(|link| {
                    view! {
                        <div class="module-entry-link">
                            <a href=link.url.clone() target="_blank" rel="noopener noreferrer">
                                {link.label.clone()}
                            </a>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Type alias kept close to the panel: adventure content, once loaded.
#[allow(dead_code)]
type LoadedAdventure = Option<AdventureModule>;
