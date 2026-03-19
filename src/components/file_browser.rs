use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use crate::components::terminal::vfs_file_icon;
#[cfg(feature = "hydrate")]
use crate::scratch_drive::ScratchDrives;
use crate::vfs::{Drive, VfsPath};

/// Context menu state: position and target item.
#[derive(Debug, Clone)]
struct ContextMenu {
    x: i32,
    y: i32,
    item: BrowserItem,
}

/// File preview overlay state.
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PreviewContent {
    /// UTF-8 text content.
    Text { name: String, text: String },
    /// Image displayable via URL (blob URL or CAS URL).
    Image { name: String, url: String },
}

/// What the file browser is currently showing.
#[derive(Debug, Clone, PartialEq)]
enum BrowserView {
    /// Root view showing available drives.
    DriveList,
    /// Directory listing within a specific drive.
    Directory(VfsPath),
}

/// One item in the icon grid.
///
/// Fields `size_bytes` and `content_type` are populated for future use
/// (status bar detail, tooltips, sort-by-size) but not yet displayed.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BrowserItem {
    name: String,
    full_path: VfsPath,
    is_directory: bool,
    size_bytes: i64,
    icon: &'static str,
    content_type: Option<String>,
}

/// Drive info for the root view.
#[derive(Debug, Clone)]
struct DriveItem {
    drive: Drive,
    label: &'static str,
    icon: &'static str,
}

const DRIVES: &[DriveItem] = &[
    DriveItem {
        drive: Drive::A,
        label: "A: Scratch",
        icon: "\u{1f4be}", // 💾
    },
    DriveItem {
        drive: Drive::B,
        label: "B: Scratch",
        icon: "\u{1f4be}", // 💾
    },
    DriveItem {
        drive: Drive::C,
        label: "C: Session",
        icon: "\u{1f4bf}", // 💿
    },
    DriveItem {
        drive: Drive::U,
        label: "U: Personal",
        icon: "\u{1f513}", // 🔓
    },
];

/// Identifies which pane is active in dual-pane mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneId {
    Left,
    Right,
}

/// State for a single browser pane. All fields are `RwSignal` so the struct is `Copy`.
#[derive(Clone, Copy)]
struct PaneState {
    id: PaneId,
    view: RwSignal<BrowserView>,
    items: RwSignal<Vec<BrowserItem>>,
    history: RwSignal<Vec<BrowserView>>,
    forward: RwSignal<Vec<BrowserView>>,
    location_input: RwSignal<String>,
    status_line: RwSignal<String>,
    loading: RwSignal<bool>,
    selected_items: RwSignal<Vec<BrowserItem>>,
}

impl PaneState {
    fn new(id: PaneId) -> Self {
        Self {
            id,
            view: RwSignal::new(BrowserView::DriveList),
            items: RwSignal::new(Vec::new()),
            history: RwSignal::new(Vec::new()),
            forward: RwSignal::new(Vec::new()),
            location_input: RwSignal::new(String::new()),
            status_line: RwSignal::new("Select a drive".to_string()),
            loading: RwSignal::new(false),
            selected_items: RwSignal::new(Vec::new()),
        }
    }
}

