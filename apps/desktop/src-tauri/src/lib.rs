#![forbid(unsafe_code)]
//! Desktop-only orchestration. Core crates remain independent of Tauri.
pub mod alerts;
pub mod avatars;
pub mod categories;
pub mod dto;
pub mod events;
pub mod model_download;
pub mod onboarding;
pub mod reminders;
pub mod remote;
pub mod rules;
#[cfg(feature = "native-runtime")]
pub mod runtime;
pub mod saved_views;
pub mod service;
pub mod settings;
pub mod worker;
pub use dto::*;
pub use service::{DesktopService, IpcError};
