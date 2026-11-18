#![forbid(unsafe_code)]
//! Desktop-only orchestration. Core crates remain independent of Tauri.
pub mod alert_schedule;
pub mod alerts;
pub mod archive;
pub mod avatars;
pub mod backup;
pub mod categories;
pub mod conversations;
pub mod dto;
pub mod events;
#[cfg(feature = "native-runtime")]
mod file_picker;
pub mod model_download;
pub mod onboarding;
mod ownership;
pub mod reminders;
pub mod remote;
pub mod rules;
#[cfg(feature = "native-runtime")]
pub mod runtime;
pub mod saved_views;
pub mod sender_review;
pub mod service;
pub mod settings;
pub mod subscriptions;
#[cfg(feature = "native-runtime")]
pub mod tray;
pub mod worker;
pub use dto::*;
pub use service::{DesktopService, IpcError};

mod embeddings;
