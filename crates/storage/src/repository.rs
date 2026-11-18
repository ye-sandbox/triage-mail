use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, AtomicU8, Ordering},
    Arc, Mutex, RwLock, RwLockReadGuard,
};

use rusqlite::{Connection, Transaction};

use crate::connection::{open_encrypted_connection, open_in_memory_connection, DatabaseKey};
use crate::migrations::SchemaMigrator;
use crate::models::{AccountRecord, AttachmentRecord, MessageRecord};
use crate::ram_vector_index::{
    build_from_conn, read_conn_versions, refresh_slot_from_conn, RamQueryFilter, RamVectorIndex,
    RamVectorState, ReadyRamIndex, RAM_VECTOR_DIM,
};
use crate::search::{
    execute_hybrid_search, execute_hybrid_search_parallel, execute_lexical_search,
    execute_semantic_search, int8_slice_to_bytes, lexical_ids, load_summaries, rrf_fuse_and_load,
};
use crate::{
    FolderSyncState, FtsTokenizerMode, HybridQuery, MailRepository, MessageRowSummary,
    MigrationRunner, SearchEngine, SearchMode, StorageError,
};

/// Encrypted SQLite repository with SQLCipher, WAL mode, FTS5, and sqlite-vec KNN search.
pub struct SqliteMailStore {
    pub(crate) conn: Mutex<Connection>,
    pub(crate) secondary_conn: Option<Mutex<Connection>>,
    pub(crate) tokenizer_mode: AtomicU8,
    pub(crate) ram_vectors: Arc<RwLock<RamVectorState>>,
    pub(crate) ram_hydrate_lock: Mutex<()>,
    pub(crate) ram_mutation_epoch: AtomicU64,
    pub(crate) ram_wipe_epoch: AtomicU64,
}

impl Drop for SqliteMailStore {
    fn drop(&mut self) {
        self.clear_ram_index();
    }
}

impl SqliteMailStore {
    /// Opens a SQLCipher encrypted mail repository on disk and runs pending migrations.
    pub fn open<P: AsRef<Path>>(
        path: P,
        key: &DatabaseKey,
        tokenizer_mode: FtsTokenizerMode,
    ) -> Result<Self, StorageError> {
        let mut conn = open_encrypted_connection(&path, key)?;
        let migrator = SchemaMigrator::new(tokenizer_mode);
        migrator.migrate(&mut conn)?;

        let secondary_conn = open_encrypted_connection(&path, key).ok().map(Mutex::new);

        Ok(Self {
            tokenizer_mode: AtomicU8::new(u8::from(
                crate::settings::read_tokenizer(&conn)? == FtsTokenizerMode::Trigram,
            )),
            conn: Mutex::new(conn),
            secondary_conn,
            ram_vectors: Arc::new(RwLock::new(RamVectorState::Uninitialized)),
            ram_hydrate_lock: Mutex::new(()),
            ram_mutation_epoch: AtomicU64::new(0),
            ram_wipe_epoch: AtomicU64::new(0),
        })
    }

    /// Opens an in-memory SQLCipher encrypted mail repository and runs pending migrations.
    pub fn open_in_memory(
        key: &DatabaseKey,
        tokenizer_mode: FtsTokenizerMode,
    ) -> Result<Self, StorageError> {
        let mut conn = open_in_memory_connection(key)?;
        let migrator = SchemaMigrator::new(tokenizer_mode);
        migrator.migrate(&mut conn)?;

        Ok(Self {
            tokenizer_mode: AtomicU8::new(u8::from(
                crate::settings::read_tokenizer(&conn)? == FtsTokenizerMode::Trigram,
            )),
            conn: Mutex::new(conn),
            secondary_conn: None,
            ram_vectors: Arc::new(RwLock::new(RamVectorState::Uninitialized)),
            ram_hydrate_lock: Mutex::new(()),
            ram_mutation_epoch: AtomicU64::new(0),
            ram_wipe_epoch: AtomicU64::new(0),
        })
    }

