//! Client-side scratch drive storage using browser IndexedDB.
//!
//! Scratch drives (A: and B:) are per-tab, stored entirely client-side.
//! Each tab gets a unique database name based on a random key so data
//! disappears when the tab closes (the database is deleted on unload).
//!
//! ## IndexedDB Schema
//!
//! Single object store `files` with key `path` (String):
//! ```text
//! { path: "/foo/bar.txt", is_directory: bool, data: Uint8Array, content_type: String,
//!   size_bytes: u32, mode: u32, created_at: u32, updated_at: u32 }
//! ```
//!
//! This module is only compiled for the `hydrate` (WASM) target.

#![cfg(feature = "hydrate")]

use js_sys::{Array, Date, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::vfs::Drive;

/// Per-tab scratch drive quota: 10 MB.
const SCRATCH_QUOTA: u64 = 10 * 1024 * 1024;

/// Entry metadata returned from scratch drive operations.
#[derive(Debug, Clone)]
pub struct ScratchEntry {
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: i64,
    pub content_type: Option<String>,
    pub mode: i32,
    pub created_at: i32,
    pub updated_at: i32,
}

/// A handle to one scratch drive's IndexedDB database.
#[derive(Clone)]
pub struct ScratchDrive {
    db: web_sys::IdbDatabase,
}

/// Open (or create) the IndexedDB database for a scratch drive.
/// `db_name` should be unique per tab (e.g. "webrpg_scratch_A_{random}").
pub async fn open_scratch_db(db_name: &str) -> Result<ScratchDrive, String> {
    let window = web_sys::window().ok_or("no window")?;
    let idb_factory = window
        .indexed_db()
        .map_err(|e| format!("{e:?}"))?
        .ok_or("IndexedDB not available")?;

    let open_req = idb_factory
        .open_with_u32(db_name, 1)
        .map_err(|e| format!("{e:?}"))?;

    // Set up schema on upgrade
    let on_upgrade = Closure::<dyn Fn(web_sys::Event)>::new(move |event: web_sys::Event| {
        let target: web_sys::IdbOpenDbRequest = event.target().unwrap().unchecked_into();
        let db: web_sys::IdbDatabase = target.result().unwrap().unchecked_into();

        if !store_exists(&db, "files") {
            let params = web_sys::IdbObjectStoreParameters::new();
            params.set_key_path(&JsValue::from_str("path"));
            let _ = db.create_object_store_with_optional_parameters("files", &params);
        }
    });
    open_req.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));
    open_req.set_onsuccess(None);
    open_req.set_onerror(None);

    let db = idb_request_to_future(&open_req).await?;
    let db: web_sys::IdbDatabase = db.unchecked_into();

    // Drop the closure after we're done (it's only needed during open)
    drop(on_upgrade);

    Ok(ScratchDrive { db })
}

fn store_exists(db: &web_sys::IdbDatabase, name: &str) -> bool {
    let names = db.object_store_names();
    for i in 0..names.length() {
        if names.get(i).as_deref() == Some(name) {
            return true;
        }
    }
    false
}

