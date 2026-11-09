use music_collector_export::{load_albums, open_readonly, Album};
use tauri_plugin_dialog::DialogExt;

/// Path to the bundled fictional sample-collection, used when the user
/// hasn't opened a real collection yet.
fn sample_collection_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sample-collection/MyCollection.mucp")
}

#[tauri::command]
fn list_albums(path: Option<String>) -> Result<Vec<Album>, String> {
    let path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(sample_collection_path);
    let conn = open_readonly(&path).map_err(|e| e.to_string())?;
    load_albums(&conn).map_err(|e| e.to_string())
}

/// Native folder picker for a .mucp package. Blocking, called from the
/// UI thread via invoke() — fine here since this is a short-lived modal
/// dialog, not a long-running operation.
#[tauri::command]
fn pick_collection(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .set_title("Open a Music Collector collection (.mucp)")
        .blocking_pick_folder()
        .map(|p| p.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![list_albums, pick_collection])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
