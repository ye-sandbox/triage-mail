#![forbid(unsafe_code)]
//! Aggregate offline grouped-search measurements over a disposable encrypted fixture.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use std::time::Instant;
use storage::{
    conversations::{ConversationQuery, ConversationSource},
    HybridQuery, MailboxScope, MailboxView, SearchMode, SqliteMailStore,
};
fn main() -> data::Result<()> {
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
    eprintln!(
        "rows={},ticks={ticks},backfill_ms={:.3},slowest_tick_ms={slowest:.3}",
        data::ROWS,
        start.elapsed().as_secs_f64() * 1000.
    );
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
            let mut samples = Vec::new();
            for sample in 0..12 {
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
                if sample >= 2 {
                    samples.push(elapsed);
                }
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "{mode:?},{},10,{:.3},{:.3},{:.3}",
                if narrow { "account-inbox" } else { "global" },
                samples[4],
                samples[9],
                samples[9]
            );
        }
    }
    Ok(())
}
