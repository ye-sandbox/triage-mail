#![forbid(unsafe_code)]
//! Offline category timings over the shared encrypted 100k-message fixture.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    categories::SenderOverride,
    saved_views::{SavedView, ViewFilters},
    HybridQuery, SearchMode, SqliteMailStore,
};
use triage_ai::TriageCategory;
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
                    "Usage: category_performance [--with-slow-baseline] [--verbose] [--help]"
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
    let mut count = 0;
    loop {
        let n = store.backfill_categories(300)?;
        count += n;
        if n == 0 {
            break;
        }
    }
    if opts.verbose {
        eprintln!(
            "backfill_rows={count},elapsed_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
    store.save_category_override(SenderOverride {
        account_id: "account-0".into(),
        sender: "sender@example.org".into(),
        revision: 0,
        category: Some(TriageCategory::Financial),
    })?;
    store.correct_category("message-000001", 0, Some(TriageCategory::Financial))?;
    if !opts.with_slow_baseline {
        store.hydrate_ram_index()?;
    }
    println!("mode,workload,eligible,samples,p50_ms,p95_ms,max_ms");
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let query = HybridQuery {
            query_text: if mode == SearchMode::Lexical {
                String::new()
            } else {
                "newsletter".into()
            },
            mode,
            query_embedding_int8: (mode != SearchMode::Lexical).then(|| data::vector(4242)),
            account_filter: None,
            limit: 200,
            offset: 0,
        };
        let view = store.save_saved_view(SavedView {
            version: 1,
            id: format!("category-{mode:?}"),
            revision: 0,
            name: "Synthetic finance".into(),
            query: query.query_text.clone(),
            mode,
            accounts: vec![],
            scope: None,
            filters: ViewFilters {
                category: Some(TriageCategory::Financial),
                is_starred: Some(true),
                ..Default::default()
            },
        })?;
        let expected = (0..data::ROWS)
            .filter(|id| data::starred(*id) && (id % 4 == 0 || *id == 1))
            .map(|id| format!("message-{id:06}"))
            .collect::<std::collections::HashSet<_>>();
        let complete = store.search_saved_view(
            &view.id,
            view.revision,
            query.query_embedding_int8.clone(),
            200,
            0,
        )?;
        assert_eq!(
            complete
                .iter()
                .map(|r| r.id.clone())
                .collect::<std::collections::HashSet<_>>(),
            expected
        );
        for narrow in [false, true] {
            let run_one = || -> data::Result<f64> {
                let start = Instant::now();
                let rows = if narrow {
                    store.search_saved_view(
                        &view.id,
                        view.revision,
                        query.query_embedding_int8.clone(),
                        200,
                        0,
                    )?
                } else {
                    store.search_category(&query, None, TriageCategory::Financial)?
                };
                let ms = start.elapsed().as_secs_f64() * 1000.0;
                assert_eq!(rows.len(), if narrow { expected.len() } else { 200 });
                for row in rows {
                    let id = row.id.strip_prefix("message-").unwrap().parse::<usize>()?;
                    assert!(id % 4 == 0 || id == 1);
                    assert_eq!(row.account_id, format!("account-{}", id % 4));
                    assert_eq!(row.folder, data::folder(id));
                    assert_eq!(row.uid, id as u32 + 1);
                    assert_eq!(row.category_badge.as_deref(), Some("Financial"));
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
                    "category+starred"
                } else {
                    "category"
                },
                if narrow { expected.len() } else { 25001 },
                samples.len(),
                percentile(&samples, 50, 100),
                percentile(&samples, 95, 100),
                *samples.last().unwrap()
            );
        }
    }
    Ok(())
}
