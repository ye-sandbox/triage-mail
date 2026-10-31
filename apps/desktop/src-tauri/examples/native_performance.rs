#![forbid(unsafe_code)]
//! Isolated native UI fixture: no service bootstrap, vault, accounts or mail workers.
#[tauri::command]
fn performance_report(app: tauri::AppHandle, report: serde_json::Value) {
    println!("{report}");
    if report.get("phase").and_then(|v| v.as_str()) == Some("complete") {
        app.exit(0);
    }
}

fn main() {
    // INVARIANT: This builder must never bootstrap the production vault or mail workers.
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![performance_report])
        .run(tauri::generate_context!(
            "../../../tools/performance/tauri.conf.json"
        ))
        .expect("native performance fixture failed");
}