#[component]
pub fn FileBrowserPanel() -> impl IntoView {
    let ctx = expect_context::<crate::pages::game::GameContext>();
    let session_id = ctx.session_id;

    let left_pane = PaneState::new(PaneId::Left);
    let right_pane = PaneState::new(PaneId::Right);

    let dual_pane = RwSignal::new(false);
    let active_pane = RwSignal::new(PaneId::Left);

    let context_menu = RwSignal::new(Option::<ContextMenu>::None);
    let preview = RwSignal::new(Option::<PreviewContent>::None);

    #[cfg(feature = "hydrate")]
    let scratch_drives =
        expect_context::<RwSignal<crate::scratch_drive::ScratchDrives, LocalStorage>>();

    // Helper: get active pane state
    let active = move || match active_pane.get() {
        PaneId::Left => left_pane,
        PaneId::Right => right_pane,
    };

    // Setup effects for both panes: location bar sync + directory fetch
    for pane in [left_pane, right_pane] {
        // Location bar sync
        Effect::new(move |_| {
            let v = pane.view.get();
            let loc = match &v {
                BrowserView::DriveList => String::new(),
                BrowserView::Directory(p) => {
                    if p.path == "/" {
                        format!("{}:/", p.drive.letter())
                    } else {
                        format!("{}:{}/", p.drive.letter(), p.path)
                    }
                }
            };
            pane.location_input.set(loc);
        });

        // Fetch directory contents when view changes
        #[cfg(feature = "hydrate")]
        Effect::new(move |_| {
            let v = pane.view.get();
            match v {
                BrowserView::DriveList => {
                    pane.items.set(Vec::new());
                    pane.status_line.set("Select a drive".to_string());
                }
                BrowserView::Directory(ref path) => {
                    let path = path.clone();
                    let sid = session_id.get();
                    pane.loading.set(true);
                    let sd = scratch_drives.get();
                    leptos::task::spawn_local(async move {
                        let result = fetch_directory(&path, sid, &sd).await;
                        match result {
                            Ok((entries, status)) => {
                                pane.items.set(entries);
                                pane.status_line.set(status);
                            }
                            Err(e) => {
                                pane.items.set(Vec::new());
                                pane.status_line.set(format!("Error: {e}"));
                            }
                        }
                        pane.loading.set(false);
                    });
                }
            }
        });
    }

    // Navigate to a view in the active pane
    let navigate_to_active = move |new_view: BrowserView| {
        let pane = active();
        let current = pane.view.get();
        if current != new_view {
            pane.history.update(|h| h.push(current));
            pane.forward.set(Vec::new());
        }
        pane.selected_items.set(Vec::new());
        pane.view.set(new_view);
    };

    // Back button
    let on_back = move |_: leptos::ev::MouseEvent| {
        let pane = active();
        let h = pane.history.get();
        if let Some(prev) = h.last().cloned() {
            let current = pane.view.get();
            pane.forward.update(|f| f.push(current));
            pane.history.update(|h| {
                h.pop();
            });
            pane.selected_items.set(Vec::new());
            pane.view.set(prev);
        }
    };

    // Forward button
    let on_forward = move |_: leptos::ev::MouseEvent| {
        let pane = active();
        let f = pane.forward.get();
        if let Some(next) = f.last().cloned() {
            let current = pane.view.get();
            pane.history.update(|h| h.push(current));
            pane.forward.update(|f| {
                f.pop();
            });
            pane.selected_items.set(Vec::new());
            pane.view.set(next);
        }
    };

    // Up button
    let on_up = move |_: leptos::ev::MouseEvent| {
        let pane = active();
        let v = pane.view.get();
        match v {
            BrowserView::DriveList => {}
            BrowserView::Directory(ref p) => {
                if p.path == "/" {
                    navigate_to_active(BrowserView::DriveList);
                } else {
                    let parent = p.parent().unwrap_or_else(|| "/".to_string());
                    navigate_to_active(BrowserView::Directory(VfsPath {
                        drive: p.drive,
                        path: parent,
                    }));
                }
            }
        }
    };

    // Toggle dual pane
    let on_toggle_dual = move |_: leptos::ev::MouseEvent| {
        dual_pane.update(|d| *d = !*d);
        if !dual_pane.get() {
            // When disabling dual pane, switch to left pane
            active_pane.set(PaneId::Left);
            right_pane.selected_items.set(Vec::new());
        }
    };

    // New folder button
    let on_new_folder = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let pane = active();
            let v = pane.view.get();
            if let BrowserView::Directory(ref path) = v {
                let window = web_sys::window().unwrap();
                if let Some(name) = window
                    .prompt_with_message("New folder name:")
                    .ok()
                    .flatten()
                {
                    let name = name.trim().to_string();
                    if name.is_empty() {
                        return;
                    }
                    let new_path = if path.path == "/" {
                        format!("/{name}")
                    } else {
                        format!("{}/{name}", path.path)
                    };
                    let drive = path.drive;
                    let sid = session_id.get();
                    let vfs_path = VfsPath {
                        drive,
                        path: new_path,
                    };
                    let sd = scratch_drives.get();
                    let refresh_view = pane.view;
                    leptos::task::spawn_local(async move {
                        let result = create_directory(&vfs_path, sid, &sd).await;
                        if let Err(e) = result {
                            log::error!("mkdir failed: {e}");
                        }
                        refresh_view.set(refresh_view.get());
                    });
                }
            }
        }
    };

    // Upload button
    let on_upload = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let pane = active();
            let v = pane.view.get();
            if let BrowserView::Directory(ref path) = v {
                let dest = path.clone();
                let sid = session_id.get();
                let sd = scratch_drives.get();
                let refresh_view = pane.view;
                let status = pane.status_line;
                leptos::task::spawn_local(async move {
                    match upload_files(&dest, sid, &sd).await {
                        Ok(msg) => status.set(msg),
                        Err(e) => status.set(format!("Upload: {e}")),
                    }
                    refresh_view.set(refresh_view.get());
                });
            }
        }
    };

    // Delete button (supports multi-select)
    let on_delete = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let pane = active();
            let sel = pane.selected_items.get();
            if sel.is_empty() {
                return;
            }
            let window = web_sys::window().unwrap();
            let msg = if sel.len() == 1 {
                let item = &sel[0];
                if item.is_directory {
                    format!("Delete folder \"{}\"?", item.name)
                } else {
                    format!("Delete \"{}\"?", item.name)
                }
            } else {
                format!("Delete {} items?", sel.len())
            };
            if window.confirm_with_message(&msg).unwrap_or(false) {
                let sid = session_id.get();
                let sd = scratch_drives.get();
                let refresh_view = pane.view;
                let status = pane.status_line;
                let selected = pane.selected_items;
                let paths: Vec<_> = sel.iter().map(|i| i.full_path.clone()).collect();
                leptos::task::spawn_local(async move {
                    let mut errors = Vec::new();
                    for path in &paths {
                        if let Err(e) = delete_entry(path, sid, &sd).await {
                            errors.push(format!("{}: {e}", path.path));
                        }
                    }
                    if !errors.is_empty() {
                        status.set(format!("Delete errors: {}", errors.join("; ")));
                    }
                    selected.set(Vec::new());
                    refresh_view.set(refresh_view.get());
                });
            }
        }
    };

    // Rename button (single selection only)
    let on_rename = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let pane = active();
            let sel = pane.selected_items.get();
            if let Some(item) = sel.first().cloned() {
                let window = web_sys::window().unwrap();
                if let Some(new_name) = window
                    .prompt_with_message_and_default("Rename to:", &item.name)
                    .ok()
                    .flatten()
                {
                    let new_name = new_name.trim().to_string();
                    if new_name.is_empty() || new_name == item.name {
                        return;
                    }
                    let parent = item.full_path.parent().unwrap_or_else(|| "/".to_string());
                    let new_file_path = if parent == "/" {
                        format!("/{new_name}")
                    } else {
                        format!("{parent}/{new_name}")
                    };
                    let old_path = item.full_path.clone();
                    let new_vfs = VfsPath {
                        drive: old_path.drive,
                        path: new_file_path,
                    };
                    let sid = session_id.get();
                    let sd = scratch_drives.get();
                    let refresh_view = pane.view;
                    let status = pane.status_line;
                    let selected = pane.selected_items;
                    leptos::task::spawn_local(async move {
                        match rename_entry(&old_path, &new_vfs, sid, &sd).await {
                            Ok(()) => {}
                            Err(e) => status.set(format!("Rename failed: {e}")),
                        }
                        selected.set(Vec::new());
                        refresh_view.set(refresh_view.get());
                    });
                }
            }
        }
    };

    // Copy to other pane
    let on_copy_to_other = move |_: leptos::ev::MouseEvent| {
        #[cfg(feature = "hydrate")]
        {
            let ap = active_pane.get();
            let (src_pane, dst_pane) = match ap {
                PaneId::Left => (left_pane, right_pane),
                PaneId::Right => (right_pane, left_pane),
            };
            let sel = src_pane.selected_items.get();
            if sel.is_empty() {
                return;
            }
            let dst_view = dst_pane.view.get();
            let dest = match dst_view {
                BrowserView::DriveList => return,
                BrowserView::Directory(ref p) => p.clone(),
            };
            let sid = session_id.get();
            let sd = scratch_drives.get();
            let src_status = src_pane.status_line;
            let src_selected = src_pane.selected_items;
            let dst_refresh = dst_pane.view;
            leptos::task::spawn_local(async move {
                match copy_items(&sel, &dest, sid, &sd).await {
                    Ok(msg) => src_status.set(msg),
                    Err(e) => src_status.set(format!("Copy failed: {e}")),
                }
                src_selected.set(Vec::new());
                // Refresh destination pane
                dst_refresh.set(dst_refresh.get());
            });
        }
    };

    // Toolbar button state derived from active pane
    let in_directory = move || matches!(active().view.get(), BrowserView::Directory(_));
    let has_history = move || !active().history.get().is_empty();
    let has_forward = move || !active().forward.get().is_empty();
    let has_selection = move || !active().selected_items.get().is_empty();
    let has_single_selection = move || active().selected_items.get().len() == 1;

    // Copy button direction: right if left has selection, left if right has,
    // bidirectional if neither
    let copy_direction = move || {
        let left_sel = !left_pane.selected_items.get().is_empty();
        let right_sel = !right_pane.selected_items.get().is_empty();
        if left_sel {
            "\u{27a1}\u{fe0f}" // ➡️
        } else if right_sel {
            "\u{2b05}\u{fe0f}" // ⬅️
        } else {
            "\u{2194}\u{fe0f}" // ↔️
        }
    };

    // Copy button enabled: selection exists AND target pane is not at DriveList
    let copy_enabled = move || {
        let ap = active_pane.get();
        let (src_pane, dst_pane) = match ap {
            PaneId::Left => (left_pane, right_pane),
            PaneId::Right => (right_pane, left_pane),
        };
        let has_sel = !src_pane.selected_items.get().is_empty();
        let dst_in_dir = matches!(dst_pane.view.get(), BrowserView::Directory(_));
        has_sel && dst_in_dir
    };

    // Close preview overlay
    let on_close_preview = move |_: leptos::ev::MouseEvent| {
        preview.set(None);
    };

    view! {
        <div class="fb-panel">
            <div class="fb-toolbar">
                <button
                    class="fb-btn"
                    on:click=on_back
                    title="Back"
                    disabled=move || !has_history()
                >{"\u{1f519}"}</button>
                <button
                    class="fb-btn"
                    on:click=on_forward
                    title="Forward"
                    disabled=move || !has_forward()
                >{"\u{27a1}\u{fe0f}"}</button>
                <button
                    class="fb-btn"
                    on:click=on_up
                    title="Up"
                    disabled=move || !in_directory()
                >{"\u{2934}\u{fe0f}"}</button>

                <button
                    class=move || if dual_pane.get() { "fb-btn fb-toggle-active" } else { "fb-btn" }
                    on:click=on_toggle_dual
                    title="Toggle dual pane"
                >{"\u{29c9}"}</button>

                {move || {
                    if dual_pane.get() {
                        Some(view! {
                            <button
                                class="fb-btn"
                                on:click=on_copy_to_other
                                title="Copy to other pane"
                                disabled=move || !copy_enabled()
                            >{copy_direction}</button>
                        })
                    } else {
                        None
                    }
                }}

                {move || {
                    if in_directory() {
                        view! {
                            <button class="fb-btn" on:click=on_new_folder title="New Folder">{"\u{1f4c1}+"}</button>
                            <button class="fb-btn" on:click=on_upload title="Upload">{"\u{1f4e4}"}</button>
                            <button
                                class="fb-btn"
                                on:click=on_rename
                                title="Rename"
                                disabled=move || !has_single_selection()
                            >{"\u{270f}\u{fe0f}"}</button>
                            <button
                                class="fb-btn"
                                on:click={
                                    move |_: leptos::ev::MouseEvent| {
                                        #[cfg(feature = "hydrate")]
                                        {
                                            let pane = active();
                                            let sel = pane.selected_items.get();
                                            let files: Vec<_> = sel.into_iter().filter(|i| !i.is_directory).collect();
                                            let sid = session_id.get();
                                            let sd = scratch_drives.get();
                                            let status = pane.status_line;
                                            leptos::task::spawn_local(async move {
                                                let mut count = 0;
                                                for item in &files {
                                                    match download_file(&item.full_path, &item.name, sid, &sd).await {
                                                        Ok(_) => count += 1,
                                                        Err(e) => {
                                                            status.set(format!("Download failed: {e}"));
                                                            return;
                                                        }
                                                    }
                                                }
                                                if count > 1 {
                                                    status.set(format!("Downloaded {count} files"));
                                                } else if count == 1 {
                                                    status.set("Downloaded".to_string());
                                                }
                                            });
                                        }
                                    }
                                }
                                title="Download"
                                disabled=move || {
                                    let sel = active().selected_items.get();
                                    sel.is_empty() || sel.iter().all(|i| i.is_directory)
                                }
                            >{"\u{1f4e5}"}</button>
                            <button
                                class="fb-btn fb-btn-danger"
                                on:click=on_delete
                                title="Delete"
                                disabled=move || !has_selection()
                            >{"\u{1f5d1}"}</button>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>

            <div class="fb-panes">
                <BrowserPaneView
                    pane=left_pane
                    active_pane=active_pane
                    other_pane=right_pane
                    context_menu=context_menu
                    preview=preview
                    session_id=session_id
                />
                {move || {
                    if dual_pane.get() {
                        Some(view! {
                            <BrowserPaneView
                                pane=right_pane
                                active_pane=active_pane
                                other_pane=left_pane
                                context_menu=context_menu
                                preview=preview
                                session_id=session_id
                            />
                        })
                    } else {
                        None
                    }
                }}
            </div>

            // Context menu overlay
            {move || {
                context_menu.get().map(|menu| {
                    let is_dir = menu.item.is_directory;
                    // Context menu: Download
                    let on_ctx_download = move |_: leptos::ev::MouseEvent| {
                        #[cfg(feature = "hydrate")]
                        if let Some(menu) = context_menu.get() {
                            let path = menu.item.full_path.clone();
                            let name = menu.item.name.clone();
                            let sid = session_id.get();
                            let sd = scratch_drives.get();
                            let status = active().status_line;
                            leptos::task::spawn_local(async move {
                                match download_file(&path, &name, sid, &sd).await {
                                    Ok(msg) => status.set(msg),
                                    Err(e) => status.set(format!("Download failed: {e}")),
                                }
                            });
                        }
                        context_menu.set(None);
                    };

                    // Context menu: Rename
                    let on_ctx_rename = move |_: leptos::ev::MouseEvent| {
                        #[cfg(feature = "hydrate")]
                        if let Some(menu) = context_menu.get() {
                            let window = web_sys::window().unwrap();
                            if let Some(new_name) = window
                                .prompt_with_message_and_default("Rename to:", &menu.item.name)
                                .ok()
                                .flatten()
                            {
                                let new_name = new_name.trim().to_string();
                                if !new_name.is_empty() && new_name != menu.item.name {
                                    let parent = menu
                                        .item
                                        .full_path
                                        .parent()
                                        .unwrap_or_else(|| "/".to_string());
                                    let new_file_path = if parent == "/" {
                                        format!("/{new_name}")
                                    } else {
                                        format!("{parent}/{new_name}")
                                    };
                                    let old_path = menu.item.full_path.clone();
                                    let new_vfs = VfsPath {
                                        drive: old_path.drive,
                                        path: new_file_path,
                                    };
                                    let sid = session_id.get();
                                    let sd = scratch_drives.get();
                                    let pane = active();
                                    let refresh_view = pane.view;
                                    let status = pane.status_line;
                                    let selected = pane.selected_items;
                                    leptos::task::spawn_local(async move {
                                        match rename_entry(&old_path, &new_vfs, sid, &sd).await {
                                            Ok(()) => {}
                                            Err(e) => status.set(format!("Rename failed: {e}")),
                                        }
                                        selected.set(Vec::new());
                                        refresh_view.set(refresh_view.get());
                                    });
                                }
                            }
                        }
                        context_menu.set(None);
                    };

                    // Context menu: Delete
                    let on_ctx_delete = move |_: leptos::ev::MouseEvent| {
                        #[cfg(feature = "hydrate")]
                        if let Some(menu) = context_menu.get() {
                            let window = web_sys::window().unwrap();
                            let msg = if menu.item.is_directory {
                                format!("Delete folder \"{}\"?", menu.item.name)
                            } else {
                                format!("Delete \"{}\"?", menu.item.name)
                            };
                            if window.confirm_with_message(&msg).unwrap_or(false) {
                                let path = menu.item.full_path.clone();
                                let sid = session_id.get();
                                let sd = scratch_drives.get();
                                let pane = active();
                                let refresh_view = pane.view;
                                let status = pane.status_line;
                                let selected = pane.selected_items;
                                leptos::task::spawn_local(async move {
                                    match delete_entry(&path, sid, &sd).await {
                                        Ok(()) => {}
                                        Err(e) => status.set(format!("Delete failed: {e}")),
                                    }
                                    selected.set(Vec::new());
                                    refresh_view.set(refresh_view.get());
                                });
                            }
                        }
                        context_menu.set(None);
                    };

                    view! {
                        <div class="fb-ctx-backdrop" on:click=move |_: leptos::ev::MouseEvent| context_menu.set(None)
                            on:contextmenu=move |ev: leptos::ev::MouseEvent| { ev.prevent_default(); context_menu.set(None); }>
                            <div class="fb-ctx-menu"
                                style=format!("left:{}px;top:{}px", menu.x, menu.y)>
                                {if !is_dir {
                                    Some(view! {
                                        <div class="fb-ctx-item" on:click=on_ctx_download>"Download"</div>
                                    })
                                } else {
                                    None
                                }}
                                <div class="fb-ctx-item" on:click=on_ctx_rename>"Rename"</div>
                                <div class="fb-ctx-item fb-ctx-danger" on:click=on_ctx_delete>"Delete"</div>
                            </div>
                        </div>
                    }
                })
            }}

            // Preview overlay
            {move || {
                preview.get().map(|content| {
                    match content {
                        PreviewContent::Text { name, text } => {
                            view! {
                                <div class="fb-preview-backdrop" on:click=on_close_preview>
                                    <div class="fb-preview" on:click:stopPropagation=|_: leptos::ev::MouseEvent| {}>
                                        <div class="fb-preview-title">
                                            <span>{name}</span>
                                            <button class="fb-btn" on:click=on_close_preview>"\u{2715}"</button>
                                        </div>
                                        <pre class="fb-preview-text">{text}</pre>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        PreviewContent::Image { name, url } => {
                            let alt = name.clone();
                            view! {
                                <div class="fb-preview-backdrop" on:click=on_close_preview>
                                    <div class="fb-preview" on:click:stopPropagation=|_: leptos::ev::MouseEvent| {}>
                                        <div class="fb-preview-title">
                                            <span>{name}</span>
                                            <button class="fb-btn" on:click=on_close_preview>"\u{2715}"</button>
                                        </div>
                                        <img class="fb-preview-image" src=url alt=alt />
                                    </div>
                                </div>
                            }.into_any()
                        }
                    }
                })
            }}
        </div>
    }
}

/// A single browser pane: location bar, content area, status bar.
#[allow(unused_variables)]
#[component]
fn BrowserPaneView(
    pane: PaneState,
    active_pane: RwSignal<PaneId>,
    other_pane: PaneState,
    context_menu: RwSignal<Option<ContextMenu>>,
    preview: RwSignal<Option<PreviewContent>>,
    session_id: ReadSignal<i32>,
) -> impl IntoView {
    #[cfg(feature = "hydrate")]
    let scratch_drives =
        expect_context::<RwSignal<crate::scratch_drive::ScratchDrives, LocalStorage>>();

    let navigate_to = move |new_view: BrowserView| {
        let current = pane.view.get();
        if current != new_view {
            pane.history.update(|h| h.push(current));
            pane.forward.set(Vec::new());
        }
        pane.selected_items.set(Vec::new());
        pane.view.set(new_view);
    };

    // Single-click handler for items (select, with Ctrl/Shift multi-select)
    let on_item_click = move |ev: &leptos::ev::MouseEvent, item: BrowserItem| {
        // Activate this pane and clear other pane's selection
        active_pane.set(pane.id);
        other_pane.selected_items.set(Vec::new());

        if ev.ctrl_key() || ev.meta_key() {
            pane.selected_items.update(|sel| {
                if let Some(pos) = sel.iter().position(|s| s.full_path == item.full_path) {
                    sel.remove(pos);
                } else {
                    sel.push(item);
                }
            });
        } else if ev.shift_key() {
            let current = pane.items.get();
            let sel = pane.selected_items.get();
            let anchor_path = sel.last().map(|s| s.full_path.clone());
            if let Some(anchor) = anchor_path {
                let anchor_idx = current.iter().position(|i| i.full_path == anchor);
                let target_idx = current.iter().position(|i| i.full_path == item.full_path);
                if let (Some(a), Some(t)) = (anchor_idx, target_idx) {
                    let (start, end) = if a <= t { (a, t) } else { (t, a) };
                    let range: Vec<BrowserItem> = current[start..=end].to_vec();
                    pane.selected_items.set(range);
                } else {
                    pane.selected_items.set(vec![item]);
                }
            } else {
                pane.selected_items.set(vec![item]);
            }
        } else {
            pane.selected_items.set(vec![item]);
        }
    };

    // Double-click handler for items
    let on_item_dblclick = move |item: BrowserItem| {
        if item.is_directory {
            navigate_to(BrowserView::Directory(item.full_path));
            return;
        }
        #[cfg(feature = "hydrate")]
        {
            let path = item.full_path.clone();
            let name = item.name.clone();
            let ct = item.content_type.clone();
            let sid = session_id.get();
            let sd = scratch_drives.get();
            leptos::task::spawn_local(async move {
                preview_or_download(&path, &name, ct.as_deref(), sid, &sd, &preview).await;
            });
        }
    };

    // Drive click handler
    let on_drive_click = move |drive: Drive| {
        navigate_to(BrowserView::Directory(VfsPath {
            drive,
            path: "/".to_string(),
        }));
    };

    // Click on empty area deselects and dismisses context menu
    let on_content_click = move |_ev: leptos::ev::MouseEvent| {
        active_pane.set(pane.id);
        other_pane.selected_items.set(Vec::new());
        pane.selected_items.set(Vec::new());
        context_menu.set(None);
    };

    // Right-click on item shows context menu
    let on_item_contextmenu = move |ev: leptos::ev::MouseEvent, item: BrowserItem| {
        ev.prevent_default();
        active_pane.set(pane.id);
        other_pane.selected_items.set(Vec::new());
        let sel = pane.selected_items.get();
        if !sel.iter().any(|s| s.full_path == item.full_path) {
            pane.selected_items.set(vec![item.clone()]);
        }
        context_menu.set(Some(ContextMenu {
            x: ev.client_x(),
            y: ev.client_y(),
            item,
        }));
    };

    // Location bar Enter handler
    let on_location_submit = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" {
            let loc = pane.location_input.get().trim().to_string();
            if loc.is_empty() {
                navigate_to(BrowserView::DriveList);
                return;
            }
            match VfsPath::parse(&loc) {
                Ok(p) => navigate_to(BrowserView::Directory(p)),
                Err(e) => pane.status_line.set(format!("Invalid path: {e}")),
            }
        }
    };

    let is_active = move || active_pane.get() == pane.id;

    view! {
        <div class=move || if is_active() { "fb-pane fb-pane-active" } else { "fb-pane" }>
            <div class="fb-location">
                <input
                    type="text"
                    prop:value=move || pane.location_input.get()
                    on:input=move |ev| pane.location_input.set(event_target_value(&ev))
                    on:keydown=on_location_submit
                    on:focus=move |_| {
                        active_pane.set(pane.id);
                        other_pane.selected_items.set(Vec::new());
                    }
                    placeholder="Enter path (e.g. C:/maps/)"
                    spellcheck="false"
                />
            </div>
            <div class="fb-content" on:click=on_content_click
                on:contextmenu=move |ev: leptos::ev::MouseEvent| { ev.prevent_default(); context_menu.set(None); }>
                {move || {
                    let v = pane.view.get();
                    let sel = pane.selected_items.get();
                    match v {
                        BrowserView::DriveList => {
                            let drives = DRIVES.iter().map(|d| {
                                let drive = d.drive;
                                let label = d.label;
                                let icon = d.icon;
                                view! {
                                    <div
                                        class="fb-item"
                                        on:dblclick=move |_| on_drive_click(drive)
                                    >
                                        <div class="fb-item-icon">{icon}</div>
                                        <div class="fb-item-label">{label}</div>
                                    </div>
                                }
                            }).collect_view();
                            drives.into_any()
                        }
                        BrowserView::Directory(_) => {
                            let current_items = pane.items.get();
                            if pane.loading.get() {
                                view! { <div class="fb-loading">"Loading..."</div> }.into_any()
                            } else if current_items.is_empty() {
                                view! { <div class="fb-empty">"(empty)"</div> }.into_any()
                            } else {
                                current_items.into_iter().map(|item| {
                                    let item_click = item.clone();
                                    let item_select = item.clone();
                                    let item_ctx = item.clone();
                                    let name = item.name.clone();
                                    let icon = item.icon;
                                    let is_selected = sel.iter().any(|s| s.full_path == item.full_path);
                                    let class = if is_selected { "fb-item fb-item-selected" } else { "fb-item" };
                                    view! {
                                        <div
                                            class=class
                                            on:click=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); on_item_click(&ev, item_select.clone()); }
                                            on:dblclick=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); on_item_dblclick(item_click.clone()); }
                                            on:contextmenu=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); on_item_contextmenu(ev, item_ctx.clone()); }
                                        >
                                            <div class="fb-item-icon">{icon}</div>
                                            <div class="fb-item-label">{name}</div>
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }
                        }
                    }
                }}
            </div>
            <div class="fb-status">{move || {
                let sel_count = pane.selected_items.get().len();
                let base = pane.status_line.get();
                if sel_count > 1 {
                    format!("{base} \u{2014} {sel_count} selected")
                } else {
                    base
                }
            }}</div>
        </div>
    }
}

