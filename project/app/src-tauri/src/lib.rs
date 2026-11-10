use std::path::Path;

use music_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, ensure_not_real_collection,
    load_albums, open_readonly, open_writable, preview_add_album, resolve_db_path, set_cover,
    AddAlbumPreview, Album, AlbumFields, Connection, ManyToManyInputs, ManyToManyReplaceFlags,
    RoleInputs, RoleReplaceFlags,
};
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};
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
    /// The actual collection path used — the frontend starts with no
    /// path (loads the bundled sample-collection by default) and must
    /// learn the real one to pass to add_album/edit_album/delete_album,
    /// which require a concrete path, not null.
    path: String,
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

/// The bare path-resolution rule ("null means the bundled sample-
/// collection"), split out from list_albums so it's testable without a
/// Tauri AppHandle at all. Canonicalized (no leftover `..` segments,
/// e.g. the sample-collection path is built from CARGO_MANIFEST_DIR +
/// "../../sample-collection/...") — the asset-protocol scope granted
/// for cover images below must exactly match how the asset request's
/// path is represented, and an unresolved `..` is a real way for a
/// "same directory, different spelling" mismatch to silently fail that
/// check (found this: covers never rendered from the default
/// collection, with no error anywhere — a blocked asset load doesn't
/// throw in the frontend, it just never resolves the <img>).
fn resolve_collection_path(path: Option<String>) -> std::path::PathBuf {
    let resolved = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(sample_collection_path);
    resolved.canonicalize().unwrap_or(resolved)
}

