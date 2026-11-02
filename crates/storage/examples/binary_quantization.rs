#![forbid(unsafe_code)]
//! Offline investigation only; no production migration or approximate search dispatch.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use data::{Result, DIMENSIONS, ROWS};
use rusqlite::{params, Connection};
use std::{collections::HashSet, time::Instant};
// Reuse private production hydration without widening the storage library's public API.
#[allow(dead_code)]
#[path = "../src/search.rs"]
mod production_search;
use production_search::{int8_slice_to_bytes, load_summaries};
use storage::{
    HybridQuery, MailboxScope, MailboxView, MessageRowSummary, SearchMode, SqliteMailStore,
    StorageError,
};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    predicate: &'static str,
}
const CASES: [Case; 7] = [
    Case { name: "Global", predicate: "1" },
    Case { name: "Inbox", predicate: "m.account_id='account-2' AND (m.snooze_until_epoch_ms IS NULL OR m.snooze_until_epoch_ms<=?1) AND EXISTS (SELECT 1 FROM message_folder_memberships f INDEXED BY idx_memberships_message_id LEFT JOIN message_folder_overrides o ON o.message_id=m.id WHERE f.message_id=m.id AND f.account_id=m.account_id AND COALESCE(o.folder,f.folder)='INBOX')" },
    Case { name: "Starred", predicate: "m.account_id='account-2' AND m.is_starred=1" },
    Case { name: "Snoozed", predicate: "m.account_id='account-2' AND m.snooze_until_epoch_ms>?1" },
    Case { name: "Archive", predicate: "m.account_id='account-2' AND EXISTS (SELECT 1 FROM message_folder_memberships f INDEXED BY idx_memberships_message_id LEFT JOIN message_folder_overrides o ON o.message_id=m.id WHERE f.message_id=m.id AND f.account_id=m.account_id AND COALESCE(o.folder,f.folder)='Archive')" },
    Case { name: "All-label", predicate: "m.account_id='account-2' AND EXISTS (SELECT 1 FROM message_folder_memberships f INDEXED BY idx_memberships_message_id LEFT JOIN message_folder_overrides o ON o.message_id=m.id WHERE f.message_id=m.id AND f.account_id=m.account_id AND COALESCE(o.folder,f.folder)='All Mail')" },
    Case { name: "Empty", predicate: "m.account_id='account-2' AND EXISTS (SELECT 1 FROM message_folder_memberships f INDEXED BY idx_memberships_message_id LEFT JOIN message_folder_overrides o ON o.message_id=m.id WHERE f.message_id=m.id AND f.account_id=m.account_id AND COALESCE(o.folder,f.folder)='Empty')" },
];
impl Case {
    fn scope(self) -> MailboxScope {
        let view = match self.name {
            "Global" => return MailboxScope::GlobalAllMail,
            "Inbox" => MailboxView::Inbox,
            "Starred" => MailboxView::Starred,
            "Snoozed" => MailboxView::Snoozed,
            "All-label" => MailboxView::Folder("All Mail".into()),
            other => MailboxView::Folder(other.into()),
        };
        MailboxScope::AccountView {
            account_id: "account-2".into(),
            view,
        }
    }
    fn eligible(self, id: usize) -> bool {
        self.name == "Global"
            || (id % 4 == 2
                && match self.name {
                    "Inbox" => data::folder(id) == "INBOX" && !data::active_snooze(id),
                    "Starred" => data::starred(id),
                    "Snoozed" => data::active_snooze(id),
                    "Archive" => data::folder(id) == "Archive",
                    "All-label" => true,
                    _ => false,
                })
    }
}

