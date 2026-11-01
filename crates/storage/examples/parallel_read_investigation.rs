#![forbid(unsafe_code)]
//! Investigation of parallel read connections for hybrid search and memory profiling of page caches.
use std::{error::Error, io::Read, time::Instant};

use rusqlite::params;
use storage::{
    connection::open_encrypted_connection,
    migrations::SchemaMigrator,
    search::{
        execute_hybrid_search, execute_hybrid_search_parallel, execute_lexical_search,
        execute_semantic_search, int8_slice_to_bytes,
    },
    DatabaseKey, FtsTokenizerMode, HybridQuery, SearchMode,
};

const ROWS: usize = 100_000;
const DIMENSIONS: usize = 384;
const WARM_SAMPLES: usize = 15;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn vector(id: usize) -> Vec<i8> {
    let mut state = id as u64 + 1;
    (0..DIMENSIONS)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 255) as i8
        })
        .collect()
}

fn seed(path: &std::path::Path, key: &DatabaseKey) -> Result<()> {
    let mut conn = open_encrypted_connection(path, key)?;
    SchemaMigrator::new(FtsTokenizerMode::default()).migrate(&mut conn)?;
    let tx = conn.transaction()?;
    for account in 0..4 {
        tx.execute(
            "INSERT INTO accounts VALUES (?1, ?2, 'synthetic', 0, 0)",
            params![
                format!("account-{account}"),
                format!("fixture{account}@example.com")
            ],
        )?;
    }
    {
        let mut message = tx.prepare(
            "INSERT INTO messages
            (id, account_id, folder, uid, subject, sender, recipients,
             received_at_epoch_ms, raw_headers, text_body, created_at_epoch_ms)
            VALUES (?1, ?2, 'INBOX', ?3, ?4, 'sender@example.org',
                    'fixture@example.com', ?3, '', ?5, 0)",
        )?;
        let mut embedding = tx.prepare(
            "INSERT INTO vec_messages(message_id, account_id, embedding_int8) VALUES (?1, ?2, vec_int8(?3))",
        )?;
        for id in 0..ROWS {
            let name = format!("message-{id}");
            let topic = if id % 100 == 0 {
                "invoice"
            } else {
                "newsletter"
            };
            let account_id = format!("account-{}", id % 4);
            message.execute(params![
                name,
                account_id,
                id + 1,
                format!("Synthetic {topic} {id}"),
                format!(
                    "{topic} receipt shipment {}",
                    "local synthetic content ".repeat(40)
                )
            ])?;
            embedding.execute(params![name, account_id, int8_slice_to_bytes(&vector(id))])?;
        }
    }
    tx.commit()?;
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    let count: usize = conn.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?;
    let vectors: usize = conn.query_row("SELECT count(*) FROM vec_messages", [], |r| r.get(0))?;
    assert_eq!((count, vectors), (ROWS, ROWS));
    println!("# seeded {ROWS} rows and vectors into encrypted SQLCipher database");
    Ok(())
}

fn get_rss_kb() -> usize {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<usize>() {
                        return kb;
                    }
                }
            }
        }
    }
    0
}

fn calculate_percentiles(mut values: Vec<f64>) -> (f64, f64, f64, f64) {
    values.sort_by(|a, b| a.total_cmp(b));
    let n = values.len();
    let min = values[0];
    let max = values[n - 1];
    let p50 = values[n / 2];
    let p95_idx = ((n as f64) * 0.95).floor() as usize;
    let p95 = values[p95_idx.min(n - 1)];
    (min, p50, p95, max)
}

