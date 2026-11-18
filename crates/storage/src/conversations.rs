//! Local header navigation. No fetching, authentication, deduplication or automation.
pub(crate) mod actions;
mod graph;
mod search;
use crate::{MessageRowSummary, SqliteMailStore, StorageError};
pub use actions::{ConversationAction, ConversationPreview, MutationStatus};
use graph::Input;
use rusqlite::{params, Connection, OptionalExtension};
pub use search::{ConversationPage, ConversationQuery, ConversationRow, ConversationSource};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("conversation operation failed".into())
}
fn stale() -> StorageError {
    StorageError::InvalidSearchQuery(
        "conversation changed or index is initializing; refresh".into(),
    )
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationProgress {
    pub indexed: usize,
    pub total: usize,
    pub ready: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMember {
    pub message: MessageRowSummary,
    pub matching: bool,
    pub pending: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMembers {
    pub generation: String,
    pub rows: Vec<ConversationMember>,
    pub total: usize,
}
fn epoch(tx: &Connection) -> Result<String, StorageError> {
    let stored: String = tx
        .query_row("SELECT generation FROM conversation_epoch", [], |r| {
            r.get(0)
        })
        .map_err(error)?;
    let now = crate::now_epoch_ms();
    // INVARIANT: Calendar and snooze eligibility can change without a database write.
    let snooze: i64 = tx.query_row("SELECT COALESCE(MAX(snooze_until_epoch_ms),0) FROM messages WHERE snooze_until_epoch_ms<=?1", [now], |r| r.get(0)).map_err(error)?;
    let digest = Sha256::digest(
        serde_json::to_vec(&(stored, now.div_euclid(86_400_000), snooze)).map_err(error)?,
    );
    Ok(format!("{:x}", digest)[..32].to_string())
}
pub(crate) fn check_generation(tx: &Connection, expected: &str) -> Result<(), StorageError> {
    if expected.len() != 32 || epoch(tx)? != expected {
        return Err(stale());
    }
    Ok(())
}
fn ready_for(tx: &Connection, accounts: &[String]) -> Result<(), StorageError> {
    let pending: bool = if accounts.is_empty() {
        tx.query_row("SELECT EXISTS(SELECT 1 FROM conversation_dirty)", [], |r| {
            r.get(0)
        })
        .map_err(error)?
    } else {
        let json = serde_json::to_string(accounts).map_err(error)?;
        tx.query_row("SELECT EXISTS(SELECT 1 FROM conversation_dirty WHERE account_id IN (SELECT value FROM json_each(?1)))",[json],|r|r.get(0)).map_err(error)?
    };
    if pending {
        return Err(stale());
    }
    Ok(())
}
impl SqliteMailStore {
    pub fn conversation_progress(&self) -> Result<ConversationProgress, StorageError> {
        self.conversation_progress_for(None)
    }
    pub fn conversation_progress_for(
        &self,
        account: Option<&str>,
    ) -> Result<ConversationProgress, StorageError> {
        self.transaction(|tx| {
            let total = tx
                .query_row(
                    "SELECT count(*) FROM messages WHERE ?1 IS NULL OR account_id=?1",
                    [account],
                    |r| r.get(0),
                )
                .map_err(error)?;
            let indexed = tx
                .query_row(
                    "SELECT count(*) FROM conversation_inputs WHERE ?1 IS NULL OR account_id=?1",
                    [account],
                    |r| r.get(0),
                )
                .map_err(error)?;
            Ok(ConversationProgress {
                indexed,
                total,
                ready: ready_for(
                    tx,
                    &account.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                )
                .is_ok(),
            })
        })
    }
    /// Stage at most 300 headers per tick; final component consolidation is linear in account size.
    pub fn backfill_conversations(&self) -> Result<usize, StorageError> {
        self.write_transaction(
            |tx| {
                let rows = tx.prepare("SELECT m.id,m.account_id,m.message_id_header,m.in_reply_to,m.references_header FROM messages m LEFT JOIN conversation_inputs i ON i.message_id=m.id WHERE i.message_id IS NULL ORDER BY m.id LIMIT 300").map_err(error)?
                    .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?))).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;
                for (id,account,header,parent,refs) in &rows {
                    let header = header.as_deref().and_then(graph::header_id);
                    let parent = parent.as_deref().and_then(graph::header_id);
                    let refs = serde_json::to_string(&graph::references(refs.as_deref().unwrap_or(""))).map_err(error)?;
                    tx.execute("INSERT INTO conversation_inputs VALUES (?1,?2,?3,?4,?5)", params![id,account,header,parent,refs]).map_err(error)?;
                }
                let account: Option<String> = tx.query_row("SELECT d.account_id FROM conversation_dirty d WHERE NOT EXISTS(SELECT 1 FROM messages m LEFT JOIN conversation_inputs i ON i.message_id=m.id WHERE m.account_id=d.account_id AND i.message_id IS NULL) ORDER BY d.account_id LIMIT 1",[],|r|r.get(0)).optional().map_err(error)?;
                if let Some(account) = account { rebuild(tx, &account)?; }
                Ok(rows.len())
            },
            |_, _, _| Ok(()),
        )
    }
}
fn rebuild(tx: &Connection, account: &str) -> Result<(), StorageError> {
    let raw = tx.prepare("SELECT message_id,header_id,parent_id,refs FROM conversation_inputs WHERE account_id=?1 ORDER BY message_id").map_err(error)?
        .query_map([account],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?))).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;
    let inputs = raw
        .into_iter()
        .map(|(id, header, parent, refs)| {
            Ok(Input {
                id,
                header,
                parent,
                refs: serde_json::from_str(&refs).map_err(error)?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    tx.execute(
        "DELETE FROM conversation_members WHERE account_id=?1",
        [account],
    )
    .map_err(error)?;
    let mut stmt = tx
        .prepare("INSERT INTO conversation_members VALUES (?1,?2,?3)")
        .map_err(error)?;
    for (id, root) in graph::group(&inputs) {
        let name = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(account, root)).map_err(error)?)
        );
        stmt.execute(params![id, account, name]).map_err(error)?;
    }
    tx.execute(
        "DELETE FROM conversation_dirty WHERE account_id=?1",
        [account],
    )
    .map_err(error)?;
    tx.execute(
        "UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)))",
        [],
    )
    .map_err(error)?;
    Ok(())
}
#[cfg(test)]
mod tests;
