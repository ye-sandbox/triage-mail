//! Local classification and corrections. Nothing here runs rules or sends IMAP mutations.
use crate::{HybridQuery, MailboxScope, MessageRowSummary, SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use triage_ai::{
    classifier::{classify_with_reason, CategoryReason, CLASSIFIER_VERSION},
    rules::sender_address,
    TriageBadge, TriageCategory,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CategorySource {
    Automatic,
    Message,
    Sender,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryMetadata {
    pub automatic: TriageCategory,
    pub effective: TriageCategory,
    pub reason: CategoryReason,
    pub version: u32,
    pub source: CategorySource,
    pub correction_revision: u32,
    pub sender_revision: u32,
    pub sender: Option<String>,
    pub durable: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SenderOverride {
    pub account_id: String,
    pub sender: String,
    pub revision: u32,
    pub category: Option<TriageCategory>,
}
fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("category operation failed".into())
}
fn invalid() -> StorageError {
    StorageError::InvalidSearchQuery("invalid or stale category request".into())
}

fn reliable_header(value: &str) -> Option<&str> {
    // INVARIANT: MIME ingestion strips angle brackets; both stored forms share one identity.
    let value = value.trim();
    let value = value
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .unwrap_or(value);
    let (local, domain) = value.split_once('@')?;
    (value.len() <= 998
        && !local.is_empty()
        && !domain.is_empty()
        && !domain.contains('@')
        && value
            .bytes()
            .all(|b| b.is_ascii_graphic() && b != b'<' && b != b'>'))
    .then_some(value)
}

fn correction_identity(
    id: &str,
    provider: Option<&str>,
    header: Option<&str>,
    received: i64,
) -> Result<(String, bool), StorageError> {
    let (identity, durable) = if let Some(provider) = provider.filter(|v| !v.trim().is_empty()) {
        (
            serde_json::to_vec(&("provider", provider)).map_err(error)?,
            true,
        )
    } else if let Some(header) = header.and_then(reliable_header) {
        (
            serde_json::to_vec(&("header", header, received)).map_err(error)?,
            true,
        )
    } else {
        (serde_json::to_vec(&("record", id)).map_err(error)?, false)
    };
    Ok((format!("{:x}", Sha256::digest(identity)), durable))
}

pub(crate) fn refresh(conn: &Connection, id: &str) -> Result<(), StorageError> {
    let mut stmt = conn.prepare_cached("SELECT account_id,substr(subject,1,8192),substr(sender,1,4096),substr(COALESCE(text_body,''),1,65536),triage_badges,one_click_unsubscribe_supported,provider_message_id,message_id_header,received_at_epoch_ms FROM messages WHERE id=?1").map_err(error)?;
    let (account, subject, sender, body, badges, one_click, provider, header, received) = stmt
        .query_row([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })
        .map_err(error)?;
    let badges: Vec<TriageBadge> = serde_json::from_str(&badges).unwrap_or_default();
    let classification = classify_with_reason(&subject, &sender, &body, &badges, one_click);
    let (key, durable) = correction_identity(id, provider.as_deref(), header.as_deref(), received)?;
    // INVARIANT: Once assigned, identity stays stable through canonical MOVE and body hydration.
    conn.execute("INSERT INTO message_categories VALUES (?1,?2,?3,?4,?5,?6,?7,?8) ON CONFLICT(message_id) DO UPDATE SET sender_key=excluded.sender_key,automatic=excluded.automatic,reason=excluded.reason,version=excluded.version",
        params![id,account,sender_address(&sender),key,durable,classification.category.as_str(),serde_json::to_string(&classification.reason).map_err(error)?,classification.version]).map_err(error)?;
    Ok(())
}
fn ensure(conn: &Connection, id: &str) -> Result<(), StorageError> {
    if id.is_empty() || id.len() > 200 {
        return Err(invalid());
    }
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(error)?;
    if !exists {
        return Err(invalid());
    }
    let current: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM message_categories WHERE message_id=?1 AND version=?2)",
            params![id, CLASSIFIER_VERSION],
            |r| r.get(0),
        )
        .map_err(error)?;
    if !current {
        refresh(conn, id)?;
    }
    Ok(())
}
pub(crate) fn metadata(conn: &Connection, id: &str) -> Result<CategoryMetadata, StorageError> {
    conn.query_row("SELECT a.automatic,a.reason,a.version,c.category,COALESCE(c.revision,0),s.category,COALESCE(s.revision,0),a.sender_key,a.durable FROM message_categories a LEFT JOIN category_corrections c ON c.account_id=a.account_id AND c.correction_key=a.correction_key LEFT JOIN category_sender_overrides s ON s.account_id=a.account_id AND s.sender_key=a.sender_key WHERE a.message_id=?1",[id],|r| {
        let automatic: String = r.get(0)?;
        let correction: Option<String> = r.get(3)?;
        let sender: Option<String> = r.get(5)?;
        Ok((automatic,r.get::<_,String>(1)?,r.get::<_,u32>(2)?,correction,r.get::<_,u32>(4)?,sender,r.get::<_,u32>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,bool>(8)?))
    }).map_err(error).and_then(|(automatic,reason,version,correction,cr,sender,sr,address,durable)| {
        let effective = correction.as_ref().or(sender.as_ref()).unwrap_or(&automatic);
        Ok(CategoryMetadata { automatic: TriageCategory::parse(&automatic).ok_or_else(invalid)?, effective: TriageCategory::parse(effective).ok_or_else(invalid)?, reason: serde_json::from_str(&reason).map_err(error)?,version,source: if correction.is_some() {CategorySource::Message} else if sender.is_some() {CategorySource::Sender} else {CategorySource::Automatic},correction_revision:cr,sender_revision:sr,sender:address,durable })
    })
}

impl SqliteMailStore {
    /// One atomic, resumable local batch. No checkpoint can advance past an unclassified row.
    pub fn backfill_categories(&self, limit: usize) -> Result<usize, StorageError> {
        if limit == 0 || limit > 300 {
            return Err(invalid());
        }
        self.transaction(|tx| {
            let ids = tx.prepare("SELECT m.id FROM messages m LEFT JOIN message_categories a ON a.message_id=m.id WHERE a.version IS NULL OR a.version<>?1 ORDER BY m.id LIMIT ?2").map_err(error)?.query_map(params![CLASSIFIER_VERSION,limit],|r|r.get::<_,String>(0)).map_err(error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(error)?;
            for id in &ids {
                refresh(tx, id)?;
            }
            Ok(ids.len())
        })
    }
    pub fn category_metadata(&self, id: &str) -> Result<CategoryMetadata, StorageError> {
        self.transaction(|tx| {
            ensure(tx, id)?;
            metadata(tx, id)
        })
    }
    pub fn correct_category(
        &self,
        id: &str,
        revision: u32,
        category: Option<TriageCategory>,
    ) -> Result<CategoryMetadata, StorageError> {
        self.transaction(|tx| {
            ensure(tx,id)?;
            let current = metadata(tx, id)?;
            if current.correction_revision != revision {
                return Err(invalid());
            }
            let revision = revision.checked_add(1).ok_or_else(invalid)?;
            // INVARIANT: Null resets retain revisions, preventing stale save/reset ABA races.
            tx.execute("INSERT INTO category_corrections SELECT account_id,correction_key,?2,?3 FROM message_categories WHERE message_id=?1 ON CONFLICT(account_id,correction_key) DO UPDATE SET revision=excluded.revision,category=excluded.category",params![id,revision,category.map(TriageCategory::as_str)]).map_err(error)?;
            metadata(tx,id)
        })
    }
    pub fn list_category_overrides(
        &self,
        offset: usize,
    ) -> Result<Vec<SenderOverride>, StorageError> {
        if i64::try_from(offset).is_err() {
            return Err(invalid());
        }
        self.transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT account_id,sender_key,revision,category FROM category_sender_overrides
                 WHERE category IS NOT NULL ORDER BY account_id,sender_key LIMIT 200 OFFSET ?1",
                )
                .map_err(error)?;
            let rows = stmt
                .query_map([offset as i64], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, u32>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                })
                .map_err(error)?;
            rows.map(|row| {
                let (account_id, sender, revision, category) = row.map_err(error)?;
                Ok(SenderOverride {
                    account_id,
                    sender,
                    revision,
                    category: category
                        .map(|v| TriageCategory::parse(&v).ok_or_else(invalid))
                        .transpose()?,
                })
            })
            .collect()
        })
    }

    pub fn save_category_override(
        &self,
        mut value: SenderOverride,
    ) -> Result<SenderOverride, StorageError> {
        if value.sender.len() > 320 || sender_address(&value.sender).as_ref() != Some(&value.sender)
        {
            return Err(invalid());
        }
        self.transaction(|tx| {
            let account: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",[&value.account_id],|r|r.get(0)).map_err(error)?;
            let revision: Option<u32> = tx.query_row("SELECT revision FROM category_sender_overrides WHERE account_id=?1 AND sender_key=?2",params![value.account_id,value.sender],|r|r.get(0)).optional().map_err(error)?;
            if !account || revision.unwrap_or(0) != value.revision {
                return Err(invalid());
            }
            value.revision=value.revision.checked_add(1).ok_or_else(invalid)?;
            tx.execute("INSERT INTO category_sender_overrides VALUES (?1,?2,?3,?4) ON CONFLICT(account_id,sender_key) DO UPDATE SET revision=excluded.revision,category=excluded.category",params![value.account_id,value.sender,value.revision,value.category.map(TriageCategory::as_str)]).map_err(error)?;
            Ok(value)
        })
    }
    pub fn search_category(
        &self,
        query: &HybridQuery,
        scope: Option<&MailboxScope>,
        category: TriageCategory,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        query.validate()?;
        self.transaction(|tx| {
            let view = crate::saved_views::SavedView {
                version: 1,
                id: "category".into(),
                revision: 0,
                name: "category".into(),
                query: query.query_text.clone(),
                mode: query.mode,
                accounts: query.account_filter.iter().cloned().collect(),
                scope: scope.cloned(),
                filters: crate::saved_views::ViewFilters {
                    category: Some(category),
                    ..Default::default()
                },
            };
            crate::saved_views::execute_view(
                tx,
                &view,
                query.query_embedding_int8.clone(),
                (query.limit, query.offset),
                crate::now_epoch_ms(),
            )
        })
    }
}

#[cfg(test)]
mod tests;
