#![forbid(unsafe_code)]
//! Desktop-only orchestration. Core crates remain independent of Tauri.
pub mod dto;
pub mod events;
pub mod onboarding;
pub mod remote;
#[cfg(feature = "native-runtime")]
pub mod runtime;
pub mod service;
pub mod worker;
pub use dto::*;
pub use service::{DesktopService, IpcError};
