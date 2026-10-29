use leptos::prelude::*;

use crate::models::{ACCESS_LEVEL_ADMIN, ACCESS_LEVEL_USER, is_admin_level};
use crate::server::api::{
    AdminAddUser, AdminDeleteMedia, AdminRemoveSession, AdminRemoveUser, AdminRenameSession,
    AdminSetSessionActive, AdminUpdateUser, admin_list_sessions, admin_list_users,
    current_user_is_admin, list_media,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Users,
    Sessions,
    Media,
}

/// Server administration console. The whole page is gated on the caller being an
/// administrator; every underlying server function enforces the same check, so a
/// non-admin who reaches this route sees only an access-denied notice.
#[component]
pub fn AdminPage() -> impl IntoView {
    let is_admin = Resource::new(|| (), |_| current_user_is_admin());
    let active = RwSignal::new(Tab::Users);

    view! {
        <div class="admin-page">
            <h1>"Server Administration"</h1>
            <Suspense fallback=move || view! { <p>"Checking access..."</p> }>
                {move || {
                    is_admin
                        .get()
                        .map(|result| match result {
                            Ok(true) => {
                                view! {
                                    <div class="admin-tabs">
                                        <button
                                            class:active=move || active.get() == Tab::Users
                                            on:click=move |_| active.set(Tab::Users)
                                        >
                                            "Users"
                                        </button>
                                        <button
                                            class:active=move || active.get() == Tab::Sessions
                                            on:click=move |_| active.set(Tab::Sessions)
                                        >
                                            "Sessions"
                                        </button>
                                        <button
                                            class:active=move || active.get() == Tab::Media
                                            on:click=move |_| active.set(Tab::Media)
                                        >
                                            "Media"
                                        </button>
                                    </div>
                                    <Show when=move || active.get() == Tab::Users>
                                        <UsersTab />
                                    </Show>
                                    <Show when=move || active.get() == Tab::Sessions>
                                        <SessionsTab />
                                    </Show>
                                    <Show when=move || active.get() == Tab::Media>
                                        <MediaTab />
                                    </Show>
                                }
                                    .into_any()
                            }
                            Ok(false) => {
                                view! {
                                    <p class="error">
                                        "Access denied. You must be an administrator to view this page."
                                    </p>
                                    <p>
                                        <a href="/sessions">"Back to sessions"</a>
                                    </p>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! { <p class="error">{format!("Error: {e}")}</p> }.into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

/// Render the error from a resolved `ServerAction` value, if any.
fn action_error(value: Option<Result<(), ServerFnError>>) -> Option<String> {
    match value {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    }
}

#[component]
fn UsersTab() -> impl IntoView {
    let users = Resource::new(|| (), |_| admin_list_users());
    let add = ServerAction::<AdminAddUser>::new();
    let update = ServerAction::<AdminUpdateUser>::new();
    let remove = ServerAction::<AdminRemoveUser>::new();

    // Refetch whenever any mutation resolves.
    Effect::new(move || {
        add.version().get();
        update.version().get();
        remove.version().get();
        users.refetch();
    });

    let status = move || {
        action_error(update.value().get())
            .or_else(|| action_error(remove.value().get()))
            .or_else(|| match add.value().get() {
                Some(Err(e)) => Some(e.to_string()),
                _ => None,
            })
    };

    view! {
        <div class="admin-section">
            <h2>"User Accounts"</h2>
            <Show when=move || status().is_some()>
                <p class="error">{move || status().unwrap_or_default()}</p>
            </Show>

            <details class="admin-add">
                <summary>"Add user"</summary>
                <leptos::form::ActionForm action=add>
                    <input type="text" name="username" placeholder="username" required />
                    <input type="text" name="display_name" placeholder="display name" required />
                    <input type="email" name="email" placeholder="email" required />
                    <input
                        type="password"
                        name="password"
                        placeholder="password"
                        minlength="8"
                        required
                    />
                    <button type="submit">"Create"</button>
                </leptos::form::ActionForm>
            </details>

            <Suspense fallback=move || view! { <p>"Loading users..."</p> }>
                {move || {
                    users
                        .get()
                        .map(|result| match result {
                            Err(e) => view! { <p class="error">{format!("Error: {e}")}</p> }.into_any(),
                            Ok(list) => {
                                view! {
                                    <table class="admin-table">
                                        <thead>
                                            <tr>
                                                <th>"ID"</th>
                                                <th>"Username"</th>
                                                <th>"Display name"</th>
                                                <th>"Email"</th>
                                                <th>"Admin"</th>
                                                <th>"Locked"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list
                                                .into_iter()
                                                .map(|u| {
                                                    let id = u.id;
                                                    let username = u.username.clone();
                                                    let display_name = RwSignal::new(u.display_name.clone());
                                                    let email = RwSignal::new(u.email.clone());
                                                    let admin = RwSignal::new(is_admin_level(u.access_level));
                                                    let locked = RwSignal::new(u.locked);
                                                    view! {
                                                        <tr>
                                                            <td>{id}</td>
                                                            <td>{username}</td>
                                                            <td>
                                                                <input
                                                                    type="text"
                                                                    prop:value=move || display_name.get()
                                                                    on:input=move |ev| {
                                                                        display_name.set(event_target_value(&ev))
                                                                    }
                                                                />
                                                            </td>
                                                            <td>
                                                                <input
                                                                    type="email"
                                                                    prop:value=move || email.get()
                                                                    on:input=move |ev| email.set(event_target_value(&ev))
                                                                />
                                                            </td>
                                                            <td>
                                                                <input
                                                                    type="checkbox"
                                                                    prop:checked=move || admin.get()
                                                                    on:change=move |ev| admin.set(event_target_checked(&ev))
                                                                />
                                                            </td>
                                                            <td>
                                                                <input
                                                                    type="checkbox"
                                                                    prop:checked=move || locked.get()
                                                                    on:change=move |ev| locked.set(event_target_checked(&ev))
                                                                />
                                                            </td>
                                                            <td>
                                                                <button on:click=move |_| {
                                                                    update
                                                                        .dispatch(AdminUpdateUser {
                                                                            id,
                                                                            display_name: display_name.get(),
                                                                            email: email.get(),
                                                                            locked: locked.get(),
                                                                            access_level: if admin.get() {
                                                                                ACCESS_LEVEL_ADMIN
                                                                            } else {
                                                                                ACCESS_LEVEL_USER
                                                                            },
                                                                        });
                                                                }>"Save"</button>
                                                                <button
                                                                    class="danger"
                                                                    on:click=move |_| {
                                                                        if leptos::prelude::window()
                                                                            .confirm_with_message(
                                                                                &format!("Remove user #{id}? This cannot be undone."),
                                                                            )
                                                                            .unwrap_or(false)
                                                                        {
                                                                            remove.dispatch(AdminRemoveUser { id });
                                                                        }
                                                                    }
                                                                >
                                                                    "Remove"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn SessionsTab() -> impl IntoView {
    let sessions = Resource::new(|| (), |_| admin_list_sessions());
    let rename = ServerAction::<AdminRenameSession>::new();
    let set_active = ServerAction::<AdminSetSessionActive>::new();
    let remove = ServerAction::<AdminRemoveSession>::new();

    Effect::new(move || {
        rename.version().get();
        set_active.version().get();
        remove.version().get();
        sessions.refetch();
    });

    let status = move || {
        action_error(rename.value().get())
            .or_else(|| action_error(set_active.value().get()))
            .or_else(|| action_error(remove.value().get()))
    };

    view! {
        <div class="admin-section">
            <h2>"Game Sessions"</h2>
            <Show when=move || status().is_some()>
                <p class="error">{move || status().unwrap_or_default()}</p>
            </Show>
            <Suspense fallback=move || view! { <p>"Loading sessions..."</p> }>
                {move || {
                    sessions
                        .get()
                        .map(|result| match result {
                            Err(e) => view! { <p class="error">{format!("Error: {e}")}</p> }.into_any(),
                            Ok(list) => {
                                view! {
                                    <table class="admin-table">
                                        <thead>
                                            <tr>
                                                <th>"ID"</th>
                                                <th>"Name"</th>
                                                <th>"GM"</th>
                                                <th>"Players"</th>
                                                <th>"Created"</th>
                                                <th>"Status"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {list
                                                .into_iter()
                                                .map(|s| {
                                                    let id = s.id;
                                                    let active = s.active;
                                                    let name = RwSignal::new(s.name.clone());
                                                    view! {
                                                        <tr>
                                                            <td>{id}</td>
                                                            <td>
                                                                <input
                                                                    type="text"
                                                                    prop:value=move || name.get()
                                                                    on:input=move |ev| name.set(event_target_value(&ev))
                                                                />
                                                            </td>
                                                            <td>{s.gm_username}</td>
                                                            <td>{s.player_count}</td>
                                                            <td>{s.created_at}</td>
                                                            <td>{if active { "active" } else { "archived" }}</td>
                                                            <td>
                                                                <button on:click=move |_| {
                                                                    rename
                                                                        .dispatch(AdminRenameSession {
                                                                            id,
                                                                            name: name.get(),
                                                                        });
                                                                }>"Rename"</button>
                                                                <button on:click=move |_| {
                                                                    set_active
                                                                        .dispatch(AdminSetSessionActive {
                                                                            id,
                                                                            active: !active,
                                                                        });
                                                                }>{if active { "Archive" } else { "Unarchive" }}</button>
                                                                <button
                                                                    class="danger"
                                                                    on:click=move |_| {
                                                                        if leptos::prelude::window()
                                                                            .confirm_with_message(
                                                                                &format!(
                                                                                    "Permanently remove session #{id} and all its data? This cannot be undone.",
                                                                                ),
                                                                            )
                                                                            .unwrap_or(false)
                                                                        {
                                                                            remove.dispatch(AdminRemoveSession { id });
                                                                        }
                                                                    }
                                                                >
                                                                    "Remove"
                                                                </button>
                                                            </td>
                                                        </tr>
                                                    }
                                                })
                                                .collect_view()}
                                        </tbody>
                                    </table>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn MediaTab() -> impl IntoView {
    let search = RwSignal::new(String::new());
    let media = Resource::new(
        move || search.get(),
        |term| {
            let term = if term.trim().is_empty() {
                None
            } else {
                Some(term)
            };
            list_media(None, term, None)
        },
    );
    let remove = ServerAction::<AdminDeleteMedia>::new();

    Effect::new(move || {
        remove.version().get();
        media.refetch();
    });

    let status = move || action_error(remove.value().get());

    view! {
        <div class="admin-section">
            <h2>"Media Storage"</h2>
            <Show when=move || status().is_some()>
                <p class="error">{move || status().unwrap_or_default()}</p>
            </Show>
            <input
                type="search"
                placeholder="Search tags..."
                prop:value=move || search.get()
                on:input=move |ev| search.set(event_target_value(&ev))
            />
            <Suspense fallback=move || view! { <p>"Loading media..."</p> }>
                {move || {
                    media
                        .get()
                        .map(|result| match result {
                            Err(e) => view! { <p class="error">{format!("Error: {e}")}</p> }.into_any(),
                            Ok(list) => {
                                if list.is_empty() {
                                    return view! { <p>"No media found."</p> }.into_any();
                                }
                                view! {
                                    <div class="admin-media-grid">
                                        {list
                                            .into_iter()
                                            .map(|m| {
                                                let id = m.id;
                                                let url = m.url.clone();
                                                let is_image = m.media_type == "image";
                                                let size_kb = m.size_bytes / 1024;
                                                let tags = m.tags.join(", ");
                                                view! {
                                                    <div class="admin-media-item">
                                                        <a href=url.clone() target="_blank">
                                                            {if is_image {
                                                                view! { <img src=url.clone() alt="media preview" /> }
                                                                    .into_any()
                                                            } else {
                                                                view! { <span class="media-file">{m.content_type}</span> }
                                                                    .into_any()
                                                            }}
                                                        </a>
                                                        <div class="media-meta">
                                                            <span>{format!("#{id} · {size_kb} KB")}</span>
                                                            <span class="media-tags">{tags}</span>
                                                        </div>
                                                        <button
                                                            class="danger"
                                                            on:click=move |_| {
                                                                if leptos::prelude::window()
                                                                    .confirm_with_message(
                                                                        &format!("Delete media #{id} from storage?"),
                                                                    )
                                                                    .unwrap_or(false)
                                                                {
                                                                    remove.dispatch(AdminDeleteMedia { media_id: id });
                                                                }
                                                            }
                                                        >
                                                            "Delete"
                                                        </button>
                                                    </div>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
