//! Typed rule orchestration; delivery is delegated to the existing IMAP outbox.
use crate::{remote::RemoteMutation, DesktopService, IpcError, TriageActionDto};
use std::sync::Arc;
use triage_ai::rules::{Rule, RuleAction, RulePlan};

fn rule_error(_: storage::StorageError) -> IpcError {
    IpcError::InvalidRequest
}

pub(crate) fn process_jobs(service: &DesktopService) -> Result<(), IpcError> {
    let changed = service
        .store
        .process_rule_jobs(100, |id, account, folder, uid, validity, action| {
            let action = match action {
                RuleAction::Star => TriageActionDto::Star,
                RuleAction::Archive => TriageActionDto::Archive,
                _ => {
                    return Err(storage::StorageError::Query(
                        "invalid remote rule action".into(),
                    ))
                }
            };
            serde_json::to_string(&RemoteMutation {
                message_id: id.into(),
                account_id: account.into(),
                folder: folder.into(),
                uid,
                uidvalidity: validity,
                action,
            })
            .map_err(|_| storage::StorageError::Query("rule action serialization failed".into()))
        })
        .map_err(|_| IpcError::StorageUnavailable)?;
    for account_id in changed {
        service.events.publish(crate::SyncProgressEvent {
            account_id,
            state: crate::SyncState::ResyncRequired,
            new_messages_count: 0,
            triage_alerts: vec![],
            embedding_errors_count: 0,
            diagnostic: None,
        });
    }
    Ok(())
}

impl DesktopService {
    pub async fn list_rules(self: &Arc<Self>) -> Result<Vec<Rule>, IpcError> {
        self.blocking(|s| s.store.list_rules().map_err(rule_error))
            .await
    }
    pub async fn save_rule(self: &Arc<Self>, rule: Rule) -> Result<Rule, IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.save_rule(rule).map_err(rule_error))
            .await
    }
    pub async fn delete_rule(self: &Arc<Self>, id: String, revision: u32) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.delete_rule(&id, revision).map_err(rule_error))
            .await
    }
    pub async fn reorder_rules(
        self: &Arc<Self>,
        revisions: Vec<(String, u32)>,
    ) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(move |s| s.store.reorder_rules(&revisions).map_err(rule_error))
            .await
    }
    pub async fn simulate_rules(
        self: &Arc<Self>,
        ids: Vec<String>,
        draft: Option<Rule>,
    ) -> Result<Vec<(String, RulePlan)>, IpcError> {
        self.blocking(move |s| s.store.simulate_rules(&ids, draft).map_err(rule_error))
            .await
    }
    pub async fn rule_draft(
        self: &Arc<Self>,
        message_id: String,
        rule_id: String,
    ) -> Result<Rule, IpcError> {
        self.blocking(move |s| {
            s.store
                .rule_draft_from_message(&message_id, rule_id)
                .map_err(rule_error)
        })
        .await
    }
    pub async fn process_rules(self: &Arc<Self>) -> Result<(), IpcError> {
        let _guard = self.mutation_gate.lock().await;
        self.blocking(|s| process_jobs(&s)).await
    }
}

/// Observer adapter stays outside core crate boundaries and reuses the durable queue.
pub(crate) struct RuleObserver<'a>(pub &'a DesktopService);
impl triage_ai::TriageEventHook for RuleObserver<'_> {
    fn on_message_triaged(
        &self,
        _: &str,
        _: &triage_ai::DeterministicTriageResult,
    ) -> Result<(), triage_ai::TriageError> {
        process_jobs(self.0)
            .map_err(|_| triage_ai::TriageError::Hook("rule processing deferred".into()))
    }
}
