pub mod api;

#[cfg(feature = "ssr")]
pub mod media_handler;

#[cfg(feature = "ssr")]
pub mod metrics;

#[cfg(feature = "ssr")]
pub mod tui;

#[cfg(feature = "ssr")]
pub mod ws_handler;