    /// Returns a reference to the secondary read connection mutex if available.
    pub fn secondary_connection(&self) -> Option<&Mutex<Connection>> {
        self.secondary_conn.as_ref()
    }

    /// Returns true when the zeroized RAM vector index is hydrated and ready.
    pub fn is_ram_index_ready(&self) -> bool {
        self.ram_vectors
            .read()
            .is_ok_and(|guard| matches!(*guard, RamVectorState::Ready(_)))
    }

    /// Returns the number of indexed slots in RAM if hydrated, or `None` otherwise.
    pub fn ram_index_len(&self) -> Option<usize> {
        self.ram_vectors
            .read()
            .ok()
            .and_then(|guard| match &*guard {
                RamVectorState::Ready(ready) => Some(ready.index.len()),
                _ => None,
            })
    }

    /// Returns `(vector_bytes, metadata_bytes)` allocated in RAM if hydrated.
    pub fn ram_index_allocated_bytes(&self) -> Option<(usize, usize)> {
        self.ram_vectors
            .read()
            .ok()
            .and_then(|guard| match &*guard {
                RamVectorState::Ready(ready) => Some(ready.index.allocated_bytes()),
                _ => None,
            })
    }

    /// Executes exact RAM-only ranking when hydrated (without SQLite hydration).
    pub fn ram_rank_only(
        &self,
        query: &[i8],
        filter: &crate::ram_vector_index::RamQueryFilter<'_>,
        offset: usize,
        limit: usize,
    ) -> Result<Option<Vec<String>>, StorageError> {
        let guard = self
            .ram_vectors
            .read()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        match &*guard {
            RamVectorState::Ready(ready) => ready
                .index
                .rank_filtered(query, filter, offset, limit)
                .map(Some),
            _ => Ok(None),
        }
    }

    /// Explicitly zeroes and releases the in-memory vector arena (e.g. on lock, quit, restore).
    pub fn clear_ram_index(&self) {
        self.ram_wipe_epoch.fetch_add(1, Ordering::SeqCst);
        let mut guard = self
            .ram_vectors
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = RamVectorState::Uninitialized;
    }

