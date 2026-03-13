use leptos::prelude::*;

use crate::models::TokenInfo;
use crate::pages::game::GameContext;
use crate::ws::messages::ClientMessage;

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
    let session_id = ctx.session_id;

    let (dragging, set_dragging) = signal(Option::<i32>::None);
    let (selected, set_selected) = signal(Option::<i32>::None);
    let show_media_browser = RwSignal::new(false);

    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // Signal to trigger redraws when images load
    #[cfg(feature = "hydrate")]
    let (image_load_counter, set_image_load_counter) = signal(0u32);

    // Image cache stored as a signal so we can update it reactively
    #[cfg(feature = "hydrate")]
    let image_cache = StoredValue::new_local(
        std::collections::HashMap::<String, web_sys::HtmlImageElement>::new(),
    );

    // Helper to get or load an image
    #[cfg(feature = "hydrate")]
    let get_or_load_image = move |url: &str| -> Option<web_sys::HtmlImageElement> {
        use wasm_bindgen::JsCast;

        let url_owned = url.to_string();

        let existing = image_cache.with_value(|cache| cache.get(&url_owned).cloned());

        if let Some(img) = existing {
            if img.complete() && img.natural_width() > 0 {
                return Some(img);
            }
            // Still loading
            return None;
        }

        // Create new image element and start loading
        let img = web_sys::HtmlImageElement::new().ok()?;
        let img_clone = img.clone();
        let url_for_cache = url_owned.clone();

        let onload = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            set_image_load_counter.update(|c| *c += 1);
        });
        img.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();

        img.set_src(&url_owned);

        image_cache.update_value(|cache| {
            cache.insert(url_for_cache, img_clone);
        });

        None
    };

    // Redraw canvas when state changes
    #[cfg(feature = "hydrate")]
    {
        Effect::new(move |_| {
            use wasm_bindgen::JsCast;

            let _map_data = map.get();
            let _tokens_data = tokens.get();
            let _fog_data = fog.get();
            let _drag = dragging.get();
            let _sel = selected.get();
            let _img_counter = image_load_counter.get();

            let Some(canvas) = canvas_ref.get() else {
                return;
            };

            let canvas_el: &web_sys::HtmlCanvasElement = canvas.as_ref();

            let ctx2d = canvas_el
                .get_context("2d")
                .ok()
                .flatten()
                .and_then(|c| c.dyn_into::<web_sys::CanvasRenderingContext2d>().ok());

            let Some(ctx2d) = ctx2d else { return };

            let map_data = map.get();
            let tokens_data = tokens.get();
            let fog_data = fog.get();
            let selected_id = selected.get();

            if let Some(ref m) = map_data {
                let w = m.width * m.cell_size;
                let h = m.height * m.cell_size;
                canvas_el.set_width(w as u32);
                canvas_el.set_height(h as u32);

                // Clear
                ctx2d.set_fill_style_str("#1a1a2e");
                ctx2d.fill_rect(0.0, 0.0, w as f64, h as f64);

                // Draw background image if set
                if let Some(ref bg_url) = m.background_url {
                    if !bg_url.is_empty() {
                        if let Some(bg_img) = get_or_load_image(bg_url) {
                            let _ = ctx2d.draw_image_with_html_image_element_and_dw_and_dh(
                                &bg_img,
                                0.0,
                                0.0,
                                w as f64,
                                h as f64,
                            );
                        }
                    }
                }

                // Draw grid
                ctx2d.set_stroke_style_str("#333355");
                ctx2d.set_line_width(0.5);
                for x in 0..=m.width {
                    let px = (x * m.cell_size) as f64;
                    ctx2d.begin_path();
                    ctx2d.move_to(px, 0.0);
                    ctx2d.line_to(px, h as f64);
                    ctx2d.stroke();
                }
                for y in 0..=m.height {
                    let py = (y * m.cell_size) as f64;
                    ctx2d.begin_path();
                    ctx2d.move_to(0.0, py);
                    ctx2d.line_to(w as f64, py);
                    ctx2d.stroke();
                }

                // Draw tokens
                let cell = m.cell_size as f64;
                for t in &tokens_data {
                    if !t.visible {
                        continue;
                    }
                    let cx = (t.x as f64 + 0.5) * cell;
                    let cy = (t.y as f64 + 0.5) * cell;
                    let radius = cell * t.size as f64 * 0.4;

                    // Draw token image or colored circle
                    if let Some(ref img_url) = t.image_url {
                        if let Some(img) = get_or_load_image(img_url) {
                            // Clip to circle and draw image
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
                            // Image loading — draw placeholder circle
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

                    if selected_id == Some(t.id) {
                        ctx2d.begin_path();
                        let _ = ctx2d.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
                        ctx2d.set_stroke_style_str("#ffff00");
                        ctx2d.set_line_width(2.0);
                        ctx2d.stroke();
                    }

                    // Label
                    ctx2d.set_fill_style_str("#ffffff");
                    ctx2d.set_font("11px sans-serif");
                    ctx2d.set_text_align("center");
                    ctx2d.set_text_baseline("middle");
                    let _ = ctx2d.fill_text(&t.label, cx, cy);

                    // HP bar
                    if let (Some(cur), Some(max)) = (t.current_hp, t.max_hp) {
                        if max > 0 {
                            let bar_w = radius * 1.6;
                            let bar_h = 4.0;
                            let bar_x = cx - bar_w / 2.0;
                            let bar_y = cy + radius + 3.0;

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
            } else {
                canvas_el.set_width(600);
                canvas_el.set_height(400);
                ctx2d.set_fill_style_str("#1a1a2e");
                ctx2d.fill_rect(0.0, 0.0, 600.0, 400.0);
                ctx2d.set_fill_style_str("#666688");
                ctx2d.set_font("16px sans-serif");
                ctx2d.set_text_align("center");
                let _ = ctx2d.fill_text("No map loaded", 300.0, 200.0);
            }
        });
    }

    let on_mousedown = {
        #[cfg(feature = "hydrate")]
        {
            let canvas_ref = canvas_ref.clone();
            move |ev: leptos::ev::MouseEvent| {
                let map_data = map.get();
                let tokens_data = tokens.get();
                let Some(m) = map_data else { return };
                let Some((mx, my)) = canvas_coords(&canvas_ref, &ev) else {
                    return;
                };

                let cell = m.cell_size as f64;

                let clicked = tokens_data.iter().rev().find(|t| {
                    if !t.visible {
                        return false;
                    }
                    let cx = (t.x as f64 + 0.5) * cell;
                    let cy = (t.y as f64 + 0.5) * cell;
                    let radius = cell * t.size as f64 * 0.4;
                    let dx = mx - cx;
                    let dy = my - cy;
                    (dx * dx + dy * dy).sqrt() <= radius
                });

                if let Some(t) = clicked {
                    set_selected.set(Some(t.id));
                    set_dragging.set(Some(t.id));
                } else {
                    set_selected.set(None);
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
                let Some(token_id) = dragging.get() else {
                    return;
                };
                let map_data = map.get();
                let Some(m) = map_data else { return };
                let Some((mx, my)) = canvas_coords(&canvas_ref, &ev) else {
                    return;
                };

                let cell = m.cell_size as f64;
                let grid_x = (mx / cell).floor() as f32;
                let grid_y = (my / cell).floor() as f32;

                tokens.update(|ts| {
                    if let Some(t) = ts.iter_mut().find(|t| t.id == token_id) {
                        t.x = grid_x;
                        t.y = grid_y;
                    }
                });
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
                let Some(token_id) = dragging.get() else {
                    return;
                };
                let map_data = map.get();
                let Some(m) = map_data else { return };
                let Some((mx, my)) = canvas_coords(&canvas_ref, &ev) else {
                    return;
                };

                let cell = m.cell_size as f64;
                let grid_x = (mx / cell).floor() as f32;
                let grid_y = (my / cell).floor() as f32;

                send.with_value(|f| {
                    if let Some(f) = f {
                        f(ClientMessage::MoveToken {
                            token_id,
                            x: grid_x,
                            y: grid_y,
                        });
                    }
                });

                set_dragging.set(None);
            }
        }
        #[cfg(not(feature = "hydrate"))]
        {
            move |_ev: leptos::ev::MouseEvent| {}
        }
    };

    let on_bg_select = Callback::new(move |media: crate::models::MediaInfo| {
        let url = media.url.clone();
        let map_data = map.get();
        let sid = session_id.get();
        if let Some(m) = map_data {
            let map_id = m.id;
            leptos::task::spawn_local(async move {
                let _ = crate::server::api::update_map_background(map_id, sid, Some(url)).await;
                // Update local state
            });
            map.update(|m_opt| {
                if let Some(m) = m_opt {
                    m.background_url = Some(media.url.clone());
                }
            });
        }
        show_media_browser.set(false);
    });

    view! {
        <div class="map-container">
            <h3>"Map"
                <button
                    class="map-bg-btn"
                    on:click=move |_| show_media_browser.set(true)
                    style="margin-left: 10px; font-size: 12px;"
                >"Set Background"</button>
            </h3>
            <canvas
                node_ref=canvas_ref
                on:mousedown=on_mousedown
                on:mousemove=on_mousemove
                on:mouseup=on_mouseup
                style="cursor: pointer; border: 1px solid #444;"
            />
            <TokenHpPopup selected=selected tokens=tokens />
            <crate::components::media_browser::MediaBrowser
                on_select=on_bg_select
                filter_type="image".to_string()
                show=show_media_browser
            />
        </div>
    }
}

#[component]
fn TokenHpPopup(
    selected: ReadSignal<Option<i32>>,
    tokens: RwSignal<Vec<TokenInfo>>,
) -> impl IntoView {
    let ctx = expect_context::<GameContext>();
    let send = ctx.send;

    let selected_token = move || {
        let sel_id = selected.get()?;
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

    view! {
        {move || {
            selected_token().and_then(|t| {
                let hp = t.current_hp?;
                let max = t.max_hp?;
                let tid = t.id;
                Some(view! {
                    <div class="token-hp-popup">
                        <strong>{t.label.clone()}</strong>
                        <span>" HP: " {hp} "/" {max}</span>
                        <button on:click=move |_| do_hp_change(tid, -1)>"-1"</button>
                        <button on:click=move |_| do_hp_change(tid, 1)>"+1"</button>
                        <button on:click=move |_| do_hp_change(tid, -5)>"-5"</button>
                        <button on:click=move |_| do_hp_change(tid, 5)>"+5"</button>
                    </div>
                })
            })
        }}
    }
}
