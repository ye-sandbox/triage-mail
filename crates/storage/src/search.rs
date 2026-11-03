use std::collections::HashMap;

use rusqlite::Connection;

use crate::{HybridQuery, MessageRowSummary, StorageError};

// INVARIANT: Eligibility and hydration resolve exactly the same correction precedence.
pub(crate) const CATEGORY_EFFECTIVE: &str = "COALESCE((
    SELECT COALESCE(c.category,s.category,a.automatic) FROM message_categories a
    LEFT JOIN category_corrections c ON c.account_id=a.account_id AND c.correction_key=a.correction_key
    LEFT JOIN category_sender_overrides s ON s.account_id=a.account_id AND s.sender_key=a.sender_key
    WHERE a.message_id=m.id),m.category_badge,'Personal')";

/// Converts an int8 slice into a byte vector for sqlite-vec.
pub fn int8_slice_to_bytes(slice: &[i8]) -> Vec<u8> {
    slice.iter().map(|&b| b as u8).collect()
}

/// Blank lexical input lists persisted messages newest first, including local metadata.
pub fn execute_lexical_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    query.validate()?;
    load_summaries(conn, &lexical_ids(conn, query)?)
}

fn lexical_ids(conn: &Connection, query: &HybridQuery) -> Result<Vec<String>, StorageError> {
    if query.query_text.trim().is_empty() {
        return if let Some(account) = query.account_filter.as_deref() {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT id FROM messages WHERE account_id=?1
                     ORDER BY received_at_epoch_ms DESC, id LIMIT ?2 OFFSET ?3",
                )
                .map_err(query_error)?;
            let rows = stmt
                .query_map(
                    rusqlite::params![account, query.limit as i64, query.offset as i64],
                    |row| row.get(0),
                )
                .map_err(query_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
        } else {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT id FROM messages
                     ORDER BY received_at_epoch_ms DESC, id LIMIT ?1 OFFSET ?2",
                )
                .map_err(query_error)?;
            let rows = stmt
                .query_map(
                    rusqlite::params![query.limit as i64, query.offset as i64],
                    |row| row.get(0),
                )
                .map_err(query_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
        };
    }
    let expression = format!("\"{}\"", query.query_text.trim().replace('"', "\"\""));
    // INVARIANT: Rank narrow candidates before reading message payload pages. rowid breaks
    // BM25 ties consistently without loading full external-content rows for every match.
    // INVARIANT: Unary + prevents FTS5 from turning rowid IN into repeated MATCH
    // point scans. The covering account index supplies membership without payload reads.
    // INVARIANT: Avoid MATERIALIZED temp-table overhead; non-materialized CTE flattens
    // into a single query without intermediate table writes while preserving tie determinism.
    if let Some(account) = query.account_filter.as_deref() {
        let sql = "WITH ranked AS (
            SELECT rowid AS rid, rank AS score FROM messages_fts
            WHERE messages_fts MATCH ?1
              AND +rowid IN (SELECT rowid FROM messages WHERE account_id=?2)
            ORDER BY rank, rowid LIMIT ?3 OFFSET ?4)
         SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid
         ORDER BY r.score, r.rid";
        let mut stmt = conn.prepare_cached(sql).map_err(query_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![expression, account, query.limit as i64, query.offset as i64],
                |row| row.get(0),
            )
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    } else {
        let sql = "WITH ranked AS (
            SELECT rowid AS rid, rank AS score FROM messages_fts
            WHERE messages_fts MATCH ?1
            ORDER BY rank, rowid LIMIT ?2 OFFSET ?3)
         SELECT m.id FROM ranked r JOIN messages m ON m.rowid=r.rid
         ORDER BY r.score, r.rid";
        let mut stmt = conn.prepare_cached(sql).map_err(query_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![expression, query.limit as i64, query.offset as i64],
                |row| row.get(0),
            )
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    }
}

/// Executes exact cosine nearest-neighbor search over eligible message vectors.
pub fn execute_semantic_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    query.validate()?;
    load_summaries(conn, &semantic_ids(conn, query)?)
}

