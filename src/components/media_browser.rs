use leptos::prelude::*;

use crate::models::MediaInfo;

#[component]
pub fn MediaBrowser(
    on_select: Callback<MediaInfo>,
    #[prop(optional)] filter_type: Option<String>,
    show: RwSignal<bool>,
) -> impl IntoView {
    let (search_text, set_search_text) = signal(String::new());
    let (selected_tag, set_selected_tag) = signal(Option::<String>::None);
    let (tag_input, set_tag_input) = signal(String::new());
    let uploading = RwSignal::new(false);
    // Counter to force refetch after upload (changing it triggers the Resource)
    let (refetch_counter, _set_refetch_counter) = signal(0u32);
    // Alias used only in hydrate cfg block
    #[cfg(feature = "hydrate")]
    let set_refetch_counter = _set_refetch_counter;

    let filter_type_for_resource = filter_type.clone();
    let media_list = Resource::new(
        move || {
            (
                search_text.get(),
                selected_tag.get(),
                show.get(),
                refetch_counter.get(),
            )
        },
        move |(search, tag, visible, _refetch)| {
            let ft = filter_type_for_resource.clone();
            async move {
                if !visible {
                    return Ok(vec![]);
                }
                let search_opt = if search.is_empty() {
                    None
                } else {
                    Some(search)
                };
                crate::server::api::list_media(ft, search_opt, tag).await
            }
        },
    );

    let tag_suggestions = Resource::new(
        move || tag_input.get(),
        |prefix| async move {
            let p = if prefix.is_empty() {
                None
            } else {
                Some(prefix)
            };
            crate::server::api::list_media_tags(p).await
        },
    );

    #[cfg(feature = "hydrate")]
    let do_upload = move |_| {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let input: web_sys::HtmlInputElement = document
            .create_element("input")
            .unwrap()
            .dyn_into()
            .unwrap();
        input.set_type("file");
        input.set_accept("image/png,image/jpeg,image/gif,image/webp,audio/wav,audio/mpeg");

        let input_clone = input.clone();
        let on_change = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
            let files = input_clone.files();
            let Some(files) = files else { return };
            let Some(file) = files.get(0) else { return };

            uploading.set(true);

            leptos::task::spawn_local(async move {
                let form_data = web_sys::FormData::new().unwrap();
                let _ = form_data.append_with_blob("file", &file);

                let opts = web_sys::RequestInit::new();
                opts.set_method("POST");
                opts.set_body(&form_data);
                opts.set_credentials(web_sys::RequestCredentials::SameOrigin);

                let request =
                    web_sys::Request::new_with_str_and_init("/api/media/upload", &opts).unwrap();

                let window = web_sys::window().unwrap();
                match JsFuture::from(window.fetch_with_request(&request)).await {
                    Ok(resp) => {
                        let resp: web_sys::Response = resp.dyn_into().unwrap();
                        if resp.ok() {
                            set_refetch_counter.update(|c| *c += 1);
                        } else {
                            log::error!("Upload failed: {}", resp.status());
                        }
                    }
                    Err(e) => {
                        log::error!("Upload error: {e:?}");
                    }
                }

                uploading.set(false);
            });
        });
        input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
        on_change.forget();
        input.click();
    };

    #[cfg(not(feature = "hydrate"))]
    let do_upload = move |_: leptos::ev::MouseEvent| {};

    let close = move |_| {
        show.set(false);
    };

    let filter_type_display = filter_type.clone();

    view! {
        <Show when=move || show.get()>
            <div class="media-browser-overlay" on:click=close>
                <div class="media-browser" on:click=move |ev| ev.stop_propagation()>
                    <div class="media-browser-header">
                        <h3>"Media Browser"
                            {filter_type_display.as_ref().map(|t| format!(" ({t})"))}
                        </h3>
                        <button class="close-btn" on:click=close>"X"</button>
                    </div>

                    <div class="media-browser-controls">
                        <input
                            type="text"
                            placeholder="Search..."
                            prop:value=search_text
                            on:input=move |ev| set_search_text.set(event_target_value(&ev))
                        />
                        <div class="tag-filter">
                            <input
                                type="text"
                                placeholder="Filter by tag..."
                                prop:value=tag_input
                                on:input=move |ev| set_tag_input.set(event_target_value(&ev))
                            />
                            <Suspense fallback=|| ()>
                                {move || {
                                    let input = tag_input.get();
                                    if input.is_empty() {
                                        return None;
                                    }
                                    let suggestions = tag_suggestions.get()
                                        .unwrap_or(Ok(vec![]))
                                        .unwrap_or_default();
                                    if suggestions.is_empty() {
                                        return None;
                                    }
                                    Some(view! {
                                        <div class="tag-suggestions">
                                            <For
                                                each=move || suggestions.clone()
                                                key=|t| t.clone()
                                                let:tag
                                            >
                                                {
                                                    let tag_val = tag.clone();
                                                    view! {
                                                        <button
                                                            class="tag-suggestion"
                                                            on:click=move |_| {
                                                                set_selected_tag.set(Some(tag_val.clone()));
                                                                set_tag_input.set(String::new());
                                                            }
                                                        >
                                                            {tag.clone()}
                                                        </button>
                                                    }
                                                }
                                            </For>
                                        </div>
                                    })
                                }}
                            </Suspense>
                        </div>
                        {move || {
                            selected_tag.get().map(|tag| {
                                view! {
                                    <div class="active-tag-filter">
                                        <span>"Tag: " {tag.clone()}</span>
                                        <button on:click=move |_| set_selected_tag.set(None)>"x"</button>
                                    </div>
                                }
                            })
                        }}
                        <button
                            class="upload-btn"
                            on:click=do_upload
                            disabled=uploading
                        >
                            {move || if uploading.get() { "Uploading..." } else { "Upload" }}
                        </button>
                    </div>

                    <div class="media-browser-grid">
                        <Suspense fallback=|| view! { <p>"Loading..."</p> }>
                            {move || {
                                let items = media_list.get()
                                    .unwrap_or(Ok(vec![]))
                                    .unwrap_or_default();
                                view! {
                                    <For
                                        each=move || items.clone()
                                        key=|m| m.id
                                        let:item
                                    >
                                        <MediaItem item=item on_select=on_select />
                                    </For>
                                }
                            }}
                        </Suspense>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn MediaItem(item: MediaInfo, on_select: Callback<MediaInfo>) -> impl IntoView {
    let item_clone = item.clone();
    let is_image = item.media_type == "image";
    let url = item.url.clone();
    let tags_display = item.tags.join(", ");

    view! {
        <div
            class="media-item"
            on:click=move |_| {
                on_select.run(item_clone.clone());
            }
        >
            {if is_image {
                view! {
                    <img src=url.clone() alt="media thumbnail" class="media-thumb" />
                }.into_any()
            } else {
                view! {
                    <div class="media-audio-icon">"audio"</div>
                }.into_any()
            }}
            <div class="media-item-info">
                <span class="media-tags">{tags_display}</span>
            </div>
        </div>
    }
}
