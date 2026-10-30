//! Additive persistence operations for desktop orchestration; no GUI dependencies.
use rusqlite::{params, OptionalExtension, Transaction};

use crate::{AccountRecord, SqliteMailStore, StorageError};

fn query_error(e: rusqlite::Error) -> StorageError {
    StorageError::Query(e.to_string())
}

/// A local mutation committed atomically with an optional remote outbox entry.
pub enum LocalMutation<'a> {
    Read(bool),
    Folder(&'a str),
    Tag(&'a str),
}

impl SqliteMailStore {
    fn transaction<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let tx = conn.transaction().map_err(query_error)?;
        let result = operation(&tx)?;
        tx.commit().map_err(query_error)?;
        Ok(result)
    }

    /// Persists safe account metadata and endpoint configuration together.
    pub fn insert_account_connection(
        &self,
        account: &AccountRecord,
        config_json: &str,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute(
                "INSERT INTO accounts VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    account.id,
                    account.email,
                    account.auth_type,
                    account.created_at_epoch_ms,
                    account.updated_at_epoch_ms
                ],
            )
            .map_err(query_error)?;
            tx.execute(
                "INSERT INTO account_connections VALUES (?1, ?2)",
                params![account.id, config_json],
            )
            .map_err(query_error)?;
            Ok(())
        })
    }

    pub fn list_account_connections(&self) -> Result<Vec<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let mut stmt = conn
            .prepare("SELECT config_json FROM account_connections ORDER BY account_id")
            .map_err(query_error)?;
        let rows = stmt.query_map([], |r| r.get(0)).map_err(query_error)?;
        rows.collect::<Result<_, _>>().map_err(query_error)
    }

    pub fn get_account_connection(&self, account_id: &str) -> Result<Option<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        conn.query_row(
            "SELECT config_json FROM account_connections WHERE account_id=?1",
            [account_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(query_error)
    }

    pub fn find_message_id(
        &self,
        account: &str,
        folder: &str,
        uid: u32,
    ) -> Result<Option<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        conn.query_row(
            "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
            params![account, folder, uid],
            |r| r.get(0),
        )
        .optional()
        .map_err(query_error)
    }

    /// Fails atomically if any message is missing. Remote entries retain original IMAP identity.
    pub fn apply_local_mutations(
        &self,
        changes: &[(String, LocalMutation<'_>, Option<String>)],
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            for (id, action, remote_payload) in changes {
                let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)", [id], |r| r.get(0)).map_err(query_error)?;
                if !exists { return Err(StorageError::Query("message not found".into())); }
                match action {
                    LocalMutation::Read(value) => { tx.execute("UPDATE messages SET is_read=?1 WHERE id=?2", params![value, id]).map_err(query_error)?; }
                    // INVARIANT: Optimistic moves never replace the canonical server folder/UID.
                    LocalMutation::Folder(folder) => { tx.execute("INSERT INTO message_folder_overrides VALUES (?1, ?2) ON CONFLICT(message_id) DO UPDATE SET folder=excluded.folder", params![id, folder]).map_err(query_error)?; }
                    LocalMutation::Tag(tag) => { tx.execute("INSERT INTO tags VALUES (?1, ?2, ?3, 0) ON CONFLICT(message_id, tag_name) DO NOTHING", params![format!("{id}:tag:{tag}"), id, tag]).map_err(query_error)?; }
                }
                if let Some(payload) = remote_payload {
                    tx.execute("INSERT INTO pending_mutations(message_id,payload_json) VALUES (?1,?2)", params![id,payload]).map_err(query_error)?;
                }
            }
            Ok(())
        })
    }

    pub fn folder_override(&self, id: &str) -> Result<Option<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        conn.query_row(
            "SELECT folder FROM message_folder_overrides WHERE message_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(query_error)
    }

    pub fn pending_mutations(&self) -> Result<Vec<(i64, String)>, StorageError> {
        self.pending_mutations_after(0)
    }

    pub fn pending_mutations_after(&self, after: i64) -> Result<Vec<(i64, String)>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id,payload_json FROM pending_mutations WHERE id>?1 ORDER BY id LIMIT 100",
            )
            .map_err(query_error)?;
        let rows = stmt
            .query_map([after], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(query_error)?;
        rows.collect::<Result<_, _>>().map_err(query_error)
    }

    pub fn complete_mutation(&self, id: i64) -> Result<(), StorageError> {
        self.transaction(|tx| {
            tx.execute("DELETE FROM pending_mutations WHERE id=?1", [id])
                .map_err(query_error)?;
            Ok(())
        })
    }

    /// Commits a MOVE mapping, checkpoint and outbox acknowledgement together.
    pub fn complete_remote_move(
        &self,
        outbox_id: i64,
        message_id: &str,
        state: &crate::FolderSyncState,
        uid: Option<u32>,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            let previous:Option<u32>=tx.query_row(
                "SELECT uidvalidity FROM folder_sync_state WHERE account_id=?1 AND folder_name=?2",
                params![state.account_id,state.folder_name],|r|r.get(0),
            ).optional().map_err(query_error)?;
            if state.uidvalidity!=0 && previous.is_some_and(|v|v!=state.uidvalidity) {
                // INVARIANT: Destination UIDs from a different generation cannot survive a MOVE mapping.
                tx.execute("DELETE FROM vec_messages WHERE message_id IN (SELECT id FROM messages WHERE account_id=?1 AND folder=?2)",params![state.account_id,state.folder_name]).map_err(query_error)?;
                tx.execute("DELETE FROM messages WHERE account_id=?1 AND folder=?2",params![state.account_id,state.folder_name]).map_err(query_error)?;
                tx.execute("DELETE FROM folder_sync_state WHERE account_id=?1 AND folder_name=?2",params![state.account_id,state.folder_name]).map_err(query_error)?;
            }

            match uid {
                Some(uid) => {
                    tx.execute(
                        "UPDATE messages SET folder=?1,uid=?2 WHERE id=?3",
                        params![state.folder_name, uid, message_id],
                    )
                    .map_err(query_error)?;
                    tx.execute(
                        "DELETE FROM message_folder_overrides WHERE message_id=?1",
                        [message_id],
                    )
                    .map_err(query_error)?;
                }
                None => {
                    // No COPYUID: never pretend a destination UID equals its source UID.
                    tx.execute("DELETE FROM vec_messages WHERE message_id=?1", [message_id])
                        .map_err(query_error)?;
                    tx.execute("DELETE FROM messages WHERE id=?1", [message_id])
                        .map_err(query_error)?;
                }
            }
            tx.execute(
                "INSERT INTO folder_sync_state VALUES (?1,?2,?3,1,NULL,0)
                 ON CONFLICT(account_id,folder_name) DO NOTHING",
                params![state.account_id, state.folder_name, state.uidvalidity],
            )
            .map_err(query_error)?;
            tx.execute("DELETE FROM pending_mutations WHERE id=?1", [outbox_id])
                .map_err(query_error)?;
            Ok(())
        })
    }

    /// Atomically saves an ingestion batch, reconciles flags and advances the durable checkpoint.
    pub fn commit_sync_batch(
        &self,
        messages: &[crate::MessageRecord],
        flags: &[(u32, bool)],
        state: &crate::FolderSyncState,
        reset: bool,
    ) -> Result<Vec<crate::MessageRecord>, StorageError> {
        self.transaction(|tx| {
            if reset {
                tx.execute("DELETE FROM vec_messages WHERE message_id IN (SELECT id FROM messages WHERE account_id=?1 AND folder=?2)", params![state.account_id,state.folder_name]).map_err(query_error)?;
                tx.execute("DELETE FROM messages WHERE account_id=?1 AND folder=?2", params![state.account_id,state.folder_name]).map_err(query_error)?;
            }
            let mut persisted=Vec::with_capacity(messages.len());
            for message in messages {
                let mut message=message.clone();
                // INVARIANT: MOVE may retain a UI ID while changing its canonical folder/UID.
                let existing:Option<String>=tx.query_row(
                    "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                    params![message.account_id,message.folder,message.uid],|r|r.get(0)
                ).optional().map_err(query_error)?;
                if let Some(id)=existing {
                    message.id=id;
                    for attachment in &mut message.attachments {
                        attachment.message_id=message.id.clone();
                        attachment.id=format!("{}:{}",message.id,attachment.part_id);
                    }
                }
                Self::insert_message_in_transaction(tx, &message)?;
                persisted.push(message);
            }
            for (uid,read) in flags {
                tx.execute("UPDATE messages SET is_read=?1 WHERE account_id=?2 AND folder=?3 AND uid=?4 AND NOT EXISTS(SELECT 1 FROM pending_mutations p WHERE p.message_id=messages.id)", params![read,state.account_id,state.folder_name,uid]).map_err(query_error)?;
            }
            tx.execute("INSERT INTO folder_sync_state VALUES (?1,?2,?3,?4,?5,0) ON CONFLICT(account_id,folder_name) DO UPDATE SET uidvalidity=excluded.uidvalidity,uidnext=excluded.uidnext,highest_modseq=excluded.highest_modseq", params![state.account_id,state.folder_name,state.uidvalidity,state.uidnext,state.highest_modseq]).map_err(query_error)?;
            Ok(persisted)
        })
    }

    pub fn list_synced_folders(&self, account: &str) -> Result<Vec<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let mut stmt = conn.prepare("SELECT folder_name FROM folder_sync_state WHERE account_id=?1 ORDER BY folder_name").map_err(query_error)?;
        let rows = stmt
            .query_map([account], |r| r.get(0))
            .map_err(query_error)?;
        rows.collect::<Result<_, _>>().map_err(query_error)
    }
}
