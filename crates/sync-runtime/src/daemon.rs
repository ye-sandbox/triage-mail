//! Headless daemon and periodic sync scheduler.
use std::time::Duration;

use serde::{Deserialize, Serialize};
use storage::SqliteMailStore;

use crate::host::{sync_accounts, HeadlessSyncError, SyncRunSummary, SyncTransport};

/// Minimum allowed sync interval in seconds.
pub const MIN_INTERVAL_SECS: u64 = 1;
/// Maximum allowed sync interval in seconds (24 hours).
pub const MAX_INTERVAL_SECS: u64 = 86_400;
/// Default interval in seconds between sync passes.
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

/// Configuration options for the background sync daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonConfig {
    pub interval_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<usize>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            interval_secs: DEFAULT_INTERVAL_SECS,
            account_filter: None,
            max_iterations: None,
        }
    }
}

impl DaemonConfig {
    pub fn new(interval_secs: u64) -> Result<Self, HeadlessSyncError> {
        if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&interval_secs) {
            return Err(HeadlessSyncError::InvalidConfig(
                "interval_secs out of bounds".to_string(),
            ));
        }
        Ok(Self {
            interval_secs,
            account_filter: None,
            max_iterations: None,
        })
    }

    pub fn with_account_filter(mut self, account: Option<String>) -> Self {
        self.account_filter = account;
        self
    }

    pub fn with_max_iterations(mut self, max: Option<usize>) -> Self {
        self.max_iterations = max;
        self
    }
}

/// Structured events emitted by the daemon lifecycle and sync passes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum DaemonEvent {
    Started {
        interval_secs: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        account_filter: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_iterations: Option<usize>,
    },
    SyncTick {
        iteration: usize,
        summary: SyncRunSummary,
    },
    Stopping {
        reason: String,
    },
    Stopped {
        total_iterations: usize,
        total_synced_messages: usize,
    },
}

/// Executes the background sync daemon loop.
///
/// Runs periodic sync passes, respects shutdown signals via `shutdown_rx`,
/// and emits structured events via `on_event`.
pub async fn run_daemon<T: SyncTransport, F>(
    store: &SqliteMailStore,
    transport: &T,
    config: DaemonConfig,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    mut on_event: F,
) -> Result<(), HeadlessSyncError>
where
    F: FnMut(DaemonEvent) + Send,
{
    if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&config.interval_secs) {
        return Err(HeadlessSyncError::InvalidConfig(
            "interval_secs out of bounds".to_string(),
        ));
    }

    on_event(DaemonEvent::Started {
        interval_secs: config.interval_secs,
        account_filter: config.account_filter.clone(),
        max_iterations: config.max_iterations,
    });

    let mut iteration = 0;
    let mut total_synced_messages = 0;
    let sleep_duration = Duration::from_secs(config.interval_secs);

    loop {
        if *shutdown_rx.borrow() {
            on_event(DaemonEvent::Stopping {
                reason: "shutdown_signal".to_string(),
            });
            break;
        }

        if config.max_iterations.is_some_and(|max| iteration >= max) {
            on_event(DaemonEvent::Stopping {
                reason: "max_iterations_reached".to_string(),
            });
            break;
        }

        iteration += 1;
        let summary = sync_accounts(store, transport, config.account_filter.as_deref()).await?;
        total_synced_messages += summary.total_synced_messages;

        on_event(DaemonEvent::SyncTick { iteration, summary });

        if config.max_iterations.is_some_and(|max| iteration >= max) {
            on_event(DaemonEvent::Stopping {
                reason: "max_iterations_reached".to_string(),
            });
            break;
        }

        tokio::select! {
            _ = tokio::time::sleep(sleep_duration) => {}
            changed = shutdown_rx.changed() => {
                if changed.is_ok() && *shutdown_rx.borrow() {
                    on_event(DaemonEvent::Stopping {
                        reason: "shutdown_signal".to_string(),
                    });
                    break;
                }
            }
        }
    }

    on_event(DaemonEvent::Stopped {
        total_iterations: iteration,
        total_synced_messages,
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_interval_bounds() {
        assert!(DaemonConfig::new(0).is_err());
        assert!(DaemonConfig::new(MIN_INTERVAL_SECS).is_ok());
        assert!(DaemonConfig::new(MAX_INTERVAL_SECS).is_ok());
        assert!(DaemonConfig::new(MAX_INTERVAL_SECS + 1).is_err());
    }

    #[test]
    fn event_json_serialization() {
        let event = DaemonEvent::Started {
            interval_secs: 60,
            account_filter: Some("acc1".into()),
            max_iterations: Some(5),
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains(r#""event":"started""#));
        assert!(json.contains(r#""interval_secs":60"#));
        assert!(json.contains(r#""account_filter":"acc1""#));

        let stop = DaemonEvent::Stopped {
            total_iterations: 3,
            total_synced_messages: 42,
        };
        let stop_json = serde_json::to_string(&stop).expect("serialize");
        assert!(stop_json.contains(r#""event":"stopped""#));
        assert!(stop_json.contains(r#""total_iterations":3"#));
        assert!(stop_json.contains(r#""total_synced_messages":42"#));
    }
}
