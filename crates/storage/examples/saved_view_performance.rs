#![forbid(unsafe_code)]
//! Offline saved-view timing and independent complete-membership checks.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    saved_views::{SavedView, ViewFilters},
    SearchMode, SqliteMailStore,
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
                    "Usage: saved_view_performance [--with-slow-baseline] [--verbose] [--help]"
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
    if !opts.with_slow_baseline {
        store.hydrate_ram_index()?;
    }
    let _ = opts.verbose;
    println!("mode,accounts,eligible,samples,p50_ms,p95_ms,max_ms");
    for mode in [
        SearchMode::Lexical,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        for selected in [false, true] {
            let view = store.save_saved_view(SavedView {
                version: 1,
                id: format!("{mode:?}-{selected}"),
                revision: 0,
                name: "Synthetic".into(),
                query: if mode == SearchMode::Lexical {
                    String::new()
                } else {
                    "newsletter".into()
                },
                mode,
                accounts: if selected {
                    vec!["account-0".into(), "account-2".into()]
                } else {
                    vec![]
                },
                scope: None,
                filters: ViewFilters {
                    is_starred: Some(true),
                    ..Default::default()
                },
            })?;
            let expected = (0..data::ROWS)
                .filter(|id| data::starred(*id) && (!selected || id % 2 == 0))
                .map(|id| format!("message-{id:06}"))
                .collect::<std::collections::HashSet<_>>();
            let embedding = (mode != SearchMode::Lexical).then(|| data::vector(4242));
            let mut actual = std::collections::HashSet::new();
            for offset in (0..expected.len() + 200).step_by(200) {
                for row in store.search_saved_view(
                    &view.id,
                    view.revision,
                    embedding.clone(),
                    200,
                    offset,
                )? {
                    let id = row.id.strip_prefix("message-").unwrap().parse::<usize>()?;
                    assert_eq!(row.account_id, format!("account-{}", id % 4));
                    assert_eq!(row.folder, data::folder(id));
                    assert_eq!(row.uid, id as u32 + 1);
                    assert!(actual.insert(row.id));
                }
            }
            assert_eq!(actual, expected);
            let run_one = || -> data::Result<f64> {
                let start = Instant::now();
                let rows =
                    store.search_saved_view(&view.id, view.revision, embedding.clone(), 200, 0)?;
                assert_eq!(rows.len(), expected.len().min(200));
                Ok(start.elapsed().as_secs_f64() * 1000.0)
            };
            let w0 = run_one()?;
            let slow_fast_mode = !opts.with_slow_baseline && w0 > 250.0;
            if !slow_fast_mode {
                let _ = run_one()?;
            }
            let target_samples = if slow_fast_mode { 3 } else { 30 };
            let mut times = Vec::with_capacity(target_samples);
            for _ in 0..target_samples {
                times.push(run_one()?);
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},{},{},{:.3},{:.3},{:.3}",
                if selected { "selected" } else { "all" },
                expected.len(),
                times.len(),
                percentile(&times, 50, 100),
                percentile(&times, 95, 100),
                *times.last().unwrap()
            );
        }
    }
    Ok(())
}
