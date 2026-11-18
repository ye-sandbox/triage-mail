//! Encrypted, resumable model generations. Expensive inference runs outside SQL locks.
use crate::{HybridQuery, SearchMode, SqliteMailStore, StorageError};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

fn error(_: impl std::fmt::Display) -> StorageError {
    StorageError::Query("embedding index unavailable".into())
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddingProgress {
    pub indexed: u64,
    pub pending: u64,
    pub failed: u64,
}
#[derive(Debug, Clone)]
pub struct EmbeddingWork {
    pub message_id: String,
    pub version: String,
    pub epoch: String,
    pub text: String,
}

fn progress(conn: &Connection) -> Result<EmbeddingProgress, StorageError> {
    conn.query_row("SELECT COALESCE(SUM(w.indexed_epoch=g.epoch),0),COALESCE(SUM(w.indexed_epoch IS NOT g.epoch),0),COALESCE(SUM(w.failed_epoch=g.epoch),0) FROM embedding_generation g LEFT JOIN embedding_work w ON 1=1 WHERE w.message_id IS NOT NULL", [], |r| Ok(EmbeddingProgress {indexed:r.get(0)?,pending:r.get(1)?,failed:r.get(2)?})).map_err(error)
}
/// Called inside the same read snapshot as eligibility/ranking/hydration.
pub(crate) fn check_ready(conn: &Connection, query: &HybridQuery) -> Result<(), StorageError> {
    if query.mode == SearchMode::Lexical {
        return Ok(());
    }
    if managed_ready(conn)? == Some(false) {
        return Err(error("rebuilding or incompatible generation"));
    }
    Ok(())
}
fn managed_ready(conn: &Connection) -> Result<Option<bool>, StorageError> {
    let generation: Option<String> = conn
        .query_row(
            "SELECT generation FROM embedding_generation WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?;
    let Some(generation) = generation else {
        return Ok(None);
    };
    // INVARIANT: Only the compiled, pinned query model may search a managed index.
    let pending: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM embedding_work WHERE indexed_epoch IS NULL)",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    Ok(Some(generation == triage_ai::onnx::GENERATION && !pending))
}

impl SqliteMailStore {
    pub fn embedding_ready(&self) -> Result<bool, StorageError> {
        self.transaction(|tx| Ok(managed_ready(tx)?.unwrap_or(false)))
    }
    /// Activates a model once, or explicitly replaces the current epoch for a rebuild.
    pub fn begin_embedding_generation(
        &self,
        generation: &str,
        rebuild: bool,
    ) -> Result<(), StorageError> {
        if generation.is_empty() || generation.len() > 200 {
            return Err(error("generation"));
        }
        self.write_transaction(
            |tx| {
                let current: Option<String> = tx
                    .query_row(
                        "SELECT generation FROM embedding_generation WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .optional()
                    .map_err(error)?;
                let rebuilt = rebuild || current.as_deref() != Some(generation);
                if rebuilt {
                    // INVARIANT: Unversioned/projection vectors can never join the learned generation.
                    tx.execute("DELETE FROM vec_messages", []).map_err(error)?;
                    tx.execute(
                        "UPDATE embedding_work SET indexed_epoch=NULL,failed_epoch=NULL",
                        [],
                    )
                    .map_err(error)?;
                    tx.execute("INSERT INTO embedding_generation VALUES(1,?1,lower(hex(randomblob(16)))) ON CONFLICT(singleton) DO UPDATE SET generation=excluded.generation,epoch=excluded.epoch", [generation]).map_err(error)?;
                    tx.execute(
                        "UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)))",
                        [],
                    )
                    .map_err(error)?;
                }
                Ok(rebuilt)
            },
            |idx, _, &rebuilt| {
                if rebuilt {
                    idx.clear();
                }
                Ok(())
            },
        )
        .map(|_| ())
    }
    pub fn embedding_progress(&self) -> Result<EmbeddingProgress, StorageError> {
        self.transaction(|tx| progress(tx))
    }
    pub fn embedding_work(
        &self,
        generation: &str,
        limit: usize,
    ) -> Result<Vec<EmbeddingWork>, StorageError> {
        if limit == 0 || limit > 32 {
            return Err(error("limit"));
        }
        self.transaction(|tx| {
            let mut stmt = tx.prepare("SELECT w.message_id,w.version,g.epoch,substr(m.subject,1,8192)||char(10)||substr(COALESCE(m.text_body,''),1,32768) FROM embedding_work w JOIN messages m ON m.id=w.message_id JOIN embedding_generation g ON g.generation=?1 WHERE w.indexed_epoch IS NULL AND w.failed_epoch IS NULL ORDER BY w.message_id LIMIT ?2").map_err(error)?;
            let rows = stmt.query_map(params![generation,limit], |r| Ok(EmbeddingWork { message_id:r.get(0)?,version:r.get(1)?,epoch:r.get(2)?,text:r.get(3)?})).map_err(error)?;
            rows.collect::<Result<Vec<_>,_>>().map_err(error)
        })
    }
    /// CAS rejects inference finishing after deletion, hydration, rebuild or model replacement.
    pub fn finish_embedding(
        &self,
        generation: &str,
        work: &EmbeddingWork,
        vector: Option<&[i8]>,
    ) -> Result<bool, StorageError> {
        if vector.is_some_and(|v| v.len() != 384 || v.iter().all(|x| *x == 0)) {
            return Err(error("vector"));
        }
        self.write_transaction(
            |tx| {
                let valid: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM embedding_work w JOIN embedding_generation g ON g.generation=?1 AND g.epoch=?2 WHERE w.message_id=?3 AND w.version=?4 AND w.indexed_epoch IS NOT g.epoch)",
                        params![generation, work.epoch, work.message_id, work.version],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if !valid {
                    return Ok(false);
                }
                if let Some(vector) = vector {
                    tx.execute("DELETE FROM vec_messages WHERE message_id=?1", [&work.message_id])
                        .map_err(error)?;
                    tx.execute(
                        "INSERT INTO vec_messages(message_id,account_id,embedding_int8) SELECT id,account_id,vec_int8(?2) FROM messages WHERE id=?1",
                        params![work.message_id, crate::search::int8_slice_to_bytes(vector)],
                    )
                    .map_err(error)?;
                    tx.execute(
                        "UPDATE embedding_work SET indexed_epoch=?2,failed_epoch=NULL WHERE message_id=?1",
                        params![work.message_id, work.epoch],
                    )
                    .map_err(error)?;
                    tx.execute(
                        "UPDATE conversation_epoch SET generation=lower(hex(randomblob(16)))",
                        [],
                    )
                    .map_err(error)?;
                    Ok(true)
                } else {
                    tx.execute(
                        "UPDATE embedding_work SET failed_epoch=?2 WHERE message_id=?1",
                        params![work.message_id, work.epoch],
                    )
                    .map_err(error)?;
                    Ok(true)
                }
            },
            |idx, conn, &committed| {
                if committed && vector.is_some() {
                    crate::ram_vector_index::refresh_slot_from_conn(idx, conn, &work.message_id)?;
                }
                Ok(())
            },
        )
    }
}
