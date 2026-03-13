use leptos::prelude::*;

use crate::components::chat::ChatPanel;
use crate::components::initiative::InitiativeTracker;
use crate::components::inventory::InventoryPanel;
use crate::components::map::MapCanvas;

#[component]
pub fn GamePage() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    let session_id = move || {
        params
            .read()
            .get("id")
            .and_then(|id| id.parse::<i32>().ok())
            .unwrap_or(0)
    };

    view! {
        <div class="game-page">
            <h1>"Game Session #" {session_id}</h1>
            <div class="game-layout">
                <div class="game-main">
                    <MapCanvas />
                </div>
                <div class="game-sidebar">
                    <ChatPanel />
                    <InitiativeTracker />
                    <InventoryPanel />
                </div>
            </div>
        </div>
    }
}
