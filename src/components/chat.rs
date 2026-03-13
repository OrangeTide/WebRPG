use leptos::prelude::*;

use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Check for dice notation: NdN, NdN+N, NdN-N, dN
fn is_dice_roll(input: &str) -> bool {
    let s = input.trim().to_lowercase();
    let base = if let Some(pos) = s.rfind('+') {
        &s[..pos]
    } else if let Some(pos) = s.rfind('-') {
        if pos == 0 {
            return false;
        }
        &s[..pos]
    } else {
        &s
    };
    let parts: Vec<&str> = base.split('d').collect();
    if parts.len() != 2 {
        return false;
    }
    let count_ok = parts[0].is_empty() || parts[0].parse::<i32>().is_ok();
    let sides_ok = parts[1].parse::<i32>().is_ok();
    count_ok && sides_ok
}

#[component]
pub fn ChatPanel() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let chat_messages = ctx.chat_messages;
    let send_handle = ctx.send;

    let (input, set_input) = signal(String::new());

    let send_ws = move |msg: ClientMessage| {
        send_handle.with_value(|f| {
            if let Some(f) = f {
                f(msg);
            }
        });
    };

    let on_submit = move |_ev: leptos::ev::Event| {
        let text = input.get();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        if is_dice_roll(&trimmed) {
            send_ws(ClientMessage::RollDice {
                expression: trimmed,
            });
        } else {
            send_ws(ClientMessage::ChatMessage { message: trimmed });
        }
        set_input.set(String::new());
    };

    let on_keydown = {
        let on_submit = on_submit.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                on_submit(ev.into());
            }
        }
    };

    view! {
        <div class="chat-panel">
            <h3>"Chat"</h3>
            <div class="chat-messages">
                <For
                    each=move || chat_messages.get()
                    key=|msg| (msg.id, msg.message.clone())
                    let:msg
                >
                    <div class=move || {
                        if msg.is_dice_roll { "chat-msg dice-roll" } else { "chat-msg" }
                    }>
                        <span class="chat-username">{msg.username.clone()}</span>
                        <span class="chat-text">{msg.message.clone()}</span>
                    </div>
                </For>
            </div>
            <div class="chat-input">
                <input
                    type="text"
                    placeholder="Type a message or roll dice (e.g. 2d6+3)"
                    prop:value=input
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    on:keydown=on_keydown
                />
                <button on:click=move |ev| on_submit(ev.into())>"Send"</button>
            </div>
        </div>
    }
}
