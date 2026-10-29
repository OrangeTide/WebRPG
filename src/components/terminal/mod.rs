//! COMMAND.COM terminal panel: a NeXTSTEP-styled DOS shell over the VFS.

use leptos::prelude::*;

use crate::vfs::{Drive, VfsPath};

mod commands;
mod parser;

pub use commands::vfs_file_icon;

use commands::{cmd_cd, cmd_help, cmd_ver};
#[cfg(feature = "hydrate")]
use commands::{execute_command, tab_complete};
use parser::{parse_args, parse_command_line};

/// Terminal color theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum TermTheme {
    GrayOnBlack,
    GreenOnBlack,
    AmberOnBlack,
    WhiteOnBlue,
    HotDogStand,
}

impl TermTheme {
    fn label(self) -> &'static str {
        match self {
            Self::GrayOnBlack => "Gray on Black",
            Self::GreenOnBlack => "Green on Black",
            Self::AmberOnBlack => "Amber on Black",
            Self::WhiteOnBlue => "White on Blue",
            Self::HotDogStand => "Hot Dog Stand",
        }
    }

    fn css_vars(self) -> (&'static str, &'static str, &'static str) {
        // Returns (fg, bg, caret) colors
        match self {
            Self::GrayOnBlack => ("#aaaaaa", "#0c0c0c", "#aaaaaa"),
            Self::GreenOnBlack => ("#00aa00", "#0c0c0c", "#00aa00"),
            Self::AmberOnBlack => ("#aa5500", "#0c0c0c", "#aa5500"),
            Self::WhiteOnBlue => ("#56ffff", "#0000aa", "#56ffff"),
            Self::HotDogStand => {
                // Randomly pick one of two combos; resolved at render time
                // We'll handle this specially in the component
                ("#000000", "#ffff00", "#000000")
            }
        }
    }

    fn all() -> &'static [Self] {
        &[
            Self::GrayOnBlack,
            Self::GreenOnBlack,
            Self::AmberOnBlack,
            Self::WhiteOnBlue,
            Self::HotDogStand,
        ]
    }
}

#[cfg(feature = "hydrate")]
const TERM_THEME_KEY: &str = "webrpg_term_theme";

#[cfg(feature = "hydrate")]
fn load_term_theme() -> TermTheme {
    web_sys::window()
        .and_then(|w| w.local_storage().ok()?)
        .and_then(|s| s.get_item(TERM_THEME_KEY).ok()?)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(TermTheme::GrayOnBlack)
}

#[cfg(feature = "hydrate")]
fn save_term_theme(theme: TermTheme) {
    if let Some(window) = web_sys::window()
        && let Ok(Some(storage)) = window.local_storage()
        && let Ok(json) = serde_json::to_string(&theme)
    {
        let _ = storage.set_item(TERM_THEME_KEY, &json);
    }
}

/// A single line of terminal output.
#[derive(Debug, Clone)]
struct TermLine {
    text: String,
}

/// Shell state for the COMMAND.COM terminal.
struct ShellState {
    /// Current working directory (always a valid VfsPath pointing to a directory).
    cwd: VfsPath,
}

impl Default for ShellState {
    fn default() -> Self {
        ShellState {
            cwd: VfsPath {
                drive: Drive::A,
                path: "/".to_string(),
            },
        }
    }
}

impl ShellState {
    fn prompt(&self) -> String {
        format!("{}> ", self.cwd)
    }
}

