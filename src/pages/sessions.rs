use leptos::prelude::*;
use leptos::form::ActionForm;

use crate::models::SessionInfo;
use crate::server::api::{CreateSession, JoinSession};

#[component]
pub fn SessionsPage() -> impl IntoView {
    let sessions = Resource::new(|| (), |_| crate::server::api::list_sessions());
    let create_action = ServerAction::<CreateSession>::new();
    let join_action = ServerAction::<JoinSession>::new();

    // Refetch sessions after creating one
    Effect::new(move || {
        if let Some(Ok(_)) = create_action.value().get() {
            sessions.refetch();
        }
    });

    view! {
        <div class="sessions-page">
            <h1>"Game Sessions"</h1>

            <div class="create-session">
                <h2>"Create New Session"</h2>
                <ActionForm action=create_action>
                    <input type="text" name="name" placeholder="Session name" required />
                    <button type="submit">"Create"</button>
                </ActionForm>
            </div>

            <div class="session-list">
                <h2>"Available Sessions"</h2>
                <Suspense fallback=move || view! { <p>"Loading sessions..."</p> }>
                    {move || {
                        sessions
                            .get()
                            .map(|result| {
                                match result {
                                    Ok(sessions) => {
                                        if sessions.is_empty() {
                                            view! {
                                                <p>"No active sessions. Create one above!"</p>
                                            }
                                                .into_any()
                                        } else {
                                            view! {
                                                <ul>
                                                    {sessions
                                                        .into_iter()
                                                        .map(|session| {
                                                            view! {
                                                                <SessionListItem
                                                                    session=session
                                                                    join_action=join_action
                                                                />
                                                            }
                                                        })
                                                        .collect_view()}
                                                </ul>
                                            }
                                                .into_any()
                                        }
                                    }
                                    Err(e) => {
                                        view! { <p class="error">{format!("Error: {e}")}</p> }
                                            .into_any()
                                    }
                                }
                            })
                    }}
                </Suspense>
            </div>
        </div>
    }
}

#[allow(unused)]
#[component]
fn SessionListItem(
    session: SessionInfo,
    join_action: ServerAction<JoinSession>,
) -> impl IntoView {
    let session_id = session.id;

    let name = session.name.clone();
    let gm = format!("(GM: {})", session.gm_username);
    let href = format!("/game/{session_id}");

    view! {
        <li class="session-item">
            <span class="session-name">{name}</span>
            <span class="session-gm">" " {gm}</span>
            <a href=href>"Enter"</a>
        </li>
    }
}