/// Convert an IdbRequest into a Future that resolves with the result.
async fn idb_request_to_future(req: &web_sys::IdbRequest) -> Result<JsValue, String> {
    use std::cell::RefCell;
    use std::rc::Rc;

    let req_ok = req.clone();
    let req_err = req.clone();

    // Wrap resolve/reject in Rc<RefCell> so we can move them into FnMut closures
    // from the outer FnMut (which Promise::new only calls once, but Rust can't prove that).
    let resolve_cell: Rc<RefCell<Option<js_sys::Function>>> = Rc::new(RefCell::new(None));
    let reject_cell: Rc<RefCell<Option<js_sys::Function>>> = Rc::new(RefCell::new(None));

    let resolve_store = resolve_cell.clone();
    let reject_store = reject_cell.clone();

    let promise = js_sys::Promise::new(&mut move |resolve, reject| {
        *resolve_store.borrow_mut() = Some(resolve);
        *reject_store.borrow_mut() = Some(reject);
    });

    // Now set callbacks using the stored resolve/reject
    let on_success = Closure::<dyn FnMut()>::new(move || {
        if let Some(resolve) = resolve_cell.borrow_mut().take() {
            let _ = resolve.call1(
                &JsValue::NULL,
                &req_ok.result().unwrap_or(JsValue::UNDEFINED),
            );
        }
    });
    req.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));
    on_success.forget();

    let on_error = Closure::<dyn FnMut()>::new(move || {
        if let Some(reject) = reject_cell.borrow_mut().take() {
            let err_msg = req_err
                .error()
                .ok()
                .flatten()
                .map(|e| e.message())
                .unwrap_or_else(|| "IndexedDB error".to_string());
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&err_msg));
        }
    });
    req.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();

    JsFuture::from(promise).await.map_err(|e| {
        e.as_string()
            .unwrap_or_else(|| "IndexedDB error".to_string())
    })
}

fn now_unix() -> i32 {
    (Date::now() / 1000.0) as i32
}

impl ScratchDrive {
    fn tx_rw(&self) -> Result<web_sys::IdbObjectStore, String> {
        let tx = self
            .db
            .transaction_with_str_and_mode("files", web_sys::IdbTransactionMode::Readwrite)
            .map_err(|e| format!("{e:?}"))?;
        tx.object_store("files").map_err(|e| format!("{e:?}"))
    }

    fn tx_ro(&self) -> Result<web_sys::IdbObjectStore, String> {
        let tx = self
            .db
            .transaction_with_str_and_mode("files", web_sys::IdbTransactionMode::Readonly)
            .map_err(|e| format!("{e:?}"))?;
        tx.object_store("files").map_err(|e| format!("{e:?}"))
    }

    fn js_to_entry(val: &JsValue) -> Option<ScratchEntry> {
        let path = Reflect::get(val, &"path".into()).ok()?.as_string()?;
        let is_directory = Reflect::get(val, &"is_directory".into())
            .ok()?
            .as_bool()
            .unwrap_or(false);
        let size_bytes = Reflect::get(val, &"size_bytes".into())
            .ok()?
            .as_f64()
            .unwrap_or(0.0) as i64;
        let content_type = Reflect::get(val, &"content_type".into())
            .ok()
            .and_then(|v| v.as_string());
        let mode = Reflect::get(val, &"mode".into())
            .ok()?
            .as_f64()
            .unwrap_or(0o666 as f64) as i32;
        let created_at = Reflect::get(val, &"created_at".into())
            .ok()?
            .as_f64()
            .unwrap_or(0.0) as i32;
        let updated_at = Reflect::get(val, &"updated_at".into())
            .ok()?
            .as_f64()
            .unwrap_or(0.0) as i32;
        Some(ScratchEntry {
            path,
            is_directory,
            size_bytes,
            content_type,
            mode,
            created_at,
            updated_at,
        })
    }

    /// Get file/directory metadata.
    pub async fn stat(&self, path: &str) -> Result<ScratchEntry, String> {
        let store = self.tx_ro()?;
        let req = store
            .get(&JsValue::from_str(path))
            .map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        if val.is_undefined() || val.is_null() {
            return Err(format!("File not found: {path}"));
        }
        Self::js_to_entry(&val).ok_or_else(|| "Invalid entry".to_string())
    }

    /// List direct children of a directory.
    pub async fn list(&self, dir_path: &str) -> Result<Vec<ScratchEntry>, String> {
        let store = self.tx_ro()?;
        let req = store.get_all().map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        let array: Array = val.unchecked_into();

        let prefix = if dir_path == "/" {
            "/".to_string()
        } else {
            format!("{}/", dir_path.trim_end_matches('/'))
        };

        let mut entries = Vec::new();
        for i in 0..array.length() {
            let item = array.get(i);
            if let Some(entry) = Self::js_to_entry(&item) {
                // Skip the directory itself
                if entry.path == dir_path {
                    continue;
                }
                // Only direct children: starts with prefix and has no further slashes
                if entry.path.starts_with(&prefix) {
                    let remainder = &entry.path[prefix.len()..];
                    if !remainder.contains('/') {
                        entries.push(entry);
                    }
                }
            }
        }

        entries.sort_by(|a, b| {
            b.is_directory
                .cmp(&a.is_directory)
                .then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
        });

        Ok(entries)
    }

