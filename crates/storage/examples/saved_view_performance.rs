#![forbid(unsafe_code)]
//! Offline saved-view timing and independent complete-membership checks.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    saved_views::{SavedView, ViewFilters},
    SearchMode, SqliteMailStore,
};

fn main() -> data::Result<()> {
    let fixture = data::seed()?;
    let store = SqliteMailStore::open(
        fixture.directory.path().join("synthetic.db"),
        &fixture.key,
        Default::default(),
    )?;
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
            let mut times = Vec::new();
            for sample in 0..32 {
                let start = Instant::now();
                let rows =
                    store.search_saved_view(&view.id, view.revision, embedding.clone(), 200, 0)?;
                assert_eq!(rows.len(), expected.len().min(200));
                if sample >= 2 {
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},{},{}, {:.3},{:.3},{:.3}",
                if selected { "selected" } else { "all" },
                expected.len(),
                times.len(),
                times[14],
                times[28],
                times[29]
            );
        }
    }
    Ok(())
}
