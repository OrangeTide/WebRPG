use leptos::prelude::*;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Context for navigating the help viewer from other windows.
/// Provided at the game page level so both GameWindow (? button) and
/// HelpViewerPanel can access it.
#[derive(Clone, Copy)]
pub struct HelpContext {
    /// Set to a topic slug to navigate the help viewer there.
    pub navigate_to: RwSignal<Option<String>>,
}

impl HelpContext {
    pub fn new() -> Self {
        Self {
            navigate_to: RwSignal::new(None),
        }
    }
}

/// An entry in a help directory listing (file or subdirectory).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HelpEntry {
    /// Full path from help root (e.g. "commands/dir" or "file-viewer").
    pub slug: String,
    /// Human-readable title (from first # heading, or humanized name).
    pub title: String,
    /// True for subdirectories, false for topic files.
    pub is_directory: bool,
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

/// Where the help viewer is currently navigated.
#[derive(Clone, Debug, PartialEq)]
enum HelpLocation {
    /// Viewing a directory listing. Empty string = root.
    Index(String),
    /// Viewing a rendered topic.
    Topic(String),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Validate a help path (no traversal, safe characters only).
#[cfg(feature = "ssr")]
fn is_valid_help_path(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    if path.contains("..") || path.starts_with('/') || path.ends_with('/') || path.contains("//") {
        return false;
    }
    path.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '/' || c == '_')
}

/// Parent of a path. Returns None if already at root.
fn parent_path(path: &str) -> Option<String> {
    if path.is_empty() {
        None
    } else {
        Some(
            path.rfind('/')
                .map(|pos| path[..pos].to_string())
                .unwrap_or_default(),
        )
    }
}

/// Convert a kebab-case name to Title Case.
fn humanize_name(name: &str) -> String {
    name.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let mut s: String = c.to_uppercase().collect();
                    s.extend(chars);
                    s
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Render markdown to HTML. Links with `help:slug` URLs are preserved
/// as-is in the output for click interception.
fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

// ---------------------------------------------------------------------------
// Server functions
// ---------------------------------------------------------------------------

/// Fetch help document content by topic slug.
/// Slugs may contain `/` for hierarchical paths (e.g. "commands/dir").
#[server(GetHelpContent)]
pub async fn get_help_content(slug: String) -> Result<String, ServerFnError> {
    if slug.is_empty() || !is_valid_help_path(&slug) {
        return Err(ServerFnError::new("Invalid topic slug"));
    }
    let path = format!("help/{}.md", slug);
    std::fs::read_to_string(&path)
        .map_err(|_| ServerFnError::new(format!("Help topic '{}' not found", slug)))
}

/// List help entries (files and subdirectories) under a parent path.
/// Pass an empty string to list the root `help/` directory.
#[server(ListHelpTopics)]
pub async fn list_help_topics(parent: String) -> Result<Vec<HelpEntry>, ServerFnError> {
    if !is_valid_help_path(&parent) {
        return Err(ServerFnError::new("Invalid help path"));
    }
    let dir = if parent.is_empty() {
        "help".to_string()
    } else {
        format!("help/{}", parent)
    };
    let read = std::fs::read_dir(&dir)
        .map_err(|_| ServerFnError::new(format!("Help directory '{}' not found", parent)))?;

    let mut entries = Vec::new();
    for item in read.flatten() {
        let path = item.path();
        let name = item.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            let slug = if parent.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", parent, name)
            };
            entries.push(HelpEntry {
                title: humanize_name(&name),
                slug,
                is_directory: true,
            });
        } else if path.extension().map_or(false, |ext| ext == "md") {
            let stem = path.file_stem().unwrap().to_string_lossy().to_string();
            let slug = if parent.is_empty() {
                stem.clone()
            } else {
                format!("{}/{}", parent, stem)
            };
            let title = std::fs::read_to_string(&path)
                .ok()
                .and_then(|c| {
                    c.lines()
                        .next()
                        .map(|l| l.trim_start_matches('#').trim().to_string())
                })
                .unwrap_or_else(|| humanize_name(&stem));
            entries.push(HelpEntry {
                slug,
                title,
                is_directory: false,
            });
        }
    }
    // Directories first, then alphabetical by title
    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.title.cmp(&b.title))
    });
    Ok(entries)
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

