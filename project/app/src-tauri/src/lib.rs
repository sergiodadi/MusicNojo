use std::path::Path;

use music_collector_export::{load_albums, open_readonly, resolve_db_path, Album};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

/// Path to the bundled fictional sample-collection, used when the user
/// hasn't opened a real collection yet.
fn sample_collection_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sample-collection/MyCollection.mucp")
}

#[derive(serde::Serialize)]
struct AlbumsResponse {
    albums: Vec<Album>,
    /// Absolute path to this collection's Images/ folder, so the
    /// frontend can turn a bare cover filename into a full path before
    /// handing it to convertFileSrc. Not part of the shared library's
    /// Album type, which deliberately keeps just the filename (same
    /// shape mc_export's CSV/JSON already expose).
    images_dir: String,
}

/// Also grants the asset protocol (used for `convertFileSrc`-loaded
/// cover images) access to this collection's own Images/ folder — a
/// static scope in tauri.conf.json would have to name every possible
/// collection location in advance, which only works on the developer's
/// own machine. Granting it here, at the moment a collection is
/// actually opened, works for any location. Best-effort: a failure
/// here only means covers won't load, not that the collection itself
/// failed to open.
fn grant_asset_scope_for_collection<R: tauri::Runtime>(app: &tauri::AppHandle<R>, db_path: &Path) {
    if let Some(collection_dir) = db_path.parent() {
        let _ = app
            .asset_protocol_scope()
            .allow_directory(collection_dir.join("Images"), false);
    }
}

#[tauri::command]
fn list_albums(app: tauri::AppHandle, path: Option<String>) -> Result<AlbumsResponse, String> {
    let path = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(sample_collection_path);
    let db_path = resolve_db_path(&path);
    grant_asset_scope_for_collection(&app, &db_path);

    let conn = open_readonly(&path).map_err(|e| e.to_string())?;
    let albums = load_albums(&conn).map_err(|e| e.to_string())?;
    let images_dir = db_path
        .parent()
        .map(|p| p.join("Images").to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(AlbumsResponse { albums, images_dir })
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
