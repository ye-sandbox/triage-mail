use super::*;
use crate::scoped_search::ScopedSearch;
use crate::{HybridQuery, MailboxScope, MailboxView};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ConversationSource {
    #[default]
    Messages,
    Category(triage_ai::TriageCategory),
    Review(crate::sender_review::ReviewState),
    Saved {
        id: String,
        revision: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationQuery {
    pub source: ConversationSource,
    pub scope: Option<MailboxScope>,
    pub generation: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRow {
    pub conversation_id: String,
    pub representative: MessageRowSummary,
    pub matching_count: usize,
    pub total_count: usize,
    pub unread_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationPage {
    pub generation: String,
    pub rows: Vec<ConversationRow>,
}
fn ranked(
    tx: &Connection,
    query: &HybridQuery,
    context: &ConversationQuery,
    ram_index: Option<&crate::RamVectorIndex>,
) -> Result<Vec<String>, StorageError> {
    let count: usize = tx
        .query_row("SELECT count(*) FROM messages", [], |r| r.get(0))
        .map_err(error)?;
    let all = HybridQuery {
        limit: count.max(1),
        offset: 0,
        ..query.clone()
    };
    let now = crate::now_epoch_ms();
    match &context.source {
        ConversationSource::Saved { id, revision } => {
            let view = crate::saved_views::read(tx, id, *revision)?;
            if view.query != query.query_text || view.mode != query.mode {
                return Err(stale());
            }
            crate::saved_views::execute_view_ids(
                tx,
                &view,
                all.query_embedding_int8,
                (all.limit, 0),
                now,
                ram_index,
            )
        }
        ConversationSource::Category(category) => {
            let view = crate::saved_views::SavedView {
                version: 1,
                id: "conversation".into(),
                revision: 0,
                name: "conversation".into(),
                query: all.query_text,
                mode: all.mode,
                accounts: all.account_filter.into_iter().collect(),
                scope: context.scope.clone(),
                filters: crate::saved_views::ViewFilters {
                    category: Some(*category),
                    ..Default::default()
                },
            };
            crate::saved_views::execute_view_ids(
                tx,
                &view,
                all.query_embedding_int8,
                (all.limit, 0),
                now,
                ram_index,
            )
        }
        _ => {
            let mut account = query.account_filter.as_deref().unwrap_or("");
            let mut folder = String::new();
            let mut ram_scope = None;
            let mut ram_review = None;
            let predicate = if let ConversationSource::Review(state) = context.source {
                folder = match state {
                    crate::sender_review::ReviewState::Pending => "Pending".into(),
                    crate::sender_review::ReviewState::Declined => "Declined".into(),
                    _ => return Err(stale()),
                };
                if let Some(MailboxScope::AccountView { account_id, .. }) = &context.scope {
                    account = account_id;
                }
                ram_review = Some(state);
                "EXISTS(SELECT 1 FROM sender_review_messages r JOIN sender_review_choices c ON c.account_id=r.account_id AND c.sender=r.sender JOIN sender_review_preferences p ON p.account_id=r.account_id WHERE r.message_id=m.id AND p.enabled=1 AND p.initializing=0 AND c.state=?2)".to_string()
            } else if let Some(MailboxScope::AccountView { account_id, view }) = &context.scope {
                account = account_id;
                folder = crate::scoped_search::validate_scope(tx, account, view)?;
                let mut predicate = crate::scoped_search::eligibility(view).to_string();
                if matches!(view, MailboxView::Inbox) {
                    predicate.push_str(&format!(" AND ({})", crate::sender_review::INBOX_VISIBLE));
                }
                ram_scope = Some((view, matches!(view, MailboxView::Inbox)));
                predicate
            } else {
                "1".into()
            };
            let ram_filter = ram_index.map(|_| {
                let opt_account = (!account.is_empty()).then_some(account);
                let scope = ram_scope.map(|(view, hide)| {
                    crate::ram_vector_index::scope_to_ram_filter(view, &folder, now, hide)
                });
                crate::ram_vector_index::RamQueryFilter {
                    account_id: opt_account,
                    scope,
                    category: None,
                    review_state: ram_review,
                }
            });
            let ids = ScopedSearch {
                conn: tx,
                account_id: account,
                folder: &folder,
                eligibility: &predicate,
                messages: "messages m",
                now,
                ram_index,
                ram_filter,
            }
            .execute(&all)?;
            Ok(ids)
        }
    }
}
fn memberships(tx: &Connection) -> Result<HashMap<String, String>, StorageError> {
    tx.prepare("SELECT message_id,conversation_id FROM conversation_members")
        .map_err(error)?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(error)?
        .collect::<Result<_, _>>()
        .map_err(error)
}
fn validate(
    tx: &Connection,
    query: &HybridQuery,
    context: &ConversationQuery,
) -> Result<String, StorageError> {
    query.validate()?;
    if let Some(MailboxScope::AccountView { account_id, .. }) = &context.scope {
        if query
            .account_filter
            .as_ref()
            .is_some_and(|account| account != account_id)
        {
            return Err(stale());
        }
    }
    let mut accounts: Vec<String> = match &context.source {
        ConversationSource::Saved { id, revision } => {
            let view = crate::saved_views::read(tx, id, *revision)?;
            if let Some(MailboxScope::AccountView { account_id, .. }) = view.scope {
                vec![account_id]
            } else {
                view.accounts
            }
        }
        _ => match &context.scope {
            Some(MailboxScope::AccountView { account_id, .. }) => vec![account_id.clone()],
            _ => query.account_filter.iter().cloned().collect(),
        },
    };
    accounts.sort();
    accounts.dedup();
    for account in &accounts {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
                [account],
                |r| r.get(0),
            )
            .map_err(error)?;
        if !exists {
            return Err(stale());
        }
    }
    ready_for(tx, &accounts)?;
    if let Some(generation) = &context.generation {
        check_generation(tx, generation)?;
    }
    if query.offset > 0 && context.generation.is_none() {
        return Err(stale());
    }
    epoch(tx)
}
impl SqliteMailStore {
    pub fn search_conversations(
        &self,
        query: &HybridQuery,
        context: &ConversationQuery,
    ) -> Result<ConversationPage, StorageError> {
        self.transaction(|tx| {
            let generation = validate(tx, query, context)?;
            let ram_guard = if query.mode != crate::SearchMode::Lexical
                && query.query_embedding_int8.is_some()
            {
                self.fresh_ram_guard(tx)?
            } else {
                None
            };
            let ram_index = ram_guard.as_deref().and_then(|st| match st {
                crate::ram_vector_index::RamVectorState::Ready(ready) => Some(&ready.index),
                _ => None,
            });
            let ranked = ranked(tx, query, context, ram_index)?;
            drop(ram_guard);
            let members = memberships(tx)?;
            let mut counts = HashMap::<String, (usize, usize)>::new();
            let totals: Vec<(String, bool)> = tx
                .prepare("SELECT c.conversation_id,m.is_read FROM conversation_members c JOIN messages m ON m.id=c.message_id")
                .map_err(error)?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(error)?
                .collect::<Result<_, _>>()
                .map_err(error)?;
            for (id, read) in totals {
                let v = counts.entry(id).or_default();
                v.0 += 1;
                v.1 += usize::from(!read);
            }
            let mut groups = Vec::<(String, String, usize)>::new();
            let mut positions = HashMap::<String, usize>::new();
            for message_id in ranked {
                let id = members.get(&message_id).ok_or_else(stale)?;
                if let Some(i) = positions.get(id) {
                    groups[*i].2 += 1;
                } else {
                    positions.insert(id.clone(), groups.len());
                    groups.push((id.clone(), message_id, 1));
                }
            }
            let page: Vec<_> = groups.into_iter().skip(query.offset).take(query.limit).collect();
            let rows = crate::search::load_summaries(
                tx,
                &page.iter().map(|v| v.1.clone()).collect::<Vec<_>>(),
            )?;
            Ok(ConversationPage {
                generation,
                rows: page
                    .into_iter()
                    .zip(rows)
                    .map(|((conversation_id, _, matching_count), representative)| {
                        let (total_count, unread_count) = counts[&conversation_id];
                        ConversationRow {
                            conversation_id,
                            representative,
                            matching_count,
                            total_count,
                            unread_count,
                        }
                    })
                    .collect(),
            })
        })
    }
    pub fn conversation_members(
        &self,
        query: &HybridQuery,
        context: &ConversationQuery,
        id: &str,
        offset: usize,
    ) -> Result<ConversationMembers, StorageError> {
        if id.len() != 64 || offset > i64::MAX as usize {
            return Err(stale());
        }
        self.transaction(|tx| {
            let generation = validate(tx, query, context)?;
            if context.generation.is_none() {
                return Err(stale());
            }
            let ram_guard = if query.mode != crate::SearchMode::Lexical
                && query.query_embedding_int8.is_some()
            {
                self.fresh_ram_guard(tx)?
            } else {
                None
            };
            let ram_index = ram_guard.as_deref().and_then(|st| match st {
                crate::ram_vector_index::RamVectorState::Ready(ready) => Some(&ready.index),
                _ => None,
            });
            let matching: HashSet<_> = ranked(tx, query, context, ram_index)?.into_iter().collect();
            drop(ram_guard);
            let ids: Vec<String> = tx
                .prepare("SELECT m.id FROM conversation_members c JOIN messages m ON m.id=c.message_id WHERE c.conversation_id=?1 ORDER BY m.received_at_epoch_ms,m.id")
                .map_err(error)?
                .query_map([id], |r| r.get(0))
                .map_err(error)?
                .collect::<Result<_, _>>()
                .map_err(error)?;
            if !ids.iter().any(|id| matching.contains(id)) {
                return Err(stale());
            }
            let total = ids.len();
            let rows = crate::search::load_summaries(
                tx,
                &ids.into_iter().skip(offset).take(100).collect::<Vec<_>>(),
            )?
            .into_iter()
            .map(|message| {
                let pending = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM pending_mutations WHERE message_id=?1)",
                        [&message.id],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                Ok(ConversationMember {
                    matching: matching.contains(&message.id),
                    message,
                    pending,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
            Ok(ConversationMembers {
                generation,
                rows,
                total,
            })
        })
    }
}
pub(super) fn matching_ids(
    tx: &Connection,
    query: &HybridQuery,
    context: &ConversationQuery,
    id: &str,
    all: bool,
) -> Result<Vec<String>, StorageError> {
    validate(tx, query, context)?;
    if context.generation.is_none() {
        return Err(stale());
    }
    let matches: HashSet<_> = ranked(tx, query, context, None)?.into_iter().collect();
    let members: Vec<String>=tx.prepare("SELECT message_id FROM conversation_members WHERE conversation_id=?1 ORDER BY message_id").map_err(error)?.query_map([id],|r|r.get(0)).map_err(error)?.collect::<Result<_,_>>().map_err(error)?;
    if !members.iter().any(|id| matches.contains(id)) {
        return Err(stale());
    }
    Ok(members
        .into_iter()
        .filter(|id| all || matches.contains(id))
        .collect())
}
