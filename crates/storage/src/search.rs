use std::collections::HashMap;

use rusqlite::Connection;

use crate::{HybridQuery, MessageRowSummary, SearchMode, StorageError};

/// Converts an int8 slice into a byte vector for sqlite-vec.
pub fn int8_slice_to_bytes(slice: &[i8]) -> Vec<u8> {
    slice.iter().map(|&b| b as u8).collect()
}

/// Executes a lexical search against `messages_fts` using BM25 ranking.
pub fn execute_lexical_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    let sanitized_query = query.query_text.trim();
    if sanitized_query.is_empty() {
        return Ok(Vec::new());
    }

    // Wrap query in double quotes if it contains spaces or punctuation, or use standard FTS5 match string
    let match_expr = format!("\"{}\"", sanitized_query.replace('"', "\"\""));

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.account_id, m.folder, m.uid, m.subject, m.sender,
                    m.received_at_epoch_ms, m.is_read, m.has_attachments, m.category_badge
             FROM messages_fts f
             JOIN messages m ON f.rowid = m.rowid
             WHERE messages_fts MATCH ?1
               AND (?2 IS NULL OR m.account_id = ?2)
             ORDER BY rank
             LIMIT ?3 OFFSET ?4;",
        )
        .map_err(|e| StorageError::Query(format!("failed to prepare lexical search: {e}")))?;

    let limit = query.limit as i64;
    let offset = query.offset as i64;

    let rows = stmt
        .query_map(
            rusqlite::params![match_expr, query.account_filter, limit, offset],
            row_to_summary,
        )
        .map_err(|e| StorageError::Query(format!("failed to execute lexical search: {e}")))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| StorageError::Query(e.to_string()))?);
    }

    Ok(results)
}

/// Executes a semantic KNN search against `vec_messages` using cosine distance.
pub fn execute_semantic_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    let embedding = query
        .query_embedding_int8
        .as_ref()
        .ok_or_else(|| StorageError::InvalidSearchQuery("missing query embedding".to_string()))?;

    let vec_bytes = int8_slice_to_bytes(embedding);
    let k = (query.limit + query.offset) as i64;

    let mut stmt = conn
        .prepare(
            "SELECT m.id, m.account_id, m.folder, m.uid, m.subject, m.sender,
                    m.received_at_epoch_ms, m.is_read, m.has_attachments, m.category_badge
             FROM vec_messages v
             JOIN messages m ON v.message_id = m.id
             WHERE v.embedding_int8 MATCH vec_int8(?1)
               AND k = ?2
               AND (?3 IS NULL OR m.account_id = ?3)
             ORDER BY v.distance ASC
             LIMIT ?4 OFFSET ?5;",
        )
        .map_err(|e| StorageError::Query(format!("failed to prepare semantic search: {e}")))?;

    let limit = query.limit as i64;
    let offset = query.offset as i64;

    let rows = stmt
        .query_map(
            rusqlite::params![vec_bytes, k, query.account_filter, limit, offset],
            row_to_summary,
        )
        .map_err(|e| StorageError::Query(format!("failed to execute semantic search: {e}")))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| StorageError::Query(e.to_string()))?);
    }

    Ok(results)
}

/// Executes a hybrid search combining lexical and semantic results via Reciprocal Rank Fusion.
pub fn execute_hybrid_search(
    conn: &Connection,
    query: &HybridQuery,
) -> Result<Vec<MessageRowSummary>, StorageError> {
    let has_lexical = !query.query_text.trim().is_empty();
    let has_semantic = query.query_embedding_int8.is_some();

    match (has_lexical, has_semantic) {
        (true, false) => execute_lexical_search(conn, query),
        (false, true) => execute_semantic_search(conn, query),
        (false, false) => Ok(Vec::new()),
        (true, true) => {
            // Expand fetch window to blend top candidates
            let candidate_limit = (query.limit + query.offset).max(50);
            let lexical_query = HybridQuery {
                mode: SearchMode::Lexical,
                limit: candidate_limit,
                offset: 0,
                ..query.clone()
            };
            let semantic_query = HybridQuery {
                mode: SearchMode::Semantic,
                limit: candidate_limit,
                offset: 0,
                ..query.clone()
            };

            let lexical_results = execute_lexical_search(conn, &lexical_query)?;
            let semantic_results = execute_semantic_search(conn, &semantic_query)?;

            // RRF formula: score = sum(1.0 / (60.0 + rank))
            const RRF_K: f64 = 60.0;
            let mut scores: HashMap<String, (f64, MessageRowSummary)> = HashMap::new();

            for (rank, item) in lexical_results.into_iter().enumerate() {
                let score = 1.0 / (RRF_K + (rank as f64) + 1.0);
                scores
                    .entry(item.id.clone())
                    .and_modify(|(s, _)| *s += score)
                    .or_insert((score, item));
            }

            for (rank, item) in semantic_results.into_iter().enumerate() {
                let score = 1.0 / (RRF_K + (rank as f64) + 1.0);
                scores
                    .entry(item.id.clone())
                    .and_modify(|(s, _)| *s += score)
                    .or_insert((score, item));
            }

            let mut blended: Vec<(f64, MessageRowSummary)> = scores.into_values().collect();
            blended.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

            let paged: Vec<MessageRowSummary> = blended
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .map(|(_, item)| item)
                .collect();

            Ok(paged)
        }
    }
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<MessageRowSummary> {
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
    })
}
