use leptos::prelude::*;

#[component]
pub fn InventoryPanel() -> impl IntoView {
    view! {
        <div class="inventory-panel">
            <h3>"Inventory"</h3>
            <p class="placeholder">"Party inventory will appear here."</p>
        </div>
    }
}
