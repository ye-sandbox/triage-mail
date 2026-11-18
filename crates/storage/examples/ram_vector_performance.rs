#![forbid(unsafe_code)]
//! Offline 100k-message exact int8 RAM vector accelerator benchmark & oracle parity verifier.
//!
//! Uses only disposable local SQLCipher data (`tools/performance/mailbox_data.rs`).
//! Zero network calls, vault secrets, or real mail.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;

use std::collections::HashSet;
use std::time::Instant;

use storage::categories::SenderOverride;
use storage::ram_vector_index::{RamQueryFilter, RamScopeFilter};
use storage::saved_views::{SavedView, ViewFilters};
use storage::sender_review::{ReviewState, SenderChoice};
use storage::{
    FtsTokenizerMode, HybridQuery, MailboxScope, MailboxView, SearchMode, SqliteMailStore,
};
use triage_ai::TriageCategory;

const RAM_SAMPLES: usize = 30;
const RAM_WARMUP: usize = 2;
const VEC0_SAMPLES: usize = 5;
const VEC0_WARMUP: usize = 1;

fn percentile(sorted: &[f64], num: usize, den: usize) -> f64 {
    let idx = (sorted.len() * num).div_ceil(den).saturating_sub(1);
    sorted[idx.min(sorted.len() - 1)]
}

fn measure_search<F>(
    warmup: usize,
    samples_count: usize,
    mut f: F,
) -> data::Result<(f64, f64, f64, Vec<String>)>
where
    F: FnMut() -> std::result::Result<Vec<storage::MessageRowSummary>, storage::StorageError>,
{
    let mut reference = Vec::new();
    let mut slow_limit: Option<usize> = None;
    for i in 0..warmup {
        let t0 = Instant::now();
        let rows = f()?;
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if ms > 250.0 {
            eprintln!("  warmup {i}: {ms:.1}ms");
        }
        if ms > 5000.0 {
            slow_limit = Some(1);
        } else if ms > 2000.0 && slow_limit.is_none() {
            slow_limit = Some(2);
        }
        let ids: Vec<String> = rows.into_iter().map(|r| r.id).collect();
        if i == 0 {
            reference = ids;
        } else {
            assert_eq!(ids, reference, "warmup query instability");
        }
    }
    let effective_samples = match slow_limit {
        Some(cap) => samples_count.min(cap),
        None => samples_count,
    };
    let mut samples = Vec::with_capacity(effective_samples);
    for i in 0..effective_samples {
        let start = Instant::now();
        let rows = f()?;
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        if ms > 250.0 {
            eprintln!("  sample {i}: {ms:.1}ms");
        }
        let ids: Vec<String> = rows.into_iter().map(|r| r.id).collect();
        if i == 0 && reference.is_empty() {
            reference = ids;
        } else {
            assert_eq!(ids, reference, "warm query instability");
        }
        samples.push(ms);
    }
    samples.sort_by(f64::total_cmp);
    Ok((
        percentile(&samples, 50, 100),
        percentile(&samples, 95, 100),
        *samples.last().unwrap(),
        reference,
    ))
}

