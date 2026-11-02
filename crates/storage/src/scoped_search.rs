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
            messages: messages_source(&tx, account_id, view, now)?,
            now,
        };
        let ids = search.execute(query)?;
        let rows = crate::search::load_summaries(&tx, &ids)?;
        tx.commit().map_err(query_error)?;
        Ok(rows)
    }
}

fn messages_source(
    conn: &Connection,
    account: &str,
    view: &MailboxView,
    now: i64,
) -> Result<&'static str, StorageError> {
    if !matches!(view, MailboxView::Snoozed) {
        return Ok("messages m");
    }
    // INVARIANT: Deadline range probes span all accounts. Bound index-only estimates
    // so a tiny account never scans another account's entire active-snooze population.
    let account_rows: usize = conn
        .query_row(
            "SELECT count(*) FROM (SELECT 1 FROM messages WHERE account_id=?1 LIMIT 16384)",
            [account],
            |r| r.get(0),
        )
        .map_err(query_error)?;
    if account_rows > 0 {
        let active_rows: usize = conn
            .query_row(
                "SELECT count(*) FROM (SELECT 1 FROM messages INDEXED BY idx_messages_snooze_until
             WHERE snooze_until_epoch_ms>?1 LIMIT ?2)",
                params![now, account_rows],
                |r| r.get(0),
            )
            .map_err(query_error)?;
        if active_rows < account_rows {
            return Ok("messages m INDEXED BY idx_messages_snooze_until");
        }
    }
    Ok("messages m INDEXED BY idx_messages_account_provider_msgid")
}