    /// Read file data. Returns `(data, content_type)`.
    pub async fn read(&self, path: &str) -> Result<(Vec<u8>, Option<String>), String> {
        let store = self.tx_ro()?;
        let req = store
            .get(&JsValue::from_str(path))
            .map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        if val.is_undefined() || val.is_null() {
            return Err(format!("File not found: {path}"));
        }

        let is_dir = Reflect::get(&val, &"is_directory".into())
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if is_dir {
            return Err("Cannot read a directory".to_string());
        }

        let data_val = Reflect::get(&val, &"data".into()).map_err(|e| format!("{e:?}"))?;
        let data = if data_val.is_undefined() || data_val.is_null() {
            Vec::new()
        } else {
            let uint8: Uint8Array = data_val.unchecked_into();
            uint8.to_vec()
        };
        let content_type = Reflect::get(&val, &"content_type".into())
            .ok()
            .and_then(|v| v.as_string());

        Ok((data, content_type))
    }

    /// Compute total bytes used across all files.
    async fn used_bytes(&self) -> Result<u64, String> {
        let store = self.tx_ro()?;
        let req = store.get_all().map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        let array: Array = val.unchecked_into();

        let mut total: u64 = 0;
        for i in 0..array.length() {
            let item = array.get(i);
            let size = Reflect::get(&item, &"size_bytes".into())
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as u64;
            total += size;
        }
        Ok(total)
    }

    /// Write a file.
    pub async fn write(
        &self,
        path: &str,
        data: &[u8],
        content_type: Option<&str>,
    ) -> Result<(), String> {
        // Check quota
        let used = self.used_bytes().await?;
        let new_total = used + data.len() as u64;
        if new_total > SCRATCH_QUOTA {
            return Err(format!(
                "Quota exceeded: {} + {} > {} bytes",
                used,
                data.len(),
                SCRATCH_QUOTA
            ));
        }

        // Ensure parent directory exists
        let parent = parent_path(path);
        if parent != "/" {
            self.ensure_dir(parent).await?;
        }

        let store = self.tx_rw()?;
        let now = now_unix();
        let uint8 = Uint8Array::from(data);

        let obj = Object::new();
        Reflect::set(&obj, &"path".into(), &JsValue::from_str(path)).unwrap();
        Reflect::set(&obj, &"is_directory".into(), &JsValue::from_bool(false)).unwrap();
        Reflect::set(&obj, &"data".into(), &uint8).unwrap();
        Reflect::set(
            &obj,
            &"content_type".into(),
            &content_type
                .map(|s| JsValue::from_str(s))
                .unwrap_or(JsValue::NULL),
        )
        .unwrap();
        Reflect::set(
            &obj,
            &"size_bytes".into(),
            &JsValue::from_f64(data.len() as f64),
        )
        .unwrap();
        Reflect::set(&obj, &"mode".into(), &JsValue::from_f64(0o666 as f64)).unwrap();
        Reflect::set(&obj, &"created_at".into(), &JsValue::from_f64(now as f64)).unwrap();
        Reflect::set(&obj, &"updated_at".into(), &JsValue::from_f64(now as f64)).unwrap();

        let req = store.put(&obj).map_err(|e| format!("{e:?}"))?;
        idb_request_to_future(&req).await?;
        Ok(())
    }