#[component]
pub fn TerminalPanel() -> impl IntoView {
    let ctx = expect_context::<crate::pages::game::GameContext>();
    let _session_id = ctx.session_id;

    let output = RwSignal::new(vec![
        TermLine {
            text: "WebRPG COMMAND.COM v1.0".to_string(),
        },
        TermLine {
            text: "Type HELP for a list of commands.".to_string(),
        },
        TermLine {
            text: String::new(),
        },
    ]);

    // ShellState is Send + Sync (just a VfsPath), so it needs no LocalStorage.
    // Using new_local here would create a SendWrapper that panics if the SSR
    // render is dropped on a different worker thread than it was built on.
    let shell = StoredValue::new(ShellState::default());
    let (input, set_input) = signal(String::new());
    let (prompt, set_prompt) = signal("A:/>  ".to_string());
    let history: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let history_pos: RwSignal<Option<usize>> = RwSignal::new(None);
    let output_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    // Color theme — default to gray-on-black (matches SSR), then load saved preference post-hydration
    let theme = RwSignal::new(TermTheme::GrayOnBlack);
    // Hot Dog Stand random variant: 0 = black-on-yellow, 1 = white-on-red
    let hotdog_variant = RwSignal::new(0u8);
    #[cfg(feature = "hydrate")]
    Effect::new(move |prev: Option<()>| {
        if prev.is_none() {
            // First run (post-hydration): load saved theme so the signal change updates the DOM
            let saved = load_term_theme();
            if saved != TermTheme::GrayOnBlack {
                theme.set(saved);
            }
            hotdog_variant.set(if js_sys::Math::random() < 0.5 { 0 } else { 1 });
        }
    });

    let theme_style = move || {
        let t = theme.get();
        if t == TermTheme::HotDogStand {
            let variant = hotdog_variant.get();
            if variant == 0 {
                "color:#000000;background:#ffff00;caret-color:#000000;".to_string()
            } else {
                "color:#ffffff;background:#aa0000;caret-color:#ffffff;".to_string()
            }
        } else {
            let (fg, bg, caret) = t.css_vars();
            format!("color:{fg};background:{bg};caret-color:{caret};")
        }
    };

    #[cfg(feature = "hydrate")]
    let scratch_drives =
        expect_context::<RwSignal<crate::scratch_drive::ScratchDrives, LocalStorage>>();

    // Auto-scroll to bottom when output changes
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let _len = output.with(|lines| lines.len());
        if let Some(el) = output_ref.get() {
            leptos::prelude::request_animation_frame(move || {
                let el: &web_sys::HtmlElement = &el;
                el.set_scroll_top(el.scroll_height());
            });
        }
    });

    // Auto-focus input on mount
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(el) = input_ref.get() {
            let _ = el.focus();
        }
    });

    let on_submit = move || {
        let text = input.get();
        let prompt_str = prompt.get();

        // Add prompt + input to output
        output.update(|lines| {
            lines.push(TermLine {
                text: format!("{}{}", prompt_str, text),
            });
        });

        let trimmed = text.trim().to_string();
        set_input.set(String::new());

        // Add to history
        if !trimmed.is_empty() {
            history.update(|h| {
                // Don't duplicate consecutive entries
                if h.last().map(|l| l.as_str()) != Some(&trimmed) {
                    h.push(trimmed.clone());
                }
            });
        }
        history_pos.set(None);

        if trimmed.is_empty() {
            // Update prompt (cwd might have changed)
            shell.with_value(|s| set_prompt.set(s.prompt()));
            return;
        }

        let parsed = parse_command_line(&trimmed);
        let (cmd, args_str) = match parsed {
            Some((c, a)) => (c, a),
            None => {
                shell.with_value(|s| set_prompt.set(s.prompt()));
                return;
            }
        };

        // Bare drive letter (e.g. "C:") — switch to that drive
        if cmd.len() == 2 && cmd.ends_with(':') && cmd.as_bytes()[0].is_ascii_alphabetic() {
            let mut cd_result = Vec::new();
            shell.update_value(|s| {
                cd_result = cmd_cd(s, std::slice::from_ref(&cmd));
            });
            output.update(|lines| {
                for line in cd_result {
                    lines.push(TermLine { text: line });
                }
            });
            shell.with_value(|s| set_prompt.set(s.prompt()));
            return;
        }

        // CLS is special — clears the output buffer
        if cmd == "CLS" {
            output.set(Vec::new());
            shell.with_value(|s| set_prompt.set(s.prompt()));
            return;
        }

        // EXIT — reset state and minimize the window
        if cmd == "EXIT" {
            output.set(vec![
                TermLine {
                    text: "WebRPG COMMAND.COM v1.0".to_string(),
                },
                TermLine {
                    text: "Type HELP for a list of commands.".to_string(),
                },
                TermLine {
                    text: String::new(),
                },
            ]);
            shell.update_value(|s| {
                *s = ShellState::default();
            });
            set_prompt.set(shell.with_value(|s| s.prompt()));
            let wm = expect_context::<crate::components::window_manager::WindowManagerContext>();
            wm.minimize_window(crate::components::window_manager::WindowId::Terminal);
            return;
        }

        // CD is synchronous
        if cmd == "CD" || cmd == "CHDIR" {
            let (_, positional) = parse_args(&args_str);
            let mut cd_result = Vec::new();
            shell.update_value(|s| {
                cd_result = cmd_cd(s, &positional);
            });
            output.update(|lines| {
                for line in cd_result {
                    lines.push(TermLine { text: line });
                }
            });
            shell.with_value(|s| set_prompt.set(s.prompt()));
            return;
        }

        // VER and HELP are synchronous
        if cmd == "VER" {
            let result = cmd_ver();
            output.update(|lines| {
                for line in result {
                    lines.push(TermLine { text: line });
                }
            });
            shell.with_value(|s| set_prompt.set(s.prompt()));
            return;
        }

        if cmd == "HELP" {
            let (_, positional) = parse_args(&args_str);
            let result = cmd_help(positional.first().map(|s| s.as_str()));
            output.update(|lines| {
                for line in result {
                    lines.push(TermLine { text: line });
                }
            });
            shell.with_value(|s| set_prompt.set(s.prompt()));
        }

        // Async commands — spawn
        #[cfg(feature = "hydrate")]
        {
            let sid = _session_id.get();
            // Copy cwd so we can use it in the async block
            let cwd = shell.with_value(|s| s.cwd.clone());
            let sd = scratch_drives.get();
            leptos::task::spawn_local(async move {
                let snap = ShellState { cwd };
                let lines = execute_command(&snap, &cmd, &args_str, sid, &sd).await;
                output.update(|out| {
                    for line in lines {
                        out.push(TermLine { text: line });
                    }
                });
                shell.with_value(|s| set_prompt.set(s.prompt()));
            });
        }
    };

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                on_submit();
            }
            "ArrowUp" => {
                ev.prevent_default();
                let h = history.get();
                if h.is_empty() {
                    return;
                }
                let pos = match history_pos.get() {
                    Some(p) if p > 0 => p - 1,
                    Some(0) => 0,
                    None => h.len() - 1,
                    Some(p) => p,
                };
                history_pos.set(Some(pos));
                set_input.set(h[pos].clone());
            }
            "ArrowDown" => {
                ev.prevent_default();
                let h = history.get();
                match history_pos.get() {
                    Some(p) if p + 1 < h.len() => {
                        let next = p + 1;
                        history_pos.set(Some(next));
                        set_input.set(h[next].clone());
                    }
                    Some(_) => {
                        history_pos.set(None);
                        set_input.set(String::new());
                    }
                    None => {}
                }
            }
            "Tab" => {
                ev.prevent_default();
                #[cfg(feature = "hydrate")]
                {
                    let current = input.get();
                    let sid = _session_id.get();
                    let cwd = shell.with_value(|s| s.cwd.clone());
                    let sd = scratch_drives.get();
                    leptos::task::spawn_local(async move {
                        let snap = ShellState { cwd };
                        let (matches, completed) = tab_complete(&snap, &current, sid, &sd).await;
                        if let Some(completed) = completed {
                            if matches.len() == 1 {
                                // Single match — insert it (add space for commands)
                                let trimmed = current.trim_start();
                                if !trimmed.contains(' ') {
                                    set_input.set(format!("{} ", completed));
                                } else {
                                    set_input.set(completed);
                                }
                            } else {
                                // Multiple matches — show them and insert common prefix
                                output.update(|out| {
                                    let prompt_str = prompt.get();
                                    out.push(TermLine {
                                        text: format!("{}{}", prompt_str, current),
                                    });
                                    out.push(TermLine {
                                        text: matches.join("  "),
                                    });
                                });
                                set_input.set(completed);
                            }
                        }
                    });
                }
            }
            _ => {}
        }
    };

    let show_theme_popup = RwSignal::new(false);
    let theme_popup_pos = RwSignal::new(None::<(i32, i32)>);

    let select_theme = move |new_theme: TermTheme| {
        theme.set(new_theme);
        #[cfg(feature = "hydrate")]
        save_term_theme(new_theme);
        if new_theme == TermTheme::HotDogStand {
            #[cfg(feature = "hydrate")]
            hotdog_variant.set(if js_sys::Math::random() < 0.5 { 0 } else { 1 });
        }
        show_theme_popup.set(false);
    };

    view! {
        <div class="terminal-panel" style=theme_style on:click=move |_| {
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
        }>
            <div class="terminal-toolbar">
                <button
                    class="terminal-theme-btn"
                    data-tooltip="Color Theme"
                    on:click=move |ev: leptos::ev::MouseEvent| {
                        ev.stop_propagation();
                        theme_popup_pos.set(Some((ev.client_x(), ev.client_y() + 4)));
                        show_theme_popup.update(|v| *v = !*v);
                    }
                >"\u{1f3a8}"</button>
            </div>
            <div class="terminal-output" node_ref=output_ref>
                <For
                    each=move || {
                        output.get().into_iter().enumerate().collect::<Vec<_>>()
                    }
                    key=|(i, _)| *i
                    let:item
                >
                    <div class="terminal-line">
                        {item.1.text.clone()}
                    </div>
                </For>
            </div>
            <div class="terminal-input">
                <span class="terminal-prompt">{prompt}</span>
                <input
                    type="text"
                    node_ref=input_ref
                    prop:value=input
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    on:keydown=on_keydown
                    spellcheck="false"
                    autocomplete="off"
                />
            </div>
        </div>

        // Theme popup — rendered outside the terminal panel to avoid overflow clipping
        {move || show_theme_popup.get().then(|| {
            let current = theme.get();
            let (px, py) = theme_popup_pos.get().unwrap_or((0, 0));
            view! {
                <div class="terminal-theme-backdrop" on:click=move |_| show_theme_popup.set(false) on:contextmenu=move |ev: leptos::ev::MouseEvent| { ev.prevent_default(); show_theme_popup.set(false); }>
                    <div class="terminal-theme-popup" style=format!("left:{}px;top:{}px;", px, py) on:click:stopPropagation=|_: leptos::ev::MouseEvent| {}>
                        {TermTheme::all().iter().map(|t| {
                            let t = *t;
                            let (fg, bg, _) = t.css_vars();
                            view! {
                                <label class="terminal-theme-option" on:click=move |_| select_theme(t)>
                                    <span class="terminal-theme-radio">{if t == current { "\u{25c9}" } else { "\u{25cb}" }}</span>
                                    <span class="terminal-theme-swatch" style=format!("color:{fg};background:{bg};")>"Aa"</span>
                                    <span>{t.label()}</span>
                                </label>
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })}
    }
}
