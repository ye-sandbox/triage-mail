use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    HybridQuery, MessageRowSummary, SearchEngine, SearchMode, SqliteMailStore, StorageError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailboxScope {
    GlobalAllMail,
    AccountView {
        account_id: String,
        view: MailboxView,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailboxView {
    Inbox,
    Starred,
    Snoozed,
    Folder(String),
}

impl SqliteMailStore {
    /// Searches a mailbox without changing the existing headless search trait.
    pub fn search_scoped(
        &self,
        query: &HybridQuery,
        scope: &MailboxScope,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        self.search_scoped_at(query, scope, crate::now_epoch_ms())
    }

    pub(crate) fn search_scoped_at(
        &self,
        query: &HybridQuery,
        scope: &MailboxScope,
        now: i64,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        let MailboxScope::AccountView { account_id, view } = scope else {
            return self.search(query);
        };
        query.validate()?;
        if query
            .account_filter
            .as_ref()
            .is_some_and(|id| id != account_id)
        {
            return Err(invalid_scope());
        }
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        // INVARIANT: Scope validation, both ranking branches, and hydration share one snapshot.
        let tx = conn.transaction().map_err(query_error)?;
        let folder = validate_scope(&tx, account_id, view)?;
        let search = ScopedSearch {
            conn: &tx,
            account_id,
            folder: &folder,
            eligibility: eligibility(view),
            now,
        };
        let ids = search.execute(query)?;
        let rows = crate::search::load_summaries(&tx, &ids)?;
        tx.commit().map_err(query_error)?;
        Ok(rows)
    }
}

fn eligibility(view: &MailboxView) -> &'static str {
    match view {
        MailboxView::Starred => "m.is_starred = 1",
        MailboxView::Snoozed => "m.snooze_until_epoch_ms > ?3",
        MailboxView::Inbox => {
            "(m.snooze_until_epoch_ms IS NULL OR m.snooze_until_epoch_ms <= ?3) AND EXISTS (
            SELECT 1 FROM message_folder_memberships membership
            LEFT JOIN message_folder_overrides o ON o.message_id=m.id
            WHERE membership.message_id=m.id AND membership.account_id=m.account_id
            AND COALESCE(o.folder, membership.folder)=?2)"
        }
        MailboxView::Folder(_) => {
            "EXISTS (
            SELECT 1 FROM message_folder_memberships membership
            LEFT JOIN message_folder_overrides o ON o.message_id=m.id
            WHERE membership.message_id=m.id AND membership.account_id=m.account_id
            AND COALESCE(o.folder, membership.folder)=?2)"
        }
    }
}

fn validate_scope(
    conn: &Connection,
    account: &str,
    view: &MailboxView,
) -> Result<String, StorageError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
            [account],
            |row| row.get(0),
        )
        .map_err(query_error)?;
    if !exists {
        return Err(invalid_scope());
    }
    let sql = match view {
        MailboxView::Inbox => "SELECT folder_name FROM folders WHERE account_id=?1 AND is_selectable=1
            AND (role='inbox' OR upper(folder_name)='INBOX') ORDER BY role='inbox' DESC, folder_name LIMIT 1",
        MailboxView::Folder(_) => "SELECT folder_name FROM folders WHERE account_id=?1 AND is_selectable=1 AND folder_name=?2",
        _ => return Ok(String::new()),
    };
    let mut stmt = conn.prepare_cached(sql).map_err(query_error)?;
    let result = if let MailboxView::Folder(folder) = view {
        stmt.query_row(params![account, folder], |row| row.get(0))
    } else {
        stmt.query_row([account], |row| row.get(0))
    };
    result
        .optional()
        .map_err(query_error)?
        .ok_or_else(invalid_scope)
}

struct ScopedSearch<'connection> {
    conn: &'connection Connection,
    account_id: &'connection str,
    folder: &'connection str,
    eligibility: &'static str,
    now: i64,
}

impl ScopedSearch<'_> {
    fn execute(&self, query: &HybridQuery) -> Result<Vec<String>, StorageError> {
        let lexical = !query.query_text.trim().is_empty();
        let semantic = query.query_embedding_int8.is_some();
        if query.mode != SearchMode::Hybrid || !lexical || !semantic {
            return self.branch(
                query,
                query.mode == SearchMode::Semantic
                    || (query.mode == SearchMode::Hybrid && semantic),
            );
        }
        let candidates = HybridQuery {
            limit: (query.limit + query.offset).max(50),
            offset: 0,
            ..query.clone()
        };
        let mut scores: HashMap<String, f64> = HashMap::new();
        for ids in [
            self.branch(&candidates, false)?,
            self.branch(&candidates, true)?,
        ] {
            for (rank, id) in ids.into_iter().enumerate() {
                *scores.entry(id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
            }
        }
        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|first, second| {
            second
                .1
                .total_cmp(&first.1)
                .then_with(|| first.0.cmp(&second.0))
        });
        Ok(ranked
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|(id, _)| id)
            .collect())
    }

    fn branch(&self, query: &HybridQuery, semantic: bool) -> Result<Vec<String>, StorageError> {
        let selection = if semantic {
            // INVARIANT: Rank every eligible vector before LIMIT; a global top-k can omit mailbox results.
            "SELECT m.id FROM messages m CROSS JOIN vec_messages v
             WHERE m.account_id=?1 AND v.message_id=m.id AND ({eligibility})
             ORDER BY vec_distance_cosine(v.embedding_int8, vec_int8(?5)), m.id LIMIT ?6 OFFSET ?7"
        } else if query.query_text.trim().is_empty() {
            "SELECT m.id FROM messages m WHERE m.account_id=?1 AND ({eligibility})
             ORDER BY m.received_at_epoch_ms DESC, m.id LIMIT ?6 OFFSET ?7"
        } else {
            "WITH ranked AS (SELECT rowid AS rid, rank AS score FROM messages_fts
             WHERE messages_fts MATCH ?4 AND +rowid IN (
                SELECT m.rowid FROM messages m WHERE m.account_id=?1 AND ({eligibility}))
             ORDER BY rank, rowid LIMIT ?6 OFFSET ?7)
             SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid ORDER BY r.score, r.rid"
        };
        let sql = selection.replace("{eligibility}", self.eligibility);
        let expression = format!("\"{}\"", query.query_text.trim().replace('"', "\"\""));
        let bytes = query
            .query_embedding_int8
            .as_deref()
            .map(crate::search::int8_slice_to_bytes);
        let mut stmt = self.conn.prepare_cached(&sql).map_err(query_error)?;
        let rows = stmt
            .query_map(
                params![
                    self.account_id,
                    self.folder,
                    self.now,
                    expression,
                    bytes,
                    query.limit as i64,
                    query.offset as i64
                ],
                |row| row.get(0),
            )
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    }
}

fn invalid_scope() -> StorageError {
    StorageError::InvalidSearchQuery("unknown or unavailable mailbox scope".into())
}

fn query_error(error: rusqlite::Error) -> StorageError {
    StorageError::Query(error.to_string())
}
