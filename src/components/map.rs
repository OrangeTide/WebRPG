use leptos::prelude::*;

use crate::models::TokenInfo;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

// ===== Condition icons =====

#[cfg(feature = "hydrate")]
fn condition_icon(name: &str) -> &str {
    match name {
        "bloodied" => "\u{1FA78}",          // 🩸
        "poisoned" => "\u{2620}\u{FE0F}",   // ☠️
        "prone" => "\u{2B07}\u{FE0F}",      // ⬇️
        "stunned" => "\u{1F4AB}",           // 💫
        "blinded" => "\u{1F648}",           // 🙈
        "frightened" => "\u{1F628}",        // 😨
        "paralyzed" => "\u{26A1}",          // ⚡
        "restrained" => "\u{26D3}\u{FE0F}", // ⛓️
        "invisible" => "\u{1F47B}",         // 👻
        "concentrating" => "\u{1F52E}",     // 🔮
        _ => "\u{2753}",                    // ❓
    }
}

// ===== Tool system =====

#[derive(Clone, Copy, PartialEq, Eq)]
enum MapTool {
    Select,
    Pan,
    Measure,
    Ping,
}

// ===== Coordinate transforms =====

#[cfg(feature = "hydrate")]
fn screen_to_world(sx: f64, sy: f64, offset: (f64, f64), zoom: f64) -> (f64, f64) {
    (sx / zoom + offset.0, sy / zoom + offset.1)
}

#[cfg(feature = "hydrate")]
fn world_to_screen(wx: f64, wy: f64, offset: (f64, f64), zoom: f64) -> (f64, f64) {
    ((wx - offset.0) * zoom, (wy - offset.1) * zoom)
}

/// Get mouse coordinates relative to the canvas element (screen space).
#[cfg(feature = "hydrate")]
fn canvas_coords(
    canvas_ref: &NodeRef<leptos::html::Canvas>,
    ev: &leptos::ev::MouseEvent,
) -> Option<(f64, f64)> {
    let canvas = canvas_ref.get()?;
    let canvas_el: &web_sys::HtmlCanvasElement = canvas.as_ref();
    let rect = canvas_el.get_bounding_client_rect();
    Some((
        ev.client_x() as f64 - rect.left(),
        ev.client_y() as f64 - rect.top(),
    ))
}

