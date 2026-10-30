use leptos::prelude::*;

use crate::components::window_manager::WindowManagerContext;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

/// Build an SVG string for a clock face: `segments` pie wedges, the first
/// `filled` of them painted in `filled_color` and the rest in `unfilled_color`.
/// Filling starts at the top (12 o'clock) and goes clockwise.
fn clock_svg(segments: i32, filled: i32, filled_color: &str, unfilled_color: &str) -> String {
    let segments = segments.max(1);
    let filled = filled.clamp(0, segments);
    let cx = 50.0_f64;
    let cy = 50.0_f64;
    let r = 46.0_f64;
    let step = std::f64::consts::TAU / segments as f64;
    // Start at the top: -90 degrees in standard orientation. With SVG's y-axis
    // pointing down, increasing the angle sweeps clockwise.
    let start = -std::f64::consts::FRAC_PI_2;

    let point = |angle: f64| -> (f64, f64) { (cx + r * angle.cos(), cy + r * angle.sin()) };

    let mut svg = String::from(
        "<svg viewBox=\"0 0 100 100\" width=\"100%\" height=\"100%\" preserveAspectRatio=\"xMidYMid meet\">",
    );

    if segments == 1 {
        // A single segment is just a disc.
        let color = if filled >= 1 {
            filled_color
        } else {
            unfilled_color
        };
        svg.push_str(&format!(
            "<circle cx=\"50\" cy=\"50\" r=\"46\" fill=\"{color}\" stroke=\"#00000055\" stroke-width=\"1\"/>",
        ));
    } else {
        for i in 0..segments {
            let a0 = start + step * i as f64;
            let a1 = start + step * (i + 1) as f64;
            let (x0, y0) = point(a0);
            let (x1, y1) = point(a1);
            let large_arc = if step > std::f64::consts::PI { 1 } else { 0 };
            let color = if i < filled {
                filled_color
            } else {
                unfilled_color
            };
            svg.push_str(&format!(
                "<path d=\"M {cx:.2} {cy:.2} L {x0:.2} {y0:.2} A {r:.2} {r:.2} 0 {large_arc} 1 {x1:.2} {y1:.2} Z\" fill=\"{color}\" stroke=\"#00000055\" stroke-width=\"1\"/>",
            ));
        }
    }

    // Outer ring for a crisp edge.
    svg.push_str(
        "<circle cx=\"50\" cy=\"50\" r=\"46\" fill=\"none\" stroke=\"#00000088\" stroke-width=\"2\"/>",
    );
    svg.push_str("</svg>");
    svg
}

#[cfg(test)]
mod tests {
    use super::clock_svg;

    #[test]
    fn renders_one_wedge_per_segment() {
        let svg = clock_svg(4, 1, "#ff0000", "#000000");
        assert_eq!(svg.matches("<path").count(), 4);
    }

    #[test]
    fn fills_the_first_n_wedges() {
        // Two filled of six: the filled color appears twice, once per filled
        // wedge; the empty color fills the remaining four.
        let svg = clock_svg(6, 2, "#ff0000", "#00ff00");
        assert_eq!(svg.matches("fill=\"#ff0000\"").count(), 2);
        assert_eq!(svg.matches("fill=\"#00ff00\"").count(), 4);
    }

    #[test]
    fn clamps_filled_to_segment_count() {
        // Over-full and negative fills are clamped into range.
        let full = clock_svg(3, 99, "#ff0000", "#00ff00");
        assert_eq!(full.matches("fill=\"#ff0000\"").count(), 3);
        assert_eq!(full.matches("fill=\"#00ff00\"").count(), 0);
        let empty = clock_svg(3, -5, "#ff0000", "#00ff00");
        assert_eq!(empty.matches("fill=\"#ff0000\"").count(), 0);
        assert_eq!(empty.matches("fill=\"#00ff00\"").count(), 3);
    }

    #[test]
    fn single_segment_is_a_disc() {
        let svg = clock_svg(1, 1, "#ff0000", "#00ff00");
        assert_eq!(svg.matches("<path").count(), 0);
        assert!(svg.contains("fill=\"#ff0000\""));
    }
}

