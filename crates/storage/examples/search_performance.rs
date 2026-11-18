#![forbid(unsafe_code)]
//! Offline, disposable SQLCipher benchmark; see docs/performance/native-validation.md.
use std::{collections::HashSet, error::Error, io::Read, time::Instant};

use rusqlite::params;
use storage::{
    connection::open_encrypted_connection,
    migrations::SchemaMigrator,
    search::{execute_hybrid_search, execute_hybrid_search_parallel, int8_slice_to_bytes},
    DatabaseKey, FtsTokenizerMode, HybridQuery, SearchEngine, SearchMode, SqliteMailStore,
};

const ROWS: usize = 100_000;
const DIMENSIONS: usize = 384;
const SAMPLES: usize = 30;
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
    println!(
        "# cipher={}, sqlite={}, vec={}",
        conn.query_row("PRAGMA cipher_version", [], |r| r.get::<_, String>(0))?,
        conn.query_row("SELECT sqlite_version()", [], |r| r.get::<_, String>(0))?,
        conn.query_row("SELECT vec_version()", [], |r| r.get::<_, String>(0))?
    );
    Ok(())
}

fn measure(
    store: &SqliteMailStore,
    query: &HybridQuery,
    neighbors: &[String],
) -> Result<(f64, usize)> {
    let start = Instant::now();
    let rows = store.search(query)?;
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let unique: HashSet<_> = rows.iter().map(|r| &r.id).collect();
    assert_eq!(unique.len(), rows.len());
    assert_eq!(
        rows.len(),
        if query.mode == SearchMode::Semantic {
            neighbors
                .len()
                .saturating_sub(query.offset)
                .min(query.limit)
        } else {
            query.limit
        },
        "complete eligible pages are required"
    );
    assert!(rows.iter().all(|r| query
        .account_filter
        .as_ref()
        .is_none_or(|a| &r.account_id == a)));
    if query.mode == SearchMode::Lexical && !query.query_text.is_empty() {
        assert!(rows.iter().all(|r| r.subject.contains(&query.query_text)));
    }
    if query.mode == SearchMode::Semantic {
        assert_eq!(
            rows.iter().map(|r| &r.id).collect::<Vec<_>>(),
            neighbors
                .iter()
                .skip(query.offset)
                .take(query.limit)
                .collect::<Vec<_>>()
        );
    }
    if query.mode == SearchMode::Lexical && query.query_text.trim().is_empty() {
        let expected: Vec<_> = (0..ROWS)
            .rev()
            .filter(|id| query.account_filter.is_none() || id % 4 == 2)
            .skip(query.offset)
            .take(query.limit)
            .map(|id| format!("message-{id}"))
            .collect();
        assert_eq!(
            rows.iter().map(|r| &r.id).collect::<Vec<_>>(),
            expected.iter().collect::<Vec<_>>()
        );
    }
    Ok((elapsed, rows.len()))
}

#[derive(Clone, Copy, Default)]
struct CliOptions {
    with_slow_baseline: bool,
    verbose: bool,
}

fn parse_cli_options() -> CliOptions {
    let mut opts = CliOptions::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--with-slow-baseline" => opts.with_slow_baseline = true,
            "--verbose" => opts.verbose = true,
            "-h" | "--help" => {
                eprintln!("Usage: search_performance [--with-slow-baseline] [--verbose] [--help]");
                std::process::exit(0);
            }
            other => {
                eprintln!("error: unknown argument '{other}'");
                std::process::exit(2);
            }
        }
    }
    opts
}

fn percentile(sorted: &[f64], num: usize, den: usize) -> f64 {
    let idx = (sorted.len() * num).div_ceil(den).saturating_sub(1);
    sorted[idx.min(sorted.len() - 1)]
}

fn run_case(
    path: &std::path::Path,
    key: &DatabaseKey,
    warm_store: &SqliteMailStore,
    name: &str,
    query: HybridQuery,
    neighbors: &[String],
    opts: CliOptions,
) -> Result<()> {
    let open = || SqliteMailStore::open(path, key, FtsTokenizerMode::default());
    if opts.with_slow_baseline {
        // INVARIANT: Reopen clears SQLite's connection cache, not the OS page cache.
        for sample in 0..5 {
            let (ms, count) = measure(&open()?, &query, neighbors)?;
            println!("{name},reopened,{sample},{ms:.6},{count}");
        }
        let store = open()?;
        for _ in 0..5 {
            measure(&store, &query, neighbors)?;
        }
        for sample in 0..SAMPLES {
            let (ms, count) = measure(&store, &query, neighbors)?;
            println!("{name},warm,{sample},{ms:.6},{count}");
        }
        return Ok(());
    }

    let (w0, _) = measure(warm_store, &query, neighbors)?;
    let slow_fast_mode = w0 > 250.0;
    if !slow_fast_mode {
        let _ = measure(warm_store, &query, neighbors)?;
    }
    let target_samples = if slow_fast_mode { 3 } else { SAMPLES };
    let mut samples = Vec::with_capacity(target_samples);
    for sample in 0..target_samples {
        let (ms, count) = measure(warm_store, &query, neighbors)?;
        if opts.verbose {
            println!("{name},warm,{sample},{ms:.6},{count}");
        }
        samples.push(ms);
    }
    if !opts.verbose {
        samples.sort_by(f64::total_cmp);
        println!(
            "{name},warm,{},{:.3},{:.3},{:.3}",
            samples.len(),
            percentile(&samples, 50, 100),
            percentile(&samples, 95, 100),
            *samples.last().unwrap(),
        );
    }
    Ok(())
}

