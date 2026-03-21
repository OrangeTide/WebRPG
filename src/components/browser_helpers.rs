/// Shared browser-side helpers for download, upload, and blob operations.
/// Used by the file browser and terminal components.

#[cfg(feature = "hydrate")]
use crate::vfs::Drive;

/// Trigger a browser download from in-memory bytes.
/// Creates a temporary blob URL, attaches it to a hidden anchor element,
/// clicks it, then cleans up.
#[cfg(feature = "hydrate")]
pub fn trigger_browser_download(filename: &str, data: &[u8], content_type: &str) {
    use js_sys::{Array, Uint8Array};
    use wasm_bindgen::JsCast;

    let uint8 = Uint8Array::from(data);
    let array = Array::new();
    array.push(&uint8.buffer());

    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type(content_type);
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&array, &opts).unwrap();
    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let a: web_sys::HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
    a.set_href(&url);
    a.set_download(filename);
    let _ = a.set_attribute("style", "display:none");
    let _ = document.body().unwrap().append_child(&a);
    a.click();
    let _ = document.body().unwrap().remove_child(&a);
    let _ = web_sys::Url::revoke_object_url(&url);
}

/// Picker mode for `open_picker`.
#[cfg(feature = "hydrate")]
pub enum PickerMode {
    /// Select multiple individual files.
    Files,
    /// Select a folder (uses webkitdirectory).
    Folder,
}

/// Open a browser file/folder picker dialog.
/// Returns the selected `FileList`, or an error string if cancelled or empty.
#[cfg(feature = "hydrate")]
async fn open_picker(mode: PickerMode) -> Result<web_sys::FileList, String> {
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
    match mode {
        PickerMode::Files => input.set_multiple(true),
        PickerMode::Folder => {
            let _ = input.set_attribute("webkitdirectory", "");
        }
    }

    let promise = js_sys::Promise::new(&mut {
        let input_for_change = input.clone();
        let input_for_cancel = input.clone();
        move |resolve, _reject| {
            let resolve_change = resolve.clone();
            let on_change = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                let _ = resolve_change.call1(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_bool(true),
                );
            });
            input_for_change.set_onchange(Some(on_change.as_ref().unchecked_ref()));
            on_change.forget();

            let on_cancel = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
                let _ = resolve.call1(
                    &wasm_bindgen::JsValue::NULL,
                    &wasm_bindgen::JsValue::from_bool(false),
                );
            });
            let _ = input_for_cancel
                .add_event_listener_with_callback("cancel", on_cancel.as_ref().unchecked_ref());
            on_cancel.forget();
        }
    });

    input.click();

    let result = JsFuture::from(promise).await;
    let selected = result
        .as_ref()
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !selected {
        return Err("Cancelled".to_string());
    }

    match input.files() {
        Some(f) if f.length() > 0 => Ok(f),
        _ => Err("No files selected".to_string()),
    }
}

/// Open a browser file picker dialog allowing multiple file selection.
#[cfg(feature = "hydrate")]
pub async fn open_file_picker() -> Result<web_sys::FileList, String> {
    open_picker(PickerMode::Files).await
}

/// Open a browser folder picker dialog (webkitdirectory).
/// Returns the selected `FileList` with webkitRelativePath on each File.
#[cfg(feature = "hydrate")]
pub async fn open_folder_picker() -> Result<web_sys::FileList, String> {
    open_picker(PickerMode::Folder).await
}

/// Upload a large file via the media CAS endpoint, then write a CAS reference to VFS.
#[cfg(feature = "hydrate")]
pub async fn upload_large_file(
    file: &web_sys::File,
    drive: &Drive,
    file_path: &str,
    size: u64,
    content_type: Option<String>,
    sid: Option<i32>,
) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let form_data = web_sys::FormData::new().unwrap();
    let _ = form_data.append_with_blob("file", file);

    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&form_data);
    opts.set_credentials(web_sys::RequestCredentials::SameOrigin);

    let request = web_sys::Request::new_with_str_and_init("/api/media/upload", &opts).unwrap();

    let window = web_sys::window().unwrap();
    let resp = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("upload error ({e:?})"))?;
    let resp: web_sys::Response = resp.dyn_into().unwrap();
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().unwrap())
        .await
        .map_err(|e| format!("parse error ({e:?})"))?;
    let hash = js_sys::Reflect::get(&json, &"hash".into())
        .ok()
        .and_then(|v| v.as_string())
        .ok_or_else(|| "missing hash in response".to_string())?;

    crate::server::api::vfs_write_cas(
        drive.as_str().to_string(),
        file_path.to_string(),
        hash,
        size as i64,
        content_type,
        sid,
    )
    .await
    .map_err(|e| e.to_string())
}
