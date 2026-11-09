use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use music_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, ensure_not_real_collection,
    open_readonly, resolve_db_path, set_cover, AlbumFields, ManyToManyInputs,
    ManyToManyReplaceFlags, RoleInputs, RoleReplaceFlags,
};
use rusqlite::Connection;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compute the SQL to insert a new album, without writing anything
    AddPlan {
        /// Path to a MyCollection.mucp package (never the live file — see the safety gate)
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        title: String,
        #[command(flatten)]
        roles: RoleInputs,
        #[command(flatten)]
        many: ManyToManyInputs,
        #[command(flatten)]
        fields: AlbumFields,
        /// One track title per occurrence, in order
        #[arg(long = "track")]
        tracks: Vec<String>,
        /// Write the plan's SQL to this file instead of stdout
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the SQL to modify an existing album (by id), without writing anything
    EditPlan {
        #[arg(long)]
        db: PathBuf,
        /// The album's Z_PK (see mc_export's JSON output, field "id")
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[command(flatten)]
        fields: AlbumFields,
        #[command(flatten)]
        replace: RoleReplaceFlags,
        #[command(flatten)]
        roles: RoleInputs,
        #[command(flatten)]
        replace_many: ManyToManyReplaceFlags,
        #[command(flatten)]
        many: ManyToManyInputs,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compute the SQL to delete an album (by id) and everything specific
    /// to it (its artists/genres links, discs, tracks) — never the shared
    /// lookups (Format/Label/.../ArtistPerson), which other albums may
    /// still reference.
    DeletePlan {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        id: i64,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Execute a plan file's SQL against the database, inside one transaction
    Commit {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        plan: PathBuf,
    },
    /// Copies a local image into the package's Images/ folder (named like
    /// the app's own covers: "<id>_f_<slug>.<ext>" / "<id>_b_<slug>.<ext>")
    /// and points the album at it. Writes immediately — unlike the plan/
    /// commit split above, this has no row-allocation ambiguity to review
    /// first, just a file copy plus one column each.
    SetCover {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        id: i64,
        /// Local image file to use as the front cover
        #[arg(long)]
        front: Option<PathBuf>,
        /// Local image file to use as the back cover
        #[arg(long)]
        back: Option<PathBuf>,
    },
}

fn open_readonly_with_path(db: &Path) -> Result<(PathBuf, Connection)> {
    ensure_not_real_collection(db)?;
    let db_path = resolve_db_path(db);
    let conn = open_readonly(db)?;
    Ok((db_path, conn))
}

fn emit_plan(plan_sql: &str, out: Option<PathBuf>, db_path: &Path) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(&path, plan_sql)
                .with_context(|| format!("écriture de {}", path.display()))?;
            println!("Plan écrit dans: {}", path.display());
        }
        None => println!("{plan_sql}"),
    }
    println!(
        "(Aucune écriture dans {} — ce SQL n'a pas été exécuté.)",
        db_path.display()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Command::AddPlan {
            db,
            title,
            roles,
            many,
            fields,
            tracks,
            out,
        } => {
            let (db_path, conn) = open_readonly_with_path(&db)?;
            let plan_sql = build_plan(&conn, &title, &roles, &many, &fields, &tracks)?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::EditPlan {
            db,
            id,
            title,
            fields,
            replace,
            roles,
            replace_many,
            many,
            out,
        } => {
            let (db_path, conn) = open_readonly_with_path(&db)?;
            let plan_sql = build_edit_plan(
                &conn,
                id,
                title.as_deref(),
                &fields,
                &replace,
                &roles,
                &replace_many,
                &many,
            )?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::DeletePlan { db, id, out } => {
            let (db_path, conn) = open_readonly_with_path(&db)?;
            let plan_sql = build_delete_plan(&conn, id)?;
            emit_plan(&plan_sql, out, &db_path)?;
        }
        Command::Commit { db, plan } => {
            ensure_not_real_collection(&db)?;
            let db_path = resolve_db_path(&db);
            let plan_sql = std::fs::read_to_string(&plan)
                .with_context(|| format!("lecture de {}", plan.display()))?;

            let backup = backup_database(&db_path)?;
            let conn = Connection::open(&db_path)
                .with_context(|| format!("ouverture de {}", db_path.display()))?;
            // plan_sql already wraps its statements in BEGIN;/COMMIT;
            conn.execute_batch(&plan_sql)
                .with_context(|| "exécution du plan")?;
            println!("Plan exécuté dans: {}", db_path.display());
            println!("Sauvegarde: {}", backup.display());
        }
        Command::SetCover {
            db,
            id,
            front,
            back,
        } => {
            set_cover(&db, id, front.as_deref(), back.as_deref())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_writable(path: &Path) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        for entry in walk(path)? {
            let mode = if entry.is_dir() { 0o755 } else { 0o644 };
            std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(mode))?;
        }
        Ok(())
    }

    fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut out = vec![dir.to_path_buf()];
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                out.extend(walk(&entry.path())?);
            } else {
                out.push(entry.path());
            }
        }
        Ok(out)
    }

    fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
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

    #[test]
    fn add_edit_and_delete_round_trip() {
        let source = Path::new("testdata/MyCollection.mucp");
        if !source.exists() {
            eprintln!(
                "testdata/MyCollection.mucp absent (copie personnelle, non committée) — test ignoré"
            );
            return;
        }

        let scratch = std::env::temp_dir().join(format!(
            "mc_add_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        copy_dir(source, &scratch).unwrap();
        // testdata/ is read-only (chmod 444/555); std::fs::copy preserves
        // that, so make this disposable scratch copy writable again.
        make_writable(&scratch).unwrap();
        // chmod/ownership issues aside, a disposable scratch copy is never
        // the shared testdata/ or testdata-rw/ fixtures, nor the real file.
        ensure_not_real_collection(&scratch).unwrap();

        let db_path = scratch.join("Music_data");
        let read_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let fields = AlbumFields {
            format: Some("CD".to_string()),
            ..Default::default()
        };
        let roles = RoleInputs {
            artist: vec!["Artiste de test".to_string()],
            composer: vec!["Compositeur de test".to_string()],
            ..Default::default()
        };
        let many = ManyToManyInputs {
            genre: vec!["Genre de test".to_string()],
            tag: vec!["Tag de test".to_string()],
            ..Default::default()
        };
        let plan_sql = build_plan(
            &read_conn,
            "Test unitaire mc_add",
            &roles,
            &many,
            &fields,
            &["Piste unique".to_string()],
        )
        .unwrap();
        drop(read_conn);

        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&plan_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let (album_id, title): (i64, String) = verify_conn
            .query_row(
                "SELECT Z_PK, ZTITLE FROM ZCOLLECTIBLE WHERE ZTITLE = 'Test unitaire mc_add'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(title, "Test unitaire mc_add");
        let composer_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM ZCOMPOSER WHERE ZALBUM = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(composer_count, 1);
        let tag_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_8TAGS WHERE Z_8COLLECTIBLES = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count, 1);

        // Edit: rename, replace the genre/composer/tag lists.
        let edit_sql = build_edit_plan(
            &verify_conn,
            album_id,
            Some("Titre modifié"),
            &AlbumFields::default(),
            &RoleReplaceFlags {
                replace_composers: true,
                ..Default::default()
            },
            &RoleInputs {
                composer: vec!["Autre compositeur".to_string()],
                ..Default::default()
            },
            &ManyToManyReplaceFlags {
                replace_genres: true,
                replace_tags: true,
                ..Default::default()
            },
            &ManyToManyInputs {
                genre: vec!["Autre genre".to_string()],
                tag: vec![],
                ..Default::default()
            },
        )
        .unwrap();
        drop(verify_conn);
        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&edit_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let renamed: String = verify_conn
            .query_row(
                "SELECT ZTITLE FROM ZCOLLECTIBLE WHERE Z_PK = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(renamed, "Titre modifié");
        let genre_count: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_10GENRES WHERE Z_10ALBUMS2 = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(genre_count, 1);
        let tag_count_after: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM Z_8TAGS WHERE Z_8COLLECTIBLES = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tag_count_after, 0);
        let new_composer_name: String = verify_conn
            .query_row(
                "SELECT p.ZDISPLAYNAME FROM ZCOMPOSER c \
                 JOIN ZCOMPOSERPERSON cp ON cp.Z_PK = c.ZCOMPOSERPERSON \
                 JOIN ZPERSONDELEGATE p ON p.Z_PK = cp.ZDELEGATE \
                 WHERE c.ZALBUM = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(new_composer_name, "Autre compositeur");

        // Delete: the album and its track should both disappear.
        let delete_sql = build_delete_plan(&verify_conn, album_id).unwrap();
        drop(verify_conn);
        let write_conn = Connection::open(&db_path).unwrap();
        write_conn.execute_batch(&delete_sql).unwrap();
        drop(write_conn);

        let verify_conn =
            Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let remaining: i64 = verify_conn
            .query_row(
                "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1",
                [album_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);

        let _ = std::fs::remove_dir_all(&scratch);
    }
}