fn binary_index(conn: &Connection) -> Result<()> {
    // INVARIANT: This derived index exists only in the disposable encrypted fixture.
    // Integer IDs are message rowids, not vec0 shadow-table IDs. No shadow tables are read.
    conn.execute_batch("CREATE VIRTUAL TABLE experiment_bq USING vec0(account_id TEXT PARTITION KEY, embedding bit[384]);
        INSERT INTO experiment_bq(rowid,account_id,embedding)
        SELECT m.rowid,m.account_id,vec_quantize_binary(v.embedding_int8)
        FROM messages m CROSS JOIN vec_messages v WHERE v.message_id=m.id;
        PRAGMA wal_checkpoint(TRUNCATE);")?;
    Ok(())
}

fn candidate_sql(case: Case) -> String {
    let partition = if case.name == "Global" {
        ""
    } else {
        "AND account_id='account-2'"
    };
    // INVARIANT: This fixture is immutable after indexing live messages. Global needs
    // no existence probe; a production design would need transactional index maintenance.
    let eligible = if case.name == "Global" {
        "AND ?1>=0".to_owned()
    } else {
        format!(
            "AND rowid IN (SELECT m.rowid FROM messages m WHERE ({}) AND ?1>=0)",
            case.predicate
        )
    };
    // INVARIANT: Mailbox eligibility precedes top-k, never filters its output.
    // MATERIALIZED keeps the bounded Hamming scan outside exact per-candidate point lookups.
    format!(
        "WITH candidates AS MATERIALIZED (
        SELECT rowid FROM experiment_bq WHERE embedding MATCH vec_quantize_binary(vec_int8(?2))
        AND k=?3 {partition} {eligible})
        SELECT m.id FROM candidates c JOIN messages m ON m.rowid=c.rowid
        CROSS JOIN vec_messages v WHERE v.message_id=m.id
        ORDER BY vec_distance_cosine(v.embedding_int8,vec_int8(?2)),m.id LIMIT ?4 OFFSET ?5"
    )
}

fn approximate(
    conn: &Connection,
    case: Case,
    query: &HybridQuery,
    budget: usize,
    now: i64,
) -> Result<Vec<String>> {
    let bytes = int8_slice_to_bytes(query.query_embedding_int8.as_ref().unwrap());
    let mut stmt = conn.prepare_cached(&candidate_sql(case))?;
    let ids = stmt
        .query_map(
            params![now, bytes, budget, query.limit, query.offset],
            |r| r.get(0),
        )?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(load_summaries(conn, &ids)?
        .into_iter()
        .map(|r| r.id)
        .collect())
}