// Independent integer dot products/norms; fixture vectors are never zero.
fn reference_neighbors(account: Option<usize>) -> Vec<String> {
    reference_ids((0..ROWS).filter(|id| account.is_none_or(|a| id % 4 == a)))
}

fn reference_ids(ids: impl Iterator<Item = usize>) -> Vec<String> {
    let query = vector(4242);
    let norm = query.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
    let mut ranked: Vec<_> = ids
        .map(|id| {
            let values = vector(id);
            let dot = values
                .iter()
                .zip(&query)
                .map(|(&a, &b)| i64::from(a) * i64::from(b))
                .sum::<i64>();
            let length = values.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
            (
                1.0 - dot as f64 / (length * norm).sqrt(),
                format!("message-{id}"),
            )
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, id)| id).collect()
}

fn profile_cache(path: &std::path::Path, key: &DatabaseKey) -> Result<()> {
    for kib in [2048, 16384, 65536] {
        let conn = open_encrypted_connection(path, key)?;
        conn.pragma_update(None, "cache_size", -kib)?;
        for (name,sql) in [
            ("vector-topk","SELECT message_id FROM vec_messages WHERE embedding_int8 MATCH vec_int8(?1) AND k=201"),
            ("fts-rank","SELECT rowid FROM messages_fts WHERE messages_fts MATCH 'newsletter' AND ?1 IS NOT NULL ORDER BY rank,rowid LIMIT 200"),
            ("summary-hydration","SELECT id,subject,sender FROM messages WHERE rowid BETWEEN 4200 AND 4399 AND ?1 IS NOT NULL"),
        ] {
            let mut stmt=conn.prepare(sql)?;
            for sample in 0..8 {
                let start=Instant::now();
                let count=stmt.query_map(params![int8_slice_to_bytes(&vector(4242))], |_|Ok(()))?
                    .collect::<std::result::Result<Vec<_>,_>>()?.len();
                let ms=start.elapsed().as_secs_f64()*1000.;
                println!("# cache_profile,{kib},{name},{sample},{ms:.6},{count},{}",rss()?);
            }
        }
    }
    Ok(())
}

fn rss() -> Result<String> {
    Ok(std::fs::read_to_string("/proc/self/status")?
        .lines()
        .find(|line| line.starts_with("VmRSS:"))
        .unwrap_or("VmRSS: unavailable")
        .to_string())
}

fn profile_parallel_reads(path: &std::path::Path, key: &DatabaseKey) -> Result<()> {
    let query = HybridQuery {
        query_text: "newsletter".into(),
        query_embedding_int8: Some(vector(4242)),
        account_filter: None,
        mode: SearchMode::Hybrid,
        limit: 200,
        offset: 0,
    };
    for kib in [16384, 32768, 65536] {
        let conn1 = open_encrypted_connection(path, key)?;
        conn1.pragma_update(None, "cache_size", -kib)?;
        let mut conn2 = open_encrypted_connection(path, key)?;
        conn2.pragma_update(None, "cache_size", -kib)?;

        for sample in 0..5 {
            let start = Instant::now();
            let rows = execute_hybrid_search(&conn1, &query)?;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            println!(
                "# parallel_read_profile,sequential,{kib},{sample},{ms:.6},{},{}",
                rows.len(),
                rss()?
            );
        }

        for sample in 0..5 {
            let start = Instant::now();
            let rows = execute_hybrid_search_parallel(&conn1, &mut conn2, &query)?;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            println!(
                "# parallel_read_profile,concurrent,{kib},{sample},{ms:.6},{},{}",
                rows.len(),
                rss()?
            );
        }
    }
    Ok(())
}

