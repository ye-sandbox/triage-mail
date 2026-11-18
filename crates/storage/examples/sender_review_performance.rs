#![forbid(unsafe_code)]
//! Disposable encrypted local review fixture; no provider or delivery calls.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    sender_review::{ReviewState, SenderChoice},
    HybridQuery, SearchMode, SqliteMailStore,
};
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
                eprintln!(
                    "Usage: sender_review_performance [--with-slow-baseline] [--verbose] [--help]"
                );
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

fn main() -> data::Result<()> {
    let opts = parse_cli_options();
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
    if opts.verbose {
        eprintln!(
            "backfill_rows={rows},elapsed_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
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
    if !opts.with_slow_baseline {
        store.hydrate_ram_index()?;
    }
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
            let run_one = || -> data::Result<f64> {
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
                Ok(ms)
            };
            let w0 = run_one()?;
            let slow_fast_mode = !opts.with_slow_baseline && w0 > 250.0;
            if !slow_fast_mode {
                let _ = run_one()?;
            }
            let target_samples = if slow_fast_mode { 3 } else { 30 };
            let mut samples = Vec::with_capacity(target_samples);
            for _ in 0..target_samples {
                samples.push(run_one()?);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},{},{},{:.3},{:.3},{:.3}",
                if narrow {
                    "account-review"
                } else {
                    "all-review"
                },
                if narrow { 25000 } else { 75000 },
                samples.len(),
                percentile(&samples, 50, 100),
                percentile(&samples, 95, 100),
                *samples.last().unwrap()
            );
        }
    }
    Ok(())
}
