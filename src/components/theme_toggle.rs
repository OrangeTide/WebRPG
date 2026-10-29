//! Light/dark theme toggle.
//!
//! The effective theme is resolved before paint by a small inline script in
//! the document head (see `app::shell`), which sets `data-theme` on the root
//! element from a stored preference or, failing that, the OS color scheme.
//! This component reads that attribute to show the right icon and flips it on
//! click, persisting the choice to localStorage.

use leptos::prelude::*;

/// localStorage key holding the user's explicit theme choice ("light"/"dark").
/// Absent means "follow the OS setting".
#[cfg(feature = "hydrate")]
const THEME_STORAGE_KEY: &str = "webrpg_theme";

/// Read the theme currently applied to the document root, if any.
#[cfg(feature = "hydrate")]
fn current_is_light() -> Option<bool> {
    let el = web_sys::window()?.document()?.document_element()?;
    match el.get_attribute("data-theme").as_deref() {
        Some("light") => Some(true),
        Some("dark") => Some(false),
        _ => None,
    }
}

/// Apply a theme to the document root and persist the choice.
#[cfg(feature = "hydrate")]
fn apply_theme(light: bool) {
    let value = if light { "light" } else { "dark" };
    if let Some(win) = web_sys::window() {
        if let Some(el) = win.document().and_then(|d| d.document_element()) {
            let _ = el.set_attribute("data-theme", value);
        }
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item(THEME_STORAGE_KEY, value);
        }
    }
}

/// A button that toggles between the dark and light themes.
#[component]
pub fn ThemeToggle() -> impl IntoView {
    // Default to dark for the server-rendered markup; the client corrects this
    // from the actual applied theme once hydrated.
    let is_light = RwSignal::new(false);

    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(light) = current_is_light() {
            is_light.set(light);
        }
    });

    let toggle = move |_| {
        let next = !is_light.get_untracked();
        #[cfg(feature = "hydrate")]
        apply_theme(next);
        is_light.set(next);
    };

    view! {
        <button
            class="theme-toggle"
            on:click=toggle
            aria-label="Toggle light and dark mode"
            title=move || if is_light.get() { "Switch to dark mode" } else { "Switch to light mode" }
        >
            {move || if is_light.get() { "\u{2600}" } else { "\u{1F319}" }}
        </button>
    }
}