#[component]
#[allow(unused_variables)]
pub fn MapCanvas() -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let map = ctx.map;
    let tokens = ctx.tokens;
    let fog = ctx.fog;
    let send = ctx.send;

    // --- Selection state ---
    let selected_ids = RwSignal::new(std::collections::HashSet::<i32>::new());
    let (dragging, set_dragging) = signal(false);
    let drag_start_world = RwSignal::new(Option::<(f64, f64)>::None);
    let selection_rect = RwSignal::new(Option::<(f64, f64, f64, f64)>::None);
    let drag_token_start = RwSignal::new(Option::<(f64, f64)>::None);
    let drag_token_origins = RwSignal::new(Vec::<(i32, f32, f32)>::new());

    // --- Viewport state ---
    let view_offset = RwSignal::new((0.0_f64, 0.0_f64));
    let view_zoom = RwSignal::new(1.0_f64);
    let panning = RwSignal::new(false);
    let pan_start_screen = RwSignal::new((0.0_f64, 0.0_f64));
    let pan_start_offset = RwSignal::new((0.0_f64, 0.0_f64));
    let space_held = RwSignal::new(false);
    let canvas_size_tick = RwSignal::new(0u32); // bumped on resize to trigger redraw

    // --- Tool state ---
    let active_tool = RwSignal::new(MapTool::Select);
    let snap_to_grid = RwSignal::new(true);

    // --- Measurement state ---
    let measure_start = RwSignal::new(Option::<(f64, f64)>::None);
    let measure_end = RwSignal::new(Option::<(f64, f64)>::None);
    let measure_cursor = RwSignal::new(Option::<(f64, f64)>::None);

    // --- Token list dropdown ---
    let show_token_list = RwSignal::new(false);

    let show_media_browser = RwSignal::new(false);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    let container_ref = NodeRef::<leptos::html::Div>::new();

    // Signal to trigger redraws when images load
    #[cfg(feature = "hydrate")]
    let (image_load_counter, set_image_load_counter) = signal(0u32);

    // Image cache stored as a signal so we can update it reactively
    #[cfg(feature = "hydrate")]
    let image_cache = StoredValue::new_local(std::collections::HashMap::<
        String,
        web_sys::HtmlImageElement,
    >::new());
    // Track URLs with pending decode-retry timeouts to avoid scheduling duplicates
    #[cfg(feature = "hydrate")]
    let image_retry_pending = StoredValue::new_local(std::collections::HashSet::<String>::new());

    // Helper to get or load an image
    #[cfg(feature = "hydrate")]
    let get_or_load_image = move |url: &str| -> Option<web_sys::HtmlImageElement> {
        use wasm_bindgen::JsCast;

        let url_owned = url.to_string();

        let existing = image_cache.with_value(|cache| cache.get(&url_owned).cloned());

        if let Some(img) = existing {
            if img.complete() && img.natural_width() > 0 {
                // Clear any pending retry flag now that image is confirmed good
                image_retry_pending.update_value(|s| {
                    s.remove(&url_owned);
                });
                return Some(img);
            }
            if img.complete() {
                // Image reports complete but has no dimensions — either a
                // load error or Firefox ESR async-decode race. Schedule a
                // single deferred redraw; if still broken on retry, evict
                // so the element gets re-created on the next render.
                let already_pending = image_retry_pending.with_value(|s| s.contains(&url_owned));
                if already_pending {
                    // Retry already fired but image is still broken — evict
                    image_cache.update_value(|cache| {
                        cache.remove(&url_owned);
                    });
                    image_retry_pending.update_value(|s| {
                        s.remove(&url_owned);
                    });
                } else {
                    image_retry_pending.update_value(|s| {
                        s.insert(url_owned.clone());
                    });
                    let handle = leptos::prelude::set_timeout(
                        move || {
                            set_image_load_counter.update(|c| *c += 1);
                        },
                        std::time::Duration::from_millis(150),
                    );
                    let _ = handle;
                }
            }
            return None;
        }

        let img = web_sys::HtmlImageElement::new().ok()?;
        let img_clone = img.clone();
        let url_for_cache = url_owned.clone();

        let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            set_image_load_counter.update(|c| *c += 1);
        });
        img.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();

        // On error, evict from cache and trigger redraw to retry
        let url_for_error = url_owned.clone();
        let onerror = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            image_cache.update_value(|cache| {
                cache.remove(&url_for_error);
            });
            set_image_load_counter.update(|c| *c += 1);
        });
        img.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        img.set_src(&url_owned);

        image_cache.update_value(|cache| {
            cache.insert(url_for_cache, img_clone);
        });

        None
    };

    // Watch for GM viewport sync
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            if let Some((x, y, zoom)) = ctx.viewport_override.get() {
                view_offset.set((x, y));
                view_zoom.set(zoom);
                ctx.viewport_override.set(None);
            }
        });
    }

    // Auto-expire old pings (older than 3 seconds)
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            let pings = ctx.pings.get();
            if pings.is_empty() {
                return;
            }
            let now = web_sys::js_sys::Date::now();
            let had_active = pings.iter().any(|(_, _, _, t)| now - t < 3000.0);
            if had_active {
                // Schedule cleanup
                let handle = leptos::prelude::set_timeout(
                    move || {
                        let now = web_sys::js_sys::Date::now();
                        ctx.pings.update(|pings| {
                            pings.retain(|(_, _, _, t)| now - t < 3000.0);
                        });
                    },
                    std::time::Duration::from_millis(100),
                );
                let _ = handle;
            }
        });
    }

    // Watch for canvas container resize → bump canvas_size_tick to trigger redraw
    #[cfg(feature = "hydrate")]
    {
        let canvas_ref_resize = canvas_ref.clone();
        Effect::new(move |_| {
            use wasm_bindgen::JsCast;
            let Some(canvas) = canvas_ref_resize.get() else {
                return;
            };
            let canvas_el: &web_sys::HtmlCanvasElement = canvas.as_ref();
            let cb = wasm_bindgen::closure::Closure::<
                dyn Fn(wasm_bindgen::JsValue, wasm_bindgen::JsValue),
            >::new(
                move |_entries: wasm_bindgen::JsValue, _observer: wasm_bindgen::JsValue| {
                    canvas_size_tick.update(|n| *n += 1);
                },
            );
            if let Ok(obs) = web_sys::ResizeObserver::new(cb.as_ref().unchecked_ref()) {
                obs.observe(canvas_el);
                // Leak both so they live for the component lifetime
                std::mem::forget(obs);
            }
            cb.forget();
        });
    }

    // Redraw canvas when state changes
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            use wasm_bindgen::JsCast;

            // Track all reactive dependencies
            let _map_data = map.get();
            let _tokens_data = tokens.get();
            let _fog_data = fog.get();
            let _drag = dragging.get();
            let _sel = selected_ids.get();
            let _img_counter = image_load_counter.get();
            let _zoom = view_zoom.get();
            let _offset = view_offset.get();
            let _sel_rect = selection_rect.get();
            let _measure_s = measure_start.get();
            let _measure_e = measure_end.get();
            let _measure_c = measure_cursor.get();
            let _tool = active_tool.get();
            let _pings = ctx.pings.get();
            let _canvas_size = canvas_size_tick.get();

            let Some(canvas) = canvas_ref.get() else {
                return;
            };

            let canvas_el: &web_sys::HtmlCanvasElement = canvas.as_ref();

            // Size canvas to its CSS layout size
            let css_w = canvas_el.client_width() as u32;
            let css_h = canvas_el.client_height() as u32;
            if css_w == 0 || css_h == 0 {
                return;
            }
            if canvas_el.width() != css_w {
                canvas_el.set_width(css_w);
            }
            if canvas_el.height() != css_h {
                canvas_el.set_height(css_h);
            }

            let ctx2d = canvas_el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());

            let Some(ctx2d) = ctx2d else { return };

            let map_data = map.get();
            let tokens_data = tokens.get();
            let fog_data = fog.get();
            let sel_ids = selected_ids.get();
            let zoom = view_zoom.get();
            let offset = view_offset.get();

            // Clear canvas in screen space
            ctx2d.reset_transform().ok();
            ctx2d.set_fill_style_str("#1a1a2e");
            ctx2d.fill_rect(0.0, 0.0, css_w as f64, css_h as f64);

            if let Some(ref m) = map_data {
                let w = (m.width * m.cell_size) as f64;
                let h = (m.height * m.cell_size) as f64;
                let cell = m.cell_size as f64;

                // Apply viewport transform for world-space drawing
                ctx2d
                    .set_transform(zoom, 0.0, 0.0, zoom, -offset.0 * zoom, -offset.1 * zoom)
                    .ok();

                // Draw background image if set
                if let Some(ref bg_url) = m.background_url {
                    if !bg_url.is_empty() {
                        if let Some(bg_img) = get_or_load_image(bg_url) {
                            let _ = ctx2d.draw_image_with_html_image_element_and_dw_and_dh(
                                &bg_img, 0.0, 0.0, w, h,
                            );
                        }
                    }
                }

                // Draw grid
                ctx2d.set_stroke_style_str("#333355");
                ctx2d.set_line_width(0.5 / zoom);
                for x in 0..=m.width {
                    let px = (x * m.cell_size) as f64;
                    ctx2d.begin_path();
                    ctx2d.move_to(px, 0.0);
                    ctx2d.line_to(px, h);
                    ctx2d.stroke();
                }
                for y in 0..=m.height {
                    let py = (y * m.cell_size) as f64;
                    ctx2d.begin_path();
                    ctx2d.move_to(0.0, py);
                    ctx2d.line_to(w, py);
                    ctx2d.stroke();
                }

                // Draw tokens
                for t in &tokens_data {
                    if !t.visible {
                        continue;
                    }
                    let cx = (t.x as f64 + 0.5) * cell;
                    let cy = (t.y as f64 + 0.5) * cell;
                    let radius = cell * t.size as f64 * 0.4;

                    // Apply rotation around token center
                    ctx2d.save();
                    ctx2d.translate(cx, cy).ok();
                    ctx2d.rotate(t.rotation as f64).ok();
                    ctx2d.translate(-cx, -cy).ok();

                    // Draw token image or colored circle
                    if let Some(ref img_url) = t.image_url {
                        if let Some(img) = get_or_load_image(img_url) {
                            ctx2d.save();
                            ctx2d.begin_path();
                            let _ = ctx2d.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
                            ctx2d.clip();
                            let _ = ctx2d.draw_image_with_html_image_element_and_dw_and_dh(
                                &img,
                                cx - radius,
                                cy - radius,
                                radius * 2.0,
                                radius * 2.0,
                            );
                            ctx2d.restore();
                        } else {
                            ctx2d.begin_path();
                            let _ = ctx2d.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
                            ctx2d.set_fill_style_str(&t.color);
                            ctx2d.fill();
                        }
                    } else {
                        ctx2d.begin_path();
                        let _ = ctx2d.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
                        ctx2d.set_fill_style_str(&t.color);
                        ctx2d.fill();
                    }

                    ctx2d.restore(); // remove rotation for labels/UI elements

                    // Selection highlight
                    if sel_ids.contains(&t.id) {
                        ctx2d.begin_path();
                        let _ = ctx2d.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
                        ctx2d.set_stroke_style_str("#ffff00");
                        ctx2d.set_line_width(2.0 / zoom);
                        ctx2d.stroke();
                    }

                    // Condition icons (above the token)
                    if !t.conditions.is_empty() {
                        let icon_size = (10.0 / zoom).max(6.0);
                        ctx2d.set_font(&format!("{icon_size}px sans-serif"));
                        ctx2d.set_text_align("center");
                        ctx2d.set_text_baseline("bottom");
                        let total_w = t.conditions.len() as f64 * icon_size * 1.2;
                        let start_x = cx - total_w / 2.0 + icon_size * 0.6;
                        let icon_y = cy - radius - 2.0 / zoom;
                        for (i, cond) in t.conditions.iter().enumerate() {
                            let icon = condition_icon(cond);
                            let ix = start_x + i as f64 * icon_size * 1.2;
                            let _ = ctx2d.fill_text(icon, ix, icon_y);
                        }
                    }

                    // Label
                    ctx2d.set_fill_style_str("#ffffff");
                    let font_size = (11.0 / zoom).max(8.0);
                    ctx2d.set_font(&format!("{font_size}px sans-serif"));
                    ctx2d.set_text_align("center");
                    ctx2d.set_text_baseline("middle");
                    let _ = ctx2d.fill_text(&t.label, cx, cy);

                    // HP bar
                    if let (Some(cur), Some(max)) = (t.current_hp, t.max_hp) {
                        if max > 0 {
                            let bar_w = radius * 1.6;
                            let bar_h = 4.0 / zoom;
                            let bar_x = cx - bar_w / 2.0;
                            let bar_y = cy + radius + 3.0 / zoom;

                            ctx2d.set_fill_style_str("#333333");
                            ctx2d.fill_rect(bar_x, bar_y, bar_w, bar_h);

                            let ratio = (cur as f64 / max as f64).clamp(0.0, 1.0);
                            let color = if ratio > 0.5 {
                                "#22cc22"
                            } else if ratio > 0.25 {
                                "#cccc22"
                            } else {
                                "#cc2222"
                            };
                            ctx2d.set_fill_style_str(color);
                            ctx2d.fill_rect(bar_x, bar_y, bar_w * ratio, bar_h);
                        }
                    }
                }

                // Selection rectangle (world space)
                if let Some((x1, y1, x2, y2)) = selection_rect.get() {
                    let rx = x1.min(x2);
                    let ry = y1.min(y2);
                    let rw = (x2 - x1).abs();
                    let rh = (y2 - y1).abs();
                    ctx2d.set_stroke_style_str("#4488ff");
                    ctx2d.set_line_width(1.0 / zoom);
                    ctx2d
                        .set_line_dash(&js_sys::Array::of2(
                            &(4.0 / zoom).into(),
                            &(4.0 / zoom).into(),
                        ))
                        .ok();
                    ctx2d.stroke_rect(rx, ry, rw, rh);
                    ctx2d.set_fill_style_str("rgba(68, 136, 255, 0.1)");
                    ctx2d.fill_rect(rx, ry, rw, rh);
                    ctx2d.set_line_dash(&js_sys::Array::new()).ok();
                }

                // Fog of war
                ctx2d.set_fill_style_str("rgba(0, 0, 0, 0.85)");
                let revealed = &fog_data;
                for gx in 0..m.width {
                    for gy in 0..m.height {
                        if !revealed.contains(&(gx, gy)) {
                            ctx2d.fill_rect(
                                (gx * m.cell_size) as f64,
                                (gy * m.cell_size) as f64,
                                cell,
                                cell,
                            );
                        }
                    }
                }

                // Draw pings (in world space, so they move with the map)
                let now = web_sys::js_sys::Date::now();
                let pings = ctx.pings.get();
                for (px, py, color, timestamp) in &pings {
                    let age = now - timestamp;
                    if age > 3000.0 {
                        continue;
                    }
                    let alpha = 1.0 - (age / 3000.0);
                    // Pulsing ring
                    let pulse = 1.0 + (age / 300.0).sin().abs() * 0.3;
                    let radius = cell * 0.6 * pulse;
                    ctx2d.save();
                    ctx2d.set_global_alpha(alpha);
                    ctx2d.begin_path();
                    let _ = ctx2d.arc(*px, *py, radius, 0.0, std::f64::consts::TAU);
                    ctx2d.set_stroke_style_str(color);
                    ctx2d.set_line_width(3.0 / zoom);
                    ctx2d.stroke();
                    // Inner glow
                    ctx2d.begin_path();
                    let _ = ctx2d.arc(*px, *py, radius * 0.5, 0.0, std::f64::consts::TAU);
                    ctx2d.set_stroke_style_str(color);
                    ctx2d.set_line_width(1.5 / zoom);
                    ctx2d.stroke();
                    ctx2d.restore();
                }

                // --- Overlays in screen space ---
                ctx2d.reset_transform().ok();

                // Measurement line
                let m_start = measure_start.get();
                let m_end = measure_end.get();
                let m_cursor = measure_cursor.get();

                if let Some((sx, sy)) = m_start {
                    let end_world = m_end.or(m_cursor);
                    if let Some((ex, ey)) = end_world {
                        let (s_sx, s_sy) = world_to_screen(sx, sy, offset, zoom);
                        let (s_ex, s_ey) = world_to_screen(ex, ey, offset, zoom);

                        ctx2d.set_stroke_style_str("#ff8800");
                        ctx2d.set_line_width(2.0);
                        ctx2d
                            .set_line_dash(&js_sys::Array::of2(&6.0.into(), &4.0.into()))
                            .ok();
                        ctx2d.begin_path();
                        ctx2d.move_to(s_sx, s_sy);
                        ctx2d.line_to(s_ex, s_ey);
                        ctx2d.stroke();
                        ctx2d.set_line_dash(&js_sys::Array::new()).ok();

                        // Distance label at midpoint
                        let dx = ex - sx;
                        let dy = ey - sy;
                        let dist_px = (dx * dx + dy * dy).sqrt();
                        let dist_sq = dist_px / cell;
                        let dist_ft = dist_sq * 5.0;

                        let mid_sx = (s_sx + s_ex) / 2.0;
                        let mid_sy = (s_sy + s_ey) / 2.0;

                        let label = format!("{:.1} sq ({:.0} ft)", dist_sq, dist_ft);
                        ctx2d.set_font("13px sans-serif");
                        ctx2d.set_text_align("center");
                        ctx2d.set_text_baseline("bottom");

                        // Background for readability
                        let tw = label.len() as f64 * 7.5;
                        ctx2d.set_fill_style_str("rgba(0, 0, 0, 0.7)");
                        ctx2d.fill_rect(mid_sx - tw / 2.0 - 4.0, mid_sy - 20.0, tw + 8.0, 20.0);
                        ctx2d.set_fill_style_str("#ff8800");
                        let _ = ctx2d.fill_text(&label, mid_sx, mid_sy - 4.0);
                    }
                }
            } else {
                // No map loaded
                canvas_el.set_width(css_w);
                canvas_el.set_height(css_h);
                ctx2d.set_fill_style_str("#1a1a2e");
                ctx2d.fill_rect(0.0, 0.0, css_w as f64, css_h as f64);
                ctx2d.set_fill_style_str("#666688");
                ctx2d.set_font("16px sans-serif");
                ctx2d.set_text_align("center");
                let _ = ctx2d.fill_text("No map loaded", css_w as f64 / 2.0, css_h as f64 / 2.0);
            }
        });
    }

    // --- Mouse event handlers ---

    let on_mousedown = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            move |ev: leptos::ev::MouseEvent| {
                let map_data = map.get();
                let Some(m) = map_data else { return };
                let Some((sx, sy)) = canvas_coords(&canvas_ref, &ev) else {
                    return;
                };

                let zoom = view_zoom.get();
                let offset = view_offset.get();
                let (wx, wy) = screen_to_world(sx, sy, offset, zoom);

                let tool = active_tool.get();

                // Pan: middle-click (any tool), space+click, or left-click in Pan mode
                let pan_button = if tool == MapTool::Pan { 0 } else { 1 };
                let select_button = if tool == MapTool::Pan { 1 } else { 0 };
                if ev.button() == pan_button || space_held.get() {
                    ev.prevent_default();
                    panning.set(true);
                    pan_start_screen.set((sx, sy));
                    pan_start_offset.set(offset);
                    return;
                }

                // Right-click — ignore (reserved for context menu / future rotation)
                if ev.button() == 2 {
                    return;
                }

                // In Pan mode, middle-click acts as select; in other modes, only left-click
                if ev.button() != select_button {
                    return;
                }

                match tool {
                    MapTool::Select | MapTool::Pan => {
                        let cell = m.cell_size as f64;
                        let tokens_data = tokens.get();

                        let clicked = tokens_data.iter().rev().find(|t| {
                            if !t.visible {
                                return false;
                            }
                            let cx = (t.x as f64 + 0.5) * cell;
                            let cy = (t.y as f64 + 0.5) * cell;
                            let radius = cell * t.size as f64 * 0.5;
                            let dx = wx - cx;
                            let dy = wy - cy;
                            (dx * dx + dy * dy).sqrt() <= radius
                        });

                        if let Some(t) = clicked {
                            let shift = ev.shift_key();
                            if shift {
                                // Toggle selection
                                selected_ids.update(|ids| {
                                    if !ids.remove(&t.id) {
                                        ids.insert(t.id);
                                    }
                                });
                            } else {
                                let already_selected = selected_ids.get().contains(&t.id);
                                if !already_selected {
                                    let mut ids = std::collections::HashSet::new();
                                    ids.insert(t.id);
                                    selected_ids.set(ids);
                                }
                                // Start dragging selected tokens
                                set_dragging.set(true);
                                drag_token_start.set(Some((wx, wy)));
                                // Capture initial positions for all selected tokens
                                let sel = selected_ids.get();
                                let origins: Vec<(i32, f32, f32)> = tokens
                                    .get()
                                    .iter()
                                    .filter(|t| sel.contains(&t.id))
                                    .map(|t| (t.id, t.x, t.y))
                                    .collect();
                                drag_token_origins.set(origins);
                            }
                        } else {
                            // Click on empty space — start selection rectangle
                            if !ev.shift_key() {
                                selected_ids.set(std::collections::HashSet::new());
                            }
                            set_dragging.set(true);
                            drag_start_world.set(Some((wx, wy)));
                        }
                    }
                    MapTool::Measure => {
                        let cell = m.cell_size as f64;
                        let snap = snap_to_grid.get();
                        let mw = if snap {
                            ((wx / cell).floor() + 0.5) * cell
                        } else {
                            wx
                        };
                        let mh = if snap {
                            ((wy / cell).floor() + 0.5) * cell
                        } else {
                            wy
                        };

                        if measure_start.get().is_some() && measure_end.get().is_none() {
                            // Set end point
                            measure_end.set(Some((mw, mh)));
                        } else {
                            // Start new measurement
                            measure_start.set(Some((mw, mh)));
                            measure_end.set(None);
                            measure_cursor.set(None);
                        }
                    }
                    MapTool::Ping => {
                        // Send ping at world position
                        send.with_value(|f| {
                            if let Some(f) = f {
                                f(ClientMessage::Ping { x: wx, y: wy });
                            }
                        });
                    }
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_mousemove = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            move |ev: leptos::ev::MouseEvent| {
                let Some((sx, sy)) = canvas_coords(&canvas_ref, &ev) else {
                    return;
                };

                let zoom = view_zoom.get();
                let offset = view_offset.get();

                // Panning
                if panning.get() {
                    let (psx, psy) = pan_start_screen.get();
                    let (pox, poy) = pan_start_offset.get();
                    let dx = (sx - psx) / zoom;
                    let dy = (sy - psy) / zoom;
                    view_offset.set((pox - dx, poy - dy));
                    return;
                }

                let (wx, wy) = screen_to_world(sx, sy, offset, zoom);

                let tool = active_tool.get();

                match tool {
                    MapTool::Select | MapTool::Pan => {
                        if !dragging.get() {
                            return;
                        }

                        // If we started on a token, move all selected tokens
                        if let Some((start_wx, start_wy)) = drag_token_start.get() {
                            let map_data = map.get();
                            let Some(m) = map_data else { return };
                            let cell = m.cell_size as f64;
                            let snap = snap_to_grid.get();

                            // Total delta from the fixed drag start position
                            let dx = wx - start_wx;
                            let dy = wy - start_wy;

                            let origins = drag_token_origins.get();
                            tokens.update(|ts| {
                                for t in ts.iter_mut() {
                                    if let Some(&(_, orig_x, orig_y)) =
                                        origins.iter().find(|(id, _, _)| *id == t.id)
                                    {
                                        let new_grid_x = orig_x as f64 + dx / cell;
                                        let new_grid_y = orig_y as f64 + dy / cell;
                                        if snap {
                                            t.x = new_grid_x.floor() as f32;
                                            t.y = new_grid_y.floor() as f32;
                                        } else {
                                            t.x = new_grid_x as f32;
                                            t.y = new_grid_y as f32;
                                        }
                                    }
                                }
                            });
                        }
                        // If drag started on empty space, update selection rect
                        else if let Some((start_wx, start_wy)) = drag_start_world.get() {
                            selection_rect.set(Some((start_wx, start_wy, wx, wy)));
                        }
                    }
                    MapTool::Measure => {
                        // Update cursor position for rubber-band line
                        if measure_start.get().is_some() && measure_end.get().is_none() {
                            let map_data = map.get();
                            if let Some(ref m) = map_data {
                                let cell = m.cell_size as f64;
                                let snap = snap_to_grid.get();
                                let mw = if snap {
                                    ((wx / cell).floor() + 0.5) * cell
                                } else {
                                    wx
                                };
                                let mh = if snap {
                                    ((wy / cell).floor() + 0.5) * cell
                                } else {
                                    wy
                                };
                                measure_cursor.set(Some((mw, mh)));
                            }
                        }
                    }
                    MapTool::Ping => {}
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_mouseup = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            move |ev: leptos::ev::MouseEvent| {
                // End panning
                if panning.get() {
                    panning.set(false);
                    return;
                }

                let tool = active_tool.get();
                let zoom = view_zoom.get();
                let offset = view_offset.get();

                if (tool == MapTool::Select || tool == MapTool::Pan) && dragging.get() {
                    // Finish selection rectangle
                    if let Some((x1, y1, x2, y2)) = selection_rect.get() {
                        let rx = x1.min(x2);
                        let ry = y1.min(y2);
                        let rw = (x2 - x1).abs();
                        let rh = (y2 - y1).abs();

                        if rw > 2.0 || rh > 2.0 {
                            let map_data = map.get();
                            if let Some(ref m) = map_data {
                                let cell = m.cell_size as f64;
                                let tokens_data = tokens.get();
                                let shift = ev.shift_key();

                                if !shift {
                                    selected_ids.set(std::collections::HashSet::new());
                                }

                                selected_ids.update(|ids| {
                                    for t in &tokens_data {
                                        if !t.visible {
                                            continue;
                                        }
                                        let cx = (t.x as f64 + 0.5) * cell;
                                        let cy = (t.y as f64 + 0.5) * cell;
                                        if cx >= rx && cx <= rx + rw && cy >= ry && cy <= ry + rh {
                                            ids.insert(t.id);
                                        }
                                    }
                                });
                            }
                        }
                        selection_rect.set(None);
                    }

                    // Finish token drag — send batch MoveTokens
                    if drag_token_start.get().is_some() {
                        let sel = selected_ids.get();
                        let tokens_data = tokens.get();
                        let moves: Vec<(i32, f32, f32)> = tokens_data
                            .iter()
                            .filter(|t| sel.contains(&t.id))
                            .map(|t| (t.id, t.x, t.y))
                            .collect();
                        if !moves.is_empty() {
                            send.with_value(|f| {
                                if let Some(f) = f {
                                    f(ClientMessage::MoveTokens { moves });
                                }
                            });
                        }
                        drag_token_start.set(None);
                        drag_token_origins.set(Vec::new());
                    }

                    drag_start_world.set(None);
                    set_dragging.set(false);
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_wheel = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            move |ev: leptos::ev::WheelEvent| {
                ev.prevent_default();
                let Some((sx, sy)) = ({
                    let canvas = canvas_ref.get();
                    canvas.map(|c| {
                        let el: &web_sys::HtmlCanvasElement = c.as_ref();
                        let rect = el.get_bounding_client_rect();
                        (
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        )
                    })
                }) else {
                    return;
                };

                let old_zoom = view_zoom.get();
                let offset = view_offset.get();

                // Zoom toward cursor position
                let factor = if ev.delta_y() < 0.0 { 1.1 } else { 1.0 / 1.1 };
                let new_zoom = (old_zoom * factor).clamp(0.25, 4.0);

                // Adjust offset so the point under cursor stays fixed
                let wx = sx / old_zoom + offset.0;
                let wy = sy / old_zoom + offset.1;
                let new_ox = wx - sx / new_zoom;
                let new_oy = wy - sy / new_zoom;

                view_zoom.set(new_zoom);
                view_offset.set((new_ox, new_oy));
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::WheelEvent| {}
        }
    };

    let on_keydown = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::KeyboardEvent| {
                let key = ev.key();
                match key.as_str() {
                    " " => {
                        ev.prevent_default();
                        space_held.set(true);
                    }
                    "v" | "V" => active_tool.set(MapTool::Select),
                    "h" | "H" => active_tool.set(MapTool::Pan),
                    "m" | "M" => {
                        active_tool.set(MapTool::Measure);
                        measure_start.set(None);
                        measure_end.set(None);
                        measure_cursor.set(None);
                    }
                    "g" | "G" => snap_to_grid.update(|v| *v = !*v),
                    "p" | "P" => active_tool.set(MapTool::Ping),
                    "t" | "T" => show_token_list.update(|v| *v = !*v),
                    "Escape" => {
                        measure_start.set(None);
                        measure_end.set(None);
                        measure_cursor.set(None);
                        if active_tool.get() == MapTool::Measure {
                            active_tool.set(MapTool::Select);
                        }
                        selected_ids.set(std::collections::HashSet::new());
                    }
                    _ => {}
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::KeyboardEvent| {}
        }
    };

    let on_keyup = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::KeyboardEvent| {
                if ev.key() == " " {
                    space_held.set(false);
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::KeyboardEvent| {}
        }
    };

    // Right-click: rotate selected tokens
    let on_contextmenu = {
        #[cfg(feature = "hydrate")]
        {
            move |ev: leptos::ev::MouseEvent| {
                ev.prevent_default();

                let sel = selected_ids.get();
                if sel.is_empty() {
                    return;
                }

                // 15 degrees per click; shift = counterclockwise
                let angle = if ev.shift_key() {
                    -std::f64::consts::PI / 12.0
                } else {
                    std::f64::consts::PI / 12.0
                };

                let mut rotations = Vec::new();
                tokens.update(|ts| {
                    for t in ts.iter_mut() {
                        if sel.contains(&t.id) {
                            t.rotation += angle as f32;
                            rotations.push((t.id, t.rotation));
                        }
                    }
                });

                if !rotations.is_empty() {
                    send.with_value(|f| {
                        if let Some(f) = f {
                            f(ClientMessage::RotateTokens { rotations });
                        }
                    });
                }
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_bg_select = Callback::new(move |media: crate::models::MediaInfo| {
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::SetMapBackground {
                    background_url: Some(media.url.clone()),
                });
            }
        });
        show_media_browser.set(false);
    });

    // Center on token helper
    let center_on_token = move |token_id: i32| {
        let tokens_data = tokens.get();
        let map_data = map.get();
        if let (Some(t), Some(m)) = (tokens_data.iter().find(|t| t.id == token_id), &map_data) {
            let cell = m.cell_size as f64;
            let world_x = (t.x as f64 + 0.5) * cell;
            let world_y = (t.y as f64 + 0.5) * cell;
            #[cfg(feature = "hydrate")]
            if let Some(canvas) = canvas_ref.get() {
                let canvas_el: &web_sys::HtmlCanvasElement = canvas.as_ref();
                let cw = canvas_el.client_width() as f64;
                let ch = canvas_el.client_height() as f64;
                let zoom = view_zoom.get();
                view_offset.set((world_x - cw / zoom / 2.0, world_y - ch / zoom / 2.0));
            }
        }
        show_token_list.set(false);
    };

    // --- Create Map form state ---
    let show_create_map = RwSignal::new(false);
    let (new_map_name, set_new_map_name) = signal("Map".to_string());
    let (new_map_width, set_new_map_width) = signal(20i32);
    let (new_map_height, set_new_map_height) = signal(15i32);
    let (new_map_cell_size, set_new_map_cell_size) = signal(40i32);
    let (new_map_bg_url, set_new_map_bg_url) = signal(Option::<String>::None);
    let show_map_image_picker = RwSignal::new(false);
    let session_id = ctx.session_id;

    // Map list for switcher
    let map_list = RwSignal::new(Vec::<crate::models::MapInfo>::new());
    let show_map_list = RwSignal::new(false);

    // Fetch map list when dropdown is opened
    #[cfg(feature = "hydrate")]
    let fetch_map_list = move || {
        let sid = session_id.get();
        if sid == 0 {
            return;
        }
        leptos::task::spawn_local(async move {
            match crate::server::api::list_maps(sid).await {
                Ok(maps) => map_list.set(maps),
                Err(e) => log::error!("Failed to list maps: {e}"),
            }
        });
    };

    let on_map_image_select = {
        Callback::new(move |media: crate::models::MediaInfo| {
            let url = media.url.clone();
            set_new_map_bg_url.set(Some(url.clone()));
            show_map_image_picker.set(false);

            // Load image to get dimensions and estimate grid size
            #[cfg(feature = "hydrate")]
            {
                use wasm_bindgen::JsCast;
                let img = web_sys::HtmlImageElement::new().unwrap();
                let img_clone = img.clone();
                let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                    let w = img_clone.natural_width();
                    let h = img_clone.natural_height();
                    if w > 0 && h > 0 {
                        // Estimate: assume ~72 DPI, 1 inch per grid square
                        // so cell_size ~72px. Clamp grid dimensions to reasonable range.
                        let dpi_estimate = 72;
                        let grid_w = (w as i32 / dpi_estimate).max(5).min(200);
                        let grid_h = (h as i32 / dpi_estimate).max(5).min(200);
                        let cell = dpi_estimate.max(10).min(200);
                        set_new_map_width.set(grid_w);
                        set_new_map_height.set(grid_h);
                        set_new_map_cell_size.set(cell);
                    }
                });
                img.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                img.set_src(&url);
            }
        })
    };

    let do_create_map = move |_| {
        let name = new_map_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        let w = new_map_width.get();
        let h = new_map_height.get();
        let cell = new_map_cell_size.get();
        let bg = new_map_bg_url.get();
        let sid = session_id.get();
        let map_signal = map;
        show_create_map.set(false);
        leptos::task::spawn_local(async move {
            match crate::server::api::create_map(sid, name, w, h, Some(cell), bg).await {
                Ok(new_map) => {
                    let mid = new_map.id;
                    map_signal.set(Some(new_map));
                    // Tell server to switch to the new map
                    send.with_value(|f| {
                        if let Some(f) = f {
                            f(ClientMessage::SetMap { map_id: mid });
                        }
                    });
                }
                Err(e) => log::error!("Failed to create map: {e}"),
            }
        });
        // Reset form
        set_new_map_name.set("Map".to_string());
        set_new_map_width.set(20);
        set_new_map_height.set(15);
        set_new_map_cell_size.set(40);
        set_new_map_bg_url.set(None);
    };

    let cancel_create_map = move |_| {
        show_create_map.set(false);
    };

    let do_delete_map = move |_| {
        let Some(m) = map.get() else { return };
        let mid = m.id;
        let sid = session_id.get();
        let map_signal = map;
        leptos::task::spawn_local(async move {
            match crate::server::api::delete_map(mid).await {
                Ok(()) => {
                    // Try to load another map
                    match crate::server::api::list_maps(sid).await {
                        Ok(maps) => {
                            if let Some(next) = maps.first() {
                                map_signal.set(Some(next.clone()));
                                send.with_value(|f| {
                                    if let Some(f) = f {
                                        f(ClientMessage::SetMap { map_id: next.id });
                                    }
                                });
                            } else {
                                map_signal.set(None);
                                tokens.set(vec![]);
                                fog.set(vec![]);
                            }
                        }
                        Err(_) => {
                            map_signal.set(None);
                            tokens.set(vec![]);
                            fog.set(vec![]);
                        }
                    }
                }
                Err(e) => log::error!("Failed to delete map: {e}"),
            }
        });
    };

    view! {
        <div
            class="map-container"
            node_ref=container_ref
            tabindex="0"
            on:keydown=on_keydown
            on:keyup=on_keyup
        >
            // Auto-show create form when no map exists
            {move || {
                if map.get().is_none() && ctx.is_gm.get() && !show_create_map.get() {
                    show_create_map.set(true);
                }
            }}

            // Create Map dialog
            <Show when=move || show_create_map.get() && ctx.is_gm.get()>
                <div class="create-map-form">
                    <h4>{move || if map.get().is_none() { "Create Map" } else { "New Map" }}</h4>
                    <div class="field-row">
                        <label>"Name"</label>
                        <input
                            type="text"
                            prop:value=new_map_name
                            on:input=move |ev| set_new_map_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field-row">
                        <label>"Background"</label>
                        <div class="create-map-bg-row">
                            <button
                                class="btn-small"
                                on:click=move |_| show_map_image_picker.set(true)
                            >{move || if new_map_bg_url.get().is_some() { "Change Image" } else { "Select Image" }}</button>
                            {move || new_map_bg_url.get().map(|url| view! {
                                <img src=url class="create-map-preview" />
                            })}
                        </div>
                    </div>
                    <div class="field-row">
                        <label>"Grid Width"</label>
                        <input
                            type="number"
                            prop:value=move || new_map_width.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                    set_new_map_width.set(v.max(1).min(200));
                                }
                            }
                            min="1" max="200"
                        />
                    </div>
                    <div class="field-row">
                        <label>"Grid Height"</label>
                        <input
                            type="number"
                            prop:value=move || new_map_height.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                    set_new_map_height.set(v.max(1).min(200));
                                }
                            }
                            min="1" max="200"
                        />
                    </div>
                    <div class="field-row">
                        <label>"Cell Size (px)"</label>
                        <input
                            type="number"
                            prop:value=move || new_map_cell_size.get().to_string()
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                    set_new_map_cell_size.set(v.max(10).min(200));
                                }
                            }
                            min="10" max="200"
                        />
                    </div>
                    <div class="form-actions">
                        <button on:click=do_create_map>"Create"</button>
                        <Show when=move || map.get().is_some()>
                            <button class="btn-cancel" on:click=cancel_create_map>"Cancel"</button>
                        </Show>
                    </div>
                </div>
                <crate::components::media_browser::MediaBrowser
                    on_select=on_map_image_select
                    filter_type="image".to_string()
                    show=show_map_image_picker
                />
            </Show>

            // Tool palette (HTML overlay)
            <div class="map-tool-palette">
                <button
                    class=move || if active_tool.get() == MapTool::Select { "map-tool-btn active" } else { "map-tool-btn" }
                    on:click=move |_| active_tool.set(MapTool::Select)
                    data-tooltip="Select (V)"
                >
                    // pointer arrow icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                        <path d="M7 2l10 10-4.5 1 2.5 6-2 1-2.5-6L7 17z"/>
                    </svg>
                </button>
                <button
                    class=move || if active_tool.get() == MapTool::Pan { "map-tool-btn active" } else { "map-tool-btn" }
                    on:click=move |_| active_tool.set(MapTool::Pan)
                    data-tooltip="Pan (H)"
                >
                    // hand/grab icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 11V6a2 2 0 0 0-4 0v1"/>
                        <path d="M14 10V4a2 2 0 0 0-4 0v6"/>
                        <path d="M10 10.5V6a2 2 0 0 0-4 0v8"/>
                        <path d="M18 8a2 2 0 0 1 4 0v6a8 8 0 0 1-8 8H12a8 8 0 0 1-6-2.7"/>
                    </svg>
                </button>
                <button
                    class=move || if active_tool.get() == MapTool::Measure { "map-tool-btn active" } else { "map-tool-btn" }
                    on:click=move |_| {
                        active_tool.set(MapTool::Measure);
                        measure_start.set(None);
                        measure_end.set(None);
                        measure_cursor.set(None);
                    }
                    data-tooltip="Measure (M)"
                >
                    // ruler icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                        <line x1="3" y1="21" x2="21" y2="3"/>
                        <line x1="7" y1="21" x2="7" y2="17"/>
                        <line x1="11" y1="21" x2="11" y2="15"/>
                        <line x1="15" y1="21" x2="15" y2="17"/>
                    </svg>
                </button>
                <button
                    class=move || if active_tool.get() == MapTool::Ping { "map-tool-btn active" } else { "map-tool-btn" }
                    on:click=move |_| active_tool.set(MapTool::Ping)
                    data-tooltip="Ping (P)"
                >
                    // ping/target icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="9"/>
                        <circle cx="12" cy="12" r="5"/>
                        <circle cx="12" cy="12" r="1" fill="currentColor"/>
                    </svg>
                </button>
                <button
                    class=move || if snap_to_grid.get() { "map-tool-btn active" } else { "map-tool-btn" }
                    on:click=move |_| snap_to_grid.update(|v| *v = !*v)
                    data-tooltip="Grid Snap (G)"
                >
                    // grid icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                        <rect x="3" y="3" width="18" height="18"/>
                        <line x1="3" y1="12" x2="21" y2="12"/>
                        <line x1="12" y1="3" x2="12" y2="21"/>
                    </svg>
                </button>
                <div class="map-tool-separator"/>
                <button
                    class="map-tool-btn"
                    on:click=move |_| show_token_list.update(|v| *v = !*v)
                    data-tooltip="Token List (T)"
                >
                    // list icon
                    <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                        <rect x="3" y="4" width="3" height="3" rx="0.5"/>
                        <rect x="9" y="4" width="12" height="3" rx="0.5"/>
                        <rect x="3" y="10.5" width="3" height="3" rx="0.5"/>
                        <rect x="9" y="10.5" width="12" height="3" rx="0.5"/>
                        <rect x="3" y="17" width="3" height="3" rx="0.5"/>
                        <rect x="9" y="17" width="12" height="3" rx="0.5"/>
                    </svg>
                </button>

                // Token list dropdown
                {move || {
                    if !show_token_list.get() {
                        return None;
                    }
                    let tokens_data = tokens.get();
                    let visible: Vec<_> = tokens_data.iter().filter(|t| t.visible).cloned().collect();
                    Some(view! {
                        <div class="map-token-list">
                            {visible.into_iter().map(|t| {
                                let tid = t.id;
                                let color = t.color.clone();
                                let label = t.label.clone();
                                view! {
                                    <div
                                        class="map-token-list-item"
                                        on:click=move |_| center_on_token(tid)
                                    >
                                        <span class="map-token-dot" style=format!("background:{color}")></span>
                                        {label}
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    })
                }}

                <div class="map-tool-separator"/>

                // Ping color picker
                <input
                    type="color"
                    class="map-ping-color"
                    data-tooltip="Ping Color"
                    prop:value=move || ctx.ping_color.get()
                    on:change=move |ev| {
                        let color: String = leptos::prelude::event_target_value(&ev);
                        ctx.ping_color.set(color.clone());
                        send.with_value(|f| {
                            if let Some(f) = f {
                                f(ClientMessage::SetPingColor { color });
                            }
                        });
                    }
                />

                // GM: Sync viewport to all players
                <Show when=move || ctx.is_gm.get()>
                    <button
                        class="map-tool-btn"
                        on:click=move |_| {
                            let offset = view_offset.get();
                            let zoom = view_zoom.get();
                            send.with_value(|f| {
                                if let Some(f) = f {
                                    f(ClientMessage::SyncViewport {
                                        x: offset.0,
                                        y: offset.1,
                                        zoom,
                                    });
                                }
                            });
                        }
                        data-tooltip="Sync Viewport to Players"
                    >
                        // broadcast/eye icon
                        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                            <circle cx="12" cy="12" r="3"/>
                        </svg>
                    </button>
                </Show>
            </div>

            <Show when=move || ctx.is_gm.get()>
                <div class="map-mgmt-bar">
                    // Map switcher dropdown
                    <Show when=move || map.get().is_some()>
                        <div class="map-switcher">
                            <button
                                class="map-mgmt-btn"
                                on:click=move |_| {
                                    let opening = !show_map_list.get();
                                    show_map_list.set(opening);
                                    if opening {
                                        #[cfg(feature = "hydrate")]
                                        fetch_map_list();
                                    }
                                }
                                data-tooltip="Switch Map"
                            >
                                {move || map.get().map(|m| m.name.clone()).unwrap_or_default()}
                                " \u{25BC}"
                            </button>
                            <Show when=move || show_map_list.get()>
                                <div class="map-list-dropdown">
                                    {move || {
                                        map_list.get().into_iter().map(|m| {
                                            let mid = m.id;
                                            let name = m.name.clone();
                                            let is_active = map.get().map(|cm| cm.id == mid).unwrap_or(false);
                                            view! {
                                                <div
                                                    class="map-list-item"
                                                    class:active=is_active
                                                    on:click=move |_| {
                                                        show_map_list.set(false);
                                                        send.with_value(|f| {
                                                            if let Some(f) = f {
                                                                f(ClientMessage::SetMap { map_id: mid });
                                                            }
                                                        });
                                                    }
                                                >{name}</div>
                                            }
                                        }).collect::<Vec<_>>()
                                    }}
                                </div>
                            </Show>
                        </div>
                    </Show>
                    <button
                        class="map-mgmt-btn"
                        data-tooltip="New Map"
                        on:click=move |_| show_create_map.set(true)
                    >"+"</button>
                    <Show when=move || map.get().is_some()>
                        <button
                            class="map-mgmt-btn"
                            on:click=move |_| show_media_browser.set(true)
                            data-tooltip="Set Background"
                        >"\u{1f5bc}"</button>
                        <button
                            class="map-mgmt-btn map-mgmt-btn-danger"
                            data-tooltip="Delete Map"
                            on:click=do_delete_map
                        >"\u{1f5d1}"</button>
                    </Show>
                </div>
            </Show>

            <canvas
                node_ref=canvas_ref
                on:mousedown=on_mousedown
                on:mousemove=on_mousemove
                on:mouseup=on_mouseup
                on:wheel=on_wheel
                on:contextmenu=on_contextmenu
                style=move || {
                    let cursor = if panning.get() {
                        "grabbing"
                    } else if active_tool.get() == MapTool::Pan {
                        "grab"
                    } else {
                        "default"
                    };
                    format!("cursor: {cursor}; display: block; width: 100%; height: 100%;")
                }
            />
            <TokenHpPopup selected_ids=selected_ids tokens=tokens />
            <crate::components::media_browser::MediaBrowser
                on_select=on_bg_select
                filter_type="image".to_string()
                show=show_media_browser
            />
        </div>
    }
}

const ALL_CONDITIONS: &[(&str, &str)] = &[
    ("bloodied", "\u{1FA78}"),
    ("poisoned", "\u{2620}\u{FE0F}"),
    ("prone", "\u{2B07}\u{FE0F}"),
    ("stunned", "\u{1F4AB}"),
    ("blinded", "\u{1F648}"),
    ("frightened", "\u{1F628}"),
    ("paralyzed", "\u{26A1}"),
    ("restrained", "\u{26D3}\u{FE0F}"),
    ("invisible", "\u{1F47B}"),
    ("concentrating", "\u{1F52E}"),
];

#[component]
fn TokenHpPopup(
    selected_ids: RwSignal<std::collections::HashSet<i32>>,
    tokens: RwSignal<Vec<TokenInfo>>,
) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let send = ctx.send;

    let selected_token = move || {
        let ids = selected_ids.get();
        if ids.len() != 1 {
            return None;
        }
        let sel_id = *ids.iter().next()?;
        tokens.get().into_iter().find(|t| t.id == sel_id)
    };

    let do_hp_change = move |token_id: i32, change: i32| {
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::UpdateTokenHp {
                    token_id,
                    hp_change: change,
                });
            }
        });
    };

    let toggle_condition = move |token_id: i32, condition: &str| {
        let condition = condition.to_string();
        let mut new_conditions = tokens
            .get()
            .iter()
            .find(|t| t.id == token_id)
            .map(|t| t.conditions.clone())
            .unwrap_or_default();
        if let Some(pos) = new_conditions.iter().position(|c| c == &condition) {
            new_conditions.remove(pos);
        } else {
            new_conditions.push(condition);
        }
        send.with_value(|f| {
            if let Some(f) = f {
                f(ClientMessage::UpdateTokenConditions {
                    token_id,
                    conditions: new_conditions,
                });
            }
        });
    };

    view! {
        {move || {
            let t = selected_token()?;
            let tid = t.id;
            let has_hp = t.current_hp.is_some() && t.max_hp.is_some();
            let hp = t.current_hp.unwrap_or(0);
            let max = t.max_hp.unwrap_or(0);
            let current_conditions = t.conditions.clone();

            Some(view! {
                <div class="token-popup">
                    <div class="token-popup-header">
                        <strong>{t.label.clone()}</strong>
                    </div>
                    {if has_hp {
                        Some(view! {
                            <div class="token-popup-hp">
                                <span>"HP: " {hp} "/" {max}</span>
                                <button on:click=move |_| do_hp_change(tid, -1)>"-1"</button>
                                <button on:click=move |_| do_hp_change(tid, 1)>"+1"</button>
                                <button on:click=move |_| do_hp_change(tid, -5)>"-5"</button>
                                <button on:click=move |_| do_hp_change(tid, 5)>"+5"</button>
                            </div>
                        })
                    } else {
                        None
                    }}
                    <div class="token-popup-conditions">
                        {ALL_CONDITIONS.iter().map(|&(name, icon)| {
                            let is_active = current_conditions.contains(&name.to_string());
                            let cond_name = name.to_string();
                            view! {
                                <button
                                    class=if is_active { "condition-btn active" } else { "condition-btn" }
                                    title=cond_name.clone()
                                    on:click=move |_| toggle_condition(tid, name)
                                >
                                    {icon}
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            })
        }}
    }
}
