#![forbid(unsafe_code)]
//! Explicit offline synthetic 256-message indexing measurement; never uses a user's database.
use std::{error::Error, io::Read, path::PathBuf, time::Instant};
use storage::{
    connection::open_encrypted_connection, migrations::SchemaMigrator, DatabaseKey, SqliteMailStore,
};
use triage_ai::{
    onnx::{OnnxEmbeddingProvider, GENERATION},
    EmbeddingModelConfig, EmbeddingProvider,
};
fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("pass the provisioned test data directory")?,
    );
    let provider = OnnxEmbeddingProvider::new(EmbeddingModelConfig::with_data_dir(
        triage_ai::embeddings::MODEL_ALL_MINILM_L6_V2,
        root,
    ));
    let start = Instant::now();
    provider.prepare()?;
    eprintln!(
        "cold_prepare_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("synthetic.db");
    let mut raw = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut raw)?;
    let key = DatabaseKey::from_bytes(raw);
    let mut conn = open_encrypted_connection(&path, &key)?;
    SchemaMigrator::new(Default::default()).migrate(&mut conn)?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO accounts VALUES('a','reader@example.com','synthetic',0,0)",
        [],
    )?;
    for n in 0..256 {
        tx.execute("INSERT INTO messages(id,account_id,folder,uid,subject,sender,recipients,received_at_epoch_ms,raw_headers,text_body,created_at_epoch_ms) VALUES (?1,'a','INBOX',?2,'Synthetic invoice','sender@example.org','reader@example.com',0,'',?3,0)",rusqlite::params![format!("synthetic-{n:04}"),n+1,if n%2==0 {"Your invoice payment is due tomorrow."} else {"A fatura vence amanhã. O pagamento está pendente."}])?;
    }
    tx.commit()?;
    drop(conn);
    let store = SqliteMailStore::open(&path, &key, Default::default())?;
    store.begin_embedding_generation(GENERATION, false)?;
    let start = Instant::now();
    let mut count = 0;
    let mut samples = Vec::new();
    loop {
        let jobs = store.embedding_work(GENERATION, 8)?;
        if jobs.is_empty() {
            break;
        }
        for job in jobs {
            let t = Instant::now();
            let vector = provider.embed_int8(&job.text)?;
            samples.push(t.elapsed().as_secs_f64() * 1000.);
            assert!(store.finish_embedding(GENERATION, &job, Some(&vector))?);
            count += 1;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    samples.sort_by(f64::total_cmp);
    eprintln!(
        "rows={count},index_ms={:.3},rows_per_second={:.3},warm_p50_ms={:.3},warm_p95_ms={:.3}",
        elapsed * 1000.,
        count as f64 / elapsed,
        samples[samples.len() / 2],
        samples[(samples.len() * 95) / 100]
    );
    assert_eq!(store.embedding_progress()?.pending, 0);
    Ok(())
}
