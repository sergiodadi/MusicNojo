use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use music_collector_export::{
    backup_database, build_delete_plan, build_edit_plan, build_plan, ensure_not_real_collection,
    is_music_collector_running, looks_like_real_collection, open_readonly, resolve_db_path,
    set_cover, AlbumFields, ManyToManyInputs, ManyToManyReplaceFlags, RoleInputs, RoleReplaceFlags,
    ENT_ALBUM,
};
use rusqlite::{Connection, OpenFlags};

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
        /// Allows writing even if --db looks like the real Music Collector
        /// collection — a typed "OUI" confirmation is still required.
        #[arg(long)]
        force: bool,
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
        /// Allows writing even if --db looks like the real Music Collector
        /// collection — a typed "OUI" confirmation is still required.
        #[arg(long)]
        force: bool,
    },
    /// Bootstraps a new, independent ".mnc" database from an existing
    /// collection (database + Images/ folder) — a one-time copy, never
    /// re-run against a destination that already exists. The result
    /// works exactly like a .mucp (same schema, just a different
    /// extension to mark it as MusicNojo's own going forward, not
    /// Music Collector's) — see CLI.md § "Why an independent database".
    Import {
        /// The existing .mucp (or any other extension — only the
        /// presence of Music_data inside matters) to copy from.
        #[arg(long)]
        source: PathBuf,
        /// Defaults to "collection/<source name>.mnc" in the current directory.
        #[arg(long)]
        dest: Option<PathBuf>,
    },
}

/// Second, independent gate on top of `--force` : même avec le flag, une
/// vraie écriture sur ce qui ressemble à la vraie collection exige de
/// taper "OUI" de façon interactive. Prend `input`/`out` plutôt que de
/// toucher stdin/stdout directement pour que la logique du prompt soit
/// testable sans terminal ni fichier réel — même forme que celle de
/// BookNojo.
fn confirm_real_write<R: std::io::BufRead, W: std::io::Write>(
    input: &mut R,
    out: &mut W,
    db_path: &Path,
    real_prefix: &Path,
) -> Result<()> {
    writeln!(
        out,
        "⚠️  ATTENTION : {} se trouve sous {} — ceci est la VRAIE collection Music Collector, pas une copie de test.",
        db_path.display(),
        real_prefix.display()
    )?;
    writeln!(out, "   Une sauvegarde horodatée sera prise avant d'écrire, mais cette opération")?;
    writeln!(out, "   modifie directement vos données réelles.")?;
    write!(out, "   Tapez exactement OUI pour continuer (autre chose annule) : ")?;
    out.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    if answer.trim() == "OUI" {
        Ok(())
    } else {
        bail!("Confirmation refusée ou incorrecte (réponse attendue : OUI, exactement) — aucune écriture effectuée.");
    }
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

/// Default destination when `--dest` is omitted: `collection/<source
/// basename>.mnc` in the current directory — `.mnc` is the chosen
/// extension for the new, independent "MusicNojo" database going
/// forward, same convention as BookNojo's own `.col`.
fn default_import_dest(source: &Path) -> PathBuf {
    let name = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "MyCollection".to_string());
    PathBuf::from("collection").join(format!("{name}.mnc"))
}

