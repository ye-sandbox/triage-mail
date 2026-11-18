//! Versioned encrypted virtual folders. Searching never changes canonical mail identity.
use crate::{
    scoped_search::ScopedSearch, HybridQuery, MessageRowSummary, SearchMode, SqliteMailStore,
    StorageError,
};
use rusqlite::{functions::FunctionFlags, params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use triage_ai::{
    rules::{sender_address, valid_date},
    TriageBadge,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ViewFilters {
    #[serde(default)]
    pub category: Option<triage_ai::TriageCategory>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub has_attachments: Option<bool>,
    pub tags: Vec<String>,
    pub sender: Option<String>,
    pub sender_domain: Option<String>,
    pub extraction: Option<ExtractionKind>,
    pub due_from: Option<String>,
    pub due_to: Option<String>,
    pub upcoming_days: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionKind {
    Boleto,
    Invoice,
    Pix,
    Tracking,
    Otp,
    Unsubscribe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedView {
    pub version: u32,
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub query: String,
    pub mode: SearchMode,
    // INVARIANT: Empty means all accounts; a nonempty selection never broadens on removal.
    pub accounts: Vec<String>,
    pub scope: Option<crate::MailboxScope>,
    pub filters: ViewFilters,
}

fn invalid() -> StorageError {
    StorageError::InvalidSearchQuery("invalid or stale saved view".into())
}
fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("saved view operation failed".into())
}
fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

impl SavedView {
    pub fn validate(&self) -> bool {
        let f = &self.filters;
        self.version == 1
            && bounded(&self.id, 100)
            && bounded(&self.name, 120)
            && self.query.len() <= 4096
            && self.accounts.len() <= 100
            && self.accounts.iter().all(|a| bounded(a, 100))
            && self
                .accounts
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                == self.accounts.len()
            && f.tags.len() <= 32
            && f.tags.iter().all(|v| bounded(v, 64))
            && f.sender
                .as_ref()
                .is_none_or(|s| bounded(s, 320) && sender_address(s).as_ref() == Some(s))
            && f.sender_domain.as_ref().is_none_or(|s| {
                bounded(s, 253)
                    && s.is_ascii()
                    && s.split('.').all(|p| {
                        !p.is_empty() && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    })
            })
            && [&f.due_from, &f.due_to]
                .iter()
                .all(|v| v.as_ref().is_none_or(|s| valid_date(s)))
            && f.due_from
                .as_ref()
                .zip(f.due_to.as_ref())
                .is_none_or(|(a, b)| a <= b)
            && f.upcoming_days.is_none_or(|d| d <= 366)
    }
}

pub(crate) fn validate_accounts(conn: &Connection, view: &SavedView) -> Result<(), StorageError> {
    for account in &view.accounts {
        if !conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
                [account],
                |r| r.get::<_, bool>(0),
            )
            .map_err(error)?
        {
            return Err(invalid());
        }
    }
    if let Some(crate::MailboxScope::AccountView {
        account_id,
        view: mailbox,
    }) = &view.scope
    {
        crate::scoped_search::validate_scope(conn, account_id, mailbox)?;
        if !view.accounts.is_empty() && !view.accounts.contains(account_id) {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(crate) fn read(conn: &Connection, id: &str, revision: u32) -> Result<SavedView, StorageError> {
    let json: String = conn
        .query_row(
            "SELECT definition_json FROM saved_views WHERE id=?1 AND revision=?2",
            params![id, revision],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?
        .ok_or_else(invalid)?;
    let view: SavedView = serde_json::from_str(&json).map_err(error)?;
    if !view.validate() {
        return Err(invalid());
    }
    validate_accounts(conn, &view)?;
    Ok(view)
}

impl SqliteMailStore {
    pub fn list_saved_views(&self) -> Result<Vec<SavedView>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx.prepare("SELECT definition_json FROM saved_views ORDER BY json_extract(definition_json,'$.name') COLLATE NOCASE,id").map_err(error)?;
            let jsons = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(error)?;
            // INVARIANT: Listing remains possible when referenced accounts have been removed.
            jsons.iter().map(|s| serde_json::from_str(s).map_err(error)).collect()
        })
    }

    pub fn save_saved_view(&self, mut view: SavedView) -> Result<SavedView, StorageError> {
        if !view.validate() {
            return Err(invalid());
        }
        self.write_transaction(
            |tx| {
                validate_accounts(tx, &view)?;
                let deleted: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM saved_view_deleted_ids WHERE id=?1)",
                        [&view.id],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                let revision: Option<u32> = tx
                    .query_row(
                        "SELECT revision FROM saved_views WHERE id=?1",
                        [&view.id],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(error)?;
                let count: u32 = tx
                    .query_row("SELECT count(*) FROM saved_views", [], |r| r.get(0))
                    .map_err(error)?;
                if deleted
                    || revision.unwrap_or(0) != view.revision
                    || (revision.is_none() && count >= 200)
                {
                    return Err(invalid());
                }
                view.revision = view.revision.checked_add(1).ok_or_else(invalid)?;
                let json = serde_json::to_string(&view).map_err(error)?;
                tx.execute(
                    "INSERT INTO saved_views VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,definition_json=excluded.definition_json",
                    params![view.id, view.revision, json],
                )
                .map_err(error)?;
                Ok(view)
            },
            |_, _, _| Ok(()),
        )
    }

    pub fn delete_saved_view(&self, id: &str, revision: u32) -> Result<(), StorageError> {
        self.write_transaction(
            |tx| {
                if tx
                    .execute(
                        "DELETE FROM saved_views WHERE id=?1 AND revision=?2",
                        params![id, revision],
                    )
                    .map_err(error)?
                    != 1
                {
                    return Err(invalid());
                }
                tx.execute("INSERT INTO saved_view_deleted_ids VALUES (?1)", [id])
                    .map_err(error)?;
                Ok(())
            },
            |_, _, _| Ok(()),
        )
    }

    /// Resolve the persisted text/mode before embedding; execution rechecks its revision.
    pub fn saved_view(&self, id: &str, revision: u32) -> Result<SavedView, StorageError> {
        self.transaction(|tx| read(tx, id, revision))
    }

    pub fn search_saved_view(
        &self,
        id: &str,
        revision: u32,
        embedding: Option<Vec<i8>>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        self.search_saved_view_at(
            id,
            revision,
            embedding,
            (limit, offset),
            crate::now_epoch_ms(),
        )
    }

    pub(crate) fn search_saved_view_at(
        &self,
        id: &str,
        revision: u32,
        embedding: Option<Vec<i8>>,
        page: (usize, usize),
        now: i64,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        let (limit, offset) = page;
        self.transaction(|tx| {
            let view = read(tx, id, revision)?;
            let ram_guard = if view.mode != SearchMode::Lexical && embedding.is_some() {
                self.fresh_ram_guard(tx)?
            } else {
                None
            };
            let ram_index = ram_guard.as_deref().and_then(|state| match state {
                crate::ram_vector_index::RamVectorState::Ready(ready) => Some(&ready.index),
                _ => None,
            });
            let res = execute_view(tx, &view, embedding, (limit, offset), now, ram_index);
            drop(ram_guard);
            res
        })
    }
}

fn can_use_pure_ram_filter(view: &SavedView) -> bool {
    let f = &view.filters;
    f.is_read.is_none()
        && f.is_starred.is_none()
        && f.has_attachments.is_none()
        && f.tags.is_empty()
        && f.sender.is_none()
        && f.sender_domain.is_none()
        && f.extraction.is_none()
        && f.due_from.is_none()
        && f.due_to.is_none()
        && f.upcoming_days.is_none()
        && (view.accounts.len() <= 1
            || matches!(view.scope, Some(crate::MailboxScope::AccountView { .. })))
}

pub(crate) fn execute_view(
    tx: &Connection,
    view: &SavedView,
    embedding: Option<Vec<i8>>,
    page: (usize, usize),
    now: i64,
    ram_index: Option<&crate::RamVectorIndex>,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    HybridQuery {
        query_text: view.query.clone(),
        mode: view.mode,
        query_embedding_int8: embedding.clone(),
        account_filter: None,
        limit: page.0,
        offset: page.1,
    }
    .validate()?;
    execute_view_all(tx, view, embedding, page, now, ram_index)
}

// INVARIANT: Only native grouped search supplies a full-result limit; IPC remains bounded.
pub(crate) fn execute_view_all(
    tx: &Connection,
    view: &SavedView,
    embedding: Option<Vec<i8>>,
    page: (usize, usize),
    now: i64,
    ram_index: Option<&crate::RamVectorIndex>,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    crate::search::load_summaries(
        tx,
        &execute_view_ids(tx, view, embedding, page, now, ram_index)?,
    )
}

pub(crate) fn execute_view_ids(
    tx: &Connection,
    view: &SavedView,
    embedding: Option<Vec<i8>>,
    page: (usize, usize),
    now: i64,
    ram_index: Option<&crate::RamVectorIndex>,
) -> Result<Vec<String>, StorageError> {
    validate_accounts(tx, view)?;
    let (limit, offset) = page;
    let query = HybridQuery {
        query_text: view.query.clone(),
        mode: view.mode,
        query_embedding_int8: embedding,
        account_filter: None,
        limit,
        offset,
    };
    let mut value = serde_json::to_value(view).map_err(error)?;
    let mut eligibility = ELIGIBILITY.to_string();
    if view.filters.category.is_some() {
        eligibility.push_str(&format!(
            " AND {}=json_extract(?2,'$.filters.category')",
            crate::search::CATEGORY_EFFECTIVE
        ));
    }
    let mut resolved_folder_owned = String::new();
    let mut scope_account_owned: Option<&str> = None;
    let mut scope_mailbox_ref: Option<&crate::MailboxView> = None;
    if let Some(crate::MailboxScope::AccountView {
        account_id,
        view: mailbox,
    }) = &view.scope
    {
        resolved_folder_owned = crate::scoped_search::validate_scope(tx, account_id, mailbox)?;
        scope_account_owned = Some(account_id.as_str());
        scope_mailbox_ref = Some(mailbox);
        value["resolved_folder"] = resolved_folder_owned.clone().into();
        eligibility
            .push_str(" AND m.account_id=json_extract(?2,'$.scope.AccountView.account_id') AND (");
        eligibility.push_str(
            &crate::scoped_search::eligibility(mailbox)
                .replace("?2", "json_extract(?2,'$.resolved_folder')"),
        );
        eligibility.push(')');
    }
    let ram_filter = ram_index.and_then(|_| {
        if !can_use_pure_ram_filter(view) {
            return None;
        }
        let account_id = scope_account_owned.or_else(|| view.accounts.first().map(String::as_str));
        let scope = scope_mailbox_ref.map(|mailbox| {
            // INVARIANT: SavedView/Category scopes intentionally do not apply INBOX_VISIBLE.
            crate::ram_vector_index::scope_to_ram_filter(
                mailbox,
                &resolved_folder_owned,
                now,
                false,
            )
        });
        Some(crate::ram_vector_index::RamQueryFilter {
            account_id,
            scope,
            category: view.filters.category,
            review_state: None,
        })
    });
    let json = serde_json::to_string(&value).map_err(error)?;
    let filters = view.filters.clone();
    // INVARIANT: The connection mutex encloses registration, all branches and hydration.
    // Sender parsing and typed extraction checks precede ranking, never loaded-page filtering.
    tx.create_scalar_function(
        "saved_facts_match",
        3,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        move |ctx| {
            let sender = ctx.get::<String>(0)?;
            let badges = ctx.get::<String>(1)?;
            let today = ctx.get::<String>(2)?;
            Ok(facts_match(&filters, &sender, &badges, &today))
        },
    )
    .map_err(error)?;
    let search = ScopedSearch {
        conn: tx,
        account_id: "",
        folder: &json,
        eligibility: &eligibility,
        messages: "messages m",
        now,
        ram_index,
        ram_filter,
    };
    let result = search.execute(&query);
    // INVARIANT: Private filter snapshots do not outlive this search in a SQL closure.
    tx.remove_function("saved_facts_match", 3).map_err(error)?;
    result
}

const ELIGIBILITY: &str = "
 (json_array_length(?2,'$.accounts')=0 OR m.account_id IN (SELECT value FROM json_each(?2,'$.accounts')))
 AND (json_extract(?2,'$.filters.is_read') IS NULL OR m.is_read=json_extract(?2,'$.filters.is_read'))
 AND (json_extract(?2,'$.filters.is_starred') IS NULL OR m.is_starred=json_extract(?2,'$.filters.is_starred'))
 AND (json_extract(?2,'$.filters.has_attachments') IS NULL OR m.has_attachments=json_extract(?2,'$.filters.has_attachments'))
 AND NOT EXISTS (SELECT 1 FROM json_each(?2,'$.filters.tags') wanted WHERE NOT EXISTS (SELECT 1 FROM tags t WHERE t.message_id=m.id AND t.tag_name=wanted.value))
 AND saved_facts_match(m.sender,m.triage_badges,date(?3/1000,'unixepoch'))";

fn kind_matches(kind: ExtractionKind, badge: &TriageBadge) -> bool {
    matches!(
        (kind, badge),
        (ExtractionKind::Boleto, TriageBadge::FebrabanBoleto { .. })
            | (ExtractionKind::Invoice, TriageBadge::NfeInvoice { .. })
            | (ExtractionKind::Pix, TriageBadge::PixPayload { .. })
            | (ExtractionKind::Tracking, TriageBadge::TrackingCode { .. })
            | (ExtractionKind::Otp, TriageBadge::OtpCode { .. })
            | (
                ExtractionKind::Unsubscribe,
                TriageBadge::UnsubscribeAvailable { .. }
            )
    )
}

fn facts_match(f: &ViewFilters, sender: &str, badges: &str, today: &str) -> bool {
    if f.sender.is_some() || f.sender_domain.is_some() {
        let Some(address) = sender_address(sender) else {
            return false;
        };
        if f.sender.as_ref().is_some_and(|s| s != &address)
            || f.sender_domain.as_ref().is_some_and(|s| {
                address
                    .rsplit_once('@')
                    .is_none_or(|(_, d)| !d.eq_ignore_ascii_case(s))
            })
        {
            return false;
        }
    }
    if f.extraction.is_none()
        && f.due_from.is_none()
        && f.due_to.is_none()
        && f.upcoming_days.is_none()
    {
        return true;
    }
    let Ok(badges) = serde_json::from_str::<Vec<TriageBadge>>(badges) else {
        return false;
    };
    if f.extraction
        .is_some_and(|kind| !badges.iter().any(|b| kind_matches(kind, b)))
    {
        return false;
    }
    if f.due_from.is_none() && f.due_to.is_none() && f.upcoming_days.is_none() {
        return true;
    }
    // INVARIANT: All date bounds refer to the same valid extracted boleto date.
    badges.iter().any(|b| match b {
        TriageBadge::FebrabanBoleto {
            due_date_iso: Some(date),
            ..
        } if valid_date(date) => {
            f.due_from.as_ref().is_none_or(|v| date >= v)
                && f.due_to.as_ref().is_none_or(|v| date <= v)
                && f.upcoming_days.is_none_or(|days| {
                    day_number(date)
                        .zip(day_number(today))
                        .is_some_and(|(d, t)| d >= t && d <= t + i64::from(days))
                })
        }
        _ => false,
    })
}

// Gregorian civil-date ordinal; no local timezone or DST enters saved date windows.
fn day_number(date: &str) -> Option<i64> {
    if !valid_date(date) {
        return None;
    }
    let mut parts = date.split('-').map(str::parse::<i64>);
    let (mut y, m, d) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    y -= i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1)
}

#[cfg(test)]
mod tests;
