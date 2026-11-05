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
    Star(bool),
    Snooze(Option<i64>),
}

impl SqliteMailStore {
    pub(crate) fn transaction<T>(
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
            // INVARIANT: Reconnection may update a cached-only recovered account,
            // but cannot replace an existing connection or another account identity.
            let cached: Option<String> = tx.query_row(
                "SELECT id FROM accounts WHERE id=?1 AND email=?2 AND NOT EXISTS(SELECT 1 FROM account_connections WHERE account_id=?1)",
                params![account.id, account.email], |r| r.get(0)).optional().map_err(query_error)?;
            if cached.is_some() {
                tx.execute("UPDATE accounts SET auth_type=?1,updated_at_epoch_ms=?2 WHERE id=?3", params![account.auth_type, account.updated_at_epoch_ms, account.id]).map_err(query_error)?;
            } else {
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
            }
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
        self.apply_local_mutations_guarded(changes, None)
    }

    /// INVARIANT: A confirmed conversation snapshot is checked inside the mutation transaction.
    pub fn apply_local_mutations_guarded(
        &self,
        changes: &[(String, LocalMutation<'_>, Option<String>)],
        generation: Option<&str>,
    ) -> Result<(), StorageError> {
        self.apply_mutations_snapshot(changes, generation, None)
    }

    pub fn apply_preview_mutations(
        &self,
        changes: &[(String, LocalMutation<'_>, Option<String>)],
        generation: &str,
        token: &str,
    ) -> Result<(), StorageError> {
        self.apply_mutations_snapshot(changes, Some(generation), Some(token))
    }

    fn apply_mutations_snapshot(
        &self,
        changes: &[(String, LocalMutation<'_>, Option<String>)],
        generation: Option<&str>,
        token: Option<&str>,
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            if let Some(generation) = generation { crate::conversations::check_generation(tx, generation)?; }
            if let (Some(token),Some(generation))=(token,generation) {
                crate::conversations::actions::consume_preview(tx,token,generation,changes)?;
            }
            for (id, action, remote_payload) in changes {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM messages WHERE id=?1)",
                        [id],
                        |r| r.get(0),
                    )
                    .map_err(query_error)?;
                if !exists {
                    return Err(StorageError::Query("message not found".into()));
                }
                Self::apply_mutation_in_transaction(tx, id, action)?;
                if matches!(action, LocalMutation::Read(true)) {
                    tx.execute("UPDATE reminder_jobs SET status='Canceled' WHERE message_id=?1 AND trigger_type='UnreadInterval' AND status='Pending'", [id]).map_err(query_error)?;
                }
                if let Some(payload) = remote_payload {
                    tx.execute("DELETE FROM conversation_attempts WHERE message_id=?1",[id]).map_err(query_error)?;
                    tx.execute(
                        "INSERT INTO pending_mutations(message_id,payload_json) VALUES (?1,?2)",
                        params![id, payload],
                    )
                    .map_err(query_error)?;
                }
            }
            Ok(())
        })
    }

    pub(crate) fn apply_mutation_in_transaction(
        tx: &Transaction<'_>,
        id: &str,
        action: &LocalMutation<'_>,
    ) -> Result<(), StorageError> {
        match action {
            LocalMutation::Read(value) => {
                tx.execute(
                    "UPDATE messages SET is_read=?1 WHERE id=?2",
                    params![value, id],
                )
                .map_err(query_error)?;
            }
            // INVARIANT: Optimistic moves never replace the canonical server folder/UID.
            LocalMutation::Folder(folder) => {
                tx.execute("INSERT INTO message_folder_overrides VALUES (?1, ?2) ON CONFLICT(message_id) DO UPDATE SET folder=excluded.folder", params![id, folder]).map_err(query_error)?;
                // Moving to Archive or Trash immediately clears any active snooze deadline.
                tx.execute(
                    "UPDATE messages SET snooze_until_epoch_ms=NULL WHERE id=?1",
                    params![id],
                )
                .map_err(query_error)?;
            }
            LocalMutation::Tag(tag) => {
                tx.execute("INSERT INTO tags VALUES (?1, ?2, ?3, 0) ON CONFLICT(message_id, tag_name) DO NOTHING", params![format!("{id}:tag:{tag}"), id, tag]).map_err(query_error)?;
            }
            LocalMutation::Star(value) => {
                tx.execute(
                    "UPDATE messages SET is_starred=?1 WHERE id=?2",
                    params![if *value { 1 } else { 0 }, id],
                )
                .map_err(query_error)?;
            }
            LocalMutation::Snooze(deadline) => {
                tx.execute(
                    "UPDATE messages SET snooze_until_epoch_ms=?1 WHERE id=?2",
                    params![deadline, id],
                )
                .map_err(query_error)?;
            }
        }
        Ok(())
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
        flags: &[crate::MessageFlagsDelta],
        state: &crate::FolderSyncState,
        reset: bool,
    ) -> Result<Vec<crate::MessageRecord>, StorageError> {
        // INVARIANT: A batch cannot introduce another account's canonical identity.
        if state.uidvalidity == 0
            || messages.iter().any(|m| {
                m.account_id != state.account_id || m.folder != state.folder_name || m.uid == 0
            })
        {
            return Err(StorageError::Query("invalid sync batch identity".into()));
        }
        self.transaction(|tx| {
            if reset {
                // 1. Remove memberships for this folder
                tx.execute(
                    "DELETE FROM message_folder_memberships WHERE account_id=?1 AND folder=?2",
                    params![state.account_id, state.folder_name],
                )
                .map_err(query_error)?;

                // 2. Remove orphan messages (no remaining memberships in any folder)
                tx.execute(
                    "DELETE FROM vec_messages WHERE message_id IN (
                        SELECT id FROM messages WHERE account_id=?1 AND folder=?2
                        AND NOT EXISTS (SELECT 1 FROM message_folder_memberships m WHERE m.message_id = messages.id)
                    )",
                    params![state.account_id, state.folder_name],
                )
                .map_err(query_error)?;

                tx.execute(
                    "DELETE FROM messages WHERE account_id=?1 AND folder=?2
                     AND NOT EXISTS (SELECT 1 FROM message_folder_memberships m WHERE m.message_id = messages.id)",
                    params![state.account_id, state.folder_name],
                )
                .map_err(query_error)?;
            }
            let mut persisted = Vec::with_capacity(messages.len());
            for message in messages {
                let message = message.clone();

                // INVARIANT: Same-account deduplication (ADR-010).
                // Check provider_message_id -> message_id_header + timestamp -> folder + uid.
                // Deduplication NEVER occurs across distinct accounts.
                let mut existing_id: Option<String> = None;
                if let Some(ref pmid) = message.provider_message_id {
                    existing_id = tx
                        .query_row(
                            "SELECT id FROM messages WHERE account_id=?1 AND provider_message_id=?2 LIMIT 1",
                            params![message.account_id, pmid],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(query_error)?;
                }
                if existing_id.is_none() {
                    if let Some(ref mid) = message.message_id_header {
                        existing_id = tx
                            .query_row(
                                "SELECT id FROM messages WHERE account_id=?1 AND message_id_header=?2 AND received_at_epoch_ms=?3 LIMIT 1",
                                params![message.account_id, mid, message.received_at_epoch_ms],
                                |r| r.get(0),
                            )
                            .optional()
                            .map_err(query_error)?;
                    }
                }
                if existing_id.is_none() {
                    existing_id = tx
                        .query_row(
                            "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                            params![message.account_id, message.folder, message.uid],
                            |r| r.get(0),
                        )
                        .optional()
                        .map_err(query_error)?;
                }

                let now_ms = crate::models::now_epoch_ms();

                if let Some(id) = existing_id {
                    // Message already exists in this account: register folder membership without duplicating content
                    tx.execute(
                        "INSERT INTO message_folder_memberships (account_id, folder, uid, message_id, created_at_epoch_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(account_id, folder, uid) DO NOTHING",
                        params![message.account_id, message.folder, message.uid, id, now_ms],
                    )
                    .map_err(query_error)?;

                    crate::sender_review::observe(tx, &id, false)?;
                    let mut deduped_message = message;
                    deduped_message.id = id;
                    persisted.push(deduped_message);
                } else {
                    // Brand new message
                    Self::insert_message_in_transaction(tx, &message)?;
                    tx.execute(
                        "INSERT INTO message_folder_memberships (account_id, folder, uid, message_id, created_at_epoch_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(account_id, folder, uid) DO NOTHING",
                        params![message.account_id, message.folder, message.uid, message.id, now_ms],
                    )
                    .map_err(query_error)?;
                    crate::sender_review::arrival(tx, &message.id, state)?;
                    crate::rules::enqueue(tx, &message, state)?;
                    persisted.push(message);
                }
            }
            for flag in flags {
                tx.execute(
                    "UPDATE messages SET
                        is_read = CASE
                            WHEN EXISTS (
                                SELECT 1 FROM pending_mutations p
                                WHERE (p.message_id = messages.id OR p.message_id IN (
                                    SELECT message_id FROM message_folder_memberships m
                                    WHERE m.account_id=?2 AND m.folder=?3 AND m.uid=?4
                                )) AND p.payload_json LIKE '%Read%'
                            ) THEN messages.is_read
                            ELSE ?1
                        END,
                        is_starred = CASE
                            WHEN EXISTS (
                                SELECT 1 FROM pending_mutations p
                                WHERE (p.message_id = messages.id OR p.message_id IN (
                                    SELECT message_id FROM message_folder_memberships m
                                    WHERE m.account_id=?2 AND m.folder=?3 AND m.uid=?4
                                )) AND (p.payload_json LIKE '%Star%' OR p.payload_json LIKE '%Unstar%')
                            ) THEN messages.is_starred
                            ELSE ?5
                        END
                     WHERE (account_id=?2 AND folder=?3 AND uid=?4)
                        OR id IN (
                            SELECT message_id FROM message_folder_memberships
                            WHERE account_id=?2 AND folder=?3 AND uid=?4
                        )",
                    params![
                        if flag.is_read { 1 } else { 0 },
                        state.account_id,
                        state.folder_name,
                        flag.uid,
                        if flag.is_starred { 1 } else { 0 },
                    ],
                )
                .map_err(query_error)?;
            }
            tx.execute("INSERT INTO folder_sync_state VALUES (?1,?2,?3,?4,?5,0) ON CONFLICT(account_id,folder_name) DO UPDATE SET uidvalidity=excluded.uidvalidity,uidnext=excluded.uidnext,highest_modseq=excluded.highest_modseq", params![state.account_id,state.folder_name,state.uidvalidity,state.uidnext,state.highest_modseq]).map_err(query_error)?;
            Ok(persisted)
        })
    }

    /// Reconciles expired snooze deadlines by clearing them back to NULL.
    /// Returns the message IDs that expired so decoupled workers can trigger view refresh.
    pub fn reconcile_expired_snoozes(&self, now_ms: i64) -> Result<Vec<String>, StorageError> {
        self.transaction(|tx| {
            let mut stmt = tx
                .prepare(
                    "SELECT id FROM messages WHERE snooze_until_epoch_ms IS NOT NULL AND snooze_until_epoch_ms <= ?1",
                )
                .map_err(query_error)?;
            let ids: Vec<String> = stmt
                .query_map(params![now_ms], |r| r.get(0))
                .map_err(query_error)?
                .collect::<Result<_, _>>()
                .map_err(query_error)?;
            if !ids.is_empty() {
                tx.execute(
                    "UPDATE messages SET snooze_until_epoch_ms = NULL WHERE snooze_until_epoch_ms IS NOT NULL AND snooze_until_epoch_ms <= ?1",
                    params![now_ms],
                )
                .map_err(query_error)?;
            }
            Ok(ids)
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

    pub fn save_folders(
        &self,
        account_id: &str,
        folders: &[crate::FolderRecord],
    ) -> Result<(), StorageError> {
        self.transaction(|tx| {
            for folder in folders {
                tx.execute(
                    "INSERT INTO folders (
                        account_id, folder_name, display_name, delimiter, role,
                        is_selectable, total_messages, unread_messages, updated_at_epoch_ms
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                     ON CONFLICT(account_id, folder_name) DO UPDATE SET
                        display_name = excluded.display_name,
                        delimiter = excluded.delimiter,
                        role = excluded.role,
                        is_selectable = excluded.is_selectable,
                        total_messages = COALESCE(excluded.total_messages, folders.total_messages),
                        unread_messages = COALESCE(excluded.unread_messages, folders.unread_messages),
                        updated_at_epoch_ms = excluded.updated_at_epoch_ms",
                    params![
                        account_id,
                        folder.folder_name,
                        folder.display_name,
                        folder.delimiter,
                        folder.role,
                        if folder.is_selectable { 1 } else { 0 },
                        folder.total_messages,
                        folder.unread_messages,
                        folder.updated_at_epoch_ms,
                    ],
                )
                .map_err(query_error)?;
            }
            Ok(())
        })
    }

    pub fn list_folders(&self, account_id: &str) -> Result<Vec<crate::FolderRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT account_id, folder_name, display_name, delimiter, role, is_selectable,
                        total_messages, unread_messages, updated_at_epoch_ms
                 FROM folders WHERE account_id = ?1 ORDER BY role = 'inbox' DESC, display_name ASC",
            )
            .map_err(query_error)?;
        let rows = stmt
            .query_map([account_id], |r| {
                Ok(crate::FolderRecord {
                    account_id: r.get(0)?,
                    folder_name: r.get(1)?,
                    display_name: r.get(2)?,
                    delimiter: r.get(3)?,
                    role: r.get(4)?,
                    is_selectable: r.get::<_, i64>(5)? != 0,
                    total_messages: r.get(6)?,
                    unread_messages: r.get(7)?,
                    updated_at_epoch_ms: r.get(8)?,
                })
            })
            .map_err(query_error)?;
        rows.collect::<Result<_, _>>().map_err(query_error)
    }

    pub fn get_message_folders(&self, message_id: &str) -> Result<Vec<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT folder FROM message_folder_memberships WHERE message_id = ?1 ORDER BY folder ASC",
            )
            .map_err(query_error)?;
        let rows = stmt
            .query_map([message_id], |r| r.get(0))
            .map_err(query_error)?;
        rows.collect::<Result<_, _>>().map_err(query_error)
    }

    pub fn find_membership(
        &self,
        account_id: &str,
        folder: &str,
        uid: u32,
    ) -> Result<Option<String>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        conn.query_row(
            "SELECT message_id FROM message_folder_memberships WHERE account_id = ?1 AND folder = ?2 AND uid = ?3",
            params![account_id, folder, uid],
            |r| r.get(0),
        )
        .optional()
        .map_err(query_error)
    }
}

impl SqliteMailStore {
    /// Cached-only identities have no endpoint and cannot start a network worker.
    pub fn disconnected_accounts(&self) -> Result<Vec<(String, String)>, StorageError> {
        self.transaction(|tx| {
            let mut statement = tx.prepare("SELECT id,email FROM accounts WHERE NOT EXISTS(SELECT 1 FROM account_connections WHERE account_id=accounts.id) ORDER BY id").map_err(query_error)?;
            let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).map_err(query_error)?.collect::<Result<_, _>>().map_err(query_error)?;
            Ok(rows)
        })
    }
}