fn main() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("parallel_perf.db");
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let key = DatabaseKey::from_bytes(bytes);

    seed(&path, &key)?;

    let query_common = HybridQuery {
        query_text: "newsletter".into(),
        query_embedding_int8: Some(vector(4242)),
        account_filter: None,
        mode: SearchMode::Hybrid,
        limit: 200,
        offset: 0,
    };

    let query_filtered = HybridQuery {
        query_text: "newsletter".into(),
        query_embedding_int8: Some(vector(4242)),
        account_filter: Some("account-2".into()),
        mode: SearchMode::Hybrid,
        limit: 200,
        offset: 0,
    };

    println!("\n================================================================================");
    println!("PARALLEL READ INVESTIGATION & PAGE CACHE MEMORY PROFILING");
    println!("================================================================================");
    println!("Workload: 100,000 messages, 384-dim int8 embeddings in SQLCipher (WAL mode)");
    println!(
        "Initial Process VmRSS: {} KiB ({:.2} MiB)",
        get_rss_kb(),
        get_rss_kb() as f64 / 1024.0
    );

    for &cache_kib in &[16384, 32768, 65536] {
        println!(
            "\n--------------------------------------------------------------------------------"
        );
        println!(
            "CACHE CONFIGURATION: {} KiB ({} MiB per connection)",
            cache_kib,
            cache_kib / 1024
        );
        println!(
            "--------------------------------------------------------------------------------"
        );

        // 1. Single Connection Baseline
        let conn1 = open_encrypted_connection(&path, &key)?;
        conn1.pragma_update(None, "cache_size", -cache_kib)?;
        let rss_after_conn1_open = get_rss_kb();

        // Warmup conn1 with lexical and semantic queries
        let _ = execute_lexical_search(&conn1, &query_common)?;
        let _ = execute_semantic_search(&conn1, &query_common)?;
        let rss_after_conn1_warm = get_rss_kb();

        // Measure individual branches on single connection
        let mut lex_times = Vec::with_capacity(WARM_SAMPLES);
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let _ = execute_lexical_search(&conn1, &query_common)?;
            lex_times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let (_, lex_p50, lex_p95, _) = calculate_percentiles(lex_times);

        let mut sem_times = Vec::with_capacity(WARM_SAMPLES);
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let _ = execute_semantic_search(&conn1, &query_common)?;
            sem_times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let (_, sem_p50, sem_p95, _) = calculate_percentiles(sem_times);

        // Measure sequential hybrid on single connection
        let mut seq_times = Vec::with_capacity(WARM_SAMPLES);
        let mut seq_results = Vec::new();
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let res = execute_hybrid_search(&conn1, &query_common)?;
            seq_times.push(start.elapsed().as_secs_f64() * 1000.0);
            seq_results = res;
        }
        let (_, seq_p50, seq_p95, _) = calculate_percentiles(seq_times);

        // 2. Open 2nd Read Connection
        let mut conn2 = open_encrypted_connection(&path, &key)?;
        conn2.pragma_update(None, "cache_size", -cache_kib)?;
        let rss_after_conn2_open = get_rss_kb();

        // Warm up conn2 with semantic queries
        for _ in 0..3 {
            let _ = execute_semantic_search(&conn2, &query_common)?;
        }
        let rss_after_conn2_warm = get_rss_kb();

        // Measure concurrent/parallel hybrid
        let mut par_times = Vec::with_capacity(WARM_SAMPLES);
        let mut par_results = Vec::new();
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let res = execute_hybrid_search_parallel(&conn1, &mut conn2, &query_common)?;
            par_times.push(start.elapsed().as_secs_f64() * 1000.0);
            par_results = res;
        }
        let (_, par_p50, par_p95, _) = calculate_percentiles(par_times);
        let rss_after_par = get_rss_kb();

        // Verify result equivalence between sequential and parallel
        assert_eq!(seq_results.len(), par_results.len());
        assert_eq!(
            seq_results.iter().map(|r| &r.id).collect::<Vec<_>>(),
            par_results.iter().map(|r| &r.id).collect::<Vec<_>>(),
            "Parallel and sequential hybrid results MUST match 100% exactly"
        );

        // Filtered hybrid comparison
        let mut seq_filt_times = Vec::with_capacity(WARM_SAMPLES);
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let _ = execute_hybrid_search(&conn1, &query_filtered)?;
            seq_filt_times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let (_, seq_filt_p50, seq_filt_p95, _) = calculate_percentiles(seq_filt_times);

        let mut par_filt_times = Vec::with_capacity(WARM_SAMPLES);
        for _ in 0..WARM_SAMPLES {
            let start = Instant::now();
            let _ = execute_hybrid_search_parallel(&conn1, &mut conn2, &query_filtered)?;
            par_filt_times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let (_, par_filt_p50, par_filt_p95, _) = calculate_percentiles(par_filt_times);

        println!("LATENCY COMPARISON (Global Common 100k, 200 rows):");
        println!(
            "  Lexical branch alone:       p50 = {:6.2} ms | p95 = {:6.2} ms",
            lex_p50, lex_p95
        );
        println!(
            "  Semantic branch alone:      p50 = {:6.2} ms | p95 = {:6.2} ms",
            sem_p50, sem_p95
        );
        println!(
            "  Sequential Hybrid (1 conn): p50 = {:6.2} ms | p95 = {:6.2} ms",
            seq_p50, seq_p95
        );
        println!(
            "  Parallel Hybrid (2 conns):  p50 = {:6.2} ms | p95 = {:6.2} ms",
            par_p50, par_p95
        );
        let speedup = (seq_p50 - par_p50) / seq_p50 * 100.0;
        println!(
            "  Latency Reduction:          {:.1}% faster ({:.2} ms savings)",
            speedup,
            seq_p50 - par_p50
        );

        println!("\nLATENCY COMPARISON (Account Filtered 25k, 200 rows):");
        println!(
            "  Sequential Filtered Hybrid: p50 = {:6.2} ms | p95 = {:6.2} ms",
            seq_filt_p50, seq_filt_p95
        );
        println!(
            "  Parallel Filtered Hybrid:   p50 = {:6.2} ms | p95 = {:6.2} ms",
            par_filt_p50, par_filt_p95
        );

        println!("\nMEMORY OVERHEAD (VmRSS):");
        println!(
            "  Conn 1 opened:              {:6} KiB ({:.2} MiB)",
            rss_after_conn1_open,
            rss_after_conn1_open as f64 / 1024.0
        );
        println!(
            "  Conn 1 warmed:              {:6} KiB ({:.2} MiB) [+{} KiB]",
            rss_after_conn1_warm,
            rss_after_conn1_warm as f64 / 1024.0,
            rss_after_conn1_warm.saturating_sub(rss_after_conn1_open)
        );
        println!(
            "  Conn 2 opened:              {:6} KiB ({:.2} MiB) [+{} KiB]",
            rss_after_conn2_open,
            rss_after_conn2_open as f64 / 1024.0,
            rss_after_conn2_open.saturating_sub(rss_after_conn1_warm)
        );
        println!(
            "  Conn 2 warmed:              {:6} KiB ({:.2} MiB) [+{} KiB]",
            rss_after_conn2_warm,
            rss_after_conn2_warm as f64 / 1024.0,
            rss_after_conn2_warm.saturating_sub(rss_after_conn2_open)
        );
        println!(
            "  After parallel execution:   {:6} KiB ({:.2} MiB)",
            rss_after_par,
            rss_after_par as f64 / 1024.0
        );
        let conn2_delta = rss_after_conn2_warm.saturating_sub(rss_after_conn1_warm);
        println!(
            "  Net Memory Cost of Conn 2:  {:6} KiB ({:.2} MiB)",
            conn2_delta,
            conn2_delta as f64 / 1024.0
        );
    }

    Ok(())
}
