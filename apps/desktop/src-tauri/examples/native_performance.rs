#![forbid(unsafe_code)]
//! Isolated native fixture: disposable SQLCipher, no production vault or mail workers.
#[path = "../../../../tools/performance/mailbox_data.rs"]
mod data;
use storage::{FtsTokenizerMode, MailboxScope, SqliteMailStore};
use triage_mail_desktop_lib::dto::SearchQueryDto;

struct FixtureState {
    store: SqliteMailStore,
    _fixture: data::Fixture,
}
#[tauri::command]
async fn fixture_search(
    state: tauri::State<'_, FixtureState>,
    query: SearchQueryDto,
) -> Result<Vec<storage::MessageRowSummary>, String> {
    let scope = query.scope.clone().unwrap_or(MailboxScope::GlobalAllMail);
    let embedding = (query.mode != storage::SearchMode::Lexical).then(|| data::vector(4242));
    let query = query.into_hybrid_query(embedding)?;
    // INVARIANT: This fixture measures real IPC/storage but deliberately excludes embedding inference.
    state
        .store
        .search_scoped(&query, &scope)
        .map_err(|_| "Fixture search failed".into())
}
#[tauri::command]
fn fixture_folders(
    state: tauri::State<'_, FixtureState>,
    account_id: String,
) -> Result<Vec<storage::FolderRecord>, String> {
    state
        .store
        .list_folders(&account_id)
        .map_err(|_| "Fixture catalog failed".into())
}
#[tauri::command]
fn performance_report(app: tauri::AppHandle, report: serde_json::Value) {
    println!("{report}");
    if report.get("phase").and_then(|v| v.as_str()) == Some("complete") {
        app.exit(0);
    }
}
fn main() -> data::Result<()> {
    let fixture = data::seed()?;
    let store = SqliteMailStore::open(
        fixture.directory.path().join("synthetic.db"),
        &fixture.key,
        FtsTokenizerMode::default(),
    )?;
    // INVARIANT: Never bootstrap the production service, secret store or account workers here.
    tauri::Builder::default()
        .manage(FixtureState {
            store,
            _fixture: fixture,
        })
        .invoke_handler(tauri::generate_handler![
            performance_report,
            fixture_search,
            fixture_folders
        ])
        .run(tauri::generate_context!(
            "../../../tools/performance/tauri.conf.json"
        ))?;
    Ok(())
}
