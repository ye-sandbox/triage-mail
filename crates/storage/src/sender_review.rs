//! Account-local familiarity, never sender authentication or provider blocking.
use crate::{HybridQuery, MessageRowSummary, SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use triage_ai::rules::sender_address;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewState {
    Known,
    Pending,
    Accepted,
    Declined,
}
impl ReviewState {
    fn name(self) -> &'static str {
        match self {
            Self::Known => "Known",
            Self::Pending => "Pending",
            Self::Accepted => "Accepted",
            Self::Declined => "Declined",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewPreferences {
    pub account_id: String,
    pub revision: u32,
    pub enabled: bool,
    pub hide_in_inbox: bool,
    pub initializing: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SenderChoice {
    pub account_id: String,
    pub sender: String,
    pub revision: u32,
    pub state: ReviewState,
}
fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("sender review operation failed".into())
}
fn invalid() -> StorageError {
    StorageError::InvalidSearchQuery("invalid or stale sender review request".into())
}
fn account(conn: &Connection, id: &str) -> Result<(), StorageError> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
        .map_err(error)?;
    if exists {
        Ok(())
    } else {
        Err(invalid())
    }
}
fn preferences(conn: &Connection, id: &str) -> Result<ReviewPreferences, StorageError> {
    account(conn, id)?;
    Ok(conn.query_row("SELECT revision,enabled,hide_in_inbox,initializing FROM sender_review_preferences WHERE account_id=?1",[id],|r| Ok(ReviewPreferences {account_id:id.into(),revision:r.get(0)?,enabled:r.get(1)?,hide_in_inbox:r.get(2)?,initializing:r.get(3)?})).optional().map_err(error)?.unwrap_or(ReviewPreferences {account_id:id.into(),revision:0,enabled:false,hide_in_inbox:false,initializing:false}))
}
fn choice(
    conn: &Connection,
    account: &str,
    sender: &str,
) -> Result<Option<SenderChoice>, StorageError> {
    let row: Option<(u32, String)> = conn
        .query_row(
            "SELECT revision,state FROM sender_review_choices WHERE account_id=?1 AND sender=?2",
            params![account, sender],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(error)?;
    row.map(|(revision, state)| {
        Ok(SenderChoice {
            account_id: account.into(),
            sender: sender.into(),
            revision,
            state: serde_json::from_value(serde_json::Value::String(state)).map_err(error)?,
        })
    })
    .transpose()
}
/// INVARIANT: Observation inserts only; history cannot override any prior decision or review.
pub(crate) fn observe(conn: &Connection, id: &str, eligible: bool) -> Result<(), StorageError> {
    let (account, raw): (String, String) = conn
        .query_row(
            "SELECT account_id,sender FROM messages WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(error)?;
    let sender = if raw.len() <= 4096 {
        sender_address(&raw).filter(|address| address.len() <= 320)
    } else {
        None
    };
    conn.execute("INSERT INTO sender_review_messages VALUES (?1,?2,?3) ON CONFLICT(message_id) DO UPDATE SET sender=excluded.sender",params![id,account,sender]).map_err(error)?;
    if let Some(sender) = sender {
        conn.execute(
            "INSERT OR IGNORE INTO sender_review_choices VALUES (?1,?2,1,?3)",
            params![account, sender, if eligible { "Pending" } else { "Known" }],
        )
        .map_err(error)?;
    }
    Ok(())
}
pub(crate) fn arrival(
    conn: &Connection,
    id: &str,
    state: &crate::FolderSyncState,
) -> Result<(), StorageError> {
    let uid: u32 = conn
        .query_row("SELECT uid FROM messages WHERE id=?1", [id], |r| r.get(0))
        .map_err(error)?;
    // INVARIANT: Eligibility uses each UID, not the checkpoint of a straddling batch.
    let eligible: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sender_review_preferences p JOIN sender_review_frontiers f ON f.account_id=p.account_id WHERE p.account_id=?1 AND p.enabled=1 AND p.initializing=0 AND f.folder=?2 AND f.uidvalidity=?3 AND f.frontier<?4)",params![state.account_id,state.folder_name,state.uidvalidity,uid],|r|r.get(0)).map_err(error)?;
    observe(conn, id, eligible)
}

impl SqliteMailStore {
    pub fn review_preferences(&self, id: &str) -> Result<ReviewPreferences, StorageError> {
        self.transaction(|tx| preferences(tx, id))
    }
    pub fn save_review_preferences(
        &self,
        value: ReviewPreferences,
    ) -> Result<ReviewPreferences, StorageError> {
        self.transaction(|tx| {
            let previous = preferences(tx,&value.account_id)?;
            if previous.revision!=value.revision { return Err(invalid()); }
            let revision = value.revision.checked_add(1).ok_or_else(invalid)?;
            let enabling = value.enabled && !previous.enabled;
            if enabling { tx.execute("DELETE FROM sender_review_frontiers WHERE account_id=?1",[&value.account_id]).map_err(error)?; }
            tx.execute("INSERT INTO sender_review_preferences VALUES (?1,?2,?3,?4,?5) ON CONFLICT(account_id) DO UPDATE SET revision=excluded.revision,enabled=excluded.enabled,hide_in_inbox=excluded.hide_in_inbox,initializing=excluded.initializing",params![value.account_id,revision,value.enabled,value.hide_in_inbox,value.enabled && (enabling || previous.initializing)]).map_err(error)?;
            preferences(tx,&value.account_id)
        })
    }
    /// Called before each sync batch; unknown UIDNEXT can later establish a usable frontier.
    pub fn establish_review_frontier(
        &self,
        account: &str,
        folder: &str,
        validity: u32,
        next: u32,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute("INSERT INTO sender_review_frontiers SELECT ?1,?2,?3,?4 WHERE EXISTS(SELECT 1 FROM sender_review_preferences WHERE account_id=?1 AND enabled=1) ON CONFLICT(account_id,folder) DO UPDATE SET uidvalidity=excluded.uidvalidity,frontier=excluded.frontier WHERE sender_review_frontiers.uidvalidity!=excluded.uidvalidity OR sender_review_frontiers.frontier=4294967295",params![account,folder,validity,if next==0 {u32::MAX} else {next-1}]).map_err(error)?;
            Ok(())
        })
    }
    pub fn backfill_sender_review(&self) -> Result<usize, StorageError> {
        self.transaction(|tx| {
            let ids = tx.prepare("SELECT m.id FROM messages m LEFT JOIN sender_review_messages r ON r.message_id=m.id WHERE r.message_id IS NULL ORDER BY m.id LIMIT 300").map_err(error)?.query_map([],|r|r.get::<_,String>(0)).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;
            for id in &ids { observe(tx,id,false)?; }
            let completed = tx.execute("UPDATE sender_review_preferences SET initializing=0 WHERE initializing=1 AND NOT EXISTS(SELECT 1 FROM messages m LEFT JOIN sender_review_messages r ON r.message_id=m.id WHERE m.account_id=sender_review_preferences.account_id AND r.message_id IS NULL)",[]).map_err(error)?;
            Ok(ids.len().max(completed))
        })
    }
    pub fn review_metadata(&self, id: &str) -> Result<Option<SenderChoice>, StorageError> {
        self.transaction(|tx| {
            let (account, sender): (String,Option<String>) = tx.query_row("SELECT m.account_id,r.sender FROM messages m LEFT JOIN sender_review_messages r ON r.message_id=m.id WHERE m.id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(error)?.ok_or_else(invalid)?;
            sender.map(|sender|choice(tx,&account,&sender)).transpose().map(Option::flatten)
        })
    }
    pub fn save_sender_choice(&self, value: SenderChoice) -> Result<SenderChoice, StorageError> {
        if value.sender.len() > 320
            || sender_address(&value.sender).as_ref() != Some(&value.sender)
            || value.state == ReviewState::Known
        {
            return Err(invalid());
        }
        self.transaction(|tx| {
            account(tx,&value.account_id)?;
            let current=choice(tx,&value.account_id,&value.sender)?.ok_or_else(invalid)?;
            if current.revision!=value.revision { return Err(invalid()); }
            let revision=value.revision.checked_add(1).ok_or_else(invalid)?;
            tx.execute("UPDATE sender_review_choices SET revision=?3,state=?4 WHERE account_id=?1 AND sender=?2",params![value.account_id,value.sender,revision,value.state.name()]).map_err(error)?;
            Ok(SenderChoice {revision,..value})
        })
    }
    pub fn list_sender_choices(
        &self,
        account_id: &str,
        offset: usize,
    ) -> Result<Vec<SenderChoice>, StorageError> {
        if i64::try_from(offset).is_err() {
            return Err(invalid());
        }
        self.transaction(|tx| {
            account(tx,account_id)?;
            let senders=tx.prepare("SELECT sender FROM sender_review_choices WHERE account_id=?1 ORDER BY sender LIMIT 200 OFFSET ?2").map_err(error)?.query_map(params![account_id,offset as i64],|r|r.get::<_,String>(0)).map_err(error)?.collect::<Result<Vec<_>,_>>().map_err(error)?;
            senders.iter().map(|sender|choice(tx,account_id,sender)?.ok_or_else(invalid)).collect()
        })
    }
    pub fn search_sender_review(
        &self,
        query: &HybridQuery,
        state: ReviewState,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        query.validate()?;
        if !matches!(state, ReviewState::Pending | ReviewState::Declined) {
            return Err(invalid());
        }
        self.transaction(|tx| {
            if let Some(account_id)=&query.account_filter { account(tx,account_id)?; }
            let predicate="EXISTS(SELECT 1 FROM sender_review_messages r JOIN sender_review_choices c ON c.account_id=r.account_id AND c.sender=r.sender JOIN sender_review_preferences p ON p.account_id=r.account_id WHERE r.message_id=m.id AND p.enabled=1 AND p.initializing=0 AND c.state=?2)";
            let ids=crate::scoped_search::ScopedSearch { conn:tx,account_id:query.account_filter.as_deref().unwrap_or(""),folder:state.name(),eligibility:predicate,messages:"messages m",now:crate::now_epoch_ms() }.execute(query)?;
            crate::search::load_summaries(tx,&ids)
        })
    }
}

// INVARIANT: Only the ordinary Inbox path applies this predicate; explicit views retain access.
pub(crate) const INBOX_VISIBLE: &str = "NOT EXISTS(SELECT 1 FROM sender_review_messages r JOIN sender_review_choices c ON c.account_id=r.account_id AND c.sender=r.sender JOIN sender_review_preferences p ON p.account_id=r.account_id WHERE r.message_id=m.id AND p.enabled=1 AND p.initializing=0 AND p.hide_in_inbox=1 AND c.state IN ('Pending','Declined'))";
#[cfg(test)]
mod tests;
