/// Window layout and startup preferences persistence via localStorage.
use super::WindowId;
#[cfg(feature = "hydrate")]
use super::{WindowState, default_window_layout};

#[cfg(feature = "hydrate")]
pub(super) const LAYOUT_STORAGE_KEY: &str = "webrpg_window_layout";

/// localStorage key for startup window preferences (which windows open on startup).
#[cfg(feature = "hydrate")]
pub(super) const STARTUP_PREFS_KEY: &str = "webrpg_startup_windows";

/// Static windows available for startup preferences.
/// Each entry is (WindowId, title). CharacterEditor is dynamic and excluded.
pub(super) const STATIC_WINDOWS: &[(WindowId, &str)] = &[
    (WindowId::Map, "Map"),
    (WindowId::Chat, "Chat"),
    (WindowId::CharacterSelection, "Character Selection"),
    (WindowId::Initiative, "Initiative"),
    (WindowId::Inventory, "Inventory"),
    (WindowId::Creatures, "Creatures"),
    (WindowId::Terminal, "COMMAND.COM"),
    (WindowId::FileBrowser, "File Viewer"),
    (WindowId::HelpViewer, "Help"),
];

/// Load startup window preferences from localStorage.
/// Returns None if no preferences are saved (use layout defaults).
/// Returns Some(set) where the set contains WindowIds that should be open (not minimized).
#[cfg(feature = "hydrate")]
pub(super) fn load_startup_prefs() -> Option<Vec<WindowId>> {
    let window = web_sys::window()?;
    let storage = window.local_storage().ok()??;
    let json = storage.get_item(STARTUP_PREFS_KEY).ok()??;
    serde_json::from_str::<Vec<WindowId>>(&json).ok()
}

/// Save startup window preferences to localStorage.
#[cfg(feature = "hydrate")]
pub(super) fn save_startup_prefs(open_windows: &[WindowId]) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(open_windows) {
                let _ = storage.set_item(STARTUP_PREFS_KEY, &json);
            }
        }
    }
}

/// Load window layout from localStorage, falling back to defaults.
/// Handles version mismatches: if a new window ID exists in defaults but not
/// in storage, the default is used; unknown stored windows are dropped.
///
/// When no saved layout exists (first visit), startup window preferences
/// are applied: windows in the preference list are restored (not minimized),
/// all others are minimized.
#[cfg(feature = "hydrate")]
pub(super) fn load_or_default_layout() -> Vec<WindowState> {
    #[cfg(feature = "hydrate")]
    {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(json)) = storage.get_item(LAYOUT_STORAGE_KEY) {
                    if let Ok(mut stored) = serde_json::from_str::<Vec<WindowState>>(&json) {
                        // Remove dynamic windows — they don't persist across reloads
                        stored.retain(|w| !w.id.is_dynamic());

                        // Merge with defaults: keep stored state for known IDs,
                        // add defaults for any new IDs.
                        let defaults = default_window_layout();
                        for default_win in &defaults {
                            if !stored.iter().any(|w| w.id == default_win.id) {
                                stored.push(default_win.clone());
                            }
                        }
                        // Remove any stored windows with IDs not in defaults
                        stored.retain(|w| defaults.iter().any(|d| d.id == w.id));
                        return stored;
                    }
                }
            }
        }
    }
    let mut layout = default_window_layout();
    // Apply startup preferences if set
    if let Some(open_ids) = load_startup_prefs() {
        for win in &mut layout {
            if !win.id.is_dynamic() {
                win.minimized = !open_ids.contains(&win.id);
            }
        }
    }
    layout
}

/// Save window layout to localStorage.
#[cfg(feature = "hydrate")]
pub(super) fn save_layout(windows: &[WindowState]) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            // Only persist static windows
            let static_wins: Vec<&WindowState> =
                windows.iter().filter(|w| !w.id.is_dynamic()).collect();
            if let Ok(json) = serde_json::to_string(&static_wins) {
                let _ = storage.set_item(LAYOUT_STORAGE_KEY, &json);
            }
        }
    }
}