#[component]
pub fn HelpViewerPanel() -> impl IntoView {
    let help_ctx = expect_context::<HelpContext>();

    // Current location (Index at root initially)
    let location = RwSignal::new(HelpLocation::Index(String::new()));
    let content_html = RwSignal::new(String::new());
    // None = loading/initial, Some(vec) = loaded (may be empty)
    let entries = RwSignal::new(Option::<Vec<HelpEntry>>::None);
    let history_back = RwSignal::new(Vec::<HelpLocation>::new());
    let history_forward = RwSignal::new(Vec::<HelpLocation>::new());

    // Low-level: go to a location without touching history
    let go_to = move |loc: HelpLocation| {
        location.set(loc.clone());
        match loc {
            HelpLocation::Index(path) => {
                entries.set(None);
                content_html.set(String::new());
                leptos::task::spawn_local(async move {
                    entries.set(Some(list_help_topics(path).await.unwrap_or_default()));
                });
            }
            HelpLocation::Topic(slug) => {
                content_html.set(String::new());
                leptos::task::spawn_local(async move {
                    match get_help_content(slug).await {
                        Ok(md) => content_html.set(render_markdown(&md)),
                        Err(e) => {
                            content_html.set(format!("<p class=\"help-error\">Error: {}</p>", e))
                        }
                    }
                });
            }
        }
    };

    // Navigate with history (pushes current location onto back stack)
    let do_navigate = move |loc: HelpLocation| {
        history_back.update(|h| h.push(location.get_untracked()));
        history_forward.update(|h| h.clear());
        go_to(loc);
    };

    // Initial fetch of root entries
    #[cfg(feature = "hydrate")]
    {
        leptos::task::spawn_local(async move {
            entries.set(Some(
                list_help_topics(String::new()).await.unwrap_or_default(),
            ));
        });
    }

    // Watch for external navigation (? button in other windows)
    Effect::new(move |_| {
        if let Some(slug) = help_ctx.navigate_to.get() {
            help_ctx.navigate_to.set(None);
            do_navigate(HelpLocation::Topic(slug));
        }
    });

    // --- Toolbar handlers ---

    let on_back = move |_: leptos::ev::MouseEvent| {
        if let Some(prev) = history_back.with_untracked(|h| h.last().cloned()) {
            history_back.update(|h| {
                h.pop();
            });
            history_forward.update(|h| h.push(location.get_untracked()));
            go_to(prev);
        }
    };

    let on_forward = move |_: leptos::ev::MouseEvent| {
        if let Some(next) = history_forward.with_untracked(|h| h.last().cloned()) {
            history_forward.update(|h| {
                h.pop();
            });
            history_back.update(|h| h.push(location.get_untracked()));
            go_to(next);
        }
    };

    // Up: go to the parent directory of the current location
    let on_up = move |_: leptos::ev::MouseEvent| {
        let parent = match location.get_untracked() {
            HelpLocation::Index(ref p) => parent_path(p),
            HelpLocation::Topic(ref s) => Some(parent_path(s).unwrap_or_default()),
        };
        if let Some(p) = parent {
            do_navigate(HelpLocation::Index(p));
        }
    };

    // Home: go to root index
    let on_home = move |_: leptos::ev::MouseEvent| {
        do_navigate(HelpLocation::Index(String::new()));
    };

    // Top: scroll content area to top
    let on_top = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if let Ok(Some(el)) = doc.query_selector(".help-content") {
                        el.set_scroll_top(0);
                    }
                }
            }
        }
    };

    // Index: go to root index (same as Home for now)
    let on_index = move |_: leptos::ev::MouseEvent| {
        do_navigate(HelpLocation::Index(String::new()));
    };

    // Handle clicks on help: links in rendered markdown content
    let on_content_click = move |_ev: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::JsCast;
            if let Some(target) = _ev.target() {
                if let Some(el) = target.dyn_ref::<web_sys::Element>() {
                    if let Ok(Some(anchor)) = el.closest("a[href^='help:']") {
                        _ev.prevent_default();
                        if let Some(href) = anchor.get_attribute("href") {
                            if let Some(slug) = href.strip_prefix("help:") {
                                do_navigate(HelpLocation::Topic(slug.to_string()));
                            }
                        }
                    }
                }
            }
        }
    };

    // --- Computed states for toolbar buttons ---

    let can_back = move || history_back.with(|h| !h.is_empty());
    let can_forward = move || history_forward.with(|h| !h.is_empty());
    let can_go_up = move || match location.get() {
        HelpLocation::Index(ref p) => !p.is_empty(),
        HelpLocation::Topic(_) => true,
    };
    let is_at_index = move || matches!(location.get(), HelpLocation::Index(_));

    // --- View ---

    view! {
        <div class="help-viewer">
            // Toolbar
            <div class="help-toolbar">
                <button
                    class="fb-btn"
                    data-tooltip="Back"
                    disabled=move || !can_back()
                    on:click=on_back
                >{"\u{1f519}"}</button>
                <button
                    class="fb-btn"
                    data-tooltip="Forward"
                    disabled=move || !can_forward()
                    on:click=on_forward
                >{"\u{27a1}\u{fe0f}"}</button>
                <button
                    class="fb-btn"
                    data-tooltip="Up"
                    disabled=move || !can_go_up()
                    on:click=on_up
                >{"\u{2934}\u{fe0f}"}</button>
                <span class="help-toolbar-sep"></span>
                <button
                    class="fb-btn"
                    data-tooltip="Home"
                    on:click=on_home
                >{"\u{1f3e0}"}</button>
                <button
                    class="fb-btn"
                    data-tooltip="Scroll to Top"
                    disabled=is_at_index
                    on:click=on_top
                >{"\u{23eb}"}</button>
                <button
                    class="fb-btn"
                    data-tooltip="Index"
                    on:click=on_index
                >{"\u{1f4d1}"}</button>
                <button
                    class="fb-btn"
                    data-tooltip="Search (not implemented)"
                    disabled=true
                >{"\u{1f50d}"}</button>
            </div>
            // Content area
            <div
                class="help-content"
                on:click=on_content_click
            >
                {move || {
                    match location.get() {
                        HelpLocation::Index(path) => {
                            let heading = if path.is_empty() {
                                "Help Topics".to_string()
                            } else {
                                humanize_name(
                                    path.rsplit('/').next().unwrap_or(&path),
                                )
                            };
                            match entries.get() {
                                None => {
                                    // Loading
                                    view! {
                                        <div class="help-index">
                                            <h1>{heading}</h1>
                                            <p class="help-empty">"Loading\u{2026}"</p>
                                        </div>
                                    }.into_any()
                                }
                                Some(items) if items.is_empty() => {
                                    view! {
                                        <div class="help-index">
                                            <h1>{heading}</h1>
                                            <p class="help-empty">"No help topics available."</p>
                                        </div>
                                    }.into_any()
                                }
                                Some(items) => {
                                    view! {
                                        <div class="help-index">
                                            <h1>{heading}</h1>
                                            <ul class="help-topic-list">
                                                {items.into_iter().map(|entry| {
                                                    let slug = entry.slug.clone();
                                                    let title = entry.title.clone();
                                                    let is_dir = entry.is_directory;
                                                    let icon = if is_dir {
                                                        "\u{1f4c1} "
                                                    } else {
                                                        "\u{1f4c4} "
                                                    };
                                                    view! {
                                                        <li>
                                                            <a
                                                                class="help-link"
                                                                href="#"
                                                                on:click=move |ev: leptos::ev::MouseEvent| {
                                                                    ev.prevent_default();
                                                                    if is_dir {
                                                                        do_navigate(HelpLocation::Index(slug.clone()));
                                                                    } else {
                                                                        do_navigate(HelpLocation::Topic(slug.clone()));
                                                                    }
                                                                }
                                                            >{icon}{title}</a>
                                                        </li>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </ul>
                                        </div>
                                    }.into_any()
                                }
                            }
                        }
                        HelpLocation::Topic(_) => {
                            let html = content_html.get();
                            if html.is_empty() {
                                view! {
                                    <div class="help-topic">
                                        <p class="help-empty">"Loading\u{2026}"</p>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="help-topic" inner_html=html></div>
                                }.into_any()
                            }
                        }
                    }
                }}
            </div>
        </div>
    }
}
