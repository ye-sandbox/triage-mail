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
fn main() -> data::Result<()> {
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
    eprintln!(
        "backfill_rows={count},elapsed_ms={:.3}",
        start.elapsed().as_secs_f64() * 1000.0
    );
    store.save_category_override(SenderOverride {
        account_id: "account-0".into(),
        sender: "sender@example.org".into(),
        revision: 0,
        category: Some(TriageCategory::Financial),
    })?;
    store.correct_category("message-000001", 0, Some(TriageCategory::Financial))?;
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
            let mut samples = Vec::new();
            for sample in 0..32 {
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
                if sample >= 2 {
                    samples.push(ms);
                }
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
                samples[14],
                samples[28],
                samples[29]
            );
        }
    }
    Ok(())
}
