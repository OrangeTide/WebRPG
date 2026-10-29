use leptos::prelude::*;

use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// GM-only panel listing the session's members and letting the owner grant or
/// revoke co-GM status. Visible to all GMs; the grant/revoke controls appear
/// only for the session owner (`can_manage_gms`).
#[component]
pub fn MembersPanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let members = ctx.members;
    let can_manage = ctx.can_manage_gms;
    let send = ctx.send;

    // Ask for the member list when the panel mounts.
    Effect::new(move |_| {
        send.send(ClientMessage::ListMembers);
    });

    view! {
        <div class="members-panel">
            <p class="members-hint">
                "GMs can place tokens and manage creatures, fog, and maps. "
                "The owner can grant or revoke GM."
            </p>
            <ul class="members-list">
                <For
                    each=move || members.get()
                    key=|m| (m.user_id, m.is_gm, m.is_owner)
                    children=move |m| {
                        let uid = m.user_id;
                        let is_owner = m.is_owner;
                        let role = if m.is_owner {
                            "owner"
                        } else if m.is_gm {
                            "GM"
                        } else {
                            "player"
                        };
                        let make_gm = !m.is_gm;
                        let btn_label = if m.is_gm { "Remove GM" } else { "Make GM" };
                        view! {
                            <li class="member-item">
                                <span class="member-name">{m.username.clone()}</span>
                                <span class=format!("member-role role-{role}")>{role}</span>
                                <Show when=move || can_manage.get() && !is_owner>
                                    <button on:click=move |_| {
                                        send.send(ClientMessage::SetMemberGm {
                                            user_id: uid,
                                            make_gm,
                                        });
                                    }>{btn_label}</button>
                                </Show>
                            </li>
                        }
                    }
                />
            </ul>
        </div>
    }
}