fn semantic_ids(conn: &Connection, query: &HybridQuery) -> Result<Vec<String>, StorageError> {
    let embedding = query
        .query_embedding_int8
        .as_ref()
        .ok_or_else(|| StorageError::InvalidSearchQuery("missing query embedding".into()))?;
    let bytes = int8_slice_to_bytes(embedding);
    let end = query.limit + query.offset;
    // INVARIANT: vec0 0.1.9 caps k at 4096. Exact ranking also handles arbitrarily
    // large distance-tie groups, so page membership never depends on a truncated tie.
    if end >= 4096 {
        return exact_semantic_ids(conn, query, &bytes);
    }
    let mut k = end + 1;
    loop {
        let mut rows = knn_candidates(conn, &bytes, k, query.account_filter.as_deref())?;
        rows.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        let complete = rows.len() < k || rows[end - 1].1 < rows[k - 1].1;
        if complete {
            let ids: Vec<_> = rows.into_iter().take(end).map(|(id, _)| id).collect();
            // Orphan vectors must not consume page slots (vec0 has no foreign keys).
            let existing: i64 = conn
                .query_row(
                    &format!(
                        "SELECT count(*) FROM messages WHERE id IN ({})",
                        vec!["?"; ids.len()].join(",")
                    ),
                    rusqlite::params_from_iter(&ids),
                    |row| row.get(0),
                )
                .map_err(query_error)?;
            return if existing as usize == ids.len() {
                Ok(ids.into_iter().skip(query.offset).collect())
            } else {
                exact_semantic_ids(conn, query, &bytes)
            };
        }
        if k == 4096 {
            return exact_semantic_ids(conn, query, &bytes);
        }
        k = (k * 2).min(4096);
    }
}

