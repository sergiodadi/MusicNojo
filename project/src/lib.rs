use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
pub use rusqlite::Connection;
use serde::Serialize;

pub mod online_search;

// Core Data entity ids for the Collectible-family hierarchy
// (Collectible -> MusicCollectible -> Album/Disc/Header/SubTrack/Track).
// These ids are stable across collections created by the same Music
// Collector version, so they are hardcoded below rather than resolved
// at runtime.
pub const ENT_ALBUM: i64 = 10;
const ENT_DISC: i64 = 11;
const ENT_TRACK: i64 = 14;

#[derive(Serialize, Clone)]
pub struct Track {
    pub position: Option<i64>,
    pub title: String,
    pub length: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct Disc {
    pub index: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Clone)]
pub struct Credit {
    pub name: String,
    // ZCREDIT.ZROLEID: no corresponding label exists anywhere in
    // Music_data itself — left unresolved, not mapped to text. Flagged
    // separately rather than guessed.
    pub role_id: Option<i64>,
}

#[derive(Serialize)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artists: Vec<String>,
    pub composers: Vec<String>,
    pub conductors: Vec<String>,
    pub musicians: Vec<String>,
    pub orchestras: Vec<String>,
    pub choruses: Vec<String>,
    pub credits: Vec<Credit>,
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub extras: Vec<String>,
    pub studios: Vec<String>,
    pub label: Option<String>,
    pub format: Option<String>,
    pub country: Option<String>,
    pub condition: Option<String>,
    pub packaging: Option<String>,
    pub vinyl_color: Option<String>,
    pub spars: Option<String>,
    pub owner: Option<String>,
    pub location: Option<String>,
    pub collection: Option<String>,
    /// When this album was added to the collection — "YYYY-MM-DD HH:MM:SS",
    /// UTC (ZDATEADDED is a Core Data reference-date timestamp: seconds
    /// since 2001-01-01, converted in SQL rather than pulling in a
    /// date/time crate just for this one field).
    pub date_added: Option<String>,
    pub release_year: Option<String>,
    pub rating: Option<i64>,
    pub notes: Option<String>,
    pub upc: Option<String>,
    pub front_cover: Option<String>,
    pub back_cover: Option<String>,
    pub thumbnail: Option<String>,
    pub backdrop: Option<String>,
    pub track_count: i64,
    pub discs: Vec<Disc>,
}

/// Opens a `.mucp` package (or its `Music_data` file directly) read-only.
pub fn open_readonly(db: &Path) -> Result<Connection> {
    let db_path = resolve_db_path(db);
    Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("ouverture de {}", db_path.display()))
}

/// Opens a `.mucp` package (or its `Music_data` file directly) for
/// writing. Exposed so callers (the app) don't need their own `rusqlite`
/// dependency just to open a connection.
pub fn open_writable(db: &Path) -> Result<Connection> {
    let db_path = resolve_db_path(db);
    Connection::open(&db_path).with_context(|| format!("ouverture de {}", db_path.display()))
}

pub fn resolve_db_path(path: &Path) -> std::path::PathBuf {
    if path.is_dir() {
        path.join("Music_data")
    } else {
        path.to_path_buf()
    }
}

// ------------------------------------------------- Import (bootstrap a .mnj)

/// Result of a successful [`import_collection_to`] call.
#[derive(Debug)]
pub struct ImportOutcome {
    pub dest: PathBuf,
    pub album_count: i64,
}

/// Computes the fixed destination for a new `.mnj` database bootstrapped
/// from `source` via [`import_collection_to`]: always
/// `~/Documents/MusicNojo/<source file stem>.mnj` — a fixed, automatic
/// location (no "Save As" dialog), same deliberate choice Serge made for
/// BookNojo's "Import Collection…" menu. Pure path computation: it does not
/// create any directory or touch the filesystem itself —
/// `import_collection_to` creates `dest`'s parent directory when it runs.
pub fn default_mnj_destination(source: &Path) -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .context("variable d'environnement HOME introuvable — impossible de déterminer ~/Documents/MusicNojo")?;
    let name = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "MyCollection".to_string());
    Ok(PathBuf::from(home)
        .join("Documents")
        .join("MusicNojo")
        .join(format!("{name}.mnj")))
}

/// Bootstraps a brand-new, independent collection at `dest` by copying the
/// whole document package found at `source` (a `.mucp`/`.mnj` path, or a
/// directory already containing `Music_data`). Meant to run **once** per
/// new database: refuses outright if `dest` already exists, rather than
/// ever overwriting one. `source` itself is only ever read, never
/// modified — this is a plain recursive copy (`cp -R`), not a live link.
///
/// Shared by `mc_add import` (the CLI, which pre-checks `dest.exists()`
/// itself first, to show its own message) and MusicNojo's "File > Import
/// Collection…" menu (which has no CLI-style message of its own to
/// preserve, and relies on this function's own checks). Mirrors BookNojo's
/// `import_collection_to` exactly — same plan, applied to this app.
pub fn import_collection_to(source: &Path, dest: &Path) -> Result<ImportOutcome> {
    let source_db = resolve_db_path(source);
    let source_pkg = source_db
        .parent()
        .context("dossier source introuvable (attendu : un .mucp/.mnj contenant Music_data)")?;

    if dest.exists() {
        bail!(
            "{} existe déjà — l'import ne sert qu'à amorcer une nouvelle base, une seule \
             fois, pour ne jamais en écraser une existante.",
            dest.display()
        );
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("création de {}", parent.display()))?;
    }

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
    let conn = Connection::open_with_flags(&dest_music_data, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("ouverture de {}", dest_music_data.display()))?;
    let album_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_ENT = ?1", [ENT_ALBUM], |row| row.get(0))?;

    Ok(ImportOutcome { dest: dest.to_path_buf(), album_count })
}

/// Bulk-loaded lookups, keyed so that assembling each album is a set of
/// in-memory map lookups instead of running fresh SQL per album (an
/// earlier version ran ~20 small queries per album — ~45,000 total for
/// a 2300-album collection; this version runs one query per lookup kind).
struct Lookups {
    simple: HashMap<&'static str, HashMap<i64, String>>,
    collections: HashMap<i64, String>,
    // Owner is a *person* (resolves through ZPERSONDELEGATE, same as
    // Artist/Composer/...), not a category value like Format/Label —
    // ZLOOKUPITEMDELEGATE has no ZOWNER column at all. Kept separate
    // from `simple` rather than forced into that shape.
    owners: HashMap<i64, String>,
    roles: HashMap<&'static str, HashMap<i64, Vec<String>>>,
    credits: HashMap<i64, Vec<Credit>>,
    many_to_many: HashMap<&'static str, HashMap<i64, Vec<String>>>,
    discs_by_album: HashMap<i64, Vec<(i64, Option<String>)>>,
    tracks_by_disc: HashMap<i64, Vec<Track>>,
}

const SIMPLE_LOOKUPS: &[&str] = &[
    "ZFORMAT",
    "ZCOUNTRY",
    "ZLABEL",
    "ZCONDITION",
    "ZPACKAGING",
    "ZVINYLCOLOR",
    "ZSPARS",
    "ZLOCATION",
];

// (role_table, person_table, person_fk) — the person_fk column name
// always matches the person_table name in this schema.
const ROLE_TABLES: &[(&str, &str, &str)] = &[
    ("ZARTIST", "ZARTISTPERSON", "ZARTISTPERSON"),
    ("ZCOMPOSER", "ZCOMPOSERPERSON", "ZCOMPOSERPERSON"),
    ("ZCONDUCTOR", "ZCONDUCTORPERSON", "ZCONDUCTORPERSON"),
    ("ZMUSICIAN", "ZMUSICIANPERSON", "ZMUSICIANPERSON"),
    ("ZORCHESTRA", "ZORCHESTRAPERSON", "ZORCHESTRAPERSON"),
    ("ZCHORUS", "ZCHORUSPERSON", "ZCHORUSPERSON"),
];

// (name, join_table, album_col, target_col, target_table)
const MANY_TO_MANY: &[(&str, &str, &str, &str, &str)] = &[
    ("genres", "Z_10GENRES", "Z_10ALBUMS2", "Z_28GENRES", "ZGENRE"),
    ("tags", "Z_8TAGS", "Z_8COLLECTIBLES", "Z_54TAGS", "ZTAG"),
    ("extras", "Z_10EXTRAS", "Z_10ALBUMS1", "Z_26EXTRAS", "ZEXTRA"),
    (
        "studios",
        "Z_10STUDIOS",
        "Z_10ALBUMS4",
        "Z_53STUDIOS",
        "ZSTUDIO",
    ),
];

fn group_by<K: std::hash::Hash + Eq, V>(pairs: Vec<(K, V)>) -> HashMap<K, Vec<V>> {
    let mut map: HashMap<K, Vec<V>> = HashMap::new();
    for (k, v) in pairs {
        map.entry(k).or_default().push(v);
    }
    map
}

