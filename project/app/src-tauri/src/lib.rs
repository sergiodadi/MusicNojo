use std::path::Path;

use music_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, ensure_not_real_collection,
    load_albums, open_readonly, open_writable, resolve_db_path, Album, AlbumFields,
    Connection, ManyToManyInputs, ManyToManyReplaceFlags, RoleInputs, RoleReplaceFlags,
};
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

/// Shared by add/edit/delete: refuses the real collection (same rule as
/// the CLI's ensure_not_real_collection — no override reachable from
/// the GUI, unlike bc_add's CLI which at least requires an explicit
/// flag), backs up, opens a writable connection.
fn open_for_write(path: &str) -> Result<Connection, String> {
    let db_path = resolve_db_path(Path::new(path));
    ensure_not_real_collection(&db_path).map_err(|e| e.to_string())?;
    backup_database(&db_path).map_err(|e| e.to_string())?;
    open_writable(&db_path).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn add_album(
    path: String,
    title: String,
    roles: RoleInputs,
    many: ManyToManyInputs,
    fields: AlbumFields,
    tracks: Vec<String>,
) -> Result<(), String> {
    let conn = open_for_write(&path)?;
    let plan = build_plan(&conn, &title, &roles, &many, &fields, &tracks).map_err(|e| e.to_string())?;
    conn.execute_batch(&plan).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
fn edit_album(
    path: String,
    id: i64,
    title: Option<String>,
    fields: AlbumFields,
    replace: RoleReplaceFlags,
    roles: RoleInputs,
    replace_many: ManyToManyReplaceFlags,
    many: ManyToManyInputs,
) -> Result<(), String> {
    let conn = open_for_write(&path)?;
    let plan = build_edit_plan(
        &conn,
        id,
        title.as_deref(),
        &fields,
        &replace,
        &roles,
        &replace_many,
        &many,
    )
    .map_err(|e| e.to_string())?;
    conn.execute_batch(&plan).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_album(path: String, id: i64) -> Result<(), String> {
    let conn = open_for_write(&path)?;
    let plan = build_delete_plan(&conn, id).map_err(|e| e.to_string())?;
    conn.execute_batch(&plan).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy_dir(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let dst_path = dst.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy_dir(&entry.path(), &dst_path)?;
            } else {
                std::fs::copy(entry.path(), &dst_path)?;
            }
        }
        Ok(())
    }

    /// Exercises add_album/edit_album/delete_album exactly as the IPC
    /// layer would call them (same functions, just invoked directly
    /// instead of through invoke()) against a disposable scratch copy —
    /// never the shared testdata/testdata-rw fixtures or the real file.
    #[test]
    fn add_edit_delete_round_trip_via_app_commands() {
        let source = std::path::Path::new("../../sample-collection/MyCollection.mucp");
        if !source.exists() {
            eprintln!("sample-collection/ absente — test ignoré");
            return;
        }

        let scratch = std::env::temp_dir().join(format!(
            "musicnojo_app_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        copy_dir(source, &scratch).unwrap();
        let path = scratch.to_string_lossy().to_string();

        let mut roles = RoleInputs::default();
        roles.artist = vec!["App Test Artist".to_string()];
        let mut many = ManyToManyInputs::default();
        many.genre = vec!["App Test Genre".to_string()];
        let fields = AlbumFields::default();

        add_album(
            path.clone(),
            "App Test Album".to_string(),
            roles,
            many,
            fields,
            vec!["Track One".to_string()],
        )
        .unwrap();

        let conn = open_readonly(&scratch).unwrap();
        let albums = load_albums(&conn).unwrap();
        let added = albums.iter().find(|a| a.title == "App Test Album").unwrap();
        assert_eq!(added.artists, vec!["App Test Artist".to_string()]);
        let id = added.id;
        drop(conn);

        edit_album(
            path.clone(),
            id,
            Some("App Test Album (renamed)".to_string()),
            AlbumFields::default(),
            RoleReplaceFlags::default(),
            RoleInputs::default(),
            ManyToManyReplaceFlags::default(),
            ManyToManyInputs::default(),
        )
        .unwrap();

        let conn = open_readonly(&scratch).unwrap();
        let albums = load_albums(&conn).unwrap();
        let renamed = albums.iter().find(|a| a.id == id).unwrap();
        assert_eq!(renamed.title, "App Test Album (renamed)");
        // replace flags were all false/default, so the artist survives the edit.
        assert_eq!(renamed.artists, vec!["App Test Artist".to_string()]);
        drop(conn);

        delete_album(path.clone(), id).unwrap();
        let conn = open_readonly(&scratch).unwrap();
        let albums = load_albums(&conn).unwrap();
        assert!(!albums.iter().any(|a| a.id == id));

        // A backup must have been made for each of the 3 writes above.
        let backups = std::fs::read_dir(scratch.join("backups")).unwrap().count();
        assert_eq!(backups, 3);

        let _ = std::fs::remove_dir_all(&scratch);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_albums,
            pick_collection,
            add_album,
            edit_album,
            delete_album,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