/// Hub window listing the session's progress clocks. Everyone sees the list and
/// can open a clock into its own window; only the GM gets the create and delete
/// controls.
#[component]
pub fn ProgressClockHub() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let wm = expect_context::<WindowManagerContext>();
    let clocks = ctx.progress_clocks;
    let is_gm = ctx.is_gm;
    let send = ctx.send;

    let show_form = RwSignal::new(false);
    let new_label = RwSignal::new(String::new());
    let new_segments = RwSignal::new(6i32);
    let new_filled_color = RwSignal::new("#cc4444".to_string());
    let new_unfilled_color = RwSignal::new("#2a2a2a".to_string());

    let do_create = move || {
        let label = new_label.get().trim().to_string();
        let label = if label.is_empty() {
            "Clock".to_string()
        } else {
            label
        };
        send.send(ClientMessage::CreateProgressClock {
            label,
            segments: new_segments.get().clamp(2, 24),
            filled_color: new_filled_color.get(),
            unfilled_color: new_unfilled_color.get(),
        });
        new_label.set(String::new());
        new_segments.set(6);
        show_form.set(false);
    };

    view! {
        <div class="clock-hub">
            <div class="panel-header">
                <h3>"Progress Clocks"</h3>
                <Show when=move || is_gm.get()>
                    <div class="panel-header-actions">
                        <button
                            class="btn-add"
                            data-tooltip="New clock"
                            on:click=move |_| show_form.update(|s| *s = !*s)
                        >"+"</button>
                    </div>
                </Show>
            </div>

            <Show when=move || is_gm.get() && show_form.get()>
                <div class="create-form">
                    <h4>"New Clock"</h4>
                    <div class="field-row">
                        <label>"Label"</label>
                        <input
                            type="text"
                            placeholder="Clock name"
                            prop:value=move || new_label.get()
                            on:input=move |ev| new_label.set(event_target_value(&ev))
                            on:keydown=move |ev| if ev.key() == "Enter" { do_create() }
                        />
                    </div>
                    <div class="field-row">
                        <label>"Segments"</label>
                        <input
                            type="number"
                            min="2"
                            max="24"
                            prop:value=move || new_segments.get()
                            on:input=move |ev| {
                                if let Ok(n) = event_target_value(&ev).parse::<i32>() {
                                    new_segments.set(n.clamp(2, 24));
                                }
                            }
                        />
                    </div>
                    <div class="field-row">
                        <label>"Filled"</label>
                        <input
                            type="color"
                            prop:value=move || new_filled_color.get()
                            on:input=move |ev| new_filled_color.set(event_target_value(&ev))
                        />
                        <label>"Empty"</label>
                        <input
                            type="color"
                            prop:value=move || new_unfilled_color.get()
                            on:input=move |ev| new_unfilled_color.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="form-actions">
                        <button on:click=move |_| do_create()>"Create"</button>
                        <button class="btn-cancel" on:click=move |_| show_form.set(false)>"Cancel"</button>
                    </div>
                </div>
            </Show>

            <Show when=move || clocks.get().is_empty()>
                <p class="clock-hub-empty">"No clocks yet."</p>
            </Show>
            <ul class="clock-hub-list">
                <For
                    each=move || clocks.get()
                    key=|c| (c.id, c.filled, c.segments, c.locked, c.label.clone())
                    children=move |clock| {
                        let cid = clock.id;
                        let label = clock.label.clone();
                        let open_label = clock.label.clone();
                        let locked = clock.locked;
                        let wm_open = wm.clone();
                        view! {
                            <li class="clock-hub-item">
                                <span
                                    class="clock-hub-swatch"
                                    inner_html=clock_svg(clock.segments, clock.filled, &clock.filled_color, &clock.unfilled_color)
                                ></span>
                                <span class="clock-hub-name">{label}</span>
                                <span class="clock-hub-count">{format!("{}/{}", clock.filled, clock.segments)}</span>
                                {locked.then(|| view! { <span class="clock-hub-lock" data-tooltip="Locked">"\u{1f512}"</span> })}
                                <button
                                    class="clock-hub-open"
                                    on:click=move |_| wm_open.open_progress_clock(cid, &open_label)
                                >"Open"</button>
                                <Show when=move || is_gm.get()>
                                    <button
                                        class="clock-hub-del"
                                        data-tooltip="Remove clock"
                                        on:click=move |_| send.send(ClientMessage::DeleteProgressClock { clock_id: cid })
                                    >"\u{00d7}"</button>
                                </Show>
                            </li>
                        }
                    }
                />
            </ul>
        </div>
    }
}

