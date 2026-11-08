use music_collector_export::{load_albums, open_readonly, Album};

/// Path to the collection to open. For now, always the bundled
/// fictional sample-collection (library-view milestone only) — a real
/// "open my collection" picker comes later.
fn sample_collection_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sample-collection/MyCollection.mucp")
}

#[tauri::command]
fn list_albums() -> Result<Vec<Album>, String> {
    let path = sample_collection_path();
    let conn = open_readonly(&path).map_err(|e| e.to_string())?;
    load_albums(&conn).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![list_albums])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
