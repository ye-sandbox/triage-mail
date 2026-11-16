use std::sync::{Arc, Mutex};

use mail_engine::{EngineError, SyncEventSink};
use tokio::sync::broadcast;
use triage_ai::{DeterministicTriageResult, TriageError, TriageEventHook};

use crate::dto::{SyncProgressEvent, SyncState};

/// Bounded broadcast decouples synchronous core hooks from IPC delivery.
#[derive(Clone)]
pub struct EventBus(broadcast::Sender<SyncProgressEvent>);

impl Default for EventBus {
    fn default() -> Self {
        Self(broadcast::channel(64).0)
    }
}

impl EventBus {
    pub fn subscribe(&self) -> broadcast::Receiver<SyncProgressEvent> {
        self.0.subscribe()
    }
    pub fn publish(&self, event: SyncProgressEvent) {
        let _ = self.0.send(event);
    }
}

/// Collects core notifications for one batch; flush only after the repository commit.
pub struct BatchEvents {
    account_id: String,
    event: Mutex<SyncProgressEvent>,
    bus: Arc<EventBus>,
}

impl BatchEvents {
    pub fn new(account_id: String, bus: Arc<EventBus>) -> Self {
        let event = SyncProgressEvent {
            account_id: account_id.clone(),
            state: SyncState::Idle,
            new_messages_count: 0,
            triage_alerts: Vec::new(),
            embedding_errors_count: 0,
            diagnostic: None,
        };
        Self {
            account_id,
            event: Mutex::new(event),
            bus,
        }
    }
    pub fn committed(&self) {
        self.committed_with_state(SyncState::Idle);
    }

    pub fn snapshot_with_state(&self, state: SyncState) -> SyncProgressEvent {
        if let Ok(event) = self.event.lock() {
            let mut event = event.clone();
            event.state = state;
            event
        } else {
            SyncProgressEvent {
                account_id: self.account_id.clone(),
                state,
                new_messages_count: 0,
                triage_alerts: Vec::new(),
                embedding_errors_count: 0,
                diagnostic: None,
            }
        }
    }

    pub fn committed_with_state(&self, state: SyncState) {
        if let Ok(event) = self.event.lock() {
            let mut event = event.clone();
            event.state = state;
            self.bus.publish(event);
        }
    }

    pub fn on_embedding_failed(&self, _message_id: &str, _error: &triage_ai::TriageError) {
        if let Ok(mut event) = self.event.lock() {
            event.embedding_errors_count = event.embedding_errors_count.saturating_add(1);
        }
    }
}

impl SyncEventSink for BatchEvents {
    fn on_sync_progress(&self, event: &mail_engine::SyncProgressEvent) -> Result<(), EngineError> {
        if event.account_id != self.account_id {
            return Err(EngineError::ImapProtocol("account mismatch".into()));
        }
        let mut output = self
            .event
            .lock()
            .map_err(|_| EngineError::ImapProtocol("event lock unavailable".into()))?;
        output.new_messages_count = event.synced_count;
        Ok(())
    }
}

impl TriageEventHook for BatchEvents {
    fn on_message_triaged(
        &self,
        _message_id: &str,
        result: &DeterministicTriageResult,
    ) -> Result<(), TriageError> {
        let mut event = self
            .event
            .lock()
            .map_err(|_| TriageError::Hook("event lock unavailable".into()))?;
        // INVARIANT: Bound preview alerts; full badges remain available through message detail.
        let available = 128_usize.saturating_sub(event.triage_alerts.len());
        event
            .triage_alerts
            .extend(result.badges.iter().take(available).cloned());
        Ok(())
    }
}