    /// Create a directory.
    pub async fn mkdir(&self, path: &str) -> Result<(), String> {
        // Check if it already exists
        let store = self.tx_ro()?;
        let req = store
            .get(&JsValue::from_str(path))
            .map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        if !val.is_undefined() && !val.is_null() {
            return Err(format!("Already exists: {path}"));
        }

        // Ensure parent exists
        let parent = parent_path(path);
        if parent != "/" && parent != path {
            self.ensure_dir(parent).await?;
        }

        self.write_dir_entry(path).await
    }

    /// Ensure a directory exists, creating parents recursively.
    async fn ensure_dir(&self, path: &str) -> Result<(), String> {
        if path == "/" {
            return Ok(());
        }
        let store = self.tx_ro()?;
        let req = store
            .get(&JsValue::from_str(path))
            .map_err(|e| format!("{e:?}"))?;
        let val = idb_request_to_future(&req).await?;
        if !val.is_undefined() && !val.is_null() {
            return Ok(()); // already exists
        }

        let parent = parent_path(path);
        if parent != "/" && parent != path {
            // Recursive call via Box::pin to allow async recursion
            Box::pin(self.ensure_dir(parent)).await?;
        }

        self.write_dir_entry(path).await
    }

    async fn write_dir_entry(&self, path: &str) -> Result<(), String> {
        let store = self.tx_rw()?;
        let now = now_unix();

        let obj = Object::new();
        Reflect::set(&obj, &"path".into(), &JsValue::from_str(path)).unwrap();
        Reflect::set(&obj, &"is_directory".into(), &JsValue::from_bool(true)).unwrap();
        Reflect::set(&obj, &"data".into(), &JsValue::NULL).unwrap();
        Reflect::set(&obj, &"content_type".into(), &JsValue::NULL).unwrap();
        Reflect::set(&obj, &"size_bytes".into(), &JsValue::from_f64(0.0)).unwrap();
        Reflect::set(&obj, &"mode".into(), &JsValue::from_f64(0o777 as f64)).unwrap();
        Reflect::set(&obj, &"created_at".into(), &JsValue::from_f64(now as f64)).unwrap();
        Reflect::set(&obj, &"updated_at".into(), &JsValue::from_f64(now as f64)).unwrap();

        let req = store.put(&obj).map_err(|e| format!("{e:?}"))?;
        idb_request_to_future(&req).await?;
        Ok(())
    }

    /// Delete a file or empty directory.
    pub async fn delete(&self, path: &str) -> Result<(), String> {
        // Check if it's a non-empty directory
        let children = self.list(path).await?;
        if !children.is_empty() {
            return Err("Directory not empty".to_string());
        }

        let store = self.tx_rw()?;
        let req = store
            .delete(&JsValue::from_str(path))
            .map_err(|e| format!("{e:?}"))?;
        idb_request_to_future(&req).await?;
        Ok(())
    }

    /// Get drive usage info.
    pub async fn drive_info(&self, drive: Drive) -> Result<(u64, u64), String> {
        let used = self.used_bytes().await?;
        let quota = drive.quota_bytes(false);
        Ok((used, quota))
    }

    /// Delete the entire database (call on tab close).
    pub fn destroy(self) {
        let name = self.db.name();
        self.db.close();
        if let Some(window) = web_sys::window() {
            if let Ok(Some(factory)) = window.indexed_db() {
                let _ = factory.delete_database(&name);
            }
        }
    }
}

/// Shared scratch drive handles for A: and B: drives.
/// Provided via Leptos context so all components share the same IndexedDB instances.
#[derive(Clone)]
pub struct ScratchDrives {
    pub a: Option<ScratchDrive>,
    pub b: Option<ScratchDrive>,
}

impl ScratchDrives {
    pub fn get(&self, drive: Drive) -> Option<&ScratchDrive> {
        match drive {
            Drive::A => self.a.as_ref(),
            Drive::B => self.b.as_ref(),
            _ => None,
        }
    }
}

fn parent_path(path: &str) -> &str {
    if path == "/" {
        return "/";
    }
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) => "/",
        Some(pos) => &trimmed[..pos],
        None => "/",
    }
}