fn cmd_import(source: PathBuf, dest: Option<PathBuf>) -> Result<()> {
    let source_db = resolve_db_path(&source);
    let source_pkg = source_db
        .parent()
        .context("dossier source introuvable (attendu : un .mucp contenant Music_data)")?;

    let dest = dest.unwrap_or_else(|| default_import_dest(&source));
    if dest.exists() {
        bail!(
            "{} existe déjà — l'import ne sert qu'à amorcer une nouvelle base, une seule \
             fois, pour ne jamais en écraser une existante. Choisissez un autre chemin \
             avec --dest, ou supprimez-le d'abord si vous êtes sûr.",
            dest.display()
        );
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("création de {}", parent.display()))?;
    }

    println!("Import de {} vers {}...", source_pkg.display(), dest.display());
    let status = std::process::Command::new("cp")
        .args(["-R", &source_pkg.to_string_lossy(), &dest.to_string_lossy()])
        .status()
        .context("exécution de cp -R")?;
    if !status.success() {
        bail!("échec de la copie (cp -R)");
    }
    std::process::Command::new("chmod")
        .args(["-R", "u+w", &dest.to_string_lossy()])
        .status()
        .context("exécution de chmod -R u+w")?;

    let dest_music_data = dest.join("Music_data");
    let conn = Connection::open_with_flags(&dest_music_data, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("ouverture de {}", dest_music_data.display()))?;
    let album_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_ENT = ?1",
        [ENT_ALBUM],
        |row| row.get(0),
    )?;

    println!();
    println!("✅ Import terminé : {album_count} album(s) dans {}.", dest.display());
    println!("   Cette base est désormais indépendante : plus aucune synchronisation");
    println!("   automatique avec la source. mc_add/l'application peuvent continuer à");
    println!("   écrire dedans normalement — c'est un .mucp comme un autre, seule");
    println!("   l'extension a changé.");

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
            let (plan_sql, album_id) = build_plan(&conn, &title, &roles, &many, &fields, &tracks)?;
            emit_plan(&plan_sql, out, &db_path)?;
            println!(
                "-- ID (Z_PK) prévu pour cet album : {album_id} (utilisable ensuite avec 'set-cover')."
            );
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
        Command::Commit { db, plan, force } => {
            match (force, looks_like_real_collection(&db)) {
                (false, Some(real_prefix)) => {
                    bail!(
                        "{} se trouve sous {} — ça ressemble à la vraie collection.\n\
                         Cette commande écrit réellement dans la base : utilisez une copie dédiée,\n\
                         ou passez --force si vous savez vraiment ce que vous faites.",
                        db.display(),
                        real_prefix.display()
                    );
                }
                (true, Some(real_prefix)) => {
                    let stdin = std::io::stdin();
                    let mut locked = stdin.lock();
                    let mut stdout = std::io::stdout();
                    confirm_real_write(&mut locked, &mut stdout, &db, &real_prefix)?;
                }
                (_, None) => {}
            }
            if is_music_collector_running()? {
                bail!("Music Collector est actuellement lancé — fermez l'application avant d'écrire.");
            }
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
            force,
        } => {
            match (force, looks_like_real_collection(&db)) {
                (false, Some(real_prefix)) => {
                    bail!(
                        "{} se trouve sous {} — ça ressemble à la vraie collection.\n\
                         Cette commande écrit réellement dans la base : utilisez une copie dédiée,\n\
                         ou passez --force si vous savez vraiment ce que vous faites.",
                        db.display(),
                        real_prefix.display()
                    );
                }
                (true, Some(real_prefix)) => {
                    let stdin = std::io::stdin();
                    let mut locked = stdin.lock();
                    let mut stdout = std::io::stdout();
                    confirm_real_write(&mut locked, &mut stdout, &db, &real_prefix)?;
                }
                (_, None) => {}
            }
            set_cover(&db, id, front.as_deref(), back.as_deref(), force)?;
        }
        Command::Import { source, dest } => {
            cmd_import(source, dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_import_dest_uses_the_mnc_extension_under_collection() {
        assert_eq!(
            default_import_dest(Path::new(
                "/Users/x/Documents/Collectorz/Music Collector/MyCollection.mucp"
            )),
            PathBuf::from("collection/MyCollection.mnc")
        );
        assert_eq!(
            default_import_dest(Path::new("Already.mnc")),
            PathBuf::from("collection/Already.mnc")
        );
    }

    fn confirm(answer: &str) -> Result<()> {
        let mut input = std::io::Cursor::new(answer.as_bytes());
        let mut out = Vec::new();
        confirm_real_write(&mut input, &mut out, Path::new("/x/Documents/Collectorz/y"), Path::new("/x/Documents/Collectorz"))
    }

    #[test]
    fn confirm_real_write_accepts_only_an_exact_oui() {
        assert!(confirm("OUI\n").is_ok());
        // Les espaces en fin de ligne venant du terminal sont tolérés...
        assert!(confirm("OUI \n").is_ok());
        // ...mais tout le reste — y compris une minuscule ou un simple
        // Entrée — doit refuser. C'est le garde-fou de sécurité : il ne
        // doit jamais réussir par accident.
        assert!(confirm("oui\n").is_err());
        assert!(confirm("yes\n").is_err());
        assert!(confirm("\n").is_err());
        assert!(confirm("").is_err());
    }

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
        let (plan_sql, _album_id) = build_plan(
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
