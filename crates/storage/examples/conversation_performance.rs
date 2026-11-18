#![forbid(unsafe_code)]
//! Aggregate offline grouped-search measurements over a disposable encrypted fixture.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    conversations::{ConversationQuery, ConversationSource},
    HybridQuery, MailboxScope, MailboxView, SearchMode, SqliteMailStore,
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
                    "Usage: conversation_performance [--with-slow-baseline] [--verbose] [--help]"
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
    let path = fixture.directory.path().join("synthetic.db");
    {
        let conn = storage::connection::open_encrypted_connection(&path, &fixture.key)?;
        // INVARIANT: Each missing ancestor is unique to one account and 100-row cohort.
        conn.execute("UPDATE messages SET message_id_header=printf('%d-%s@example.org',uid,account_id),in_reply_to=printf('root-%d-%s@example.org',(uid-1)/100,account_id)",[])?;
    }
    let store = SqliteMailStore::open(&path, &fixture.key, Default::default())?;
    let start = Instant::now();
    let mut ticks = 0;
    let mut slowest = 0f64;
    while !store.conversation_progress()?.ready {
        let tick = Instant::now();
        store.backfill_conversations()?;
        slowest = slowest.max(tick.elapsed().as_secs_f64() * 1000.);
        ticks += 1;
    }
    if opts.verbose {
        eprintln!(
            "rows={},ticks={ticks},backfill_ms={:.3},slowest_tick_ms={slowest:.3}",
            data::ROWS,
            start.elapsed().as_secs_f64() * 1000.
        );
    }
    if !opts.with_slow_baseline {
        store.hydrate_ram_index()?;
    }
    println!("mode,workload,samples,p50_ms,p95_ms,max_ms");
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
                query_embedding_int8: (mode != SearchMode::Lexical).then(|| data::vector(4242)),
                account_filter: None,
                mode,
                limit: 200,
                offset: 0,
            };
            let context = ConversationQuery {
                source: ConversationSource::Messages,
                scope: narrow.then(|| MailboxScope::AccountView {
                    account_id: "account-0".into(),
                    view: MailboxView::Inbox,
                }),
                generation: None,
            };
            let run_one = || -> data::Result<f64> {
                let start = Instant::now();
                let result = store.search_conversations(&query, &context)?;
                let elapsed = start.elapsed().as_secs_f64() * 1000.;
                assert_eq!(result.rows.len(), 200);
                for row in result.rows {
                    assert_eq!(row.total_count, 25);
                    if narrow {
                        assert!((17..=18).contains(&row.matching_count));
                    } else {
                        assert_eq!(row.matching_count, 25);
                    }
                }
                Ok(elapsed)
            };
            let w0 = run_one()?;
            let slow_fast_mode = !opts.with_slow_baseline && w0 > 250.0;
            if !slow_fast_mode {
                let _ = run_one()?;
            }
            let target_samples = if slow_fast_mode { 3 } else { 10 };
            let mut samples = Vec::with_capacity(target_samples);
            for _ in 0..target_samples {
                samples.push(run_one()?);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},{},{:.3},{:.3},{:.3}",
                if narrow { "account-inbox" } else { "global" },
                samples.len(),
                percentile(&samples, 50, 100),
                percentile(&samples, 95, 100),
                *samples.last().unwrap()
            );
        }
    }
    Ok(())
}