fn load_simple_lookup_map(conn: &Connection, table: &str) -> Result<HashMap<i64, String>> {
    let sql = format!(
        "SELECT t.Z_PK, l.ZDISPLAYNAME FROM {table} t \
         JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

fn load_owner_map(conn: &Connection) -> Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare(
        "SELECT o.Z_PK, p.ZDISPLAYNAME FROM ZOWNER o \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = o.ZDELEGATE",
    )?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

fn load_role_map(
    conn: &Connection,
    role_table: &str,
    person_table: &str,
    person_fk: &str,
) -> Result<HashMap<i64, Vec<String>>> {
    let sql = format!(
        "SELECT r.ZALBUM, p.ZDISPLAYNAME FROM {role_table} r \
         JOIN {person_table} pt ON pt.Z_PK = r.{person_fk} \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = pt.ZDELEGATE \
         WHERE r.ZALBUM IS NOT NULL ORDER BY r.ZALBUM, r.ZRANK"
    );
    let mut stmt = conn.prepare(&sql)?;
    let pairs: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(group_by(pairs))
}

fn load_credit_map(conn: &Connection) -> Result<HashMap<i64, Vec<Credit>>> {
    let mut stmt = conn.prepare(
        "SELECT c.ZALBUM, p.ZDISPLAYNAME, c.ZROLEID FROM ZCREDIT c \
         JOIN ZCREDITPERSON cp ON cp.Z_PK = c.ZCREDITPERSON \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = cp.ZDELEGATE \
         WHERE c.ZALBUM IS NOT NULL ORDER BY c.ZALBUM, c.ZRANK",
    )?;
    let pairs: Vec<(i64, Credit)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                Credit {
                    name: row.get(1)?,
                    role_id: row.get(2)?,
                },
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(group_by(pairs))
}

fn load_many_to_many_map(
    conn: &Connection,
    join_table: &str,
    album_col: &str,
    target_col: &str,
    target_table: &str,
) -> Result<HashMap<i64, Vec<String>>> {
    let sql = format!(
        "SELECT j.{album_col}, l.ZDISPLAYNAME FROM {join_table} j \
         JOIN {target_table} t ON t.Z_PK = j.{target_col} \
         JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE"
    );
    let mut stmt = conn.prepare(&sql)?;
    let pairs: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(group_by(pairs))
}

fn load_collection_map(conn: &Connection) -> Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare("SELECT Z_PK, ZTITLE FROM ZCOLLECTION")?;
    let map = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

#[allow(clippy::type_complexity)]
fn load_discs_by_album(conn: &Connection) -> Result<HashMap<i64, Vec<(i64, Option<String>)>>> {
    let mut stmt = conn.prepare(
        "SELECT ZALBUM, Z_PK, ZDISCINDEX FROM ZCOLLECTIBLE \
         WHERE Z_ENT = ?1 ORDER BY ZALBUM, ZDISCINDEX",
    )?;
    let pairs: Vec<(i64, (i64, Option<String>))> = stmt
        .query_map([ENT_DISC], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                (row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(group_by(pairs))
}

fn load_tracks_by_disc(conn: &Connection) -> Result<HashMap<i64, Vec<Track>>> {
    let mut stmt = conn.prepare(
        "SELECT ZDISC1, ZPOSITION, ZTITLE, ZLENGTHMINS, ZLENGTHSECS FROM ZCOLLECTIBLE \
         WHERE Z_ENT = ?1 ORDER BY ZDISC1, ZPOSITION",
    )?;
    let pairs: Vec<(i64, Track)> = stmt
        .query_map([ENT_TRACK], |row| {
            let disc_pk: i64 = row.get(0)?;
            let position: Option<i64> = row.get(1)?;
            let title: String = row.get::<_, Option<String>>(2)?.unwrap_or_default();
            let mins: Option<String> = row.get(3)?;
            let secs: Option<String> = row.get(4)?;
            let length = match (mins, secs) {
                (Some(m), Some(s)) if !m.is_empty() || !s.is_empty() => Some(format!("{m}:{s}")),
                _ => None,
            };
            Ok((
                disc_pk,
                Track {
                    position,
                    title,
                    length,
                },
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(group_by(pairs))
}

fn load_lookups(conn: &Connection) -> Result<Lookups> {
    let mut simple = HashMap::new();
    for table in SIMPLE_LOOKUPS {
        simple.insert(*table, load_simple_lookup_map(conn, table)?);
    }

    let mut roles = HashMap::new();
    for (role_table, person_table, person_fk) in ROLE_TABLES {
        roles.insert(
            *role_table,
            load_role_map(conn, role_table, person_table, person_fk)?,
        );
    }

    let mut many_to_many = HashMap::new();
    for (name, join_table, album_col, target_col, target_table) in MANY_TO_MANY {
        let mut map = load_many_to_many_map(conn, join_table, album_col, target_col, target_table)?;
        // Join-table order isn't meaningful (these are sets, not ranked
        // lists like the role tables) — sort for deterministic output.
        for names in map.values_mut() {
            names.sort();
        }
        many_to_many.insert(*name, map);
    }

    Ok(Lookups {
        simple,
        collections: load_collection_map(conn)?,
        owners: load_owner_map(conn)?,
        roles,
        credits: load_credit_map(conn)?,
        many_to_many,
        discs_by_album: load_discs_by_album(conn)?,
        tracks_by_disc: load_tracks_by_disc(conn)?,
    })
}

impl Lookups {
    fn simple(&self, table: &str, pk: Option<i64>) -> Option<String> {
        pk.and_then(|pk| self.simple.get(table).and_then(|m| m.get(&pk)).cloned())
    }

    fn role(&self, table: &str, album_pk: i64) -> Vec<String> {
        self.roles
            .get(table)
            .and_then(|m| m.get(&album_pk))
            .cloned()
            .unwrap_or_default()
    }

    fn many(&self, name: &str, album_pk: i64) -> Vec<String> {
        self.many_to_many
            .get(name)
            .and_then(|m| m.get(&album_pk))
            .cloned()
            .unwrap_or_default()
    }

    fn discs(&self, album_pk: i64) -> Vec<Disc> {
        self.discs_by_album
            .get(&album_pk)
            .map(|discs| {
                discs
                    .iter()
                    .map(|(disc_pk, index)| Disc {
                        index: index.clone(),
                        tracks: self.tracks_by_disc.get(disc_pk).cloned().unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

struct AlbumRow {
    id: i64,
    title: String,
    format_pk: Option<i64>,
    country_pk: Option<i64>,
    label_pk: Option<i64>,
    condition_pk: Option<i64>,
    packaging_pk: Option<i64>,
    vinyl_color_pk: Option<i64>,
    spars_pk: Option<i64>,
    owner_pk: Option<i64>,
    location_pk: Option<i64>,
    collection_pk: Option<i64>,
    date_added: Option<String>,
    release_year: Option<String>,
    rating: Option<i64>,
    notes: Option<String>,
    upc: Option<String>,
    front_cover: Option<String>,
    back_cover: Option<String>,
    thumbnail: Option<String>,
    backdrop: Option<String>,
}

pub fn load_albums(conn: &Connection) -> Result<Vec<Album>> {
    let lookups = load_lookups(conn)?;

    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZFORMAT, ZCOUNTRY, ZLABEL, ZCONDITION, ZPACKAGING, \
                ZVINYLCOLOR, ZSPARS, ZOWNER, ZLOCATION, ZCOLLECTION, ZRELEASEYEAR, \
                ZMYRATING, ZNOTES, ZUPC, ZFRONTCOVER, ZBACKCOVER, ZTHUMBFILEPATH, \
                ZBACKDROP, \
                datetime(ZDATEADDED + 978307200, 'unixepoch') \
         FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 ORDER BY Z_PK",
    )?;
    let rows: Vec<AlbumRow> = stmt
        .query_map([ENT_ALBUM], |row| {
            Ok(AlbumRow {
                id: row.get(0)?,
                title: row.get(1)?,
                format_pk: row.get(2)?,
                country_pk: row.get(3)?,
                label_pk: row.get(4)?,
                condition_pk: row.get(5)?,
                packaging_pk: row.get(6)?,
                vinyl_color_pk: row.get(7)?,
                spars_pk: row.get(8)?,
                owner_pk: row.get(9)?,
                location_pk: row.get(10)?,
                collection_pk: row.get(11)?,
                release_year: row.get(12)?,
                rating: row.get(13)?,
                notes: row.get(14)?,
                upc: row.get(15)?,
                front_cover: row.get(16)?,
                back_cover: row.get(17)?,
                thumbnail: row.get(18)?,
                backdrop: row.get(19)?,
                date_added: row.get(20)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut albums = Vec::with_capacity(rows.len());
    for row in rows {
        let AlbumRow {
            id,
            title,
            format_pk,
            country_pk,
            label_pk,
            condition_pk,
            packaging_pk,
            vinyl_color_pk,
            spars_pk,
            owner_pk,
            location_pk,
            collection_pk,
            date_added,
            release_year,
            rating,
            notes,
            upc,
            front_cover,
            back_cover,
            thumbnail,
            backdrop,
        } = row;

        let discs = lookups.discs(id);
        // The raw ZNRTRACKS column (ignored above) can be 0 even when the
        // album's own tracks are present and correctly linked — observed in
        // a real collection. The actual disc/track rows are the source of
        // truth, so they're counted directly rather than trusting the
        // stored column.
        let track_count = discs.iter().map(|d| d.tracks.len() as i64).sum::<i64>();

        albums.push(Album {
            id,
            title,
            artists: lookups.role("ZARTIST", id),
            composers: lookups.role("ZCOMPOSER", id),
            conductors: lookups.role("ZCONDUCTOR", id),
            musicians: lookups.role("ZMUSICIAN", id),
            orchestras: lookups.role("ZORCHESTRA", id),
            choruses: lookups.role("ZCHORUS", id),
            credits: lookups.credits.get(&id).cloned().unwrap_or_default(),
            genres: lookups.many("genres", id),
            tags: lookups.many("tags", id),
            extras: lookups.many("extras", id),
            studios: lookups.many("studios", id),
            label: lookups.simple("ZLABEL", label_pk),
            format: lookups.simple("ZFORMAT", format_pk),
            country: lookups.simple("ZCOUNTRY", country_pk),
            condition: lookups.simple("ZCONDITION", condition_pk),
            packaging: lookups.simple("ZPACKAGING", packaging_pk),
            vinyl_color: lookups.simple("ZVINYLCOLOR", vinyl_color_pk),
            spars: lookups.simple("ZSPARS", spars_pk),
            owner: owner_pk.and_then(|pk| lookups.owners.get(&pk).cloned()),
            location: lookups.simple("ZLOCATION", location_pk),
            collection: collection_pk.and_then(|pk| lookups.collections.get(&pk).cloned()),
            date_added,
            release_year,
            rating,
            notes,
            upc,
            front_cover,
            back_cover,
            thumbnail,
            backdrop,
            track_count,
            discs,
        });
    }
    Ok(albums)
}

const STATIC_EXPORT_CSS: &str = r##"
body { font-family: -apple-system, system-ui, sans-serif; margin: 0; color: #1a1a1a; height: 100vh; display: flex; flex-direction: column; }
header { display: flex; flex-wrap: wrap; align-items: center; gap: 0.6rem; row-gap: 0.5rem; padding: 0.75rem 1rem; border-bottom: 1px solid #ddd; background: #fff; flex-shrink: 0; }
header h1 { font-size: 1.1rem; margin: 0; flex-shrink: 0; }
#search { flex: 1 1 12rem; min-width: 8rem; max-width: 24rem; font-size: 1rem; padding: 0.4rem 0.6rem; }
#count { color: #666; font-size: 0.85rem; flex-shrink: 0; margin-left: auto; }
#stats-btn { padding: 0.4rem 0.8rem; cursor: pointer; flex-shrink: 0; white-space: nowrap; }
#stats-btn.active { background: #6a7fe0; color: #fff; border-color: #6a7fe0; }
#lang-toggle { display: flex; flex-shrink: 0; border: 1px solid #ddd; border-radius: 6px; overflow: hidden; }
#lang-toggle button { border: none; background: #fff; padding: 0.3rem 0.5rem; min-width: 1.8rem; font-size: 0.75rem; font-weight: 600; cursor: pointer; color: #666; white-space: nowrap; }
#lang-toggle button:hover { background: #f5f7ff; }
#lang-toggle button + button { border-left: 1px solid #ddd; }
#lang-toggle button.active { background: #6a7fe0; color: #fff; }
#tabs { display: flex; gap: 0.4rem; padding: 0.4rem 1rem; overflow-x: auto; background: #f0f2ff; border-bottom: 1px solid #ddd; flex-shrink: 0; }
#tabs .tab { flex: 0 0 auto; padding: 0.3rem 0.7rem; border-radius: 999px; font-size: 0.75rem; font-weight: 500; white-space: nowrap; cursor: pointer; background: #e4e9ff; border: 1px solid #c3d2f7; color: #2e4bb0; }
#tabs .tab:hover { background: #dce5fb; border-color: #6a7fe0; }
#tabs .tab .n { color: #2e4bb0; opacity: 0.7; margin-left: 0.3rem; }
#tabs .tab.active { background: #6a7fe0; border-color: #6a7fe0; color: #fff; font-weight: 600; }
#tabs .tab.active .n { color: #dce8ff; opacity: 1; }
#main { display: flex; flex: 1; min-height: 0; }
#groups { width: 14rem; flex-shrink: 0; overflow-y: auto; display: flex; flex-direction: column; background: #fbfbfc; border-right: 1px solid #ddd; }
#right-col { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
#list-pane { flex: 0 0 55%; display: flex; flex-direction: column; min-height: 0; overflow: auto; }
#group-field { margin: 0.6rem; padding: 0.3rem; }
#azbar { display: flex; flex-wrap: wrap; gap: 2px; padding: 0.3rem 0.5rem; border-bottom: 1px solid #eee; flex-shrink: 0; }
#azbar button { border: 1px solid transparent; background: none; font-size: 0.7rem; font-weight: 600; color: #888; padding: 0.1rem 0.25rem; border-radius: 4px; cursor: pointer; min-width: 1.1rem; }
#azbar button:hover { background: #f5f7ff; color: #1a1a1a; }
#azbar button.active { background: #6a7fe0; color: #fff; }
#group-list { flex: 1; overflow-y: auto; }
#group-list div { padding: 0.35rem 0.8rem; cursor: pointer; font-size: 0.9rem; display: flex; justify-content: space-between; gap: 0.5rem; }
#group-list div:hover { background: #f5f7ff; }
#group-list div.selected { background: #e4e9ff; font-weight: 600; }
#group-list .group-count { color: #888; font-size: 0.8rem; }
#group-list .group-count.sortable { cursor: pointer; padding: 0.05rem 0.35rem; border-radius: 999px; }
#group-list .group-count.sortable:hover { background: #e4e9ff; color: #2e4bb0; }
#group-list .group-count.sort-active { background: #6a7fe0; color: #fff; }
#list-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.8rem; border-bottom: 1px solid #eee; background: #fafafa; flex-shrink: 0; }
#sort { padding: 0.25rem 0.4rem; font-size: 0.85rem; }
#view-toggle { display: flex; border: 1px solid #ddd; border-radius: 6px; overflow: hidden; }
#view-toggle button { border: none; background: #fff; padding: 0.25rem 0.5rem; font-size: 0.8rem; cursor: pointer; color: #666; }
#view-toggle button:hover { background: #f5f7ff; color: #1a1a1a; }
#view-toggle button + button { border-left: 1px solid #ddd; }
#view-toggle button.active { background: #6a7fe0; color: #fff; }
#grid-view { display: none; grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr)); gap: 0.8rem; padding: 0.8rem; align-content: start; overflow-y: auto; flex: 1; min-height: 0; }
#list-pane.view-grid table { display: none; }
#list-pane.view-grid #grid-view { display: grid; }
.grid-tile { cursor: pointer; border-radius: 6px; padding: 0.3rem; }
.grid-tile:hover, .grid-tile.active { background: #f0f2ff; }
.grid-tile .thumb { width: 100%; aspect-ratio: 1; object-fit: cover; border-radius: 4px; background: #e4e4e8; display: block; }
.grid-tile .t { font-size: 0.8rem; font-weight: 600; margin-top: 0.3rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.grid-tile .a { font-size: 0.72rem; color: #666; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
#pager { display: flex; align-items: center; justify-content: center; gap: 0.8rem; padding: 0.4rem; border-top: 1px solid #eee; background: #fafafa; flex-shrink: 0; font-size: 0.8rem; }
#pager button { padding: 0.25rem 0.6rem; cursor: pointer; }
#pager button:disabled { cursor: default; opacity: 0.4; }
#page-info { color: #666; }
table { border-collapse: collapse; width: 100%; }
th, td { text-align: left; padding: 0.4rem 0.8rem; border-bottom: 1px solid #eee; }
th { cursor: pointer; user-select: none; background: #fafafa; position: sticky; top: 0; }
tbody tr { cursor: pointer; }
tr:hover td { background: #f5f7ff; }
#detail { flex: 1; min-height: 0; padding: 1rem; overflow-y: auto; background: #fff; }
#detail-empty { color: #888; font-style: italic; }
#detail-content .cover { width: 220px; height: 220px; object-fit: cover; border-radius: 4px; margin-bottom: 0.75rem; }
#detail-content h2 { font-size: 1.05rem; margin: 0 1.5rem 0.75rem 0; }
#detail-content dl { display: grid; grid-template-columns: 6rem 1fr; gap: 0.25rem 0.5rem; font-size: 0.9rem; margin: 0 0 1rem 0; }
#detail-content dt { color: #666; }
#detail-content dd { margin: 0; }
#detail-content .tag-link { cursor: pointer; border-bottom: 1px dashed #999; }
#detail-content .tag-link:hover { color: #2e4bb0; border-bottom-color: #2e4bb0; }
#detail-content ol { padding-left: 1.2rem; font-size: 0.9rem; }
#detail-content .disc-heading { font-weight: 600; margin-top: 0.75rem; font-size: 0.85rem; color: #444; }
#stats-pane { display: none; flex: 1; overflow-y: auto; background: #fbfbfc; padding: 1.5rem; min-height: 0; }
body.panel-stats #main { display: none; }
body.panel-stats #stats-pane { display: block; }
.stats-wrap { max-width: 60rem; margin: 0 auto; }
.stats-wrap h2 { font-size: 1.1rem; margin: 0 0 0.9rem; }
.stat-summary { display: flex; flex-wrap: wrap; gap: 1rem; font-size: 0.85rem; color: #666; margin-bottom: 1.2rem; }
.stat-summary strong { color: #1a1a1a; }
.stat-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr)); gap: 1.1rem; }
.stat-block { background: #fff; border: 1px solid #ddd; border-radius: 8px; padding: 0.9rem 1rem; }
.stat-block h3 { font-size: 0.7rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; color: #888; margin: 0 0 0.6rem; }
.stat-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.8rem; margin-bottom: 0.3rem; }
.stat-row .label { width: 9rem; flex-shrink: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.stat-row .bar-wrap { flex: 1; background: #eceef2; border-radius: 3px; height: 0.6rem; overflow: hidden; }
.stat-row .bar { display: block; background: #6a7fe0; height: 100%; }
.stat-row .val { width: 2.3rem; text-align: right; color: #888; font-size: 0.7rem; flex-shrink: 0; }
.stat-empty { font-size: 0.8rem; color: #888; }
"##;

// Adapted from the Tauri app's own app.js: same groups/A-Z bar/tabs/
// sort/list-grid/pagination/detail/stats logic, operating on the
// embedded #data array instead of invoke() calls, with every write
// action (add/edit/delete/set-cover) and the EN/FR toggle left out —
// this page is read-only and English-only. mc_add already covers
// writing, directly and more safely (a reviewable plan before any
// commit), so there's no "download a JSON, pass it to mc_add" form
// here either, unlike BookNojo's own static export.
const STATIC_EXPORT_JS: &str = r##"
const albums = JSON.parse(document.getElementById("data").textContent);
let sortKey = "title", sortAsc = true, groupField = "artists", selectedGroup = null;
let selectedLetter = null, groupSortByCount = false, viewMode = "list", page = 1;
let currentDetailId = null, panel = "library";
let collCounts = new Map(), collectionsList = [], currentCollection = null;
const PAGE_SIZE = 60;
const DEFAULT_COLLECTION_KEY = "__default__";

// Same i18n shape and FR copy as the app itself (app/src/app.js) — kept
// deliberately separate from that larger dictionary since this export
// only ever needs the read/browse-only subset of keys (no add/edit/
// delete, no cover-setting).
const I18N = {
  en: {
    search_placeholder: "Search by title or artist…",
    count_albums: "{n} album(s)",
    nav_stats: "📊 Statistics",
    nav_back_to_collection: "← Back to collection",
    group_all: "All",
    azbar_all: "All",
    group_sort_tooltip_active: "Sorted by count — click to sort alphabetically",
    group_sort_tooltip_inactive: "Sorted alphabetically — click to sort by count",
    collection_default: "My Collection",
    opt_group_artists: "Artist",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Country",
    opt_group_label: "Label",
    opt_sort_title: "Title (A→Z)",
    opt_sort_artist: "Artist (A→Z)",
    opt_sort_year_desc: "Year (newest first)",
    opt_sort_label: "Label (A→Z)",
    list_view_list: "☰ List",
    list_view_grid: "▦ Grid",
    list_prev: "‹ Prev.",
    list_next: "Next ›",
    list_page_info: "Page {page} of {total}",
    col_title: "Title",
    col_artists: "Artist(s)",
    col_genres: "Genre(s)",
    col_format: "Format",
    col_year: "Year",
    col_tracks: "Tracks",
    detail_select: "Select an album.",
    detail_disc: "Disc {n}",
    detail_tracks: "Tracks",
    detail_field_artists: "Artist(s)",
    detail_field_composers: "Composer(s)",
    detail_field_conductors: "Conductor(s)",
    detail_field_musicians: "Musician(s)",
    detail_field_orchestras: "Orchestra(s)",
    detail_field_choruses: "Chorus(es)",
    detail_field_genres: "Genre(s)",
    detail_field_tags: "Tag(s)",
    detail_field_label: "Label",
    detail_field_format: "Format",
    detail_field_country: "Country",
    detail_field_year: "Year",
    detail_field_rating: "Rating",
    detail_field_notes: "Notes",
    detail_field_upc: "UPC",
    detail_field_date_added: "Date added",
    stats_title: "Statistics — {name}",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} track(s)",
    stats_no_cover: "{n} with no cover",
    stats_no_artist: "{n} with no artist",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Countries",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "By release decade",
    stats_by_added_year: "By year added",
    stats_no_data: "No data.",
  },
  fr: {
    search_placeholder: "Rechercher par titre ou artiste…",
    count_albums: "{n} album(s)",
    nav_stats: "📊 Statistiques",
    nav_back_to_collection: "← Retour à la collection",
    group_all: "Tous",
    azbar_all: "Tout",
    group_sort_tooltip_active: "Trié par nombre — cliquer pour trier par ordre alphabétique",
    group_sort_tooltip_inactive: "Trié par ordre alphabétique — cliquer pour trier par nombre",
    collection_default: "Ma collection",
    opt_group_artists: "Artiste",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Pays",
    opt_group_label: "Label",
    opt_sort_title: "Titre (A→Z)",
    opt_sort_artist: "Artiste (A→Z)",
    opt_sort_year_desc: "Année (plus récent d'abord)",
    opt_sort_label: "Label (A→Z)",
    list_view_list: "☰ Liste",
    list_view_grid: "▦ Grille",
    list_prev: "‹ Préc.",
    list_next: "Suiv. ›",
    list_page_info: "Page {page} / {total}",
    col_title: "Titre",
    col_artists: "Artiste(s)",
    col_genres: "Genre(s)",
    col_format: "Format",
    col_year: "Année",
    col_tracks: "Pistes",
    detail_select: "Sélectionnez un album.",
    detail_disc: "Disque {n}",
    detail_tracks: "Pistes",
    detail_field_artists: "Artiste(s)",
    detail_field_composers: "Compositeur(s)",
    detail_field_conductors: "Chef(s) d'orchestre",
    detail_field_musicians: "Musicien(s)",
    detail_field_orchestras: "Orchestre(s)",
    detail_field_choruses: "Chœur(s)",
    detail_field_genres: "Genre(s)",
    detail_field_tags: "Étiquette(s)",
    detail_field_label: "Label",
    detail_field_format: "Format",
    detail_field_country: "Pays",
    detail_field_year: "Année",
    detail_field_rating: "Note",
    detail_field_notes: "Notes",
    detail_field_upc: "UPC",
    detail_field_date_added: "Date d'ajout",
    stats_title: "Statistiques — {name}",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} piste(s)",
    stats_no_cover: "{n} sans pochette",
    stats_no_artist: "{n} sans artiste",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Pays",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "Par décennie de sortie",
    stats_by_added_year: "Par année d'ajout",
    stats_no_data: "Aucune donnée.",
  },
};
let lang = "en";
function t(key, vars) {
  let text = (I18N[lang] && I18N[lang][key]) || I18N.en[key] || key;
  if (vars) for (const [k, v] of Object.entries(vars)) text = text.replace(`{${k}}`, v);
  return text;
}
function applyStaticLabels() {
  document.querySelectorAll("[data-i18n]").forEach(el => { el.textContent = t(el.dataset.i18n); });
  document.querySelectorAll("[data-i18n-placeholder]").forEach(el => { el.placeholder = t(el.dataset.i18nPlaceholder); });
  document.getElementById("lang-en").classList.toggle("active", lang === "en");
  document.getElementById("lang-fr").classList.toggle("active", lang === "fr");
  document.documentElement.lang = lang;
}
function setLang(newLang) {
  lang = newLang;
  applyStaticLabels();
  renderTabs();
  renderGroups();
  render(document.getElementById("search").value);
  if (currentDetailId !== null) showDetail(currentDetailId); else clearDetail();
  if (panel === "stats") renderStats();
}
document.getElementById("lang-en").addEventListener("click", () => setLang("en"));
document.getElementById("lang-fr").addEventListener("click", () => setLang("fr"));

function escapeHtml(s) {
  return String(s ?? "").replace(/[&<>"']/g, c => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
}
function collectionKey(a) { return a.collection || DEFAULT_COLLECTION_KEY; }
function collectionLabel(k) { return k === DEFAULT_COLLECTION_KEY ? t("collection_default") : k; }
function groupValues(a, field) {
  const v = a[field];
  if (Array.isArray(v)) return v;
  return v ? [v] : [];
}
function letterOf(value) {
  const ch = (value || "").trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : "#";
}

function computeCollections() {
  collCounts = new Map();
  for (const a of albums) {
    const k = collectionKey(a);
    collCounts.set(k, (collCounts.get(k) ?? 0) + 1);
  }
  collectionsList = [...collCounts.keys()].sort((a, b) =>
    collCounts.get(b) - collCounts.get(a) || collectionLabel(a).localeCompare(collectionLabel(b)));
  currentCollection = collectionsList[0] ?? null;
}
function renderTabs() {
  const el = document.getElementById("tabs");
  el.innerHTML = "";
  for (const key of collectionsList) {
    const tab = document.createElement("div");
    tab.className = "tab" + (key === currentCollection ? " active" : "");
    tab.innerHTML = `<span>${escapeHtml(collectionLabel(key))}</span><span class="n">${collCounts.get(key)}</span>`;
    tab.addEventListener("click", () => {
      if (key === currentCollection) return;
      currentCollection = key;
      selectedGroup = null; selectedLetter = null; page = 1;
      renderTabs(); renderGroups(); render(document.getElementById("search").value);
      if (panel === "stats") renderStats();
    });
    el.appendChild(tab);
  }
}

function computeAvailableLetters() {
  const set = new Set();
  for (const a of albums) {
    if (collectionKey(a) !== currentCollection) continue;
    for (const v of groupValues(a, groupField)) if (v) set.add(letterOf(v));
  }
  return [...set].sort();
}
function renderAzBar() {
  const el = document.getElementById("azbar");
  el.innerHTML = "";
  const all = document.createElement("button");
  all.textContent = t("azbar_all");
  all.className = selectedLetter === null ? "active" : "";
  all.addEventListener("click", () => { selectedLetter = null; renderGroups(); });
  el.appendChild(all);
  for (const letter of computeAvailableLetters()) {
    const btn = document.createElement("button");
    btn.textContent = letter;
    btn.className = selectedLetter === letter ? "active" : "";
    btn.addEventListener("click", () => { selectedLetter = letter; renderGroups(); });
    el.appendChild(btn);
  }
}
function renderGroups() {
  renderAzBar();
  const counts = new Map();
  let total = 0;
  for (const a of albums) {
    if (collectionKey(a) !== currentCollection) continue;
    total++;
    for (const v of groupValues(a, groupField)) {
      if (selectedLetter && letterOf(v) !== selectedLetter) continue;
      counts.set(v, (counts.get(v) ?? 0) + 1);
    }
  }
  const sorted = groupSortByCount
    ? [...counts.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    : [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  const list = document.getElementById("group-list");
  list.innerHTML =
    `<div class="${selectedGroup === null ? "selected" : ""}" data-group="">
       <span>${t("group_all")}</span><span class="group-count sortable${groupSortByCount ? " sort-active" : ""}" id="group-sort-toggle" title="${groupSortByCount ? t("group_sort_tooltip_active") : t("group_sort_tooltip_inactive")}">${total}</span>
     </div>` +
    sorted.map(([value, count]) => `<div class="${value === selectedGroup ? "selected" : ""}" data-group="${escapeHtml(value)}">
      <span>${escapeHtml(value)}</span><span class="group-count">${count}</span>
    </div>`).join("");
  list.querySelectorAll("div").forEach(div => div.addEventListener("click", () => {
    selectedGroup = div.dataset.group || null;
    page = 1;
    renderGroups();
    render(document.getElementById("search").value);
  }));
  document.getElementById("group-sort-toggle").addEventListener("click", e => {
    e.stopPropagation();
    groupSortByCount = !groupSortByCount;
    renderGroups();
  });
}

function coverImg(a) {
  return a.front_cover_url ? `<img class="cover" src="${a.front_cover_url}" alt="">` : "";
}
function roleList(label, names) {
  return names && names.length ? `<dt>${label}</dt><dd>${escapeHtml(names.join(", "))}</dd>` : "";
}
function clickableRoleList(label, names, field) {
  if (!names || !names.length) return "";
  const spans = names.map(n => `<span class="tag-link" data-group-field="${field}" data-value="${escapeHtml(n)}">${escapeHtml(n)}</span>`).join(", ");
  return `<dt>${label}</dt><dd>${spans}</dd>`;
}
function field(label, value) {
  return value ? `<dt>${label}</dt><dd>${escapeHtml(value)}</dd>` : "";
}
function filterByGroupValue(f, value) {
  groupField = f;
  document.getElementById("group-field").value = f;
  selectedGroup = value; selectedLetter = null; page = 1;
  renderGroups();
  render(document.getElementById("search").value);
}
function showDetail(id) {
  currentDetailId = id;
  const a = albums.find(x => x.id === id);
  if (!a) return;
  const discs = (a.discs || []).map((disc, i) => {
    const heading = disc.index ? t("detail_disc", { n: escapeHtml(disc.index) }) : (a.discs.length > 1 ? t("detail_disc", { n: i + 1 }) : t("detail_tracks"));
    const tracks = disc.tracks.map(tr => `<li>${escapeHtml(tr.title)}${tr.length ? ` — ${escapeHtml(tr.length)}` : ""}</li>`).join("");
    return `<div class="disc-heading">${heading}</div><ol>${tracks}</ol>`;
  }).join("");
  document.getElementById("detail-content").innerHTML = `
    ${coverImg(a)}
    <h2>${escapeHtml(a.title)}</h2>
    <dl>
      ${clickableRoleList(t("detail_field_artists"), a.artists, "artists")}
      ${roleList(t("detail_field_composers"), a.composers)}
      ${roleList(t("detail_field_conductors"), a.conductors)}
      ${roleList(t("detail_field_musicians"), a.musicians)}
      ${roleList(t("detail_field_orchestras"), a.orchestras)}
      ${roleList(t("detail_field_choruses"), a.choruses)}
      ${clickableRoleList(t("detail_field_genres"), a.genres, "genres")}
      ${roleList(t("detail_field_tags"), a.tags)}
      ${field(t("detail_field_label"), a.label)}
      ${field(t("detail_field_format"), a.format)}
      ${field(t("detail_field_country"), a.country)}
      ${field(t("detail_field_year"), a.release_year)}
      ${field(t("detail_field_rating"), a.rating != null ? String(a.rating) : "")}
      ${field(t("detail_field_notes"), a.notes)}
      ${field(t("detail_field_upc"), a.upc)}
      ${field(t("detail_field_date_added"), a.date_added)}
    </dl>
    ${discs}
  `;
  document.querySelectorAll("#detail-content .tag-link").forEach(el =>
    el.addEventListener("click", () => filterByGroupValue(el.dataset.groupField, el.dataset.value)));
}
function clearDetail() {
  currentDetailId = null;
  document.getElementById("detail-content").innerHTML = `<p id="detail-empty">${t("detail_select")}</p>`;
}

function render(filterText) {
  const needle = filterText.trim().toLowerCase();
  const rows = albums.filter(a => {
    const matchesCollection = collectionKey(a) === currentCollection;
    const matchesSearch = !needle || a.title.toLowerCase().includes(needle) || a.artists.join(" ").toLowerCase().includes(needle);
    const matchesGroup = selectedGroup === null || groupValues(a, groupField).includes(selectedGroup);
    return matchesCollection && matchesSearch && matchesGroup;
  });
  rows.sort((a, b) => {
    const va = Array.isArray(a[sortKey]) ? a[sortKey].join(", ") : a[sortKey];
    const vb = Array.isArray(b[sortKey]) ? b[sortKey].join(", ") : b[sortKey];
    const cmp = typeof va === "number" ? va - vb : String(va ?? "").localeCompare(String(vb ?? ""));
    return sortAsc ? cmp : -cmp;
  });
  const totalPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  page = Math.min(Math.max(1, page), totalPages);
  const start = (page - 1) * PAGE_SIZE;
  const pageRows = rows.slice(start, start + PAGE_SIZE);

  document.getElementById("count").textContent = t("count_albums", { n: rows.length });
  const body = document.querySelector("#albums tbody");
  body.innerHTML = pageRows.map(a => `<tr data-id="${a.id}">
    <td>${escapeHtml(a.title)}</td>
    <td>${escapeHtml(a.artists.join(", "))}</td>
    <td>${escapeHtml(a.genres.join(", "))}</td>
    <td>${escapeHtml(a.format)}</td>
    <td>${escapeHtml(a.release_year)}</td>
    <td>${a.track_count}</td>
  </tr>`).join("");
  body.querySelectorAll("tr").forEach(tr => tr.addEventListener("click", () => showDetail(Number(tr.dataset.id))));

  const grid = document.getElementById("grid-view");
  grid.innerHTML = pageRows.map(a => {
    const thumb = a.front_cover_url ? `<img class="thumb" loading="lazy" src="${a.front_cover_url}" alt="">` : `<div class="thumb"></div>`;
    return `<div class="grid-tile" data-id="${a.id}">${thumb}<div class="t">${escapeHtml(a.title)}</div><div class="a">${escapeHtml(a.artists.join(", "))}</div></div>`;
  }).join("");
  grid.querySelectorAll(".grid-tile").forEach(tile => tile.addEventListener("click", () => showDetail(Number(tile.dataset.id))));

  document.getElementById("page-info").textContent = t("list_page_info", { page, total: totalPages });
  document.getElementById("prev-page").disabled = page <= 1;
  document.getElementById("next-page").disabled = page >= totalPages;
}

document.getElementById("group-field").addEventListener("change", e => {
  groupField = e.target.value; selectedGroup = null; selectedLetter = null; page = 1;
  renderGroups(); render(document.getElementById("search").value);
});
document.getElementById("search").addEventListener("input", e => { page = 1; render(e.target.value); });

const SORT_PRESETS = {
  title: { key: "title", asc: true },
  artist: { key: "artists", asc: true },
  year_desc: { key: "release_year", asc: false },
  label: { key: "label", asc: true },
};
const sortEl = document.getElementById("sort");
function syncSortDropdown() {
  const match = Object.entries(SORT_PRESETS).find(([, p]) => p.key === sortKey && p.asc === sortAsc);
  if (match) sortEl.value = match[0];
}
sortEl.addEventListener("change", () => {
  const preset = SORT_PRESETS[sortEl.value];
  sortKey = preset.key; sortAsc = preset.asc; page = 1;
  render(document.getElementById("search").value);
});
document.querySelectorAll("th").forEach(th => th.addEventListener("click", () => {
  const k = th.dataset.k;
  sortAsc = sortKey === k ? !sortAsc : true;
  sortKey = k; page = 1;
  syncSortDropdown();
  render(document.getElementById("search").value);
}));
document.getElementById("prev-page").addEventListener("click", () => { page--; render(document.getElementById("search").value); });
document.getElementById("next-page").addEventListener("click", () => { page++; render(document.getElementById("search").value); });

const viewListBtn = document.getElementById("view-list");
const viewGridBtn = document.getElementById("view-grid");
const listPaneEl = document.getElementById("list-pane");
function setViewMode(mode) {
  viewMode = mode;
  listPaneEl.classList.toggle("view-grid", mode === "grid");
  viewListBtn.classList.toggle("active", mode === "list");
  viewGridBtn.classList.toggle("active", mode === "grid");
}
viewListBtn.addEventListener("click", () => setViewMode("list"));
viewGridBtn.addEventListener("click", () => setViewMode("grid"));

// --- Statistics ---
function computeStats() {
  const scoped = albums.filter(a => collectionKey(a) === currentCollection);
  const tally = getter => {
    const m = new Map();
    for (const a of scoped) for (const v of getter(a)) if (v) m.set(v, (m.get(v) || 0) + 1);
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  };
  const decades = new Map();
  for (const a of scoped) {
    const y = parseInt(a.release_year, 10);
    if (!y) continue;
    const d = Math.floor(y / 10) * 10;
    decades.set(d, (decades.get(d) || 0) + 1);
  }
  const addedYears = new Map();
  for (const a of scoped) {
    if (!a.date_added) continue;
    const y = a.date_added.slice(0, 4);
    addedYears.set(y, (addedYears.get(y) || 0) + 1);
  }
  const noCoverAlbums = scoped.filter(a => !a.front_cover_url).sort((a, b) => a.title.localeCompare(b.title));
  const noArtistAlbums = scoped.filter(a => !(a.artists && a.artists.length)).sort((a, b) => a.title.localeCompare(b.title));
  return {
    totalAlbums: scoped.length,
    totalTracks: scoped.reduce((s, a) => s + (a.track_count || 0), 0),
    noCover: noCoverAlbums.length, noArtist: noArtistAlbums.length,
    noCoverAlbums, noArtistAlbums,
    genres: tally(a => a.genres || []),
    formats: tally(a => a.format ? [a.format] : []),
    countries: tally(a => a.country ? [a.country] : []),
    labels: tally(a => a.label ? [a.label] : []),
    decades: [...decades.entries()].sort((a, b) => a[0] - b[0]).map(([d, c]) => [`${d}s`, c]),
    addedYears: [...addedYears.entries()].sort((a, b) => a[0].localeCompare(b[0])),
  };
}
function barList(title, entries, limit) {
  const top = limit ? entries.slice(0, limit) : entries;
  const max = top.reduce((m, [, c]) => Math.max(m, c), 1);
  const rows = top.map(([label, count]) => `
    <div class="stat-row">
      <span class="label">${escapeHtml(label)}</span>
      <span class="bar-wrap"><span class="bar" style="width:${((count / max) * 100).toFixed(1)}%"></span></span>
      <span class="val">${count}</span>
    </div>`).join("");
  return `<div class="stat-block"><h3>${escapeHtml(title)}</h3>${rows || `<p class="stat-empty">${t("stats_no_data")}</p>`}</div>`;
}
function albumListHtml(albumsList) {
  if (!albumsList.length) return `<p class="stat-empty">${t("stats_no_data")}</p>`;
  return `<ul class="stat-detail-list">${albumsList.map(a => `<li>${escapeHtml(a.artists.join(", ") || "—")} — ${escapeHtml(a.title)}</li>`).join("")}</ul>`;
}
const statsExpanded = new Set();
function statLink(key, labelKey, n) {
  const expanded = statsExpanded.has(key);
  return `<span class="stat-link${expanded ? " expanded" : ""}" data-stat-key="${key}">${t(labelKey, { n: `<strong>${n}</strong>` })}</span>`;
}
function renderStats() {
  const s = computeStats();
  const detailBlock = (key, labelKey, n, list) => {
    if (!statsExpanded.has(key)) return "";
    return `<div class="stat-detail"><h4>${escapeHtml(t(labelKey, { n }))}</h4>${albumListHtml(list)}</div>`;
  };
  document.getElementById("stats-pane").innerHTML = `
    <div class="stats-wrap">
      <h2>${escapeHtml(t("stats_title", { name: collectionLabel(currentCollection) }))}</h2>
      <div class="stat-summary">
        <span>${t("stats_albums", { n: `<strong>${s.totalAlbums}</strong>` })}</span>
        <span>${t("stats_tracks", { n: `<strong>${s.totalTracks}</strong>` })}</span>
        ${statLink("noCover", "stats_no_cover", s.noCover)}
        ${statLink("noArtist", "stats_no_artist", s.noArtist)}
      </div>
      ${detailBlock("noCover", "stats_no_cover", s.noCover, s.noCoverAlbums)}
      ${detailBlock("noArtist", "stats_no_artist", s.noArtist, s.noArtistAlbums)}
      <div class="stat-grid">
        ${barList(t("stats_genres"), s.genres, 15)}
        ${barList(t("stats_formats"), s.formats)}
        ${barList(t("stats_countries"), s.countries)}
        ${barList(t("stats_labels_top10"), s.labels, 10)}
        ${barList(t("stats_by_decade"), s.decades)}
        ${barList(t("stats_by_added_year"), s.addedYears)}
      </div>
    </div>`;
  document.querySelectorAll(".stat-link").forEach(el => el.addEventListener("click", () => {
    const key = el.dataset.statKey;
    if (statsExpanded.has(key)) statsExpanded.delete(key); else statsExpanded.add(key);
    renderStats();
  }));
}
const statsBtn = document.getElementById("stats-btn");
function setPanel(next) {
  panel = next;
  document.body.classList.toggle("panel-stats", panel === "stats");
  statsBtn.classList.toggle("active", panel === "stats");
  statsBtn.textContent = panel === "stats" ? t("nav_back_to_collection") : t("nav_stats");
  if (panel === "stats") renderStats();
}
statsBtn.addEventListener("click", () => setPanel(panel === "stats" ? "library" : "stats"));

applyStaticLabels();
computeCollections();
renderTabs();
renderGroups();
render("");
clearDetail();
"##;

/// `file://` URL for a local path — percent-encodes everything outside
/// the unreserved set, same construction as BookNojo's own `file_url`.
/// Used for cover images in the static HTML export: it's opened
/// straight from disk (`file://` itself), and `file://` pages can
/// reference other local files the same way a normal page references
/// same-origin images.
fn file_url(path: &str) -> String {
    const UNRESERVED: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let mut out = String::from("file://");
    for byte in path.bytes() {
        if byte == b'/' || UNRESERVED.contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// Nearly the full `Album` shape, plus a resolved `front_cover_url`
/// (a `file://` URL, or null) in place of the bare filename — the
/// static export has no Tauri asset protocol to go through.
#[derive(Serialize)]
struct HtmlAlbum<'a> {
    id: i64,
    title: &'a str,
    artists: &'a [String],
    composers: &'a [String],
    conductors: &'a [String],
    musicians: &'a [String],
    orchestras: &'a [String],
    choruses: &'a [String],
    genres: &'a [String],
    tags: &'a [String],
    label: &'a Option<String>,
    format: &'a Option<String>,
    country: &'a Option<String>,
    release_year: &'a Option<String>,
    rating: Option<i64>,
    notes: &'a Option<String>,
    upc: &'a Option<String>,
    date_added: &'a Option<String>,
    track_count: i64,
    collection: &'a Option<String>,
    front_cover_url: Option<String>,
    discs: &'a [Disc],
}

/// A standalone HTML page reproducing most of the application's own
/// interface (groups/A-Z bar, tabs, sort, list/grid, pagination,
/// detail card, basic statistics) against the static data embedded in
/// the page — same purpose as BookNojo's own `--html` export, though
/// built independently rather than through a shared template with the
/// Tauri app (see `BUILD.md` for why). English only (no EN/FR
/// toggle), and read-only: no add/edit/delete form, since `mc_add`
/// already covers that directly.
pub fn write_html(path: &Path, albums: &[Album], images_dir: &Path) -> Result<()> {
    // A `file://` URL needs an absolute path — the caller may well have
    // passed a relative one (e.g. `--db sample-collection/...`), which
    // would silently produce a broken (not just wrong-looking) image:
    // `file://sample-collection/...` parses as host "sample-collection"
    // with an empty path, not a relative path. Found via a screenshot
    // the user took of the very first generated export, same genre of
    // bug as the Tauri app's own unresolved-".." cover bug (v0.25.0).
    let images_dir_abs = images_dir.canonicalize().unwrap_or_else(|_| images_dir.to_path_buf());
    let images_dir_str = images_dir_abs.to_string_lossy();
    let html_albums: Vec<HtmlAlbum> = albums
        .iter()
        .map(|a| HtmlAlbum {
            id: a.id,
            title: &a.title,
            artists: &a.artists,
            composers: &a.composers,
            conductors: &a.conductors,
            musicians: &a.musicians,
            orchestras: &a.orchestras,
            choruses: &a.choruses,
            genres: &a.genres,
            tags: &a.tags,
            label: &a.label,
            format: &a.format,
            country: &a.country,
            release_year: &a.release_year,
            rating: a.rating,
            notes: &a.notes,
            upc: &a.upc,
            date_added: &a.date_added,
            track_count: a.track_count,
            collection: &a.collection,
            front_cover_url: a
                .front_cover
                .as_deref()
                .map(|f| file_url(&format!("{images_dir_str}/{f}"))),
            discs: &a.discs,
        })
        .collect();
    // Escaped so a title/artist containing "</script>" can't break out of
    // the embedding <script> tag.
    let data_json = serde_json::to_string(&html_albums)?.replace("</", "<\\/");

    let html = format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>MusicNojo — Collection</title>
<style>
{CSS}
</style>
</head>
<body>
<header>
  <h1>MusicNojo</h1>
  <input id="search" type="search" data-i18n-placeholder="search_placeholder" placeholder="Search by title or artist…" autofocus>
  <span id="count"></span>
  <button id="stats-btn" data-i18n="nav_stats">📊 Statistics</button>
  <span id="lang-toggle">
    <button id="lang-en" type="button">EN</button>
    <button id="lang-fr" type="button">FR</button>
  </span>
</header>
<div id="tabs"></div>
<main id="main">
  <aside id="groups">
    <select id="group-field">
      <option value="artists" data-i18n="opt_group_artists">Artist</option>
      <option value="genres" data-i18n="opt_group_genres">Genre</option>
      <option value="format" data-i18n="opt_group_format">Format</option>
      <option value="country" data-i18n="opt_group_country">Country</option>
      <option value="label" data-i18n="opt_group_label">Label</option>
    </select>
    <div id="azbar"></div>
    <div id="group-list"></div>
  </aside>
  <div id="right-col">
    <div id="list-pane">
      <div id="list-toolbar">
        <select id="sort">
          <option value="title" data-i18n="opt_sort_title">Title (A→Z)</option>
          <option value="artist" data-i18n="opt_sort_artist">Artist (A→Z)</option>
          <option value="year_desc" data-i18n="opt_sort_year_desc">Year (newest first)</option>
          <option value="label" data-i18n="opt_sort_label">Label (A→Z)</option>
        </select>
        <div id="view-toggle">
          <button id="view-list" class="active" data-i18n="list_view_list">☰ List</button>
          <button id="view-grid" data-i18n="list_view_grid">▦ Grid</button>
        </div>
      </div>
      <table id="albums">
        <thead><tr>
          <th data-k="title" data-i18n="col_title">Title</th>
          <th data-k="artists" data-i18n="col_artists">Artist(s)</th>
          <th data-k="genres" data-i18n="col_genres">Genre(s)</th>
          <th data-k="format" data-i18n="col_format">Format</th>
          <th data-k="release_year" data-i18n="col_year">Year</th>
          <th data-k="track_count" data-i18n="col_tracks">Tracks</th>
        </tr></thead>
        <tbody></tbody>
      </table>
      <div id="grid-view"></div>
      <div id="pager">
        <button id="prev-page" data-i18n="list_prev">‹ Prev.</button>
        <span id="page-info"></span>
        <button id="next-page" data-i18n="list_next">Next ›</button>
      </div>
    </div>
    <aside id="detail">
      <div id="detail-content"><p id="detail-empty" data-i18n="detail_select">Select an album.</p></div>
    </aside>
  </div>
</main>
<div id="stats-pane"></div>
<script id="data" type="application/json">{data_json}</script>
<script>
{JS}
</script>
</body>
</html>
"##,
        CSS = STATIC_EXPORT_CSS,
        JS = STATIC_EXPORT_JS,
        data_json = data_json,
    );
    std::fs::write(path, html).with_context(|| format!("écriture de {}", path.display()))?;
    Ok(())
}

pub fn write_csv(path: &Path, albums: &[Album]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "id",
        "title",
        "artists",
        "composers",
        "conductors",
        "musicians",
        "orchestras",
        "choruses",
        "credits",
        "genres",
        "tags",
        "extras",
        "studios",
        "label",
        "format",
        "country",
        "condition",
        "packaging",
        "vinyl_color",
        "spars",
        "owner",
        "location",
        "collection",
        "date_added",
        "release_year",
        "rating",
        "notes",
        "upc",
        "front_cover",
        "back_cover",
        "thumbnail",
        "backdrop",
        "track_count",
    ])?;
    for album in albums {
        let credits = album
            .credits
            .iter()
            .map(|c| match c.role_id {
                Some(id) => format!("{} (role_id={})", c.name, id),
                None => c.name.clone(),
            })
            .collect::<Vec<_>>()
            .join("; ");
        writer.write_record([
            album.id.to_string(),
            album.title.clone(),
            album.artists.join("; "),
            album.composers.join("; "),
            album.conductors.join("; "),
            album.musicians.join("; "),
            album.orchestras.join("; "),
            album.choruses.join("; "),
            credits,
            album.genres.join("; "),
            album.tags.join("; "),
            album.extras.join("; "),
            album.studios.join("; "),
            album.label.clone().unwrap_or_default(),
            album.format.clone().unwrap_or_default(),
            album.country.clone().unwrap_or_default(),
            album.condition.clone().unwrap_or_default(),
            album.packaging.clone().unwrap_or_default(),
            album.vinyl_color.clone().unwrap_or_default(),
            album.spars.clone().unwrap_or_default(),
            album.owner.clone().unwrap_or_default(),
            album.location.clone().unwrap_or_default(),
            album.collection.clone().unwrap_or_default(),
            album.date_added.clone().unwrap_or_default(),
            album.release_year.clone().unwrap_or_default(),
            album.rating.map(|r| r.to_string()).unwrap_or_default(),
            album.notes.clone().unwrap_or_default(),
            album.upc.clone().unwrap_or_default(),
            album.front_cover.clone().unwrap_or_default(),
            album.back_cover.clone().unwrap_or_default(),
            album.thumbnail.clone().unwrap_or_default(),
            album.backdrop.clone().unwrap_or_default(),
            album.track_count.to_string(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

// --------------------------------------------------------------------
// Writing: mc_add's plan-then-commit workflow (add/edit/delete/cover).
// --------------------------------------------------------------------

const ENT_ARTIST: i64 = 3;
const ENT_ARTIST_PERSON: i64 = 4;
const ENT_CHORUS: i64 = 6;
const ENT_CHORUS_PERSON: i64 = 7;
const ENT_COMPOSER: i64 = 16;
const ENT_COMPOSER_PERSON: i64 = 17;
const ENT_CONDUCTOR: i64 = 21;
const ENT_CONDUCTOR_PERSON: i64 = 22;
const ENT_MUSICIAN: i64 = 40;
const ENT_MUSICIAN_PERSON: i64 = 41;
const ENT_ORCHESTRA: i64 = 42;
const ENT_ORCHESTRA_PERSON: i64 = 43;
const ENT_FORMAT: i64 = 27;
const ENT_GENRE: i64 = 28;
const ENT_TAG: i64 = 54;
const ENT_EXTRA: i64 = 26;
const ENT_STUDIO: i64 = 53;
const ENT_LABEL: i64 = 32;
const ENT_COUNTRY: i64 = 23;
const ENT_CONDITION: i64 = 20;
const ENT_PACKAGING: i64 = 45;
const ENT_VINYL_COLOR: i64 = 58;
const ENT_SPARS: i64 = 50;
const ENT_OWNER: i64 = 44;
const ENT_LOCATION: i64 = 38;
const ENT_LOOKUP_ITEM_DELEGATE: i64 = 39;
const ENT_PERSON_DELEGATE: i64 = 46;

#[derive(clap::Args, serde::Deserialize, Default)]
pub struct AlbumFields {
    #[arg(long)]
    pub format: Option<String>,
    #[arg(long)]
    pub label: Option<String>,
    #[arg(long)]
    pub country: Option<String>,
    #[arg(long)]
    pub condition: Option<String>,
    #[arg(long)]
    pub packaging: Option<String>,
    #[arg(long = "vinyl-color")]
    pub vinyl_color: Option<String>,
    #[arg(long)]
    pub spars: Option<String>,
    #[arg(long)]
    pub owner: Option<String>,
    #[arg(long)]
    pub location: Option<String>,
    #[arg(long = "release-year")]
    pub release_year: Option<String>,
    #[arg(long)]
    pub rating: Option<i64>,
    #[arg(long)]
    pub notes: Option<String>,
    #[arg(long)]
    pub upc: Option<String>,
    /// Must already exist (one of Music Collector's own named
    /// sub-collections, e.g. a default collection plus any the user
    /// created) — this never creates a new one, same restriction
    /// BookNojo's own bc_add has for a book's sub-collection.
    #[arg(long)]
    pub collection: Option<String>,
}

/// Artist/Composer/Conductor/Musician/Orchestra/Chorus each have their
/// own dedicated entity + person-delegate pair (see load_role_names's
/// shape above). One repeatable flag per role.
#[derive(clap::Args, serde::Deserialize, Default)]
pub struct RoleInputs {
    #[arg(long = "artist")]
    pub artist: Vec<String>,
    #[arg(long = "composer")]
    pub composer: Vec<String>,
    #[arg(long = "conductor")]
    pub conductor: Vec<String>,
    #[arg(long = "musician")]
    pub musician: Vec<String>,
    #[arg(long = "orchestra")]
    pub orchestra: Vec<String>,
    #[arg(long = "chorus")]
    pub chorus: Vec<String>,
}

impl RoleInputs {
    pub fn pairs(&self) -> [(&'static str, &[String]); 6] {
        [
            ("artist", &self.artist),
            ("composer", &self.composer),
            ("conductor", &self.conductor),
            ("musician", &self.musician),
            ("orchestra", &self.orchestra),
            ("chorus", &self.chorus),
        ]
    }

    pub fn get(&self, key: &str) -> &[String] {
        self.pairs()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap_or(&[])
    }
}

/// Mirrors RoleInputs' keys — whether to replace (rather than leave
/// untouched) each role's list on an edit. Named in the plural, as
/// the CLI flags read more naturally that way (--replace-artists).
#[derive(clap::Args, serde::Deserialize, Default)]
pub struct RoleReplaceFlags {
    #[arg(long)]
    pub replace_artists: bool,
    #[arg(long)]
    pub replace_composers: bool,
    #[arg(long)]
    pub replace_conductors: bool,
    #[arg(long)]
    pub replace_musicians: bool,
    #[arg(long)]
    pub replace_orchestras: bool,
    #[arg(long)]
    pub replace_choruses: bool,
}

impl RoleReplaceFlags {
    pub fn pairs(&self) -> [(&'static str, bool); 6] {
        [
            ("artist", self.replace_artists),
            ("composer", self.replace_composers),
            ("conductor", self.replace_conductors),
            ("musician", self.replace_musicians),
            ("orchestra", self.replace_orchestras),
            ("chorus", self.replace_choruses),
        ]
    }
}

struct RoleSpec {
    key: &'static str,
    role_table: &'static str,
    role_entity: &'static str,
    role_ent_id: i64,
    person_table: &'static str,
    person_entity: &'static str,
    person_ent_id: i64,
    back_ref: &'static str,
}

const ROLE_SPECS: &[RoleSpec] = &[
    RoleSpec {
        key: "artist",
        role_table: "ZARTIST",
        role_entity: "Artist",
        role_ent_id: ENT_ARTIST,
        person_table: "ZARTISTPERSON",
        person_entity: "ArtistPerson",
        person_ent_id: ENT_ARTIST_PERSON,
        back_ref: "ZARTISTPERSON",
    },
    RoleSpec {
        key: "composer",
        role_table: "ZCOMPOSER",
        role_entity: "Composer",
        role_ent_id: ENT_COMPOSER,
        person_table: "ZCOMPOSERPERSON",
        person_entity: "ComposerPerson",
        person_ent_id: ENT_COMPOSER_PERSON,
        back_ref: "ZCOMPOSERPERSON",
    },
    RoleSpec {
        key: "conductor",
        role_table: "ZCONDUCTOR",
        role_entity: "Conductor",
        role_ent_id: ENT_CONDUCTOR,
        person_table: "ZCONDUCTORPERSON",
        person_entity: "ConductorPerson",
        person_ent_id: ENT_CONDUCTOR_PERSON,
        back_ref: "ZCONDUCTORPERSON",
    },
    RoleSpec {
        key: "musician",
        role_table: "ZMUSICIAN",
        role_entity: "Musician",
        role_ent_id: ENT_MUSICIAN,
        person_table: "ZMUSICIANPERSON",
        person_entity: "MusicianPerson",
        person_ent_id: ENT_MUSICIAN_PERSON,
        back_ref: "ZMUSICIANPERSON",
    },
    RoleSpec {
        key: "orchestra",
        role_table: "ZORCHESTRA",
        role_entity: "Orchestra",
        role_ent_id: ENT_ORCHESTRA,
        person_table: "ZORCHESTRAPERSON",
        person_entity: "OrchestraPerson",
        person_ent_id: ENT_ORCHESTRA_PERSON,
        back_ref: "ZORCHESTRAPERSON",
    },
    RoleSpec {
        key: "chorus",
        role_table: "ZCHORUS",
        role_entity: "Chorus",
        role_ent_id: ENT_CHORUS,
        person_table: "ZCHORUSPERSON",
        person_entity: "ChorusPerson",
        person_ent_id: ENT_CHORUS_PERSON,
        back_ref: "ZCHORUSPERSON",
    },
];

fn role_spec(key: &str) -> &'static RoleSpec {
    ROLE_SPECS
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("clé de rôle inconnue: {key}"))
}

/// Genre/Tag/Extra/Studio are many-to-many via a Core Data join table
/// (e.g. Z_10GENRES) linking the album to a simple lookup entity — same
/// resolve-or-create as Format/Label/..., just through a join table
/// instead of a direct column on ZCOLLECTIBLE.
#[derive(clap::Args, serde::Deserialize, Default)]
pub struct ManyToManyInputs {
    #[arg(long = "genre")]
    pub genre: Vec<String>,
    #[arg(long = "tag")]
    pub tag: Vec<String>,
    #[arg(long = "extra")]
    pub extra: Vec<String>,
    #[arg(long = "studio")]
    pub studio: Vec<String>,
}

impl ManyToManyInputs {
    pub fn pairs(&self) -> [(&'static str, &[String]); 4] {
        [
            ("genre", &self.genre),
            ("tag", &self.tag),
            ("extra", &self.extra),
            ("studio", &self.studio),
        ]
    }

    pub fn get(&self, key: &str) -> &[String] {
        self.pairs()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap_or(&[])
    }
}

#[derive(clap::Args, serde::Deserialize, Default)]
pub struct ManyToManyReplaceFlags {
    #[arg(long)]
    pub replace_genres: bool,
    #[arg(long)]
    pub replace_tags: bool,
    #[arg(long)]
    pub replace_extras: bool,
    #[arg(long)]
    pub replace_studios: bool,
}

impl ManyToManyReplaceFlags {
    pub fn pairs(&self) -> [(&'static str, bool); 4] {
        [
            ("genre", self.replace_genres),
            ("tag", self.replace_tags),
            ("extra", self.replace_extras),
            ("studio", self.replace_studios),
        ]
    }
}

struct ManyToManySpec {
    key: &'static str,
    join_table: &'static str,
    album_col: &'static str,
    target_col: &'static str,
    target_table: &'static str,
    target_entity: &'static str,
    target_ent_id: i64,
    back_ref: &'static str,
}

const MANY_TO_MANY_SPECS: &[ManyToManySpec] = &[
    ManyToManySpec {
        key: "genre",
        join_table: "Z_10GENRES",
        album_col: "Z_10ALBUMS2",
        target_col: "Z_28GENRES",
        target_table: "ZGENRE",
        target_entity: "Genre",
        target_ent_id: ENT_GENRE,
        back_ref: "ZGENRE",
    },
    ManyToManySpec {
        key: "tag",
        join_table: "Z_8TAGS",
        album_col: "Z_8COLLECTIBLES",
        target_col: "Z_54TAGS",
        target_table: "ZTAG",
        target_entity: "Tag",
        target_ent_id: ENT_TAG,
        back_ref: "ZTAG",
    },
    ManyToManySpec {
        key: "extra",
        join_table: "Z_10EXTRAS",
        album_col: "Z_10ALBUMS1",
        target_col: "Z_26EXTRAS",
        target_table: "ZEXTRA",
        target_entity: "Extra",
        target_ent_id: ENT_EXTRA,
        back_ref: "ZEXTRA",
    },
    ManyToManySpec {
        key: "studio",
        join_table: "Z_10STUDIOS",
        album_col: "Z_10ALBUMS4",
        target_col: "Z_53STUDIOS",
        target_table: "ZSTUDIO",
        target_entity: "Studio",
        target_ent_id: ENT_STUDIO,
        back_ref: "ZSTUDIO",
    },
];

fn many_to_many_spec(key: &str) -> &'static ManyToManySpec {
    MANY_TO_MANY_SPECS
        .iter()
        .find(|s| s.key == key)
        .unwrap_or_else(|| panic!("clé many-to-many inconnue: {key}"))
}

/// Refuses to operate on what looks like a real, live Music Collector
/// collection — only ever a test/working copy. Same hard rule as the
/// BookNojo project's tools.
/// Renvoie `~/Documents/Collectorz` si `path` ressemble à une vraie
/// collection Music Collector en cours d'utilisation — permet à un
/// appelant de réagir différemment (le `--force` de la CLI) plutôt que
/// de toujours refuser comme `ensure_not_real_collection` ci-dessous.
/// Même forme que le `looks_like_real_collection` de BookNojo, juste
/// avec une détection propre à cette app.
pub fn looks_like_real_collection(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase();
    if text.contains("collectorz") && (text.contains("music collector") || text.contains("music_collector")) {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Documents").join("Collectorz"))
    } else {
        None
    }
}

pub fn ensure_not_real_collection(path: &Path) -> Result<()> {
    if looks_like_real_collection(path).is_some() {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        bail!(
            "{} ressemble au chemin d'une vraie collection Music Collector \
             (~/Documents/Collectorz/...) — refusé. N'utiliser que des copies de test.",
            canonical.display()
        );
    }
    Ok(())
}

/// Vérifie si Music Collector lui-même est actuellement lancé, via
/// `pgrep -x`. Écrire dans `Music_data` pendant que l'application le
/// garde ouvert risque un WAL incohérent, ou que l'application écrase
/// nos changements à sa prochaine sauvegarde — ceci doit renvoyer
/// `false` avant toute écriture réelle. Même vérification et même
/// raisonnement que `is_book_collector_running` de BookNojo
/// (`pgrep -x "Music Collector"` correspond au `CFBundleExecutable`
/// exact du bundle de l'application, confirmé directement depuis
/// l'Info.plist de l'application installée).
pub fn is_music_collector_running() -> Result<bool> {
    let output = std::process::Command::new("pgrep")
        .args(["-x", "Music Collector"])
        .output()
        .context("exécution de pgrep pour vérifier si Music Collector est lancé")?;
    // pgrep sort avec le code 0 s'il a trouvé au moins un processus
    // correspondant, 1 s'il n'en a trouvé aucun — les deux sont des
    // vérifications réussies, pas des échecs de pgrep lui-même, donc
    // seul son code de sortie (pas une erreur de lancement) distingue
    // les deux cas.
    Ok(output.status.success())
}

/// Copies the Music_data file to a timestamped backup next to it, before
/// any write — cheap (a few MB at most, no images involved) and undoes
/// the one real risk (a bad add/edit/delete), without needing to touch
/// Images/, which these operations never modify.
pub fn backup_database(db_path: &Path) -> Result<PathBuf> {
    let backups_dir = db_path
        .parent()
        .map(|p| p.join("backups"))
        .unwrap_or_else(|| PathBuf::from("backups"));
    std::fs::create_dir_all(&backups_dir)?;
    // Nanoseconds, not seconds: successive writes within the same second
    // (e.g. add then immediately edit) would otherwise collide on the
    // same filename and silently overwrite each other's backup.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut backup_path = backups_dir.join(format!("Music_data.{timestamp}.bak"));
    let mut suffix = 0;
    while backup_path.exists() {
        suffix += 1;
        backup_path = backups_dir.join(format!("Music_data.{timestamp}-{suffix}.bak"));
    }
    std::fs::copy(db_path, &backup_path)
        .with_context(|| format!("sauvegarde de {}", db_path.display()))?;
    Ok(backup_path)
}

/// The package's Images/ folder, given either the package directory or
/// its Music_data file directly.
pub fn images_dir(db: &Path) -> PathBuf {
    if db.is_dir() {
        db.join("Images")
    } else {
        db.parent()
            .map(|p| p.join("Images"))
            .unwrap_or_else(|| PathBuf::from("Images"))
    }
}

/// Lowercase alphanumeric words joined by underscores, truncated — same
/// shape as the filenames already used for existing covers (observed in
/// the package's own Images/ folder, e.g. "6_f_parsley_sage_rosemary_a.jpg").
pub fn slugify(title: &str, max_len: usize) -> String {
    let mut slug = String::new();
    for c in title.chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('_') && !slug.is_empty() {
            slug.push('_');
        }
    }
    let slug = slug.trim_matches('_');
    slug.chars().take(max_len).collect()
}

pub fn set_cover(
    db: &Path,
    album_id: i64,
    front: Option<&Path>,
    back: Option<&Path>,
    force: bool,
) -> Result<()> {
    if !force {
        ensure_not_real_collection(db)?;
    }
    if is_music_collector_running()? {
        bail!("Music Collector est actuellement lancé — fermez l'application avant d'écrire.");
    }
    let db_path = resolve_db_path(db);
    let images = images_dir(db);
    std::fs::create_dir_all(&images)?;
    backup_database(&db_path)?;

    let conn = Connection::open(&db_path)
        .with_context(|| format!("ouverture de {}", db_path.display()))?;
    let title: String = conn
        .query_row(
            "SELECT ZTITLE FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
            [album_id, ENT_ALBUM],
            |row| row.get(0),
        )
        .with_context(|| format!("Aucun album avec l'id {album_id}"))?;
    let slug = slugify(&title, 28);

    for (source, side, column) in [(front, "f", "ZFRONTCOVER"), (back, "b", "ZBACKCOVER")] {
        if let Some(source) = source {
            let ext = source
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            let filename = format!("{album_id}_{side}_{slug}.{ext}");
            std::fs::copy(source, images.join(&filename)).with_context(|| {
                format!("copie de {} vers Images/{filename}", source.display())
            })?;
            conn.execute(
                &format!("UPDATE ZCOLLECTIBLE SET {column} = ?1 WHERE Z_PK = ?2"),
                rusqlite::params![filename, album_id],
            )?;
            println!("Images/{filename} <- {}", source.display());
        }
    }
    Ok(())
}

/// Tracks Z_PRIMARYKEY.Z_MAX per entity across a whole plan (several new
/// rows of the same kind in one call must get distinct, sequential ids
/// before anything is actually written).
struct PkAllocator<'a> {
    conn: &'a Connection,
    cached_max: HashMap<String, i64>,
}

impl<'a> PkAllocator<'a> {
    fn new(conn: &'a Connection) -> Self {
        PkAllocator {
            conn,
            cached_max: HashMap::new(),
        }
    }

    fn next(&mut self, entity_name: &str) -> Result<i64> {
        if !self.cached_max.contains_key(entity_name) {
            let current: i64 = self.conn.query_row(
                "SELECT Z_MAX FROM Z_PRIMARYKEY WHERE Z_NAME = ?1",
                [entity_name],
                |row| row.get(0),
            )?;
            self.cached_max.insert(entity_name.to_string(), current);
        }
        let next = self.cached_max[entity_name] + 1;
        self.cached_max.insert(entity_name.to_string(), next);
        Ok(next)
    }

    /// One UPDATE per touched entity, reflecting the final allocated max.
    fn bump_statements(&self) -> Vec<String> {
        self.cached_max
            .iter()
            .map(|(name, max)| {
                format!("UPDATE Z_PRIMARYKEY SET Z_MAX = {max} WHERE Z_NAME = '{name}';")
            })
            .collect()
    }
}

fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Finds an existing simple lookup (Format/Label/Country/...) by exact
/// display name, or plans the two inserts needed to create one
/// (the lookup entity row + its ZLOOKUPITEMDELEGATE row).
#[allow(clippy::too_many_arguments)]
fn resolve_or_create_simple_lookup(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    table: &str,
    entity_name: &str,
    ent_id: i64,
    back_ref_column: &str,
    name: &str,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT t.Z_PK FROM {table} t \
                 JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE \
                 WHERE l.ZDISPLAYNAME = ?1"
            ),
            [name],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_id = pks.next(entity_name)?;
    let new_delegate_id = pks.next("LookupItemDelegate")?;
    sql.push(format!(
        "INSERT INTO {table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE, ZITEMDELEGATE) \
         VALUES ({new_id}, {ent_id}, 1, {new_delegate_id}, NULL);"
    ));
    sql.push(format!(
        "INSERT INTO ZLOOKUPITEMDELEGATE (Z_PK, Z_ENT, Z_OPT, {back_ref_column}, ZDISPLAYNAME) \
         VALUES ({new_delegate_id}, {ent}, 1, {new_id}, {name_sql});",
        ent = ENT_LOOKUP_ITEM_DELEGATE,
        name_sql = sql_quote(name)
    ));
    Ok(new_id)
}

/// Finds an existing Artist/Composer/.../person by exact display name,
/// or plans the delegate + role-person rows needed to create one.
#[allow(clippy::too_many_arguments)]
fn resolve_or_create_person(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    person_table: &str,
    person_entity_name: &str,
    person_ent_id: i64,
    back_ref_column: &str,
    name: &str,
) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            &format!(
                "SELECT pt.Z_PK FROM {person_table} pt \
                 JOIN ZPERSONDELEGATE p ON p.Z_PK = pt.ZDELEGATE \
                 WHERE p.ZDISPLAYNAME = ?1"
            ),
            [name],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let new_person_id = pks.next(person_entity_name)?;
    let new_delegate_id = pks.next("PersonDelegate")?;
    sql.push(format!(
        "INSERT INTO {person_table} (Z_PK, Z_ENT, Z_OPT, ZDELEGATE, ZITEMDELEGATE) \
         VALUES ({new_person_id}, {person_ent_id}, 1, {new_delegate_id}, NULL);"
    ));
    sql.push(format!(
        "INSERT INTO ZPERSONDELEGATE (Z_PK, Z_ENT, Z_OPT, {back_ref_column}, ZDISPLAYNAME, ZSORTNAME) \
         VALUES ({new_delegate_id}, {ent}, 1, {new_person_id}, {name_sql}, {name_sql});",
        ent = ENT_PERSON_DELEGATE,
        name_sql = sql_quote(name)
    ));
    Ok(new_person_id)
}

/// Resolves-or-creates each named person for one role, and plans the
/// role-link row (e.g. a ZARTIST row) for each, ranked in list order.
fn add_role_rows(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    spec: &RoleSpec,
    album_id: i64,
    names: &[String],
) -> Result<()> {
    for (rank, name) in names.iter().enumerate() {
        let person_id = resolve_or_create_person(
            conn,
            pks,
            sql,
            spec.person_table,
            spec.person_entity,
            spec.person_ent_id,
            spec.back_ref,
            name,
        )?;
        let role_id = pks.next(spec.role_entity)?;
        sql.push(format!(
            "INSERT INTO {role_table} (Z_PK, Z_ENT, Z_OPT, ZRANK, ZALBUM, {back_ref}) \
             VALUES ({role_id}, {role_ent_id}, 1, {rank}, {album_id}, {person_id});",
            role_table = spec.role_table,
            role_ent_id = spec.role_ent_id,
            back_ref = spec.back_ref,
        ));
    }
    Ok(())
}

/// Resolves-or-creates each named lookup for one many-to-many field
/// (Genre/Tag/Extra/Studio), and plans the join-table row for each.
fn add_many_to_many_rows(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    spec: &ManyToManySpec,
    album_id: i64,
    names: &[String],
) -> Result<()> {
    for name in names {
        let target_id = resolve_or_create_simple_lookup(
            conn,
            pks,
            sql,
            spec.target_table,
            spec.target_entity,
            spec.target_ent_id,
            spec.back_ref,
            name,
        )?;
        sql.push(format!(
            "INSERT INTO {join_table} ({album_col}, {target_col}) VALUES ({album_id}, {target_id});",
            join_table = spec.join_table,
            album_col = spec.album_col,
            target_col = spec.target_col,
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn resolve_simple_fields(
    conn: &Connection,
    pks: &mut PkAllocator,
    sql: &mut Vec<String>,
    fields: &AlbumFields,
) -> Result<HashMap<&'static str, i64>> {
    let mut ids = HashMap::new();
    macro_rules! resolve {
        ($field:expr, $key:literal, $table:literal, $entity:literal, $ent_id:expr, $back_ref:literal) => {
            if let Some(name) = $field.as_deref() {
                let id = resolve_or_create_simple_lookup(
                    conn, pks, sql, $table, $entity, $ent_id, $back_ref, name,
                )?;
                ids.insert($key, id);
            }
        };
    }
    resolve!(fields.format, "format", "ZFORMAT", "Format", ENT_FORMAT, "ZFORMAT");
    resolve!(fields.label, "label", "ZLABEL", "Label", ENT_LABEL, "ZLABEL");
    resolve!(fields.country, "country", "ZCOUNTRY", "Country", ENT_COUNTRY, "ZCOUNTRY");
    resolve!(fields.condition, "condition", "ZCONDITION", "Condition", ENT_CONDITION, "ZCONDITION");
    resolve!(fields.packaging, "packaging", "ZPACKAGING", "Packaging", ENT_PACKAGING, "ZPACKAGING");
    resolve!(fields.vinyl_color, "vinyl_color", "ZVINYLCOLOR", "VinylColor", ENT_VINYL_COLOR, "ZVINYLCOLOR");
    resolve!(fields.spars, "spars", "ZSPARS", "Spars", ENT_SPARS, "ZSPARS");
    resolve!(fields.location, "location", "ZLOCATION", "Location", ENT_LOCATION, "ZLOCATION");
    // Owner is a person (ZPERSONDELEGATE), not a category value — see
    // the Lookups::owners doc comment. resolve_or_create_person is the
    // same helper Artist/Composer/... use.
    if let Some(name) = fields.owner.as_deref() {
        let id = resolve_or_create_person(conn, pks, sql, "ZOWNER", "Owner", ENT_OWNER, "ZOWNER", name)?;
        ids.insert("owner", id);
    }
    if let Some(name) = fields.collection.as_deref() {
        let id = resolve_collection_pk(conn, name)?;
        ids.insert("collection", id);
    }
    Ok(ids)
}

/// A sub-collection resolves by exact title match only — unlike every
/// other lookup field, there is no resolve-or-*create* here: an
/// album's sub-collection is always one the app/CLI caller already
/// knows exists (it comes from a list of real, already-loaded tabs),
/// never free-typed, so creating one on a miss would only ever mask a
/// caller bug.
fn resolve_collection_pk(conn: &Connection, name: &str) -> Result<i64> {
    conn.query_row("SELECT Z_PK FROM ZCOLLECTION WHERE ZTITLE = ?1", [name], |row| row.get(0))
        .with_context(|| format!("sous-collection « {name} » introuvable — elle doit déjà exister"))
}

fn scalar_text_assignments(fields: &AlbumFields) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    if let Some(v) = &fields.release_year {
        out.push(("ZRELEASEYEAR", sql_quote(v)));
    }
    if let Some(v) = fields.rating {
        out.push(("ZMYRATING", v.to_string()));
    }
    if let Some(v) = &fields.notes {
        out.push(("ZNOTES", sql_quote(v)));
    }
    if let Some(v) = &fields.upc {
        out.push(("ZUPC", sql_quote(v)));
    }
    out
}

/// One field of an `AddAlbumPreview` — a single artist/genre/format/...
/// value the user typed, and whether it already exists in the
/// collection or would be created brand-new.
#[derive(Debug, Serialize)]
pub struct ResolvedFieldSummary {
    pub label: String,
    pub value: String,
    pub is_new: bool,
}

/// Preview returned to the app's confirmation step before any write —
/// which values (if any) a `build_plan` call with these same inputs
/// would create as brand-new taxonomy entries. Same purpose as
/// BookNojo's own `prepare_add_book`/`AddBookPreview`: catch a typo
/// (an unintended near-duplicate artist/genre/...) before it becomes a
/// permanent entry, by showing it before the real write happens.
#[derive(Debug, Serialize)]
pub struct AddAlbumPreview {
    pub title: String,
    pub fields: Vec<ResolvedFieldSummary>,
    pub new_count: usize,
}

fn person_exists(conn: &Connection, person_table: &str, name: &str) -> bool {
    conn.query_row(
        &format!(
            "SELECT pt.Z_PK FROM {person_table} pt \
             JOIN ZPERSONDELEGATE p ON p.Z_PK = pt.ZDELEGATE \
             WHERE p.ZDISPLAYNAME = ?1"
        ),
        [name],
        |row| row.get::<_, i64>(0),
    )
    .is_ok()
}

fn simple_lookup_exists(conn: &Connection, table: &str, name: &str) -> bool {
    conn.query_row(
        &format!(
            "SELECT t.Z_PK FROM {table} t \
             JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE \
             WHERE l.ZDISPLAYNAME = ?1"
        ),
        [name],
        |row| row.get::<_, i64>(0),
    )
    .is_ok()
}

fn role_label(key: &'static str) -> &'static str {
    match key {
        "artist" => "Artist",
        "composer" => "Composer",
        "conductor" => "Conductor",
        "musician" => "Musician",
        "orchestra" => "Orchestra",
        "chorus" => "Chorus",
        _ => key,
    }
}

fn many_to_many_label(key: &'static str) -> &'static str {
    match key {
        "genre" => "Genre",
        "tag" => "Tag",
        "extra" => "Extra",
        "studio" => "Studio",
        _ => key,
    }
}

/// Read-only: resolves `roles`/`many`/`fields` against `conn` exactly as
/// `build_plan` would, but only to report which values are new versus
/// already-existing — never allocates a row or writes anything. Plain
/// text/number fields (release_year, rating, notes, upc) are skipped —
/// they can't create unwanted taxonomy entries, so there's nothing to
/// flag about them.
pub fn preview_add_album(
    conn: &Connection,
    title: &str,
    roles: &RoleInputs,
    many: &ManyToManyInputs,
    fields: &AlbumFields,
) -> Result<AddAlbumPreview> {
    let mut summary = Vec::new();

    for spec in ROLE_SPECS {
        for name in roles.get(spec.key) {
            summary.push(ResolvedFieldSummary {
                label: role_label(spec.key).to_string(),
                value: name.clone(),
                is_new: !person_exists(conn, spec.person_table, name),
            });
        }
    }
    for spec in MANY_TO_MANY_SPECS {
        for name in many.get(spec.key) {
            summary.push(ResolvedFieldSummary {
                label: many_to_many_label(spec.key).to_string(),
                value: name.clone(),
                is_new: !simple_lookup_exists(conn, spec.target_table, name),
            });
        }
    }
    macro_rules! simple {
        ($field:expr, $label:literal, $table:literal) => {
            if let Some(name) = $field.as_deref() {
                summary.push(ResolvedFieldSummary {
                    label: $label.to_string(),
                    value: name.to_string(),
                    is_new: !simple_lookup_exists(conn, $table, name),
                });
            }
        };
    }
    simple!(fields.format, "Format", "ZFORMAT");
    simple!(fields.label, "Label", "ZLABEL");
    simple!(fields.country, "Country", "ZCOUNTRY");
    simple!(fields.condition, "Condition", "ZCONDITION");
    simple!(fields.packaging, "Packaging", "ZPACKAGING");
    simple!(fields.vinyl_color, "Vinyl color", "ZVINYLCOLOR");
    simple!(fields.spars, "Spars", "ZSPARS");
    simple!(fields.location, "Location", "ZLOCATION");
    if let Some(name) = fields.owner.as_deref() {
        summary.push(ResolvedFieldSummary {
            label: "Owner".to_string(),
            value: name.to_string(),
            is_new: !person_exists(conn, "ZOWNER", name),
        });
    }

    let new_count = summary.iter().filter(|f| f.is_new).count();
    Ok(AddAlbumPreview {
        title: title.to_string(),
        fields: summary,
        new_count,
    })
}

/// Current time as a Core Data reference-date timestamp (seconds since
/// 2001-01-01), the same representation `ZDATEADDED` itself uses —
/// stamped automatically when an album is added, same as the real
/// Music Collector app does. Unlike every other field in `build_plan`,
/// this is never optional or caller-supplied: there's no sensible
/// user-typed value for "when this was added", and editing an album
/// never touches it (it stays the original add time).
fn current_core_data_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        - 978_307_200.0
}

pub fn build_plan(
    conn: &Connection,
    title: &str,
    roles: &RoleInputs,
    many: &ManyToManyInputs,
    fields: &AlbumFields,
    tracks: &[String],
) -> Result<(String, i64)> {
    let mut pks = PkAllocator::new(conn);
    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];

    let lookup_ids = resolve_simple_fields(conn, &mut pks, &mut sql, fields)?;

    let album_id = pks.next("Collectible")?;
    let mut columns = vec![
        "Z_PK".to_string(),
        "Z_ENT".to_string(),
        "Z_OPT".to_string(),
        "ZCOLLECTIONSTATUS".to_string(),
        "ZNRTRACKS".to_string(),
        "ZTITLE".to_string(),
        "ZDATEADDED".to_string(),
    ];
    let mut values = vec![
        album_id.to_string(),
        ENT_ALBUM.to_string(),
        "1".to_string(),
        "2".to_string(),
        tracks.len().to_string(),
        sql_quote(title),
        current_core_data_timestamp().to_string(),
    ];
    for (key, column) in [
        ("format", "ZFORMAT"),
        ("label", "ZLABEL"),
        ("country", "ZCOUNTRY"),
        ("condition", "ZCONDITION"),
        ("packaging", "ZPACKAGING"),
        ("vinyl_color", "ZVINYLCOLOR"),
        ("spars", "ZSPARS"),
        ("owner", "ZOWNER"),
        ("location", "ZLOCATION"),
        ("collection", "ZCOLLECTION"),
    ] {
        if let Some(id) = lookup_ids.get(key) {
            columns.push(column.to_string());
            values.push(id.to_string());
        }
    }
    for (column, value) in scalar_text_assignments(fields) {
        columns.push(column.to_string());
        values.push(value);
    }
    sql.push(format!(
        "INSERT INTO ZCOLLECTIBLE ({}) VALUES ({});",
        columns.join(", "),
        values.join(", ")
    ));

    for (key, names) in roles.pairs() {
        if !names.is_empty() {
            add_role_rows(conn, &mut pks, &mut sql, role_spec(key), album_id, names)?;
        }
    }

    for (key, names) in many.pairs() {
        if !names.is_empty() {
            add_many_to_many_rows(
                conn,
                &mut pks,
                &mut sql,
                many_to_many_spec(key),
                album_id,
                names,
            )?;
        }
    }

    if !tracks.is_empty() {
        let disc_id = pks.next("Collectible")?;
        sql.push(format!(
            "INSERT INTO ZCOLLECTIBLE (Z_PK, Z_ENT, Z_OPT, ZALBUM) \
             VALUES ({disc_id}, {ENT_DISC}, 1, {album_id});"
        ));
        for (position, track_title) in tracks.iter().enumerate() {
            let track_id = pks.next("Collectible")?;
            sql.push(format!(
                "INSERT INTO ZCOLLECTIBLE (Z_PK, Z_ENT, Z_OPT, ZDISC1, ZPOSITION, ZTITLE) \
                 VALUES ({track_id}, {ENT_TRACK}, 1, {disc_id}, {position}, {title_sql});",
                position = position + 1,
                title_sql = sql_quote(track_title),
            ));
        }
    }

    sql.extend(pks.bump_statements());
    sql.push("COMMIT;".to_string());
    Ok((sql.join("\n"), album_id))
}

#[allow(clippy::too_many_arguments)]
pub fn build_edit_plan(
    conn: &Connection,
    album_id: i64,
    title: Option<&str>,
    fields: &AlbumFields,
    replace: &RoleReplaceFlags,
    roles: &RoleInputs,
    replace_many: &ManyToManyReplaceFlags,
    many: &ManyToManyInputs,
) -> Result<String> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
        [album_id, ENT_ALBUM],
        |row| row.get(0),
    )?;
    if exists == 0 {
        bail!("Aucun album avec l'id {album_id}");
    }

    let mut pks = PkAllocator::new(conn);
    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];

    let lookup_ids = resolve_simple_fields(conn, &mut pks, &mut sql, fields)?;
    let mut assignments: Vec<String> = Vec::new();
    if let Some(t) = title {
        assignments.push(format!("ZTITLE = {}", sql_quote(t)));
    }
    for (key, column) in [
        ("format", "ZFORMAT"),
        ("label", "ZLABEL"),
        ("country", "ZCOUNTRY"),
        ("condition", "ZCONDITION"),
        ("packaging", "ZPACKAGING"),
        ("vinyl_color", "ZVINYLCOLOR"),
        ("spars", "ZSPARS"),
        ("owner", "ZOWNER"),
        ("location", "ZLOCATION"),
        ("collection", "ZCOLLECTION"),
    ] {
        if let Some(id) = lookup_ids.get(key) {
            assignments.push(format!("{column} = {id}"));
        }
    }
    for (column, value) in scalar_text_assignments(fields) {
        assignments.push(format!("{column} = {value}"));
    }
    if !assignments.is_empty() {
        sql.push(format!(
            "UPDATE ZCOLLECTIBLE SET {} WHERE Z_PK = {album_id};",
            assignments.join(", ")
        ));
    }

    for (key, do_replace) in replace.pairs() {
        if do_replace {
            let spec = role_spec(key);
            sql.push(format!(
                "DELETE FROM {} WHERE ZALBUM = {album_id};",
                spec.role_table
            ));
            add_role_rows(conn, &mut pks, &mut sql, spec, album_id, roles.get(key))?;
        }
    }

    for (key, do_replace) in replace_many.pairs() {
        if do_replace {
            let spec = many_to_many_spec(key);
            sql.push(format!(
                "DELETE FROM {} WHERE {} = {album_id};",
                spec.join_table, spec.album_col
            ));
            add_many_to_many_rows(conn, &mut pks, &mut sql, spec, album_id, many.get(key))?;
        }
    }

    sql.extend(pks.bump_statements());
    sql.push("COMMIT;".to_string());
    Ok(sql.join("\n"))
}

/// Deletes an album and everything specific to it — discs, tracks, and
/// its role/many-to-many link rows — but never the shared lookups
/// (Format/Label/.../ArtistPerson/PersonDelegate), since other albums
/// may still reference them. Leaving an unreferenced lookup row behind
/// is harmless; deleting one still referenced elsewhere is not.
pub fn build_delete_plan(conn: &Connection, album_id: i64) -> Result<String> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ZCOLLECTIBLE WHERE Z_PK = ?1 AND Z_ENT = ?2",
        [album_id, ENT_ALBUM],
        |row| row.get(0),
    )?;
    if exists == 0 {
        bail!("Aucun album avec l'id {album_id}");
    }

    let disc_ids: Vec<i64> = conn
        .prepare("SELECT Z_PK FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 AND ZALBUM = ?2")?
        .query_map([ENT_DISC, album_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    let disc_list = disc_ids
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let mut sql: Vec<String> = vec!["BEGIN;".to_string()];
    for role_table in [
        "ZARTIST", "ZCOMPOSER", "ZCONDUCTOR", "ZMUSICIAN", "ZORCHESTRA", "ZCHORUS", "ZCREDIT",
    ] {
        sql.push(format!("DELETE FROM {role_table} WHERE ZALBUM = {album_id};"));
    }
    sql.push(format!(
        "DELETE FROM Z_10GENRES WHERE Z_10ALBUMS2 = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_8TAGS WHERE Z_8COLLECTIBLES = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_10EXTRAS WHERE Z_10ALBUMS1 = {album_id};"
    ));
    sql.push(format!(
        "DELETE FROM Z_10STUDIOS WHERE Z_10ALBUMS4 = {album_id};"
    ));
    if !disc_ids.is_empty() {
        sql.push(format!(
            "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_TRACK} AND ZDISC1 IN ({disc_list});"
        ));
        sql.push(format!(
            "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_DISC} AND Z_PK IN ({disc_list});"
        ));
    }
    sql.push(format!(
        "DELETE FROM ZCOLLECTIBLE WHERE Z_ENT = {ENT_ALBUM} AND Z_PK = {album_id};"
    ));
    sql.push("COMMIT;".to_string());
    Ok(sql.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // `HOME` is process-global; tests run on separate threads by default,
    // so any test mutating it must hold this lock for its whole duration
    // (same precaution BookNojo's own equivalent test needed after a real
    // race was caught there — see that project's `HOME_ENV_LOCK`).
    static HOME_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_root(test_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("mc_export_it_{test_name}_{}_{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_mnj_destination_uses_the_fixed_documents_musicnojo_folder() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", "/tmp/mc_export_test_home");

        assert_eq!(
            default_mnj_destination(Path::new("/Users/x/Documents/Collectorz/Music Collector/MyCollection.mucp"))
                .unwrap(),
            PathBuf::from("/tmp/mc_export_test_home/Documents/MusicNojo/MyCollection.mnj")
        );

        match original_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }

    #[test]
    fn default_mnj_destination_fails_clearly_without_home() {
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let original_home = std::env::var("HOME").ok();
        std::env::remove_var("HOME");

        let err = default_mnj_destination(Path::new("MyCollection.mucp")).unwrap_err();
        assert!(err.to_string().contains("HOME"));

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    /// Uses the committed `sample-collection/` fixture (unlike
    /// `testdata/`, gitignored/personal — see this file's other test)
    /// so this one always runs, anywhere this repo is cloned.
    #[test]
    fn import_collection_to_copies_the_whole_package_and_counts_albums() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-collection/MyCollection.mucp");
        let root = temp_root("import_collection_to_copy");
        let dest = root.join("Imported.mnj");

        let outcome = import_collection_to(&source, &dest).expect("import_collection_to");

        assert_eq!(outcome.dest, dest);
        assert_eq!(outcome.album_count, 12);
        assert!(dest.join("Music_data").is_file(), "Music_data should have been copied");
        assert!(dest.join("Images").is_dir(), "Images/ should have been copied");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn import_collection_to_refuses_to_overwrite_an_existing_destination() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("sample-collection/MyCollection.mucp");
        let root = temp_root("import_collection_to_no_overwrite");
        let dest = root.join("AlreadyThere.mnj");
        std::fs::create_dir_all(&dest).expect("pre-create destination");

        let err = import_collection_to(&source, &dest).expect_err("should refuse an existing destination");
        assert!(err.to_string().contains("existe déjà"), "unexpected error message: {err}");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Replays, outside of any dialog callback, the full chain "File >
    /// Import Collection…" runs once a `.mucp` has been picked:
    /// `default_mnj_destination` then `import_collection_to`, against the
    /// real personal `testdata/` fixture (which — unlike `sample-collection/`
    /// — has a real `preferences.plist`, matching what a real Music
    /// Collector export looks like). Gitignored/personal, like this file's
    /// other `testdata/`-dependent test — skipped where absent.
    #[test]
    fn the_full_import_chain_bootstraps_an_mnj_at_the_fixed_destination() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/MyCollection.mucp");
        if !source.exists() {
            eprintln!("testdata/MyCollection.mucp absent (copie personnelle, non committée) — test ignoré");
            return;
        }
        let _guard = HOME_ENV_LOCK.lock().unwrap();
        let fake_home = std::env::temp_dir().join("musicnojo_full_import_chain_test_home");
        let _ = std::fs::remove_dir_all(&fake_home);
        std::fs::create_dir_all(&fake_home).unwrap();
        let previous_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &fake_home);

        let dest = default_mnj_destination(&source).expect("dest computation must succeed");
        assert_eq!(dest, fake_home.join("Documents/MusicNojo/MyCollection.mnj"));

        let outcome = import_collection_to(&source, &dest).expect("import must succeed");
        assert!(dest.join("Music_data").is_file());
        assert!(dest.join("Images").is_dir());
        assert!(outcome.album_count > 0);

        match previous_home {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&fake_home);
    }

    #[test]
    fn exports_against_local_test_copy_if_present() {
        let package = std::path::Path::new("testdata/MyCollection.mucp");
        if !package.exists() {
            eprintln!(
                "testdata/MyCollection.mucp absent (copie personnelle, non committée) — test ignoré"
            );
            return;
        }
        let conn = open_readonly(package).unwrap();
        let albums = load_albums(&conn).unwrap();
        assert!(!albums.is_empty());
        let total_tracks: i64 = albums
            .iter()
            .flat_map(|a| &a.discs)
            .map(|d| d.tracks.len() as i64)
            .sum();
        assert!(total_tracks > 0);
    }
}
