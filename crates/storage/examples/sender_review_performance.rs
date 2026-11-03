#![forbid(unsafe_code)]
//! Disposable encrypted local review fixture; no provider or delivery calls.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    sender_review::{ReviewState, SenderChoice},
    HybridQuery, SearchMode, SqliteMailStore,
};
fn main() -> data::Result<()> {
    let fixture = data::seed()?;
    let store = SqliteMailStore::open(
        fixture.directory.path().join("synthetic.db"),
        &fixture.key,
        Default::default(),
    )?;
    let start = Instant::now();
    let mut rows = 0;
    loop {
        let n = store.backfill_sender_review()?;
        rows += n;
        if n == 0 {
            break;
        }
    }
    eprintln!(
        "backfill_rows={rows},elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    for id in 0..4 {
        let account = format!("account-{id}");
        let mut p = store.review_preferences(&account)?;
        p.enabled = true;
        p.hide_in_inbox = true;
        store.save_review_preferences(p)?;
        let choice = store.list_sender_choices(&account, 0)?.remove(0);
        store.save_sender_choice(SenderChoice {
            state: if id == 1 {
                ReviewState::Accepted
            } else {
                ReviewState::Pending
            },
            ..choice
        })?;
    }
    store.backfill_sender_review()?;
    println!("mode,workload,eligible,samples,p50_ms,p95_ms,max_ms");
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        for narrow in [false, true] {
            let query = HybridQuery {
                query_text: if mode == SearchMode::Lexical {
                    String::new()
                } else {
                    "newsletter".into()
                },
                mode,
                query_embedding_int8: (mode != SearchMode::Lexical).then(|| data::vector(4242)),
                account_filter: narrow.then(|| "account-0".into()),
                limit: 200,
                offset: 0,
            };
            let mut samples = vec![];
            for sample in 0..32 {
                let start = Instant::now();
                let rows = store.search_sender_review(&query, ReviewState::Pending)?;
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(rows.len(), 200);
                for row in rows {
                    let id = row.id.strip_prefix("message-").unwrap().parse::<usize>()?;
                    assert!(id % 4 != 1 && (!narrow || id % 4 == 0));
                    assert_eq!(row.account_id, format!("account-{}", id % 4));
                    assert_eq!(row.folder, data::folder(id));
                    assert_eq!(row.uid, id as u32 + 1);
                }
                if sample >= 2 {
                    samples.push(ms);
                }
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},{},30,{:.3},{:.3},{:.3}",
                if narrow {
                    "account-review"
                } else {
                    "all-review"
                },
                if narrow { 25000 } else { 75000 },
                samples[14],
                samples[28],
                samples[29]
            );
        }
    }
    Ok(())
}