fn independent_cosine_oracle(eligible_ids: &[usize], limit: usize, offset: usize) -> Vec<String> {
    let query = data::vector(4242);
    let q_norm = query.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
    let mut ranked: Vec<(f64, usize)> = eligible_ids
        .iter()
        .map(|&id| {
            let v = data::vector(id);
            let dot = v
                .iter()
                .zip(&query)
                .map(|(&a, &b)| i64::from(a) * i64::from(b))
                .sum::<i64>();
            let v_norm = v.iter().map(|&x| i64::from(x).pow(2)).sum::<i64>() as f64;
            (1.0 - (dot as f64 / (v_norm * q_norm).sqrt()), id)
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(_, id)| format!("message-{id:06}"))
        .collect()
}

fn measure_ram_kernel_only(
    store: &SqliteMailStore,
    query_vec: &[i8],
    filter: RamQueryFilter<'_>,
    limit: usize,
    offset: usize,
) -> (f64, f64, f64) {
    for _ in 0..RAM_WARMUP {
        let ids = store
            .ram_rank_only(query_vec, &filter, offset, limit)
            .unwrap()
            .expect("RAM index must be ready");
        assert!(!ids.is_empty());
    }
    let mut samples = Vec::with_capacity(RAM_SAMPLES);
    for _ in 0..RAM_SAMPLES {
        let start = Instant::now();
        let ids = store
            .ram_rank_only(query_vec, &filter, offset, limit)
            .unwrap()
            .expect("RAM index must be ready");
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
        assert!(!ids.is_empty());
    }
    samples.sort_by(f64::total_cmp);
    (
        percentile(&samples, 50, 100),
        percentile(&samples, 95, 100),
        *samples.last().unwrap(),
    )
}

fn main() -> data::Result<()> {
    let fixture = data::seed()?;
    let path = fixture.directory.path().join("synthetic.db");
    let store = SqliteMailStore::open(&path, &fixture.key, FtsTokenizerMode::default())?;

    // Backfill categories + sender override + single correction (matching category_performance.rs)
    while store.backfill_categories(300)? > 0 {}
    store.save_category_override(SenderOverride {
        account_id: "account-0".into(),
        sender: "sender@example.org".into(),
        revision: 0,
        category: Some(TriageCategory::Financial),
    })?;
    store.correct_category("message-000001", 0, Some(TriageCategory::Financial))?;

    // Backfill sender review + set 3 accounts Pending, account-2 Accepted, hide_in_inbox=true
    while store.backfill_sender_review()? > 0 {}
    for id in 0..4 {
        let account = format!("account-{id}");
        let mut prefs = store.review_preferences(&account)?;
        prefs.enabled = true;
        prefs.hide_in_inbox = true;
        store.save_review_preferences(prefs)?;
        let choice = store.list_sender_choices(&account, 0)?.remove(0);
        store.save_sender_choice(SenderChoice {
            state: if id == 2 {
                ReviewState::Accepted
            } else {
                ReviewState::Pending
            },
            ..choice
        })?;
    }
    store.backfill_sender_review()?;

    let query_vec = data::vector(4242);
    let saved_cat_star = store.save_saved_view(SavedView {
        version: 1,
        id: "cat-starred".into(),
        revision: 0,
        name: "Financial Starred".into(),
        query: "newsletter".into(),
        mode: SearchMode::Semantic,
        accounts: vec![],
        scope: None,
        filters: ViewFilters {
            category: Some(TriageCategory::Financial),
            is_starred: Some(true),
            ..Default::default()
        },
    })?;

    // Hydrate RAM index & record footprint + cold hydration duration
    store.clear_ram_index();
    assert!(!store.is_ram_index_ready());
    let hyd_start = Instant::now();
    store.hydrate_ram_index()?;
    let hydration_ms = hyd_start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(store.ram_index_len(), Some(data::ROWS));
    let (vec_bytes, meta_bytes) = store.ram_index_allocated_bytes().unwrap();

    println!(
        "# rows={}, hydration_ms={hydration_ms:.3}, vec_bytes={vec_bytes} ({:.2} MiB), meta_bytes={meta_bytes} ({:.2} MiB), total_mib={:.2}",
        data::ROWS,
        vec_bytes as f64 / (1024.0 * 1024.0),
        meta_bytes as f64 / (1024.0 * 1024.0),
        (vec_bytes + meta_bytes) as f64 / (1024.0 * 1024.0),
    );

    // Measure pure in-RAM vector kernel latency first (100k vectors, excluding SQLite summary hydration)
    let now_ms = storage::now_epoch_ms();
    println!("# Pure RAM rank_filtered kernel (100k vectors, excluding SQLite summary hydration):");
    println!("kernel_scope,offset,k_p50_ms,k_p95_ms,k_max_ms");
    for (label, filter, offset) in [
        ("Global", RamQueryFilter::default(), 0usize),
        ("Global-deep", RamQueryFilter::default(), 4000usize),
        (
            "Inbox(account-2)",
            RamQueryFilter {
                account_id: Some("account-2"),
                scope: Some(RamScopeFilter::Inbox {
                    folder: "INBOX",
                    now_ms,
                    apply_review_hide: true,
                }),
                category: None,
                review_state: None,
            },
            0,
        ),
        (
            "Category(Financial)",
            RamQueryFilter {
                account_id: None,
                scope: None,
                category: Some(TriageCategory::Financial),
                review_state: None,
            },
            0,
        ),
        (
            "Review(All-Pending)",
            RamQueryFilter {
                account_id: None,
                scope: None,
                category: None,
                review_state: Some(ReviewState::Pending),
            },
            0,
        ),
    ] {
        let (p50, p95, max) = measure_ram_kernel_only(&store, &query_vec, filter, 200, offset);
        println!("{label},{offset},{p50:.3},{p95:.3},{max:.3}");
    }

    println!("scope,mode,offset,eligible,vec0_p50_ms,vec0_p95_ms,vec0_max_ms,ram_p50_ms,ram_p95_ms,ram_max_ms,speedup_p95,exact_parity");

    let run_case = |label: &str,
                    mode: SearchMode,
                    offset: usize,
                    eligible: usize,
                    indep_oracle: Option<Vec<String>>,
                    run_fn: &dyn Fn() -> std::result::Result<
        Vec<storage::MessageRowSummary>,
        storage::StorageError,
    >|
     -> data::Result<()> {
        eprintln!("running {label} {mode:?} offset={offset} ...");
        store.clear_ram_index();
        assert!(!store.is_ram_index_ready());
        let (v_p50, v_p95, v_max, vec0_ids) = measure_search(VEC0_WARMUP, VEC0_SAMPLES, run_fn)?;
        if let Some(expected) = &indep_oracle {
            assert_eq!(
                &vec0_ids, expected,
                "vec0 diverged from independent oracle on {label}"
            );
        }
        assert_eq!(
            vec0_ids.iter().collect::<HashSet<_>>().len(),
            vec0_ids.len(),
            "duplicate IDs on {label}"
        );

        store.hydrate_ram_index()?;
        assert!(store.is_ram_index_ready());
        let (r_p50, r_p95, r_max, ram_ids) = measure_search(RAM_WARMUP, RAM_SAMPLES, run_fn)?;
        assert_eq!(
            ram_ids, vec0_ids,
            "RAM index diverged from sqlite-vec oracle on {label} {mode:?} offset={offset}"
        );
        if let Some(expected) = &indep_oracle {
            assert_eq!(
                &ram_ids, expected,
                "RAM diverged from independent oracle on {label}"
            );
        }

        let speedup = v_p95 / r_p95;
        println!(
            "{label},{mode:?},{offset},{eligible},{v_p50:.3},{v_p95:.3},{v_max:.3},{r_p50:.3},{r_p95:.3},{r_max:.3},{speedup:.1}x,true"
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        Ok(())
    };

    // Independent oracle indices for Global, Inbox(account-2), Archive(account-2)
    let global_ids: Vec<usize> = (0..data::ROWS).collect();
    let inbox_ids: Vec<usize> = (0..data::ROWS)
        .filter(|&id| id % 4 == 2 && data::folder(id) == "INBOX" && !data::active_snooze(id))
        .collect();
    let archive_ids: Vec<usize> = (0..data::ROWS)
        .filter(|&id| id % 4 == 2 && data::folder(id) == "Archive")
        .collect();

    // 1. Global Semantic (offset 0 and 4000) & Hybrid (offset 0)
    for (mode, offset, oracle) in [
        (
            SearchMode::Semantic,
            0,
            Some(independent_cosine_oracle(&global_ids, 200, 0)),
        ),
        (
            SearchMode::Semantic,
            4000,
            Some(independent_cosine_oracle(&global_ids, 200, 4000)),
        ),
        (SearchMode::Hybrid, 0, None),
    ] {
        let q = HybridQuery {
            query_text: "newsletter".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: None,
            mode,
            limit: 200,
            offset,
        };
        run_case("Global", mode, offset, data::ROWS, oracle, &|| {
            store.search_scoped(&q, &MailboxScope::GlobalAllMail)
        })?;
    }

    // 2. Account-2 Inbox Semantic (offset 0, 4000) & Hybrid (offset 0)
    let inbox_scope = MailboxScope::AccountView {
        account_id: "account-2".into(),
        view: MailboxView::Inbox,
    };
    for (mode, offset, oracle) in [
        (
            SearchMode::Semantic,
            0,
            Some(independent_cosine_oracle(&inbox_ids, 200, 0)),
        ),
        (
            SearchMode::Semantic,
            4000,
            Some(independent_cosine_oracle(&inbox_ids, 200, 4000)),
        ),
        (SearchMode::Hybrid, 0, None),
    ] {
        let q = HybridQuery {
            query_text: "newsletter".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: None,
            mode,
            limit: 200,
            offset,
        };
        run_case("Inbox", mode, offset, inbox_ids.len(), oracle, &|| {
            store.search_scoped(&q, &inbox_scope)
        })?;
    }

    // 3. Starred, Snoozed, Archive on account-2
    for (label, view, eligible, oracle) in [
        ("Starred", MailboxView::Starred, 31usize, None),
        ("Snoozed", MailboxView::Snoozed, 2500usize, None),
        (
            "Archive",
            MailboxView::Folder("Archive".into()),
            archive_ids.len(),
            Some(independent_cosine_oracle(&archive_ids, 200, 0)),
        ),
    ] {
        let scope = MailboxScope::AccountView {
            account_id: "account-2".into(),
            view,
        };
        let q = HybridQuery {
            query_text: "newsletter".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: None,
            mode: SearchMode::Semantic,
            limit: 200,
            offset: 0,
        };
        run_case(label, SearchMode::Semantic, 0, eligible, oracle, &|| {
            store.search_scoped(&q, &scope)
        })?;
    }

    // 4. Category (Financial = 25,001 eligible) & Category + Starred (32 eligible)
    for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
        let q = HybridQuery {
            query_text: "newsletter".into(),
            query_embedding_int8: Some(query_vec.clone()),
            account_filter: None,
            mode,
            limit: 200,
            offset: 0,
        };
        run_case("Category(Financial)", mode, 0, 25_001, None, &|| {
            store.search_category(&q, None, TriageCategory::Financial)
        })?;
    }
    run_case(
        "Category+Starred",
        SearchMode::Semantic,
        0,
        32,
        None,
        &|| {
            store.search_saved_view(
                &saved_cat_star.id,
                saved_cat_star.revision,
                Some(query_vec.clone()),
                200,
                0,
            )
        },
    )?;

    // 5. Sender Review (All Pending = 75,000 eligible; account-0 Pending = 25,000 eligible)
    for (label, acct, eligible) in [
        ("Review(All-Pending)", None, 75_000usize),
        (
            "Review(Account-0)",
            Some("account-0".to_string()),
            25_000usize,
        ),
    ] {
        for mode in [SearchMode::Semantic, SearchMode::Hybrid] {
            let q = HybridQuery {
                query_text: "newsletter".into(),
                query_embedding_int8: Some(query_vec.clone()),
                account_filter: acct.clone(),
                mode,
                limit: 200,
                offset: 0,
            };
            run_case(label, mode, 0, eligible, None, &|| {
                store.search_sender_review(&q, ReviewState::Pending)
            })?;
        }
    }

    Ok(())
}