/// The app's HTML shell has no build step to burn a version string into it,
/// so the frontend asks for it over IPC instead — same pattern as BookNojo's
/// own `app_version` command.
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn list_albums(app: tauri::AppHandle, path: Option<String>) -> Result<AlbumsResponse, String> {
    let path = resolve_collection_path(path);
    let db_path = resolve_db_path(&path);
    grant_asset_scope_for_collection(&app, &db_path);

    let conn = open_readonly(&path).map_err(|e| e.to_string())?;
    let albums = load_albums(&conn).map_err(|e| e.to_string())?;
    let images_dir = db_path
        .parent()
        .map(|p| p.join("Images").to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(AlbumsResponse {
        albums,
        images_dir,
        path: path.to_string_lossy().to_string(),
    })
}

/// Native picker for a .mucp package. Uses the dialog plugin's async
/// callback form, not `blocking_pick_file()` — that one blocks the
/// calling thread waiting on the native panel, which on macOS must run
/// on the main thread; called from a Tauri command (off the main
/// thread) this deadlocks instead of returning (observed: the window
/// never regains control, spinning cursor). The chosen path is sent to
/// the frontend as a "collection-picked" event, since the picker itself
/// is asynchronous — same pattern as BookNojo's own import dialog.
///
/// `pick_file`, not `pick_folder`: macOS's Launch Services knows `.mucp`
/// is a package (registered by Music Collector.app itself) and
/// NSOpenPanel treats a recognized package as a *file*, not a folder —
/// a folder-only picker (`canChooseFiles: false`, what `pick_folder`
/// sets) grays it out entirely (user-reported: "MyCollection.mucp ...
/// c'est grisé"). `pick_file` with an extension filter lets the package
/// be selected as a single atomic item, matching how Finder normally
/// handles package-style bundles — same fix BookNojo already uses for
/// its own `.bkcp` picker.
#[tauri::command]
fn pick_collection(app: tauri::AppHandle) {
    app.dialog()
        .file()
        .set_title("Open a Music Collector collection (.mucp)")
        .add_filter("Collection", &["mucp"])
        .pick_file(move |file| {
            if let Some(file) = file {
                let _ = app.emit("collection-picked", file.to_string());
            }
        });
}

/// Native picker for a cover image, same async-callback shape as
/// `pick_collection` (and for the same reason — a blocking pick from a
/// command deadlocks). `set_cover` already does its own real-collection
/// refusal and backup internally (see its own doc comment), so this
/// calls it directly rather than going through `open_for_write` — same
/// pattern `mc_add set-cover`'s CLI already uses. Only the front cover
/// is offered here: the entry point is the stats panel's "no cover"
/// list, which only cares about the front cover albums are missing.
/// Result goes back as an event (not the command's return value) since
/// the picker is asynchronous; `cover-set-error`'s payload is the plain
/// error string Tauri's IPC can carry.
#[tauri::command]
fn pick_and_set_cover(app: tauri::AppHandle, path: String, id: i64) {
    app.dialog()
        .file()
        .set_title("Choose a cover image")
        .add_filter("Image", &["jpg", "jpeg", "png", "gif", "bmp"])
        .pick_file(move |file| {
            let Some(file) = file else { return };
            let Some(local_path) = file.as_path() else { return };
            match set_cover(Path::new(&path), id, Some(local_path), None) {
                Ok(()) => {
                    let _ = app.emit("cover-set", id);
                }
                Err(e) => {
                    let _ = app.emit("cover-set-error", e.to_string());
                }
            }
        });
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

/// Read-only half of the "Add Album" flow, called before `add_album`
/// actually writes anything — lets the form show which artists/genres/
/// etc. would be newly created, same purpose as BookNojo's own
/// `prepare_add_book`.
#[tauri::command]
fn prepare_add_album(
    path: String,
    title: String,
    roles: RoleInputs,
    many: ManyToManyInputs,
    fields: AlbumFields,
) -> Result<AddAlbumPreview, String> {
    let db_path = resolve_db_path(Path::new(&path));
    let conn = open_readonly(&db_path).map_err(|e| e.to_string())?;
    preview_add_album(&conn, &title, &roles, &many, &fields).map_err(|e| e.to_string())
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

    /// Reproduces the exact bug the user hit: on first load the
    /// frontend calls list_albums with path=null (meaning "use the
    /// bundled sample-collection"). Before the fix, AlbumsResponse had
    /// no `path` field, so the frontend kept `currentPath = null` and
    /// later calls to add_album (which requires a concrete String)
    /// failed IPC deserialization of that null. This only checks that
    /// list_albums(None) now echoes back a real, non-empty, existing
    /// path — it never writes to the committed sample-collection/, to
    /// avoid mutating a tracked fixture from a test run.
    #[test]
    fn list_albums_with_no_path_returns_a_usable_path() {
        let resolved = resolve_collection_path(None);
        assert!(resolved.exists());
        assert_eq!(resolved, sample_collection_path().canonicalize().unwrap());
        // No leftover ".." segments — see resolve_collection_path's doc
        // comment for why this specifically matters (asset-protocol
        // scope matching, not just cosmetic).
        assert!(!resolved.to_string_lossy().contains(".."));

        // The resolved path must round-trip through load_albums the
        // same way list_albums itself does.
        let conn = open_readonly(&resolved).unwrap();
        assert!(!load_albums(&conn).unwrap().is_empty());
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
/// Native macOS menu bar — same shape as BookNojo's own `build_menu`.
fn build_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    #[cfg(target_os = "macos")]
    let app_menu = SubmenuBuilder::new(app, "MusicNojo")
        .about(None)
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .text("open_collection", "Open Collection…")
        .separator()
        .close_window()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;

    // A submenu titled exactly "Help" is swallowed entirely by macOS/
    // Tauri in this Tauri version — "MusicNojo Help" sidesteps it,
    // same finding/workaround as BookNojo's own menu.
    let help_menu = SubmenuBuilder::new(app, "MusicNojo Help")
        .text("about_musicnojo", "About MusicNojo")
        .build()?;

    let mut builder = MenuBuilder::new(app);
    #[cfg(target_os = "macos")]
    {
        builder = builder.item(&app_menu);
    }
    builder
        .item(&file_menu)
        .item(&edit_menu)
        .item(&window_menu)
        .item(&help_menu)
        .build()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_version,
            list_albums,
            pick_collection,
            pick_and_set_cover,
            prepare_add_album,
            add_album,
            edit_album,
            delete_album,
        ])
        .setup(|app| {
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_collection" => pick_collection(app.clone()),
            "about_musicnojo" => {
                app.dialog()
                    .message(format!("MusicNojo\nVersion {}", env!("CARGO_PKG_VERSION")))
                    .title("About MusicNojo")
                    .show(|_| {});
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