    /// Hydrates the zeroized in-memory `int8[384]` arena and eligibility columns from SQLite.
    ///
    /// INVARIANT: Uses `secondary_conn` when available so normal primary reads/writes do not block.
    /// Concurrent writes or clear/wipe requests abort or retry cleanly without publishing stale state.
    pub fn hydrate_ram_index(&self) -> Result<(), StorageError> {
        let _hydrate_guard = self
            .ram_hydrate_lock
            .lock()
            .map_err(|e| StorageError::Query(format!("hydrate lock poisoned: {e}")))?;

        if let Some(ref sec_mutex) = self.secondary_conn {
            for _ in 0..3 {
                let (tc0, dv0, epoch0, wipe0) = {
                    let conn = self
                        .conn
                        .lock()
                        .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
                    let (tc, dv) = read_conn_versions(&conn)?;
                    (
                        tc,
                        dv,
                        self.ram_mutation_epoch.load(Ordering::Acquire),
                        self.ram_wipe_epoch.load(Ordering::Acquire),
                    )
                };
                {
                    let mut guard = self
                        .ram_vectors
                        .write()
                        .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
                    if let RamVectorState::Ready(ref ready) = *guard {
                        if ready.synced_total_changes == tc0 && ready.synced_data_version == dv0 {
                            return Ok(());
                        }
                    }
                    *guard = RamVectorState::Hydrating;
                }
                let built = {
                    let mut sec = sec_mutex.lock().map_err(|e| {
                        StorageError::Query(format!("secondary mutex poisoned: {e}"))
                    })?;
                    let tx = sec
                        .transaction()
                        .map_err(|e| StorageError::Query(e.to_string()))?;
                    let built = build_from_conn(&tx)?;
                    drop(tx);
                    built
                };
                let conn = self
                    .conn
                    .lock()
                    .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
                let (tc1, dv1) = read_conn_versions(&conn)?;
                let epoch1 = self.ram_mutation_epoch.load(Ordering::Acquire);
                let wipe1 = self.ram_wipe_epoch.load(Ordering::Acquire);
                let mut guard = self
                    .ram_vectors
                    .write()
                    .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
                if wipe1 != wipe0 {
                    if matches!(*guard, RamVectorState::Hydrating) {
                        *guard = RamVectorState::Uninitialized;
                    }
                    return Ok(());
                }
                if tc1 == tc0 && dv1 == dv0 && epoch1 == epoch0 {
                    if matches!(*guard, RamVectorState::Hydrating) {
                        *guard = RamVectorState::Ready(Box::new(ReadyRamIndex {
                            index: built,
                            synced_total_changes: tc1,
                            synced_data_version: dv1,
                        }));
                    }
                    return Ok(());
                }
                if matches!(*guard, RamVectorState::Hydrating) {
                    *guard = RamVectorState::Uninitialized;
                }
            }
            return Ok(());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let wipe0 = self.ram_wipe_epoch.load(Ordering::Acquire);
        let (tc0, dv0) = read_conn_versions(&conn)?;
        {
            let mut guard = self
                .ram_vectors
                .write()
                .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
            if let RamVectorState::Ready(ref ready) = *guard {
                if ready.synced_total_changes == tc0 && ready.synced_data_version == dv0 {
                    return Ok(());
                }
            }
            *guard = RamVectorState::Hydrating;
        }
        let built = match build_from_conn(&conn) {
            Ok(built) => built,
            Err(err) => {
                if let Ok(mut guard) = self.ram_vectors.write() {
                    if matches!(*guard, RamVectorState::Hydrating) {
                        *guard = RamVectorState::Uninitialized;
                    }
                }
                return Err(err);
            }
        };
        let (tc1, dv1) = read_conn_versions(&conn)?;
        let wipe1 = self.ram_wipe_epoch.load(Ordering::Acquire);
        let mut guard = self
            .ram_vectors
            .write()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        if wipe1 == wipe0 && matches!(*guard, RamVectorState::Hydrating) {
            *guard = RamVectorState::Ready(Box::new(ReadyRamIndex {
                index: built,
                synced_total_changes: tc1,
                synced_data_version: dv1,
            }));
        } else if matches!(*guard, RamVectorState::Hydrating) {
            *guard = RamVectorState::Uninitialized;
        }
        Ok(())
    }

    pub(crate) fn fresh_ram_guard<'a>(
        &'a self,
        conn: &Connection,
    ) -> Result<Option<RwLockReadGuard<'a, RamVectorState>>, StorageError> {
        let guard = self
            .ram_vectors
            .read()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        let RamVectorState::Ready(ref ready) = *guard else {
            return Ok(None);
        };
        let (tc, dv) = read_conn_versions(conn)?;
        if ready.synced_total_changes == tc && ready.synced_data_version == dv {
            return Ok(Some(guard));
        }
        drop(guard);
        let mut write_guard = self
            .ram_vectors
            .write()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        if let RamVectorState::Ready(ref ready) = *write_guard {
            let (tc, dv) = read_conn_versions(conn)?;
            if ready.synced_total_changes != tc || ready.synced_data_version != dv {
                // INVARIANT: Raw SQL on self.conn or commits from another connection immediately
                // invalidate and zeroize the RAM arena rather than serving stale results.
                *write_guard = RamVectorState::Uninitialized;
            } else {
                drop(write_guard);
                let read_guard = self
                    .ram_vectors
                    .read()
                    .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
                if matches!(*read_guard, RamVectorState::Ready(_)) {
                    return Ok(Some(read_guard));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn check_and_prepare_write(&self, conn: &Connection) -> Result<bool, StorageError> {
        self.ram_mutation_epoch.fetch_add(1, Ordering::AcqRel);
        let mut guard = self
            .ram_vectors
            .write()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        if let RamVectorState::Ready(ref ready) = *guard {
            let (tc, dv) = read_conn_versions(conn)?;
            if ready.synced_total_changes == tc && ready.synced_data_version == dv {
                return Ok(true);
            }
            *guard = RamVectorState::Uninitialized;
        }
        Ok(false)
    }

    pub(crate) fn finish_sync_write(
        &self,
        conn: &Connection,
        was_in_sync: bool,
        updater: impl FnOnce(&mut RamVectorIndex, &Connection) -> Result<(), StorageError>,
    ) -> Result<(), StorageError> {
        self.ram_mutation_epoch.fetch_add(1, Ordering::Release);
        if !was_in_sync {
            return Ok(());
        }
        let mut guard = self
            .ram_vectors
            .write()
            .map_err(|_| StorageError::Query("ram index lock poisoned".into()))?;
        if let RamVectorState::Ready(ref mut ready) = *guard {
            if let Err(err) = updater(&mut ready.index, conn) {
                *guard = RamVectorState::Uninitialized;
                return Err(err);
            }
            let (tc, dv) = read_conn_versions(conn)?;
            ready.synced_total_changes = tc;
            ready.synced_data_version = dv;
        }
        Ok(())
    }

    pub(crate) fn write_transaction<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T, StorageError>,
        ram_sync: impl FnOnce(&mut RamVectorIndex, &Connection, &T) -> Result<(), StorageError>,
    ) -> Result<T, StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| StorageError::Query("storage lock unavailable".into()))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;
        let tx = conn
            .transaction()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let result = operation(&tx)?;
        tx.commit()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        self.finish_sync_write(&conn, was_in_sync, |idx, conn| ram_sync(idx, conn, &result))?;
        Ok(result)
    }

