use std::path::Path;

use scanner_core::{RelocateReport, ScanOptions, ScanResult};

/// Runs the (blocking, CPU/IO-heavy) scan off the main thread so the UI stays
/// responsive, streaming throttled "scan-progress" events while it walks.
#[tauri::command]
async fn scan_path(app: tauri::AppHandle, path: String) -> Result<ScanResult, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        scanner_core::scan_with_progress(Path::new(&path), &ScanOptions::default(), &mut |p| {
            let _ = app.emit("scan-progress", &p);
        })
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Moves a directory to another drive and leaves a junction at the old path.
/// The engine keeps the original as a backup until the junction is verified.
/// Streams throttled "relocate-progress" events to the UI while it runs.
#[tauri::command]
async fn relocate_dir(
    app: tauri::AppHandle,
    src: String,
    dest_parent: String,
) -> Result<RelocateReport, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        scanner_core::relocate_dir_with_progress(
            Path::new(&src),
            Path::new(&dest_parent),
            &mut |p| {
                let _ = app.emit("relocate-progress", &p);
            },
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn list_drives() -> Vec<String> {
    ('A'..='Z')
        .filter_map(|letter| {
            let drive = format!("{letter}:\\");
            Path::new(&drive).exists().then_some(drive)
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan_path, list_drives, relocate_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