/// A single progress clock shown in its own window. The GM sees advance,
/// lock, and edit controls; players see the clock read-only.
#[component]
pub fn ProgressClockWindowPanel(clock_id: i32) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let clocks = ctx.progress_clocks;
    let is_gm = ctx.is_gm;
    let send = ctx.send;

    let find = move || clocks.get().into_iter().find(|c| c.id == clock_id);

    // Edit mode for the label and description (GM only).
    let editing = RwSignal::new(false);
    let draft_label = RwSignal::new(String::new());
    let draft_desc = RwSignal::new(String::new());

    let start_edit = move || {
        if let Some(c) = clocks
            .get_untracked()
            .into_iter()
            .find(|c| c.id == clock_id)
        {
            draft_label.set(c.label);
            draft_desc.set(c.description);
            editing.set(true);
        }
    };
    let save_edit = move || {
        send.send(ClientMessage::EditProgressClock {
            clock_id,
            label: draft_label.get().trim().to_string(),
            description: draft_desc.get(),
        });
        editing.set(false);
    };

    let dec = move |_| {
        if let Some(c) = clocks
            .get_untracked()
            .into_iter()
            .find(|c| c.id == clock_id)
        {
            send.send(ClientMessage::SetProgressClockFilled {
                clock_id,
                filled: (c.filled - 1).max(0),
            });
        }
    };
    let inc = move |_| {
        if let Some(c) = clocks
            .get_untracked()
            .into_iter()
            .find(|c| c.id == clock_id)
        {
            send.send(ClientMessage::SetProgressClockFilled {
                clock_id,
                filled: (c.filled + 1).min(c.segments),
            });
        }
    };
    let toggle_lock = move |_| {
        if let Some(c) = clocks
            .get_untracked()
            .into_iter()
            .find(|c| c.id == clock_id)
        {
            send.send(ClientMessage::SetProgressClockLock {
                clock_id,
                locked: !c.locked,
            });
        }
    };

    view! {
        <div class="clock-window">
            {move || {
                let Some(clock) = find() else {
                    return view! { <p class="clock-removed">"This clock was removed."</p> }.into_any();
                };
                let gm = is_gm.get();
                let locked = clock.locked;
                let at_min = clock.filled <= 0;
                let at_max = clock.filled >= clock.segments;
                let svg = clock_svg(clock.segments, clock.filled, &clock.filled_color, &clock.unfilled_color);

                view! {
                    <div class="clock-topbar">
                        <span class="clock-window-label">{clock.label.clone()}</span>
                        <Show when=move || gm>
                            <button
                                class="clock-lock-btn"
                                class:locked=move || locked
                                data-tooltip=move || if locked { "Unlock" } else { "Lock" }
                                on:click=toggle_lock
                            >{move || if locked { "\u{1f512}" } else { "\u{1f513}" }}</button>
                        </Show>
                        <Show when=move || !gm && locked>
                            <span class="clock-lock-indicator" data-tooltip="Locked">"\u{1f512}"</span>
                        </Show>
                    </div>

                    <div class="clock-face" inner_html=svg></div>
                    <div class="clock-count">{format!("{} / {}", clock.filled, clock.segments)}</div>

                    <Show when=move || gm>
                        <div class="clock-controls">
                            <button
                                class="clock-step"
                                prop:disabled=move || locked || at_min
                                on:click=dec
                            >"\u{2212}"</button>
                            <button
                                class="clock-step"
                                prop:disabled=move || locked || at_max
                                on:click=inc
                            >"+"</button>
                        </div>
                    </Show>

                    {
                        let desc = clock.description.clone();
                        move || if gm && editing.get() {
                            view! {
                                <div class="clock-edit">
                                    <input
                                        type="text"
                                        class="clock-edit-label"
                                        placeholder="Label"
                                        prop:value=move || draft_label.get()
                                        on:input=move |ev| draft_label.set(event_target_value(&ev))
                                    />
                                    <textarea
                                        class="clock-edit-desc"
                                        placeholder="Description"
                                        prop:value=move || draft_desc.get()
                                        on:input=move |ev| draft_desc.set(event_target_value(&ev))
                                    ></textarea>
                                    <div class="form-actions">
                                        <button on:click=move |_| save_edit()>"Save"</button>
                                        <button class="btn-cancel" on:click=move |_| editing.set(false)>"Cancel"</button>
                                    </div>
                                </div>
                            }.into_any()
                        } else {
                            let desc = desc.clone();
                            view! {
                                <div class="clock-desc-area">
                                    {(!desc.is_empty()).then(|| view! { <p class="clock-desc">{desc.clone()}</p> })}
                                    <Show when=move || gm>
                                        <button
                                            class="clock-edit-btn"
                                            prop:disabled=move || locked
                                            on:click=move |_| start_edit()
                                        >"Edit"</button>
                                    </Show>
                                </div>
                            }.into_any()
                        }
                    }
                }.into_any()
            }}
        </div>
    }
}