    /// Returns the active FTS5 tokenizer configuration mode.
    pub fn tokenizer_mode(&self) -> FtsTokenizerMode {
        if self.tokenizer_mode.load(Ordering::Acquire) == 1 {
            FtsTokenizerMode::Trigram
        } else {
            FtsTokenizerMode::Unicode61RemoveDiacritics
        }
    }

    /// Inserts an account record.
    pub fn insert_account(&self, account: &AccountRecord) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;

        conn.execute(
            "INSERT INTO accounts (id, email, auth_type, created_at_epoch_ms, updated_at_epoch_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                email = excluded.email,
                auth_type = excluded.auth_type,
                updated_at_epoch_ms = excluded.updated_at_epoch_ms;",
            rusqlite::params![
                account.id,
                account.email,
                account.auth_type,
                account.created_at_epoch_ms,
                account.updated_at_epoch_ms
            ],
        )
        .map_err(|e| StorageError::Query(format!("failed to insert account: {e}")))?;

        self.finish_sync_write(&conn, was_in_sync, |_, _| Ok(()))?;
        Ok(())
    }

    /// Inserts a message record along with its attachments and tags in a single transaction.
    ///
    /// INVARIANT: Inserting into `messages` automatically synchronizes `messages_fts`
    /// via the SQLite `messages_ai` external-content trigger.
    pub fn insert_message(&self, message: &MessageRecord) -> Result<(), StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;

        let tx = conn
            .transaction()
            .map_err(|e| StorageError::Query(format!("failed to start tx: {e}")))?;

        Self::insert_message_in_transaction(&tx, message)?;
        let canonical: String = tx
            .query_row(
                "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                rusqlite::params![message.account_id, message.folder, message.uid],
                |r| r.get(0),
            )
            .map_err(|_| StorageError::Query("sender identity unavailable".into()))?;
        crate::sender_review::observe(&tx, &canonical, false)?;

        tx.commit()
            .map_err(|e| StorageError::Query(format!("failed to commit message tx: {e}")))?;

        self.finish_sync_write(&conn, was_in_sync, |idx, conn| {
            if idx.contains_id(&canonical) {
                refresh_slot_from_conn(idx, conn, &canonical)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    pub(crate) fn insert_message_in_transaction(
        tx: &rusqlite::Transaction<'_>,
        message: &MessageRecord,
    ) -> Result<(), StorageError> {
        let triage_badges_json =
            serde_json::to_string(&message.triage_badges).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            "INSERT INTO messages (
                id, account_id, folder, uid, message_id_header, in_reply_to, references_header,
                subject, sender, recipients, received_at_epoch_ms, is_read, has_attachments,
                category_badge, triage_badges, raw_headers, text_body, html_body, list_unsubscribe,
                list_unsubscribe_post, one_click_unsubscribe_supported, created_at_epoch_ms,
                is_starred, snooze_until_epoch_ms, provider_message_id
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22,
                ?23, ?24, ?25
             ) ON CONFLICT(account_id, folder, uid) DO UPDATE SET
                subject = excluded.subject,
                sender = excluded.sender,
                recipients = excluded.recipients,
                received_at_epoch_ms = excluded.received_at_epoch_ms,
                is_read = CASE WHEN EXISTS (
                    SELECT 1 FROM pending_mutations p WHERE p.message_id = messages.id AND p.payload_json LIKE '%Read%'
                ) THEN messages.is_read ELSE excluded.is_read END,
                has_attachments = excluded.has_attachments,
                category_badge = excluded.category_badge,
                triage_badges = excluded.triage_badges,
                raw_headers = excluded.raw_headers,
                text_body = excluded.text_body,
                html_body = excluded.html_body,
                list_unsubscribe = excluded.list_unsubscribe,
                list_unsubscribe_post = excluded.list_unsubscribe_post,
                one_click_unsubscribe_supported = excluded.one_click_unsubscribe_supported,
                is_starred = CASE WHEN EXISTS (
                    SELECT 1 FROM pending_mutations p WHERE p.message_id = messages.id AND (p.payload_json LIKE '%Star%' OR p.payload_json LIKE '%Unstar%')
                ) THEN messages.is_starred ELSE excluded.is_starred END,
                snooze_until_epoch_ms = messages.snooze_until_epoch_ms,
                provider_message_id = excluded.provider_message_id;",
            rusqlite::params![
                message.id,
                message.account_id,
                message.folder,
                message.uid,
                message.message_id_header,
                message.in_reply_to,
                message.references_header,
                message.subject,
                message.sender,
                message.recipients,
                message.received_at_epoch_ms,
                if message.is_read { 1 } else { 0 },
                if message.has_attachments { 1 } else { 0 },
                message.category_badge,
                triage_badges_json,
                message.raw_headers,
                message.text_body,
                message.html_body,
                message.list_unsubscribe,
                message.list_unsubscribe_post,
                if message.one_click_unsubscribe_supported { 1 } else { 0 },
                message.created_at_epoch_ms,
                if message.is_starred { 1 } else { 0 },
                message.snooze_until_epoch_ms,
                message.provider_message_id,
            ],
        )
        .map_err(|e| StorageError::Query(format!("failed to insert message: {e}")))?;

        // INVARIANT: Upsert conflicts retain the stored canonical ID, not a proposed replacement ID.
        let canonical: String = tx
            .query_row(
                "SELECT id FROM messages WHERE account_id=?1 AND folder=?2 AND uid=?3",
                rusqlite::params![message.account_id, message.folder, message.uid],
                |r| r.get(0),
            )
            .map_err(|_| StorageError::Query("category identity unavailable".into()))?;
        crate::categories::refresh(tx, &canonical)?;

        for att in &message.attachments {
            tx.execute(
                "INSERT INTO attachments (
                    id, message_id, part_id, filename, content_type, size_bytes, content_id, is_inline
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(message_id, part_id) DO UPDATE SET
                    filename = excluded.filename,
                    content_type = excluded.content_type,
                    size_bytes = excluded.size_bytes;",
                rusqlite::params![
                    att.id,
                    att.message_id,
                    att.part_id,
                    att.filename,
                    att.content_type,
                    att.size_bytes,
                    att.content_id,
                    if att.is_inline { 1 } else { 0 },
                ],
            )
            .map_err(|e| StorageError::Query(format!("failed to insert attachment: {e}")))?;
        }

        for tag in &message.tags {
            tx.execute(
                "INSERT INTO tags (id, message_id, tag_name, created_at_epoch_ms)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(message_id, tag_name) DO NOTHING;",
                rusqlite::params![
                    format!("{}_{}", message.id, tag),
                    message.id,
                    tag,
                    message.created_at_epoch_ms
                ],
            )
            .map_err(|e| StorageError::Query(format!("failed to insert tag: {e}")))?;
        }

        Ok(())
    }

    /// Inserts or updates an int8 vector embedding for a message in `vec_messages`.
    pub fn insert_message_embedding(
        &self,
        message_id: &str,
        embedding_int8: &[i8],
    ) -> Result<(), StorageError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;
        let tx = conn
            .transaction()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let managed: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM embedding_generation)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| StorageError::Query(e.to_string()))?;
        if managed {
            return Err(StorageError::Query(
                "versioned embedding write required".into(),
            ));
        }
        let vec_bytes = int8_slice_to_bytes(embedding_int8);
        tx.execute("DELETE FROM vec_messages WHERE message_id=?1", [message_id])
            .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.execute(
            "INSERT INTO vec_messages(message_id,account_id,embedding_int8)
             SELECT id,account_id,vec_int8(?2) FROM messages WHERE id=?1",
            rusqlite::params![message_id, vec_bytes],
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.execute(
            "UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)))",
            [],
        )
        .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.commit()
            .map_err(|e| StorageError::Query(e.to_string()))?;

        self.finish_sync_write(&conn, was_in_sync, |idx, conn| {
            refresh_slot_from_conn(idx, conn, message_id)
        })?;

        Ok(())
    }

    /// Updates read status of a message.
    pub fn update_message_is_read(
        &self,
        message_id: &str,
        is_read: bool,
    ) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;

        conn.execute(
            "UPDATE messages SET is_read = ?1 WHERE id = ?2;",
            rusqlite::params![if is_read { 1 } else { 0 }, message_id],
        )
        .map_err(|e| StorageError::Query(format!("failed to update is_read: {e}")))?;

        self.finish_sync_write(&conn, was_in_sync, |_, _| Ok(()))?;
        Ok(())
    }

    /// Retrieves full message detail by ID.
    pub fn get_message_by_id(&self, id: &str) -> Result<Option<MessageRecord>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, folder, uid, message_id_header, in_reply_to, references_header,
                        subject, sender, recipients, received_at_epoch_ms, is_read, has_attachments,
                        category_badge, triage_badges, raw_headers, text_body, html_body, list_unsubscribe,
                        list_unsubscribe_post, one_click_unsubscribe_supported, created_at_epoch_ms,
                        is_starred, snooze_until_epoch_ms, provider_message_id
                 FROM messages WHERE id = ?1;",
            )
            .map_err(|e| StorageError::Query(format!("failed to prepare select message: {e}")))?;

        let mut rows = stmt
            .query(rusqlite::params![id])
            .map_err(|e| StorageError::Query(format!("failed to query message: {e}")))?;

        let row = match rows
            .next()
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            Some(r) => r,
            None => return Ok(None),
        };

        let msg_id: String = row.get(0).map_err(|e| StorageError::Query(e.to_string()))?;

        // Query attachments
        let mut att_stmt = conn
            .prepare(
                "SELECT id, message_id, part_id, filename, content_type, size_bytes, content_id, is_inline
                 FROM attachments WHERE message_id = ?1;",
            )
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let attachments = att_stmt
            .query_map(rusqlite::params![msg_id], |r| {
                Ok(AttachmentRecord {
                    id: r.get(0)?,
                    message_id: r.get(1)?,
                    part_id: r.get(2)?,
                    filename: r.get(3)?,
                    content_type: r.get(4)?,
                    size_bytes: r.get(5)?,
                    content_id: r.get(6)?,
                    is_inline: r.get::<_, i64>(7)? != 0,
                })
            })
            .map_err(|e| StorageError::Query(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::Query(e.to_string()))?;

        // Query tags
        let mut tag_stmt = conn
            .prepare("SELECT tag_name FROM tags WHERE message_id = ?1 ORDER BY tag_name;")
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let tags = tag_stmt
            .query_map(rusqlite::params![msg_id], |r| r.get(0))
            .map_err(|e| StorageError::Query(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(Some(MessageRecord {
            id: msg_id,
            account_id: row.get(1).map_err(|e| StorageError::Query(e.to_string()))?,
            folder: row.get(2).map_err(|e| StorageError::Query(e.to_string()))?,
            uid: row.get(3).map_err(|e| StorageError::Query(e.to_string()))?,
            message_id_header: row.get(4).map_err(|e| StorageError::Query(e.to_string()))?,
            in_reply_to: row.get(5).map_err(|e| StorageError::Query(e.to_string()))?,
            references_header: row.get(6).map_err(|e| StorageError::Query(e.to_string()))?,
            subject: row.get(7).map_err(|e| StorageError::Query(e.to_string()))?,
            sender: row.get(8).map_err(|e| StorageError::Query(e.to_string()))?,
            recipients: row.get(9).map_err(|e| StorageError::Query(e.to_string()))?,
            received_at_epoch_ms: row
                .get(10)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            is_read: row
                .get::<_, i64>(11)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0,
            has_attachments: row
                .get::<_, i64>(12)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0,
            category_badge: row
                .get(13)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            triage_badges: row
                .get::<_, Option<String>>(14)
                .map_err(|e| StorageError::Query(e.to_string()))?
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default(),
            raw_headers: row
                .get(15)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            text_body: row
                .get(16)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            html_body: row
                .get(17)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            list_unsubscribe: row
                .get(18)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            list_unsubscribe_post: row
                .get(19)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            one_click_unsubscribe_supported: row
                .get::<_, i64>(20)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0,
            created_at_epoch_ms: row
                .get(21)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            attachments,
            tags,
            is_starred: row
                .get::<_, i64>(22)
                .map_err(|e| StorageError::Query(e.to_string()))?
                != 0,
            snooze_until_epoch_ms: row
                .get(23)
                .map_err(|e| StorageError::Query(e.to_string()))?,
            provider_message_id: row
                .get(24)
                .map_err(|e| StorageError::Query(e.to_string()))?,
        }))
    }
}

impl MigrationRunner for SqliteMailStore {
    fn run_migrations(&self) -> Result<u32, StorageError> {
        self.clear_ram_index();
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let migrator = SchemaMigrator::new(self.tokenizer_mode());
        migrator.migrate(&mut conn)
    }
}

impl MailRepository for SqliteMailStore {
    fn get_folder_sync_state(
        &self,
        account_id: &str,
        folder_name: &str,
    ) -> Result<Option<FolderSyncState>, StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT uidvalidity, uidnext, highest_modseq FROM folder_sync_state
                 WHERE account_id = ?1 AND folder_name = ?2;",
            )
            .map_err(|e| StorageError::Query(format!("failed to query folder sync state: {e}")))?;

        let mut rows = stmt
            .query(rusqlite::params![account_id, folder_name])
            .map_err(|e| StorageError::Query(format!("failed to execute query: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Query(e.to_string()))?
        {
            let uidvalidity: u32 = row.get(0).map_err(|e| StorageError::Query(e.to_string()))?;
            let uidnext: u32 = row.get(1).map_err(|e| StorageError::Query(e.to_string()))?;
            let modseq_val: Option<i64> =
                row.get(2).map_err(|e| StorageError::Query(e.to_string()))?;

            Ok(Some(FolderSyncState {
                account_id: account_id.to_string(),
                folder_name: folder_name.to_string(),
                uidvalidity,
                uidnext,
                highest_modseq: modseq_val.map(|m| m as u64),
            }))
        } else {
            Ok(None)
        }
    }

    fn upsert_folder_sync_state(&self, state: &FolderSyncState) -> Result<(), StorageError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        let was_in_sync = self.check_and_prepare_write(&conn)?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        conn.execute(
            "INSERT INTO folder_sync_state (
                account_id, folder_name, uidvalidity, uidnext, highest_modseq, updated_at_epoch_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(account_id, folder_name) DO UPDATE SET
                uidvalidity = excluded.uidvalidity,
                uidnext = excluded.uidnext,
                highest_modseq = excluded.highest_modseq,
                updated_at_epoch_ms = excluded.updated_at_epoch_ms;",
            rusqlite::params![
                state.account_id,
                state.folder_name,
                state.uidvalidity,
                state.uidnext,
                state.highest_modseq.map(|m| m as i64),
                now_ms
            ],
        )
        .map_err(|e| StorageError::Query(format!("failed to upsert sync state: {e}")))?;

        self.finish_sync_write(&conn, was_in_sync, |_, _| Ok(()))?;
        Ok(())
    }
}

impl SqliteMailStore {
    fn search_with_ram_or_fallback(
        &self,
        conn: &Connection,
        query: &HybridQuery,
        allow_sec_conn: bool,
    ) -> Result<Vec<MessageRowSummary>, StorageError> {
        let has_valid_emb = query
            .query_embedding_int8
            .as_deref()
            .is_some_and(|e| e.len() == RAM_VECTOR_DIM);
        if has_valid_emb {
            if let Some(guard) = self.fresh_ram_guard(conn)? {
                if let RamVectorState::Ready(ref ready) = *guard {
                    let emb = query.query_embedding_int8.as_deref().unwrap_or(&[]);
                    let filter = RamQueryFilter {
                        account_id: query.account_filter.as_deref(),
                        ..Default::default()
                    };
                    if query.mode == SearchMode::Semantic
                        || (query.mode == SearchMode::Hybrid && query.query_text.trim().is_empty())
                    {
                        let ids =
                            ready
                                .index
                                .rank_filtered(emb, &filter, query.offset, query.limit)?;
                        drop(guard);
                        return load_summaries(conn, &ids);
                    }
                    let candidates = HybridQuery {
                        limit: (query.limit + query.offset).max(50),
                        offset: 0,
                        ..query.clone()
                    };
                    let cand_ref = &candidates;
                    let idx_ref = &ready.index;
                    let (lex_res, sem_res) = std::thread::scope(|scope| {
                        let sem_handle = scope
                            .spawn(move || idx_ref.rank_filtered(emb, &filter, 0, cand_ref.limit));
                        let lex_res = lexical_ids(conn, cand_ref);
                        let sem_res = sem_handle.join().unwrap_or_else(|_| {
                            Err(StorageError::Query(
                                "ram semantic search thread panicked".into(),
                            ))
                        });
                        (lex_res, sem_res)
                    });
                    drop(guard);
                    return rrf_fuse_and_load(conn, lex_res?, sem_res?, query);
                }
            }
        }

        match query.mode {
            SearchMode::Semantic => execute_semantic_search(conn, query),
            _ => {
                if allow_sec_conn {
                    if let Some(ref sec_mutex) = self.secondary_conn {
                        if let Ok(mut sec_conn) = sec_mutex.try_lock() {
                            return execute_hybrid_search_parallel(conn, &mut sec_conn, query);
                        }
                    }
                }
                execute_hybrid_search(conn, query)
            }
        }
    }
}

impl SearchEngine for SqliteMailStore {
    fn search(&self, query: &HybridQuery) -> Result<Vec<MessageRowSummary>, StorageError> {
        query.validate()?;

        if query.mode == SearchMode::Lexical {
            let conn = self
                .conn
                .lock()
                .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
            return execute_lexical_search(&conn, query);
        }

        let managed = self.transaction(|tx| {
            tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM embedding_generation)",
                [],
                |r| r.get::<_, bool>(0),
            )
            .map_err(|e| StorageError::Query(e.to_string()))
        })?;
        if managed {
            return self.transaction(|tx| {
                crate::embeddings::check_ready(tx, query)?;
                self.search_with_ram_or_fallback(tx, query, false)
            });
        }

        let conn = self
            .conn
            .lock()
            .map_err(|e| StorageError::Query(format!("mutex poisoned: {e}")))?;
        self.search_with_ram_or_fallback(&conn, query, true)
    }
}