// Independent i64 dot/norm oracle. No SQL cosine or production ranking is reused.
fn oracle(case: Case, query: &[i8]) -> Vec<String> {
    let norm = query.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
    let mut ranked: Vec<_> = (0..ROWS)
        .filter(|&id| case.eligible(id))
        .map(|id| {
            let values = data::vector(id);
            let dot = values
                .iter()
                .zip(query)
                .map(|(&a, &b)| i64::from(a) * i64::from(b))
                .sum::<i64>();
            let length = values.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
            (1.0 - dot as f64 / (length * norm).sqrt(), id)
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .map(|(_, id)| format!("message-{id:06}"))
        .collect()
}

fn measure(fixture: &data::Fixture, case: Case, seed: usize, offset: usize) -> Result<()> {
    let query = HybridQuery {
        query_text: "newsletter".into(),
        query_embedding_int8: Some(data::vector(seed)),
        account_filter: None,
        mode: SearchMode::Semantic,
        limit: 200,
        offset,
    };
    let ranked = oracle(case, query.query_embedding_int8.as_ref().unwrap());
    let expected: Vec<_> = ranked.iter().skip(offset).take(200).cloned().collect();
    for budget in [0, 200, 800, 4096] {
        let fallback = offset + query.limit > 4096 && budget > 0;
        let algorithm = if budget == 0 {
            "production"
        } else if fallback {
            "bq-fallback"
        } else {
            "bq"
        };
        let path = fixture.directory.path().join("synthetic.db");
        let store = SqliteMailStore::open(&path, &fixture.key, Default::default())?;
        let conn = storage::connection::open_encrypted_connection(&path, &fixture.key)?;
        // INVARIANT: Candidate collection, reranking and hydration share a read snapshot.
        let execute = || -> Result<Vec<String>> {
            if budget == 0 || fallback {
                Ok(store
                    .search_scoped(&query, &case.scope())?
                    .into_iter()
                    .map(|r| r.id)
                    .collect())
            } else {
                conn.execute_batch("BEGIN")?;
                let result = approximate(&conn, case, &query, budget, storage::now_epoch_ms());
                conn.execute_batch("ROLLBACK")?;
                result
            }
        };
        for sample in 0..36 {
            let start = Instant::now();
            let actual = execute()?;
            let ms = start.elapsed().as_secs_f64() * 1000.;
            let hits = actual.iter().filter(|id| expected.contains(id)).count();
            let equivalent = actual == expected;
            assert_eq!(actual.iter().collect::<HashSet<_>>().len(), actual.len());
            for id in &actual {
                assert!(case.eligible(id.trim_start_matches("message-").parse()?));
            }
            if budget == 0 || fallback {
                assert!(
                    equivalent,
                    "production oracle mismatch {} seed={seed} offset={offset}",
                    case.name
                );
            }
            // First query is reopened, next five are warmups, then 30 warm samples.
            if sample == 0 || sample >= 6 {
                let cache = if sample == 0 { "reopened" } else { "warm" };
                println!("{},{seed},{offset},{},{algorithm},{budget},{cache},{sample},{ms:.6},{},{hits},{},{}",
                    case.name,ranked.len(),actual.len(),expected.len(),u8::from(equivalent));
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let fixture = data::seed()?;
    let path = fixture.directory.path().join("synthetic.db");
    let conn = storage::connection::open_encrypted_connection(&path, &fixture.key)?;
    let before = std::fs::metadata(&path)?.len();
    let start = Instant::now();
    binary_index(&conn)?;
    println!(
        "# rows={ROWS},dimensions={DIMENSIONS},before_bytes={before},after_bytes={},index_ms={:.3}",
        std::fs::metadata(&path)?.len(),
        start.elapsed().as_secs_f64() * 1000.
    );
    println!(
        "# sqlite={},cipher={},vec={}",
        conn.query_row("SELECT sqlite_version()", [], |r| r.get::<_, String>(0))?,
        conn.query_row("PRAGMA cipher_version", [], |r| r.get::<_, String>(0))?,
        conn.query_row("SELECT vec_version()", [], |r| r.get::<_, String>(0))?
    );
    for detail in conn
        .prepare(&format!("EXPLAIN QUERY PLAN {}", candidate_sql(CASES[1])))?
        .query_map(
            params![storage::now_epoch_ms(), vec![1_u8; 384], 200, 200, 0],
            |r| r.get::<_, String>(3),
        )?
    {
        println!("# plan,{}", detail?);
    }
    drop(conn);
    println!("view,seed,offset,eligible,algorithm,budget,cache,sample,ms,results,oracle_hits,oracle_count,equivalent");
    for case in CASES {
        for offset in [0, 4000] {
            measure(&fixture, case, 4242, offset)?;
        }
    }
    for seed in [17, 90001] {
        measure(&fixture, CASES[0], seed, 0)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::{connection::open_in_memory_connection, migrations::SchemaMigrator, DatabaseKey};

    fn fixture(count: usize, values: &[i8]) -> Result<Connection> {
        let conn = open_in_memory_connection(&DatabaseKey::from_bytes([0; 32]))?;
        let mut conn = conn;
        SchemaMigrator::new(Default::default()).migrate(&mut conn)?;
        conn.execute(
            "INSERT INTO accounts VALUES ('account-2','fixture@example.com','synthetic',0,0)",
            [],
        )?;
        let tx = conn.transaction()?;
        for id in 0..count {
            let name = format!("message-{id:06}");
            tx.execute("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,created_at_epoch_ms) VALUES (?1,'account-2','INBOX',?2,'synthetic','sender@example.org','reader@example.com',0,'',0)",params![name,id+1])?;
            tx.execute("INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,'account-2',vec_int8(?2))",params![name,int8_slice_to_bytes(values)])?;
        }
        tx.commit()?;
        Ok(conn)
    }
    fn query(values: Vec<i8>) -> HybridQuery {
        HybridQuery {
            query_text: String::new(),
            query_embedding_int8: Some(values),
            account_filter: None,
            mode: SearchMode::Semantic,
            limit: 200,
            offset: 0,
        }
    }
    fn replace(conn: &Connection, id: usize, values: &[i8]) -> Result<()> {
        let id = format!("message-{id:06}");
        // INVARIANT: Match production replacement writes; vec0 UPDATE can lose subtypes.
        conn.execute("DELETE FROM vec_messages WHERE message_id=?1", [&id])?;
        conn.execute(
            "INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,'account-2',vec_int8(?2))",
            params![id, int8_slice_to_bytes(values)],
        )?;
        Ok(())
    }
    fn exact(conn: &Connection, query: &HybridQuery) -> Result<Vec<String>> {
        // Independent Rust oracle over actual stored inputs, including SQL NULL-first semantics.
        let q = query.query_embedding_int8.as_ref().unwrap();
        let mut stmt = conn.prepare("SELECT m.id,v.embedding_int8 FROM messages m CROSS JOIN vec_messages v WHERE v.message_id=m.id")?;
        let mut ranked = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .map(|row| {
                let (id, bytes) = row?;
                let v: Vec<_> = bytes.into_iter().map(|b| b as i8).collect();
                let dot: i64 = v
                    .iter()
                    .zip(q)
                    .map(|(&a, &b)| i64::from(a) * i64::from(b))
                    .sum();
                let a: i64 = v.iter().map(|&v| i64::from(v).pow(2)).sum();
                let b: i64 = q.iter().map(|&v| i64::from(v).pow(2)).sum();
                let distance = if a == 0 || b == 0 {
                    None
                } else {
                    Some(1. - dot as f64 / (a as f64 * b as f64).sqrt())
                };
                Ok((distance, id))
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
        Ok(ranked
            .into_iter()
            .skip(query.offset)
            .take(query.limit)
            .map(|(_, id)| id)
            .collect())
    }
    #[test]
    fn sign_loss_can_exclude_the_unique_best_cosine_neighbor() -> Result<()> {
        let mut q = vec![1; DIMENSIONS];
        q[0] = 127;
        let mut near = vec![-1; DIMENSIONS];
        near[0] = 127;
        let conn = fixture(201, &vec![1; DIMENSIONS])?;
        replace(&conn, 200, &near)?;
        binary_index(&conn)?;
        let query = query(q);
        let expected = exact(&conn, &query)?;
        assert_eq!(expected[0], "message-000200");
        let actual = approximate(&conn, CASES[0], &query, 200, 1)?;
        assert_eq!(actual.len(), 200);
        assert!(!actual.contains(&expected[0]));
        assert_eq!(approximate(&conn, CASES[0], &query, 800, 1)?, expected);
        Ok(())
    }
    #[test]
    fn zero_vectors_missing_embeddings_and_orphans_do_not_certify_coverage() -> Result<()> {
        let conn = fixture(203, &vec![1; DIMENSIONS])?;
        replace(&conn, 200, &vec![0; DIMENSIONS])?;
        conn.execute(
            "DELETE FROM vec_messages WHERE message_id='message-000201'",
            [],
        )?;
        conn.execute("DELETE FROM messages WHERE id='message-000202'", [])?;
        // Even an explicit orphan never enters the derived index or consumes a candidate slot.
        conn.execute("INSERT OR REPLACE INTO vec_messages(message_id,account_id,embedding_int8) VALUES ('orphan','account-2',vec_int8(?1))",[int8_slice_to_bytes(&vec![1;DIMENSIONS])])?;
        binary_index(&conn)?;
        assert_eq!(
            conn.query_row("SELECT count(*) FROM experiment_bq", [], |r| r
                .get::<_, usize>(0))?,
            201
        );
        let query = query(vec![1; DIMENSIONS]);
        let expected = exact(&conn, &query)?;
        assert_eq!(expected[0], "message-000200");
        assert!(!approximate(&conn, CASES[0], &query, 200, 1)?.contains(&expected[0]));
        assert_eq!(approximate(&conn, CASES[0], &query, 800, 1)?, expected);
        Ok(())
    }
    #[test]
    fn oversized_hamming_ties_and_zero_queries_need_exhaustive_fallback() -> Result<()> {
        let mut q = vec![1; DIMENSIONS];
        q[0] = 127;
        let conn = fixture(5000, &vec![1; DIMENSIONS])?;
        binary_index(&conn)?;
        let candidates = approximate(&conn, CASES[0], &query(q.clone()), 4096, 1)?;
        // All sign codes tie. Find an ID outside the actual 4096-candidate frontier.
        let mut stmt=conn.prepare("SELECT rowid FROM experiment_bq WHERE embedding MATCH vec_quantize_binary(vec_int8(?1)) AND k=4096")?;
        let frontier = stmt
            .query_map([int8_slice_to_bytes(&q)], |r| r.get::<_, usize>(0))?
            .collect::<rusqlite::Result<HashSet<_>>>()?;
        let excluded = (0..5000).find(|id| !frontier.contains(&(id + 1))).unwrap();
        replace(&conn, excluded, &q)?; // Same signs: binary index is still valid.
        let expected = exact(&conn, &query(q.clone()))?;
        assert_eq!(expected[0], format!("message-{excluded:06}"));
        assert!(!approximate(&conn, CASES[0], &query(q), 4096, 1)?.contains(&expected[0]));
        assert_eq!(candidates.len(), 200);
        // A zero query gives NULL for every exact distance, hence canonical-ID order.
        let zero = query(vec![0; DIMENSIONS]);
        assert_eq!(exact(&conn, &zero)?[0], "message-000000");
        // Inspect a deep canonical page: no 4096-row frontier can supply 4200 results.
        let deep = HybridQuery {
            offset: 4000,
            ..zero
        };
        assert_eq!(exact(&conn, &deep)?.len(), 200);
        assert_eq!(approximate(&conn, CASES[0], &deep, 4096, 1)?.len(), 96);
        Ok(())
    }
}
