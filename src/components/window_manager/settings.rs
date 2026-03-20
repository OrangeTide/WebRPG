/// Settings dialog with tabbed interface.
use leptos::prelude::*;

use super::persistence::STATIC_WINDOWS;
use super::{WindowId, default_window_layout};
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

#[cfg(feature = "hydrate")]
use super::persistence::{load_startup_prefs, save_startup_prefs};

/// Settings dialog tabs.
#[derive(Clone, Copy, PartialEq)]
enum SettingsTab {
    Startup,
    Options,
}

/// Settings dialog with tabbed interface.
#[component]
pub(super) fn SettingsDialog(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let active_tab = RwSignal::new(SettingsTab::Startup);
    let ctx = expect_context::<GameContext>();

    // Load current startup prefs into checkbox state.
    // Default: use the layout tier defaults (Map and Chat open for most screens).
    let startup_checked: RwSignal<Vec<(WindowId, RwSignal<bool>)>> = RwSignal::new({
        #[cfg(feature = "hydrate")]
        let saved = load_startup_prefs();
        #[cfg(not(feature = "hydrate"))]
        let saved: Option<Vec<WindowId>> = None;

        STATIC_WINDOWS
            .iter()
            .map(|(id, _title)| {
                let checked = match &saved {
                    Some(open_ids) => open_ids.contains(id),
                    // No prefs saved yet — use the layout defaults
                    None => {
                        let defaults = default_window_layout();
                        defaults
                            .iter()
                            .find(|w| w.id == *id)
                            .map_or(false, |w| !w.minimized)
                    }
                };
                (*id, RwSignal::new(checked))
            })
            .collect()
    });

    // Local signal for tooltip checkbox (show tooltips = !suppress)
    let show_tooltips = RwSignal::new(!ctx.suppress_tooltips.get());

    let on_save = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let open_ids: Vec<WindowId> = startup_checked
                .get()
                .iter()
                .filter(|(_, sig)| sig.get())
                .map(|(id, _)| *id)
                .collect();
            save_startup_prefs(&open_ids);
        }

        // Save tooltip preference
        let suppress = !show_tooltips.get();
        ctx.suppress_tooltips.set(suppress);
        ctx.send_message(ClientMessage::SetSuppressTooltips { suppress });

        on_close.run(());
    };

    let on_cancel = move |_: leptos::ev::MouseEvent| {
        on_close.run(());
    };

    view! {
        <div class="settings-backdrop" on:click=move |ev: leptos::ev::MouseEvent| {
            // Only close when clicking the backdrop itself, not children
            if let (Some(target), Some(current)) = (ev.target(), ev.current_target()) {
                if target == current {
                    on_close.run(());
                }
            }
        }>
            <div class="settings-dialog">
                <div class="settings-title">
                    <span>"Settings"</span>
                    <button class="settings-close" on:click=move |_| on_close.run(())>"\u{2715}"</button>
                </div>

                // Tab bar
                <div class="settings-tabs">
                    <button
                        class=move || if active_tab.get() == SettingsTab::Startup { "settings-tab settings-tab-active" } else { "settings-tab" }
                        on:click=move |_| active_tab.set(SettingsTab::Startup)
                    >"Startup"</button>
                    <button
                        class=move || if active_tab.get() == SettingsTab::Options { "settings-tab settings-tab-active" } else { "settings-tab" }
                        on:click=move |_| active_tab.set(SettingsTab::Options)
                    >"Options"</button>
                </div>

                // Tab content
                <div class="settings-body">
                    {move || match active_tab.get() {
                        SettingsTab::Startup => {
                            let items = startup_checked.get();
                            view! {
                                <div class="settings-startup">
                                    <p class="settings-hint">"Choose which windows are open on startup. The rest start minimized in the dock."</p>
                                    <div class="settings-checklist">
                                        {items.into_iter().map(|(id, checked)| {
                                            let title = STATIC_WINDOWS.iter()
                                                .find(|(wid, _)| *wid == id)
                                                .map(|(_, t)| *t)
                                                .unwrap_or("?");
                                            let icon = id.dock_icon();
                                            view! {
                                                <label class="settings-check-row">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || checked.get()
                                                        on:change=move |ev| {
                                                            let val = event_target_checked(&ev);
                                                            checked.set(val);
                                                        }
                                                    />
                                                    <span class="settings-check-icon">{icon}</span>
                                                    <span>{title}</span>
                                                </label>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>
                            }.into_any()
                        }
                        SettingsTab::Options => {
                            view! {
                                <div class="settings-startup">
                                    <p class="settings-hint">"Interface preferences."</p>
                                    <div class="settings-checklist">
                                        <label class="settings-check-row">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || show_tooltips.get()
                                                on:change=move |ev| {
                                                    let val = event_target_checked(&ev);
                                                    show_tooltips.set(val);
                                                }
                                            />
                                            <span>"Show tooltips"</span>
                                        </label>
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>

                // Footer with Save/Cancel
                <div class="settings-footer">
                    <button class="fb-btn" on:click=on_save>"Save"</button>
                    <button class="fb-btn" on:click=on_cancel>"Cancel"</button>
                </div>
            </div>
        </div>
    }
}
