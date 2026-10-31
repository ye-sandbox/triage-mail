#![forbid(unsafe_code)]
//! Offline, disposable SQLCipher benchmark; see docs/performance/native-validation.md.
use std::{collections::HashSet, error::Error, io::Read, time::Instant};

use rusqlite::params;
use storage::{
    connection::open_encrypted_connection, migrations::SchemaMigrator, search::int8_slice_to_bytes,
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
            "INSERT INTO vec_messages(message_id, embedding_int8) VALUES (?1, vec_int8(?2))",
        )?;
        for id in 0..ROWS {
            let name = format!("message-{id}");
            let topic = if id % 100 == 0 {
                "invoice"
            } else {
                "newsletter"
            };
            message.execute(params![
                name,
                format!("account-{}", id % 4),
                id + 1,
                format!("Synthetic {topic} {id}"),
                format!(
                    "{topic} receipt shipment {}",
                    "local synthetic content ".repeat(40)
                )
            ])?;
            embedding.execute(params![name, int8_slice_to_bytes(&vector(id))])?;
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
        query.limit,
        "all benchmark cases have enough eligible rows"
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

fn run_case(
    path: &std::path::Path,
    key: &DatabaseKey,
    name: &str,
    query: HybridQuery,
    neighbors: &[String],
) -> Result<()> {
    let open = || SqliteMailStore::open(path, key, FtsTokenizerMode::default());
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
    Ok(())
}

// Independent integer dot products/norms; fixture vectors are never zero.
fn reference_neighbors(account: Option<usize>) -> Vec<String> {
    let query = vector(4242);
    let norm = query.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
    let mut ranked: Vec<_> = (0..ROWS)
        .filter(|id| account.is_none_or(|a| id % 4 == a))
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

fn main() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("synthetic.db");
    // INVARIANT: Generate an ephemeral key at runtime; never persist benchmark credentials.
    let mut bytes = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let key = DatabaseKey::from_bytes(bytes);
    seed(&path, &key)?;
    let mut header = [0; 16];
    std::fs::File::open(&path)?.read_exact(&mut header)?;
    assert_ne!(&header, b"SQLite format 3\0");
    println!(
        "# rows={ROWS}, dimensions={DIMENSIONS}, database_bytes={}",
        std::fs::metadata(&path)?.len()
    );
    println!("case,cache,sample,ms,results");
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
            )?;
        }
    }
    run_case(
        &path,
        &key,
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
    )?;
    Ok(())
}