fn eligibility(view: &MailboxView) -> &'static str {
    match view {
        MailboxView::Starred => "m.is_starred = 1",
        MailboxView::Snoozed => "m.snooze_until_epoch_ms > ?3",
        MailboxView::Inbox => {
            "(m.snooze_until_epoch_ms IS NULL OR m.snooze_until_epoch_ms <= ?3) AND EXISTS (
            SELECT 1 FROM message_folder_memberships membership INDEXED BY idx_memberships_message_id
            LEFT JOIN message_folder_overrides o ON o.message_id=m.id
            WHERE membership.message_id=m.id AND membership.account_id=m.account_id
            AND COALESCE(o.folder, membership.folder)=?2)"
        }
        MailboxView::Folder(_) => {
            "EXISTS (
            SELECT 1 FROM message_folder_memberships membership INDEXED BY idx_memberships_message_id
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
    messages: &'static str,
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

    fn sql(&self, query: &HybridQuery, semantic: bool) -> String {
        let selection = if semantic {
            // INVARIANT: Rank every eligible vector before LIMIT; a global top-k can omit mailbox results.
            "SELECT m.id FROM {messages} CROSS JOIN vec_messages v
             WHERE m.account_id=?1 AND v.message_id=m.id AND ({eligibility})
             ORDER BY vec_distance_cosine(v.embedding_int8, vec_int8(?5)), m.id LIMIT ?6 OFFSET ?7"
        } else if query.query_text.trim().is_empty() {
            "SELECT m.id FROM {messages} WHERE m.account_id=?1 AND ({eligibility})
             ORDER BY m.received_at_epoch_ms DESC, m.id LIMIT ?6 OFFSET ?7"
        } else {
            "WITH ranked AS (SELECT rowid AS rid, rank AS score FROM messages_fts
             WHERE messages_fts MATCH ?4 AND +rowid IN (
                SELECT m.rowid FROM {messages} WHERE m.account_id=?1 AND ({eligibility}))
             ORDER BY rank, rowid LIMIT ?6 OFFSET ?7)
             SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid ORDER BY r.score, r.rid"
        };
        // INVARIANT: Correlated membership probes must use message identity, not scan an
        // entire account per candidate. Snoozed uses bounded metadata estimates to
        // choose an account scan or active-deadline range without changing eligibility.
        selection
            .replace("{eligibility}", self.eligibility)
            .replace("{messages}", self.messages)
    }

    fn branch(&self, query: &HybridQuery, semantic: bool) -> Result<Vec<String>, StorageError> {
        let sql = self.sql(query, semantic);
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

#[cfg(test)]
mod plan_tests {
    use super::*;
    use crate::{connection::open_in_memory_connection, migrations::SchemaMigrator, DatabaseKey};

    fn connection() -> Connection {
        let mut conn = open_in_memory_connection(&DatabaseKey::from_bytes([47; 32])).unwrap();
        SchemaMigrator::new(Default::default())
            .migrate(&mut conn)
            .unwrap();
        conn
    }

    fn query(text: &str) -> HybridQuery {
        HybridQuery {
            query_text: text.into(),
            query_embedding_int8: Some(vec![1; 384]),
            account_filter: None,
            mode: SearchMode::Lexical,
            limit: 200,
            offset: 0,
        }
    }

    fn search<'a>(conn: &'a Connection, view: &MailboxView) -> ScopedSearch<'a> {
        ScopedSearch {
            conn,
            account_id: "a",
            folder: "INBOX",
            eligibility: eligibility(view),
            messages: messages_source(conn, "a", view, 1000).unwrap(),
            now: 1000,
        }
    }

    #[test]
    fn every_folder_branch_probes_memberships_by_message_identity() {
        let conn = connection();
        for view in [MailboxView::Inbox, MailboxView::Folder("INBOX".into())] {
            for (text, semantic) in [("", false), ("synthetic", false), ("", true)] {
                let sql = format!(
                    "EXPLAIN QUERY PLAN {}",
                    search(&conn, &view).sql(&query(text), semantic)
                );
                let mut stmt = conn.prepare(&sql).unwrap();
                let details = stmt
                    .query_map(
                        params!["a", "INBOX", 1000, "synthetic", vec![1_u8; 384], 200, 0],
                        |r| r.get::<_, String>(3),
                    )
                    .unwrap()
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .unwrap();
                assert!(
                    details
                        .iter()
                        .any(|line| line.contains("idx_memberships_message_id (message_id=?)")),
                    "{details:?}"
                );
                assert!(
                    !details
                        .iter()
                        .any(|line| line.contains("membership") && line.contains("(account_id=?)")),
                    "{details:?}"
                );
            }
        }
    }

    #[test]
    fn snoozed_branches_enumerate_deadlines_when_the_active_range_is_smaller() {
        let conn = connection();
        conn.execute_batch(
            "INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0);
            WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<1000)
            INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
                received_at_epoch_ms,raw_headers,created_at_epoch_ms,snooze_until_epoch_ms)
            SELECT printf('message-%05d',i),'a','INBOX',i,'synthetic','sender@example.org',
                'reader@example.com',i,'',0,CASE WHEN i=1 THEN 2000 ELSE NULL END FROM n;",
        )
        .unwrap();
        for (text, semantic) in [("", false), ("synthetic", false), ("", true)] {
            let sql = format!(
                "EXPLAIN QUERY PLAN {}",
                search(&conn, &MailboxView::Snoozed).sql(&query(text), semantic)
            );
            let mut stmt = conn.prepare(&sql).unwrap();
            let details = stmt
                .query_map(
                    params!["a", "", 1000, "synthetic", vec![1_u8; 384], 200, 0],
                    |r| r.get::<_, String>(3),
                )
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            assert!(
                details.iter().any(
                    |line| line.contains("idx_messages_snooze_until (snooze_until_epoch_ms>?)")
                ),
                "{details:?}"
            );
        }
    }

    #[test]
    fn dense_other_account_snoozes_do_not_force_a_tiny_account_range_scan() {
        let conn = connection();
        conn.execute_batch(
            "INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0),
            ('b','b@example.org','synthetic',0,0);
            WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<5031)
            INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
                received_at_epoch_ms,raw_headers,created_at_epoch_ms,snooze_until_epoch_ms)
            SELECT printf('message-%05d',i),CASE WHEN i<=31 THEN 'a' ELSE 'b' END,'INBOX',i,
                'synthetic','sender@example.org','reader@example.com',i,'',0,
                CASE WHEN i>31 THEN 2000 ELSE NULL END FROM n;",
        )
        .unwrap();
        let scoped = search(&conn, &MailboxView::Snoozed);
        assert_eq!(
            scoped.messages,
            "messages m INDEXED BY idx_messages_account_provider_msgid"
        );
        let mut stmt = conn.prepare(&scoped.sql(&query(""), false)).unwrap();
        let rows = stmt
            .query_map(params!["a", "", 1000, "", vec![1_u8; 384], 200, 0], |r| {
                r.get::<_, String>(0)
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(rows.is_empty());
        assert!(stmt.get_status(rusqlite::StatementStatus::VmStep) < 31 * 50);
        assert_eq!(
            messages_source(&conn, "missing", &MailboxView::Snoozed, 1000).unwrap(),
            "messages m INDEXED BY idx_messages_account_provider_msgid"
        );
    }

    #[test]
    fn blank_inbox_vm_work_stays_linear_with_duplicate_label_memberships() {
        let conn = connection();
        conn.execute_batch("INSERT INTO accounts VALUES ('a','a@example.com','synthetic',0,0);
            WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<5000)
            INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
                received_at_epoch_ms,raw_headers,created_at_epoch_ms)
            SELECT printf('message-%05d',i),'a','INBOX',i,'synthetic','sender@example.org',
                'reader@example.com',i,'',0 FROM n;
            INSERT INTO message_folder_memberships SELECT 'a','INBOX',uid,id,0 FROM messages;
            INSERT INTO message_folder_memberships SELECT 'a','Archive',uid+5000,id,0 FROM messages;").unwrap();
        let sql = search(&conn, &MailboxView::Inbox).sql(&query(""), false);
        let mut stmt = conn.prepare(&sql).unwrap();
        let ids = stmt
            .query_map(
                params!["a", "INBOX", 1000, "", vec![1_u8; 384], 200, 0],
                |r| r.get::<_, String>(0),
            )
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(
            ids,
            (4801..=5000)
                .rev()
                .map(|id| format!("message-{id:05}"))
                .collect::<Vec<_>>()
        );
        // INVARIANT: Count VM work, not hardware-dependent milliseconds. An account scan
        // per candidate grows quadratically and exceeds this generous linear-work budget.
        let steps = stmt.get_status(rusqlite::StatementStatus::VmStep);
        assert!(
            steps < 5000 * 200,
            "unexpected membership amplification: {steps} VM steps"
        );
    }
}