/// Copy items to a destination path, handling all drive type combinations.
#[cfg(feature = "hydrate")]
async fn copy_items(
    items: &[BrowserItem],
    dest: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<String, String> {
    use crate::models::VfsFileData;

    let mut copied = 0u32;
    let mut errors = Vec::new();
    let total = items.len();

    for item in items {
        let src = &item.full_path;
        let dest_path = if dest.path == "/" {
            format!("/{}", item.name)
        } else {
            format!("{}/{}", dest.path, item.name)
        };
        let dst = VfsPath {
            drive: dest.drive,
            path: dest_path,
        };

        let src_scratch = src.drive.is_scratch();
        let dst_scratch = dst.drive.is_scratch();

        if item.is_directory {
            if src_scratch || dst_scratch {
                errors.push(format!(
                    "{}: directory copy not supported for scratch drives",
                    item.name
                ));
                continue;
            }
            // Both server drives — use vfs_copy_file
            let sid = src
                .drive
                .session_id(session_id)
                .or(dst.drive.session_id(session_id));
            match crate::server::api::vfs_copy_file(
                src.drive.as_str().to_string(),
                src.path.clone(),
                dst.drive.as_str().to_string(),
                dst.path.clone(),
                sid,
            )
            .await
            {
                Ok(()) => copied += 1,
                Err(e) => errors.push(format!("{}: {e}", item.name)),
            }
            continue;
        }

        // File copy
        if !src_scratch && !dst_scratch {
            // Both server drives
            let sid = src
                .drive
                .session_id(session_id)
                .or(dst.drive.session_id(session_id));
            match crate::server::api::vfs_copy_file(
                src.drive.as_str().to_string(),
                src.path.clone(),
                dst.drive.as_str().to_string(),
                dst.path.clone(),
                sid,
            )
            .await
            {
                Ok(()) => copied += 1,
                Err(e) => errors.push(format!("{}: {e}", item.name)),
            }
        } else if src_scratch && dst_scratch {
            // Both scratch
            let src_sd = match scratch.get(src.drive) {
                Some(sd) => sd,
                None => {
                    errors.push(format!("{}: source scratch not initialized", item.name));
                    continue;
                }
            };
            let dst_sd = match scratch.get(dst.drive) {
                Some(sd) => sd,
                None => {
                    errors.push(format!("{}: dest scratch not initialized", item.name));
                    continue;
                }
            };
            match src_sd.read(&src.path).await {
                Ok((data, ct)) => match dst_sd.write(&dst.path, &data, ct.as_deref()).await {
                    Ok(()) => copied += 1,
                    Err(e) => errors.push(format!("{}: write: {e}", item.name)),
                },
                Err(e) => errors.push(format!("{}: read: {e}", item.name)),
            }
        } else if src_scratch {
            // Source scratch, dest server
            let src_sd = match scratch.get(src.drive) {
                Some(sd) => sd,
                None => {
                    errors.push(format!("{}: source scratch not initialized", item.name));
                    continue;
                }
            };
            let sid = dst.drive.session_id(session_id);
            match src_sd.read(&src.path).await {
                Ok((data, ct)) => {
                    match crate::server::api::vfs_write_file(
                        dst.drive.as_str().to_string(),
                        dst.path.clone(),
                        data,
                        ct,
                        sid,
                    )
                    .await
                    {
                        Ok(()) => copied += 1,
                        Err(e) => errors.push(format!("{}: write: {e}", item.name)),
                    }
                }
                Err(e) => errors.push(format!("{}: read: {e}", item.name)),
            }
        } else {
            // Source server, dest scratch
            let dst_sd = match scratch.get(dst.drive) {
                Some(sd) => sd,
                None => {
                    errors.push(format!("{}: dest scratch not initialized", item.name));
                    continue;
                }
            };
            let sid = src.drive.session_id(session_id);
            match crate::server::api::vfs_read_file(
                src.drive.as_str().to_string(),
                src.path.clone(),
                sid,
            )
            .await
            {
                Ok(file_data) => match file_data {
                    VfsFileData::Inline { data, content_type } => {
                        match dst_sd
                            .write(&dst.path, &data, content_type.as_deref())
                            .await
                        {
                            Ok(()) => copied += 1,
                            Err(e) => errors.push(format!("{}: write: {e}", item.name)),
                        }
                    }
                    VfsFileData::CasUrl { .. } => {
                        errors.push(format!(
                            "{}: large CAS files cannot be copied to scratch drives",
                            item.name
                        ));
                    }
                },
                Err(e) => errors.push(format!("{}: read: {e}", item.name)),
            }
        }
    }

    if errors.is_empty() {
        Ok(format!("Copied {copied}/{total} items"))
    } else {
        Ok(format!(
            "Copied {copied}/{total} ({} failed: {})",
            errors.len(),
            errors.join("; ")
        ))
    }
}

#[cfg(feature = "hydrate")]
async fn fetch_directory(
    path: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<(Vec<BrowserItem>, String), String> {
    use crate::vfs::{format_bytes, path_extension};

    if path.drive.is_scratch() {
        let sd = scratch
            .get(path.drive)
            .ok_or_else(|| "Scratch drive not initialized".to_string())?;
        let entries = sd.list(&path.path).await?;
        let items: Vec<BrowserItem> = entries
            .iter()
            .map(|e| {
                let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_string();
                let ext = path_extension(&e.path);
                let icon = vfs_file_icon(ext, e.content_type.as_deref(), e.is_directory);
                BrowserItem {
                    name,
                    full_path: VfsPath {
                        drive: path.drive,
                        path: e.path.clone(),
                    },
                    is_directory: e.is_directory,
                    size_bytes: e.size_bytes,
                    icon,
                    content_type: e.content_type.clone(),
                }
            })
            .collect();

        let (used, quota) = sd.drive_info(path.drive).await.unwrap_or((0, 0));
        let free = quota.saturating_sub(used);
        let status = format!(
            "{} items | {} used / {} total ({} free)",
            items.len(),
            format_bytes(used),
            format_bytes(quota),
            format_bytes(free)
        );
        return Ok((items, status));
    }

    let sid = path.drive.session_id(session_id);

    let entries =
        crate::server::api::vfs_list_dir(path.drive.as_str().to_string(), path.path.clone(), sid)
            .await
            .map_err(|e| e.to_string())?;

    let items: Vec<BrowserItem> = entries
        .iter()
        .map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_string();
            let ext = path_extension(&e.path);
            let icon = vfs_file_icon(ext, e.content_type.as_deref(), e.is_directory);
            let full_path = VfsPath {
                drive: path.drive,
                path: e.path.clone(),
            };
            BrowserItem {
                name,
                full_path,
                is_directory: e.is_directory,
                size_bytes: e.size_bytes,
                icon,
                content_type: e.content_type.clone(),
            }
        })
        .collect();

    // Build status line with drive info
    let info = crate::server::api::vfs_get_drive_info(path.drive.as_str().to_string(), sid).await;

    let status = match info {
        Ok(info) => {
            let free = info.quota_bytes.saturating_sub(info.used_bytes);
            format!(
                "{} items | {} used / {} total ({} free)",
                items.len(),
                format_bytes(info.used_bytes),
                format_bytes(info.quota_bytes),
                format_bytes(free)
            )
        }
        Err(_) => format!("{} items", items.len()),
    };

    Ok((items, status))
}

