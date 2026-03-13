use leptos::prelude::*;

#[component]
pub fn ChatPanel() -> impl IntoView {
    view! {
        <div class="chat-panel">
            <h3>"Chat"</h3>
            <div class="chat-messages">
                <p class="placeholder">"Chat messages will appear here."</p>
            </div>
            <div class="chat-input">
                <input type="text" placeholder="Type a message or roll dice (e.g. 2d6+3)" />
                <button>"Send"</button>
            </div>
        </div>
    }
}