fn knn_candidates(
    conn: &Connection,
    bytes: &[u8],
    k: usize,
    account: Option<&str>,
) -> Result<Vec<(String, f64)>, StorageError> {
    // INVARIANT: vec0 prunes account partitions before top-k, never after truncation.
    if let Some(account_id) = account {
        let mut stmt = conn
            .prepare_cached(
                "SELECT message_id,distance FROM vec_messages
                 WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2 AND account_id=?3",
            )
            .map_err(query_error)?;
        let rows = stmt
            .query_map(rusqlite::params![bytes, k as i64, account_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    } else {
        let mut stmt = conn
            .prepare_cached(
                "SELECT message_id,distance FROM vec_messages
                 WHERE embedding_int8 MATCH vec_int8(?1) AND k=?2",
            )
            .map_err(query_error)?;
        let rows = stmt
            .query_map(rusqlite::params![bytes, k as i64], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    }
}

fn exact_semantic_ids(
    conn: &Connection,
    query: &HybridQuery,
    bytes: &[u8],
) -> Result<Vec<String>, StorageError> {
    // INVARIANT: Account eligibility is resolved before distance ranking and LIMIT.
    // CROSS JOIN keeps the account-index scan outside point lookups into vec0, avoiding
    // a full vector scan followed by post-filtering. Missing embeddings simply don't join.
    if let Some(account) = query.account_filter.as_deref() {
        let sql = "SELECT v.message_id FROM messages m CROSS JOIN vec_messages v
         WHERE m.account_id=?1 AND v.message_id=m.id
         ORDER BY vec_distance_cosine(v.embedding_int8, vec_int8(?2)), v.message_id
         LIMIT ?3 OFFSET ?4";
        let mut stmt = conn.prepare_cached(sql).map_err(query_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![account, bytes, query.limit as i64, query.offset as i64],
                |row| row.get(0),
            )
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    } else {
        let sql = "SELECT v.message_id FROM vec_messages v JOIN messages m ON m.id=v.message_id
         ORDER BY vec_distance_cosine(v.embedding_int8, vec_int8(?1)), v.message_id
         LIMIT ?2 OFFSET ?3";
        let mut stmt = conn.prepare_cached(sql).map_err(query_error)?;
        let rows = stmt
            .query_map(
                rusqlite::params![bytes, query.limit as i64, query.offset as i64],
                |row| row.get(0),
            )
            .map_err(query_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(query_error)
    }
}

/// Combines lexical and semantic candidates with Reciprocal Rank Fusion.
pub fn execute_hybrid_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    query.validate()?;
    match (
        !query.query_text.trim().is_empty(),
        query.query_embedding_int8.is_some(),
    ) {
        (false, false) | (true, false) => execute_lexical_search(conn, query),
        (false, true) => execute_semantic_search(conn, query),
        (true, true) => {
            let candidates = HybridQuery {
                limit: (query.limit + query.offset).max(50),
                offset: 0,
                ..query.clone()
            };
            let mut scores: HashMap<String, f64> = HashMap::with_capacity(candidates.limit * 2);
            for ids in [
                lexical_ids(conn, &candidates)?,
                semantic_ids(conn, &candidates)?,
            ] {
                for (rank, id) in ids.into_iter().enumerate() {
                    *scores.entry(id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
                }
            }
            let mut ranked: Vec<_> = scores.into_iter().collect();
            // INVARIANT: HashMap iteration must never decide equal-score page membership.
            ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let ids: Vec<_> = ranked
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|(id, _)| id)
                .collect();
            load_summaries(conn, &ids)
        }
    }
}

/// Combines lexical and semantic candidates concurrently using two SQLCipher connections.
pub fn execute_hybrid_search_parallel(
    conn_lex: &Connection,
    conn_sem: &mut Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    query.validate()?;
    match (
        !query.query_text.trim().is_empty(),
        query.query_embedding_int8.is_some(),
    ) {
        (false, false) | (true, false) => execute_lexical_search(conn_lex, query),
        (false, true) => execute_semantic_search(conn_sem, query),
        (true, true) => {
            let candidates = HybridQuery {
                limit: (query.limit + query.offset).max(50),
                offset: 0,
                ..query.clone()
            };
            let sem_candidates = &candidates;
            let (lex_res, sem_res) = std::thread::scope(|s| {
                let sem_handle = s.spawn(move || semantic_ids(conn_sem, sem_candidates));
                let lex_res = lexical_ids(conn_lex, &candidates);
                let sem_res = sem_handle.join().unwrap_or_else(|_| {
                    Err(StorageError::Query(
                        "semantic search thread panicked".into(),
                    ))
                });
                (lex_res, sem_res)
            });
            let mut scores: HashMap<String, f64> = HashMap::with_capacity(candidates.limit * 2);
            for ids in [lex_res?, sem_res?] {
                for (rank, id) in ids.into_iter().enumerate() {
                    *scores.entry(id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
                }
            }
            let mut ranked: Vec<_> = scores.into_iter().collect();
            ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let ids: Vec<_> = ranked
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|(id, _)| id)
                .collect();
            load_summaries(conn_lex, &ids)
        }
    }
}

fn query_error(error: rusqlite::Error) -> StorageError {
    StorageError::Query(error.to_string())
}

pub(crate) fn load_summaries(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<MessageRowSummary>, StorageError> {
    let mut summaries = HashMap::new();
    // INVARIANT: Hydrate only the final page, once after fusion, without changing rank order.
    for chunk in ids.chunks(500) {
        let sql = format!(
            "SELECT id, account_id, folder, uid, subject, sender, received_at_epoch_ms,
             is_read, has_attachments, {}, triage_badges, is_starred, snooze_until_epoch_ms FROM messages m WHERE id IN ({})",
            CATEGORY_EFFECTIVE,
            vec!["?"; chunk.len()].join(",")
        );
        let mut stmt = conn.prepare_cached(&sql).map_err(query_error)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(chunk), row_to_summary)
            .map_err(query_error)?;
        for row in rows {
            let row = row.map_err(query_error)?;
            summaries.insert(row.id.clone(), row);
        }
    }
    let mut rows: Vec<_> = ids.iter().filter_map(|id| summaries.remove(id)).collect();
    hydrate_metadata(conn, &mut rows)?;
    Ok(rows)
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageRowSummary> {
    let triage_badges = row
        .get::<_, Option<String>>(10)?
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default();
    Ok(MessageRowSummary {
        id: row.get(0)?,
        account_id: row.get(1)?,
        folder: row.get(2)?,
        uid: row.get(3)?,
        subject: row.get(4)?,
        sender: row.get(5)?,
        received_at_epoch_ms: row.get(6)?,
        is_read: row.get::<_, i64>(7)? != 0,
        has_attachments: row.get::<_, i64>(8)? != 0,
        category_badge: row.get(9)?,
        tags: Vec::new(),
        folder_override: None,
        triage_badges,
        is_starred: row.get::<_, i64>(11)? != 0,
        snooze_until_epoch_ms: row.get(12)?,
    })
}

/// Hydrates only ranked candidates; joins must not alter result order or pagination.
fn hydrate_metadata(
    conn: &Connection,
    summaries: &mut [MessageRowSummary],
) -> Result<(), StorageError> {
    // INVARIANT: Bound SQL parameters, and never join tags into the ranked query (fan-out).
    for chunk in summaries.chunks_mut(500) {
        let positions: HashMap<String, usize> = chunk
            .iter()
            .enumerate()
            .map(|(index, row)| (row.id.clone(), index))
            .collect();
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!(
            "SELECT m.id, o.folder, t.tag_name FROM messages m
             LEFT JOIN message_folder_overrides o ON o.message_id=m.id
             LEFT JOIN tags t ON t.message_id=m.id
             WHERE m.id IN ({placeholders}) ORDER BY t.tag_name"
        );
        let mut statement = conn
            .prepare_cached(&sql)
            .map_err(|e| StorageError::Query(e.to_string()))?;
        let ids: Vec<&str> = chunk.iter().map(|row| row.id.as_str()).collect();
        let records = statement
            .query_map(rusqlite::params_from_iter(ids), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|e| StorageError::Query(e.to_string()))?;
        for record in records {
            let (id, folder, tag) = record.map_err(|e| StorageError::Query(e.to_string()))?;
            let summary = &mut chunk[positions[&id]];
            summary.folder_override = folder;
            if let Some(tag) = tag {
                summary.tags.push(tag);
            }
        }
    }
    Ok(())
}