#[cfg(feature = "hydrate")]
async fn create_directory(
    path: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<(), String> {
    if path.drive.is_scratch() {
        let sd = scratch
            .get(path.drive)
            .ok_or_else(|| "Scratch drive not initialized".to_string())?;
        return sd.mkdir(&path.path).await;
    }

    let sid = path.drive.session_id(session_id);

    crate::server::api::vfs_mkdir_dir(path.drive.as_str().to_string(), path.path.clone(), sid)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(feature = "hydrate")]
async fn delete_entry(
    path: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<(), String> {
    if path.drive.is_scratch() {
        let sd = scratch
            .get(path.drive)
            .ok_or_else(|| "Scratch drive not initialized".to_string())?;
        return sd.delete(&path.path).await;
    }

    let sid = path.drive.session_id(session_id);

    crate::server::api::vfs_delete_file(path.drive.as_str().to_string(), path.path.clone(), sid)
        .await
        .map_err(|e| e.to_string())
}

/// Maximum directory nesting depth for scratch drive rename operations.
/// Scratch drives emulate rename via read+write+delete, recursing into
/// subdirectories.  This cap prevents runaway recursion on pathological
/// directory trees (WASM has a limited call stack).
#[cfg(feature = "hydrate")]
const SCRATCH_RENAME_MAX_DEPTH: u32 = 64;

#[cfg(feature = "hydrate")]
async fn rename_entry(
    old_path: &VfsPath,
    new_path: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<(), String> {
    if old_path.drive.is_scratch() {
        return scratch_rename(old_path, new_path, scratch, 0).await;
    }

    let sid = old_path.drive.session_id(session_id);

    crate::server::api::vfs_rename_file(
        old_path.drive.as_str().to_string(),
        old_path.path.clone(),
        new_path.path.clone(),
        sid,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Rename on scratch drives (IndexedDB has no native rename).
/// Files: read → write → delete.
/// Directories: create new dir, recursively move children, delete old dir.
#[cfg(feature = "hydrate")]
async fn scratch_rename(
    old_path: &VfsPath,
    new_path: &VfsPath,
    scratch: &ScratchDrives,
    depth: u32,
) -> Result<(), String> {
    if depth >= SCRATCH_RENAME_MAX_DEPTH {
        return Err(format!(
            "Rename aborted: directory nesting exceeds {} levels",
            SCRATCH_RENAME_MAX_DEPTH
        ));
    }

    let sd = scratch
        .get(old_path.drive)
        .ok_or_else(|| "Scratch drive not initialized".to_string())?;

    let entry = sd.stat(&old_path.path).await?;
    if entry.is_directory {
        // Ignore "Already exists" — target dir may have been created by a
        // previous partial rename or the caller.
        if let Err(e) = sd.mkdir(&new_path.path).await {
            if !e.contains("Already exists") {
                return Err(e);
            }
        }
        let children = sd.list(&old_path.path).await?;
        for child in &children {
            let child_name = child.path.rsplit('/').next().unwrap_or(&child.path);
            let new_child_path = if new_path.path == "/" {
                format!("/{child_name}")
            } else {
                format!("{}/{child_name}", new_path.path)
            };
            let child_old = VfsPath {
                drive: old_path.drive,
                path: child.path.clone(),
            };
            let child_new = VfsPath {
                drive: old_path.drive,
                path: new_child_path,
            };
            Box::pin(scratch_rename(&child_old, &child_new, scratch, depth + 1)).await?;
        }
        sd.delete(&old_path.path).await?;
    } else {
        let (data, ct) = sd.read(&old_path.path).await?;
        sd.write(&new_path.path, &data, ct.as_deref()).await?;
        sd.delete(&old_path.path).await?;
    }
    Ok(())
}

/// Download a single file to the browser.
#[cfg(feature = "hydrate")]
async fn download_file(
    path: &VfsPath,
    filename: &str,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<String, String> {
    use crate::models::VfsFileData;
    use crate::vfs::format_bytes;
    use wasm_bindgen::JsCast;

    if path.drive.is_scratch() {
        let sd = scratch
            .get(path.drive)
            .ok_or_else(|| "Scratch drive not initialized".to_string())?;
        let (data, content_type) = sd.read(&path.path).await?;
        let ct = content_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        trigger_browser_download(filename, &data, ct);
        return Ok(format!(
            "Downloaded {} ({})",
            filename,
            format_bytes(data.len() as u64)
        ));
    }

    let sid = path.drive.session_id(session_id);

    let data =
        crate::server::api::vfs_read_file(path.drive.as_str().to_string(), path.path.clone(), sid)
            .await
            .map_err(|e| e.to_string())?;

    match data {
        VfsFileData::Inline { data, content_type } => {
            let ct = content_type
                .as_deref()
                .unwrap_or("application/octet-stream");
            trigger_browser_download(filename, &data, ct);
            Ok(format!(
                "Downloaded {} ({})",
                filename,
                format_bytes(data.len() as u64)
            ))
        }
        VfsFileData::CasUrl {
            url, size_bytes, ..
        } => {
            let window = web_sys::window().unwrap();
            let document = window.document().unwrap();
            let a: web_sys::HtmlAnchorElement =
                document.create_element("a").unwrap().dyn_into().unwrap();
            a.set_href(&url);
            a.set_download(filename);
            let _ = a.set_attribute("style", "display:none");
            let _ = document.body().unwrap().append_child(&a);
            a.click();
            let _ = document.body().unwrap().remove_child(&a);
            Ok(format!(
                "Downloaded {} ({})",
                filename,
                format_bytes(size_bytes as u64)
            ))
        }
    }
}

#[cfg(feature = "hydrate")]
use crate::components::browser_helpers::trigger_browser_download;

/// Preview a file if it's text or image, otherwise download it.
#[cfg(feature = "hydrate")]
async fn preview_or_download(
    path: &VfsPath,
    filename: &str,
    content_type: Option<&str>,
    session_id: i32,
    scratch: &ScratchDrives,
    preview: &RwSignal<Option<PreviewContent>>,
) {
    use crate::models::VfsFileData;

    let ct = content_type.unwrap_or("");
    let is_text = ct.starts_with("text/") || ct == "application/json" || ct == "application/xml";
    let is_image = ct.starts_with("image/");

    // For scratch drives, read directly
    if path.drive.is_scratch() {
        let sd = match scratch.get(path.drive) {
            Some(sd) => sd,
            None => return,
        };
        match sd.read(&path.path).await {
            Ok((data, _ct)) => {
                if is_image {
                    let blob_url = blob_url_from_bytes(&data, ct);
                    preview.set(Some(PreviewContent::Image {
                        name: filename.to_string(),
                        url: blob_url,
                    }));
                } else if is_text {
                    match String::from_utf8(data) {
                        Ok(text) => {
                            preview.set(Some(PreviewContent::Text {
                                name: filename.to_string(),
                                text,
                            }));
                        }
                        Err(_) => {
                            // Binary — just download
                            let _ = download_file(path, filename, session_id, scratch).await;
                        }
                    }
                } else {
                    let _ = download_file(path, filename, session_id, scratch).await;
                }
            }
            Err(_) => {
                let _ = download_file(path, filename, session_id, scratch).await;
            }
        }
        return;
    }

    let sid = path.drive.session_id(session_id);

    let data = match crate::server::api::vfs_read_file(
        path.drive.as_str().to_string(),
        path.path.clone(),
        sid,
    )
    .await
    {
        Ok(d) => d,
        Err(_) => return,
    };

    match data {
        VfsFileData::Inline { data, .. } => {
            if is_image {
                let blob_url = blob_url_from_bytes(&data, ct);
                preview.set(Some(PreviewContent::Image {
                    name: filename.to_string(),
                    url: blob_url,
                }));
            } else if is_text {
                match String::from_utf8(data) {
                    Ok(text) => {
                        preview.set(Some(PreviewContent::Text {
                            name: filename.to_string(),
                            text,
                        }));
                    }
                    Err(_) => {
                        let _ = download_file(path, filename, session_id, scratch).await;
                    }
                }
            } else {
                let _ = download_file(path, filename, session_id, scratch).await;
            }
        }
        VfsFileData::CasUrl { url, .. } => {
            if is_image {
                preview.set(Some(PreviewContent::Image {
                    name: filename.to_string(),
                    url,
                }));
            } else {
                // Large non-image files — download directly
                let _ = download_file(path, filename, session_id, scratch).await;
            }
        }
    }
}

/// Create a blob URL from raw bytes for preview display.
#[cfg(feature = "hydrate")]
fn blob_url_from_bytes(data: &[u8], content_type: &str) -> String {
    use js_sys::{Array, Uint8Array};

    let uint8 = Uint8Array::from(data);
    let array = Array::new();
    array.push(&uint8.buffer());

    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(content_type);
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&array, &opts).unwrap();
    web_sys::Url::create_object_url_with_blob(&blob).unwrap()
}

/// Open a browser file picker and upload files to the current directory.
/// Continues uploading remaining files on per-file errors (like the terminal's PUT).
#[cfg(feature = "hydrate")]
async fn upload_files(
    dest: &VfsPath,
    session_id: i32,
    scratch: &ScratchDrives,
) -> Result<String, String> {
    use wasm_bindgen_futures::JsFuture;

    use crate::components::browser_helpers::{open_file_picker, upload_large_file};

    let dest_scratch = dest.drive.is_scratch();
    if dest_scratch && scratch.get(dest.drive).is_none() {
        return Err("Scratch drive not initialized".to_string());
    }

    let files = open_file_picker().await?;

    let sid = dest.drive.session_id(session_id);

    let mut uploaded = 0u32;
    let mut errors = Vec::new();
    let total = files.length();

    for i in 0..total {
        let file = match files.get(i) {
            Some(f) => f,
            None => continue,
        };

        let name = file.name();
        let size = file.size() as u64;

        let array_buffer = match JsFuture::from(file.array_buffer()).await {
            Ok(ab) => ab,
            Err(e) => {
                errors.push(format!("{name}: read error ({e:?})"));
                continue;
            }
        };
        let uint8 = js_sys::Uint8Array::new(&array_buffer);
        let data = uint8.to_vec();

        let file_path = if dest.path == "/" {
            format!("/{name}")
        } else {
            format!("{}/{name}", dest.path)
        };

        let content_type = {
            let t = file.type_();
            if t.is_empty() { None } else { Some(t) }
        };

        if dest_scratch {
            let sd = scratch.get(dest.drive).unwrap();
            match sd.write(&file_path, &data, content_type.as_deref()).await {
                Ok(()) => uploaded += 1,
                Err(e) => errors.push(format!("{name}: {e}")),
            }
            continue;
        }

        // Server drives: small files inline, large files via media upload
        if size <= 8192 {
            match crate::server::api::vfs_write_file(
                dest.drive.as_str().to_string(),
                file_path,
                data,
                content_type,
                sid,
            )
            .await
            {
                Ok(()) => uploaded += 1,
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        } else {
            match upload_large_file(&file, &dest.drive, &file_path, size, content_type, sid).await {
                Ok(()) => uploaded += 1,
                Err(e) => errors.push(format!("{name}: {e}")),
            }
        }
    }

    if errors.is_empty() {
        Ok(format!("Uploaded {uploaded}/{total} files"))
    } else {
        Ok(format!(
            "Uploaded {uploaded}/{total} ({} failed: {})",
            errors.len(),
            errors.join("; ")
        ))
    }
}
