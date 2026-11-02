#![forbid(unsafe_code)]
//! Offline scoped-search correctness and timing; no vault/provider bootstrap.
#[path = "../../../tools/performance/mailbox_data.rs"]
mod data;
use data::{Result, ROWS};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use storage::{
    FtsTokenizerMode, HybridQuery, MailboxScope, MailboxView, SearchMode, SqliteMailStore,
};

fn eligible(id: usize, view: Option<&MailboxView>) -> bool {
    match view {
        None => true,
        Some(view) => {
            id % 4 == 2
                && match view {
                    MailboxView::Inbox => data::folder(id) == "INBOX" && !data::active_snooze(id),
                    MailboxView::Starred => data::starred(id),
                    MailboxView::Snoozed => data::active_snooze(id),
                    MailboxView::Folder(folder) => {
                        folder == data::folder(id) || folder == "All Mail"
                    }
                }
        }
    }
}
// Independent integer dot-product oracle, evaluated outside query timing.
fn neighbors(ids: &[usize]) -> Vec<usize> {
    let query = data::vector(4242);
    let norm = query.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
    let mut ranked: Vec<_> = ids
        .iter()
        .map(|&id| {
            let values = data::vector(id);
            let dot = values
                .iter()
                .zip(&query)
                .map(|(&a, &b)| i64::from(a) * i64::from(b))
                .sum::<i64>();
            let length = values.iter().map(|&v| i64::from(v).pow(2)).sum::<i64>() as f64;
            (1.0 - dot as f64 / (length * norm).sqrt(), id)
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, id)| id).collect()
}
fn expected(
    ids: &[usize],
    semantic: &[usize],
    mode: SearchMode,
    blank: bool,
    offset: usize,
) -> Vec<String> {
    let ranked = match mode {
        SearchMode::Lexical if blank => ids.iter().rev().copied().collect(),
        SearchMode::Lexical => ids.to_vec(), // Equal FTS content/length: BM25 ties use insertion rowid.
        SearchMode::Semantic => semantic.to_vec(),
        SearchMode::Hybrid => {
            let mut scores = HashMap::<usize, f64>::new();
            for branch in [ids, semantic] {
                for (rank, &id) in branch.iter().take((offset + 200).max(50)).enumerate() {
                    *scores.entry(id).or_default() += 1.0 / (61.0 + rank as f64);
                }
            }
            let mut scored: Vec<_> = scores.into_iter().collect();
            scored.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
            scored.into_iter().map(|(id, _)| id).collect()
        }
    };
    ranked
        .into_iter()
        .skip(offset)
        .take(200)
        .map(|id| format!("message-{id:06}"))
        .collect()
}
fn run(fixture: &data::Fixture, label: &str, view: Option<MailboxView>) -> Result<()> {
    let scope = view
        .clone()
        .map(|view| MailboxScope::AccountView {
            account_id: "account-2".into(),
            view,
        })
        .unwrap_or(MailboxScope::GlobalAllMail);
    let ids: Vec<_> = (0..ROWS)
        .filter(|&id| eligible(id, view.as_ref()))
        .collect();
    let semantic = neighbors(&ids);
    let path = fixture.directory.path().join("synthetic.db");
    for (mode, blank) in [
        (SearchMode::Lexical, true),
        (SearchMode::Lexical, false),
        (SearchMode::Semantic, false),
        (SearchMode::Hybrid, false),
    ] {
        for offset in [0, 4000] {
            let query = HybridQuery {
                query_text: if blank {
                    String::new()
                } else {
                    "newsletter".into()
                },
                query_embedding_int8: Some(data::vector(4242)),
                account_filter: None,
                mode,
                limit: 200,
                offset,
            };
            let oracle = expected(&ids, &semantic, mode, blank, offset);
            let measure = |store: &SqliteMailStore| -> Result<(f64, usize)> {
                let start = Instant::now();
                let rows = store.search_scoped(&query, &scope)?;
                let ms = start.elapsed().as_secs_f64() * 1000.;
                let actual: Vec<_> = rows.iter().map(|r| r.id.clone()).collect();
                assert_eq!(
                    actual, oracle,
                    "{label} {mode:?} blank={blank} offset={offset}"
                );
                assert_eq!(actual.iter().collect::<HashSet<_>>().len(), actual.len());
                for row in &rows {
                    let id: usize = row.id.trim_start_matches("message-").parse()?;
                    assert!(eligible(id, view.as_ref()));
                    assert_eq!(row.folder, data::folder(id));
                    assert_eq!(row.uid as usize, id + 1);
                }
                Ok((ms, rows.len()))
            };
            for sample in 0..3 {
                let store =
                    SqliteMailStore::open(&path, &fixture.key, FtsTokenizerMode::default())?;
                let (ms, count) = measure(&store)?;
                println!(
                    "{label},{mode:?},{blank},{offset},{},reopened,{sample},{ms:.6},{count}",
                    ids.len()
                );
            }
            let store = SqliteMailStore::open(&path, &fixture.key, FtsTokenizerMode::default())?;
            for _ in 0..5 {
                measure(&store)?;
            }
            for sample in 0..30 {
                let (ms, count) = measure(&store)?;
                println!(
                    "{label},{mode:?},{blank},{offset},{},warm,{sample},{ms:.6},{count}",
                    ids.len()
                );
            }
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let fixture = data::seed()?;
    let path = fixture.directory.path().join("synthetic.db");
    let conn = storage::connection::open_encrypted_connection(&path, &fixture.key)?;
    println!(
        "# rows={ROWS}, dimensions={}, bytes={}, sqlite={}, cipher={}, vec={}",
        data::DIMENSIONS,
        std::fs::metadata(&path)?.len(),
        conn.query_row("SELECT sqlite_version()", [], |r| r.get::<_, String>(0))?,
        conn.query_row("PRAGMA cipher_version", [], |r| r.get::<_, String>(0))?,
        conn.query_row("SELECT vec_version()", [], |r| r.get::<_, String>(0))?
    );
    if std::env::args().nth(1).as_deref() == Some("Plan") {
        // INVARIANT: Diagnostic SQL mirrors the production blank Inbox branch, including
        // its message-ID index requirement. Never tune statistics or indices only in fixtures.
        let mut stmt = conn.prepare("EXPLAIN QUERY PLAN SELECT m.id FROM messages m
            WHERE m.account_id=?1 AND (m.snooze_until_epoch_ms IS NULL OR m.snooze_until_epoch_ms<=?3)
            AND EXISTS (SELECT 1 FROM message_folder_memberships membership INDEXED BY idx_memberships_message_id
            LEFT JOIN message_folder_overrides o ON o.message_id=m.id
            WHERE membership.message_id=m.id AND membership.account_id=m.account_id
            AND COALESCE(o.folder,membership.folder)=?2)
            ORDER BY m.received_at_epoch_ms DESC,m.id LIMIT ?6 OFFSET ?7")?;
        for row in stmt.query_map(
            (
                "account-2",
                "INBOX",
                storage::now_epoch_ms(),
                "",
                "",
                200,
                0,
            ),
            |r| r.get::<_, String>(3),
        )? {
            println!("# plan,{}", row?);
        }
        return Ok(());
    }
    println!("view,mode,blank,offset,eligible,cache,sample,ms,results");
    for (label, view) in [
        ("Global", None),
        ("Inbox", Some(MailboxView::Inbox)),
        ("Starred", Some(MailboxView::Starred)),
        ("Snoozed", Some(MailboxView::Snoozed)),
        ("Archive", Some(MailboxView::Folder("Archive".into()))),
        ("All-label", Some(MailboxView::Folder("All Mail".into()))),
        ("Empty", Some(MailboxView::Folder("Empty".into()))),
    ] {
        if std::env::args()
            .nth(1)
            .is_none_or(|selected| selected == label)
        {
            run(&fixture, label, view)?;
        }
    }
    Ok(())
}
