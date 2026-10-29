//! Leptos `#[server]` functions, grouped by area. Each submodule is re-exported
//! here so call sites keep using `crate::server::api::<fn>`.

mod admin;
mod auth;
mod characters;
mod media;
mod session;
mod vfs;

pub use admin::*;
pub use auth::*;
pub use characters::*;
pub use media::*;
pub use session::*;
pub use vfs::*;