fn sparse_cases(
    path: &std::path::Path,
    key: &DatabaseKey,
    warm_store: &SqliteMailStore,
    neighbors: &[String],
    opts: CliOptions,
) -> Result<()> {
    run_case(
        path,
        key,
        warm_store,
        "Semantic-filtered-page2",
        HybridQuery {
            query_text: String::new(),
            query_embedding_int8: Some(vector(4242)),
            account_filter: Some("account-2".into()),
            mode: SearchMode::Semantic,
            limit: 200,
            offset: 200,
        },
        neighbors,
        opts,
    )?;
    let mut conn = open_encrypted_connection(path, key)?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        "INSERT INTO accounts VALUES ('account-sparse','sparse@example.com','synthetic',0,0),
        ('account-empty','empty@example.com','synthetic',0,0)",
    )?;
    for id in ROWS..ROWS + 31 {
        let name = format!("message-{id}");
        tx.execute("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,
            received_at_epoch_ms,raw_headers,created_at_epoch_ms)
            VALUES (?1,'account-sparse','INBOX',?2,'synthetic','sender@example.org','reader@example.com',?2,'',0)",
            params![name,id+1])?;
        tx.execute(
            "INSERT INTO vec_messages(message_id,account_id,embedding_int8) VALUES (?1,'account-sparse',vec_int8(?2))",
            params![name, int8_slice_to_bytes(&vector(id))],
        )?;
    }
    tx.commit()?;
    drop(conn);
    if !opts.with_slow_baseline {
        warm_store.hydrate_ram_index()?;
    }
    println!(
        "# sparse_phase,total_rows={},sparse_vectors=31,large_account_vectors=25000",
        ROWS + 31
    );
    let sparse = reference_ids(ROWS..ROWS + 31);
    for (label, account, offset, expected) in [
        ("Semantic-sparse", "account-sparse", 0, sparse.as_slice()),
        (
            "Semantic-sparse-page2",
            "account-sparse",
            20,
            sparse.as_slice(),
        ),
        ("Semantic-empty-account", "account-empty", 0, &[]),
    ] {
        run_case(
            path,
            key,
            warm_store,
            label,
            HybridQuery {
                query_text: String::new(),
                query_embedding_int8: Some(vector(4242)),
                account_filter: Some(account.into()),
                mode: SearchMode::Semantic,
                limit: 20,
                offset,
            },
            expected,
            opts,
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let opts = parse_cli_options();
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("synthetic.db");
    // INVARIANT: Generate an ephemeral key at runtime; never persist benchmark credentials.
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let key = DatabaseKey::from_bytes(bytes);
    seed(&path, &key)?;
    if opts.with_slow_baseline {
        profile_cache(&path, &key)?;
        profile_parallel_reads(&path, &key)?;
    }
    let mut header = [0; 16];
    std::fs::File::open(&path)?.read_exact(&mut header)?;
    assert_ne!(&header, b"SQLite format 3\0");
    println!(
        "# rows={ROWS}, dimensions={DIMENSIONS}, database_bytes={}",
        std::fs::metadata(&path)?.len()
    );
    if opts.with_slow_baseline || opts.verbose {
        println!("case,cache,sample,ms,results");
    } else {
        println!("case,cache,samples,p50_ms,p95_ms,max_ms");
    }
    let warm_store = SqliteMailStore::open(&path, &key, FtsTokenizerMode::default())?;
    if !opts.with_slow_baseline {
        warm_store.hydrate_ram_index()?;
    }
    let neighbors = reference_neighbors(None);
    let filtered_neighbors = reference_neighbors(Some(2));
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        for (label, text, filter, offset) in [
            ("rare", "invoice", None, 0),
            ("common", "newsletter", None, 0),
            ("filtered", "newsletter", Some("account-2".to_string()), 0),
            ("page2", "newsletter", None, 200),
        ] {
            run_case(
                &path,
                &key,
                &warm_store,
                &format!("{mode:?}-{label}"),
                HybridQuery {
                    query_text: text.into(),
                    query_embedding_int8: Some(vector(4242)),
                    account_filter: filter,
                    mode,
                    limit: 200,
                    offset,
                },
                if label == "filtered" {
                    &filtered_neighbors
                } else {
                    &neighbors
                },
                opts,
            )?;
        }
    }
    run_case(
        &path,
        &key,
        &warm_store,
        "Lexical-empty-inbox",
        HybridQuery {
            query_text: String::new(),
            query_embedding_int8: None,
            account_filter: None,
            mode: SearchMode::Lexical,
            limit: 200,
            offset: 0,
        },
        &neighbors,
        opts,
    )?;
    sparse_cases(&path, &key, &warm_store, &filtered_neighbors, opts)?;
    println!("# final_process_rss,{}", rss()?);
    Ok(())
}
