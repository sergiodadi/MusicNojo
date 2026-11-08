use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

// Core Data entity ids for the Collectible-family hierarchy
// (Collectible -> MusicCollectible -> Album/Disc/Header/SubTrack/Track).
// These ids are stable across collections created by the same Music
// Collector version, so they are hardcoded below rather than resolved
// at runtime.
const ENT_ALBUM: i64 = 10;
const ENT_DISC: i64 = 11;
const ENT_TRACK: i64 = 14;

#[derive(Parser)]
struct Args {
    /// Path to a MyCollection.mucp package (read-only copy only, never the live file)
    #[arg(long)]
    db: PathBuf,

    /// Write a flat per-album summary to this CSV file
    #[arg(long)]
    csv: Option<PathBuf>,

    /// Write the full nested collection (albums/discs/tracks) to this JSON file
    #[arg(long)]
    json: Option<PathBuf>,
}

#[derive(Serialize, Clone)]
struct Track {
    position: Option<i64>,
    title: String,
    length: Option<String>,
}

#[derive(Serialize)]
struct Disc {
    index: Option<String>,
    tracks: Vec<Track>,
}

#[derive(Serialize, Clone)]
struct Credit {
    name: String,
    // ZCREDIT.ZROLEID: no corresponding label exists anywhere in
    // Music_data itself — left unresolved, not mapped to text. Flagged
    // separately rather than guessed.
    role_id: Option<i64>,
}

#[derive(Serialize)]
struct Album {
    id: i64,
    title: String,
    artists: Vec<String>,
    composers: Vec<String>,
    conductors: Vec<String>,
    musicians: Vec<String>,
    orchestras: Vec<String>,
    choruses: Vec<String>,
    credits: Vec<Credit>,
    genres: Vec<String>,
    tags: Vec<String>,
    extras: Vec<String>,
    studios: Vec<String>,
    label: Option<String>,
    format: Option<String>,
    country: Option<String>,
    condition: Option<String>,
    packaging: Option<String>,
    vinyl_color: Option<String>,
    spars: Option<String>,
    owner: Option<String>,
    location: Option<String>,
    collection: Option<String>,
    release_year: Option<String>,
    rating: Option<i64>,
    notes: Option<String>,
    upc: Option<String>,
    front_cover: Option<String>,
    back_cover: Option<String>,
    thumbnail: Option<String>,
    backdrop: Option<String>,
    track_count: i64,
    discs: Vec<Disc>,
}

/// Bulk-loaded lookups, keyed so that assembling each album is a set of
/// in-memory map lookups instead of running fresh SQL per album (the
/// previous version ran ~20 small queries per album — ~45,000 total for
/// a 2300-album collection; this version runs one query per lookup kind).
struct Lookups {
    simple: HashMap<&'static str, HashMap<i64, String>>,
    collections: HashMap<i64, String>,
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
    "ZOWNER",
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
    release_year: Option<String>,
    rating: Option<i64>,
    notes: Option<String>,
    upc: Option<String>,
    front_cover: Option<String>,
    back_cover: Option<String>,
    thumbnail: Option<String>,
    backdrop: Option<String>,
    track_count: i64,
}

fn load_albums(conn: &Connection) -> Result<Vec<Album>> {
    let lookups = load_lookups(conn)?;

    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZFORMAT, ZCOUNTRY, ZLABEL, ZCONDITION, ZPACKAGING, \
                ZVINYLCOLOR, ZSPARS, ZOWNER, ZLOCATION, ZCOLLECTION, ZRELEASEYEAR, \
                ZMYRATING, ZNOTES, ZUPC, ZFRONTCOVER, ZBACKCOVER, ZTHUMBFILEPATH, \
                ZBACKDROP, ZNRTRACKS \
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
                track_count: row.get(20)?,
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
            release_year,
            rating,
            notes,
            upc,
            front_cover,
            back_cover,
            thumbnail,
            backdrop,
            track_count,
        } = row;

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
            owner: lookups.simple("ZOWNER", owner_pk),
            location: lookups.simple("ZLOCATION", location_pk),
            collection: collection_pk.and_then(|pk| lookups.collections.get(&pk).cloned()),
            release_year,
            rating,
            notes,
            upc,
            front_cover,
            back_cover,
            thumbnail,
            backdrop,
            track_count,
            discs: lookups.discs(id),
        });
    }
    Ok(albums)
}

fn write_csv(path: &PathBuf, albums: &[Album]) -> Result<()> {
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

fn main() -> Result<()> {
    let args = Args::parse();

    let db_path = if args.db.is_dir() {
        args.db.join("Music_data")
    } else {
        args.db.clone()
    };

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("ouverture de {}", db_path.display()))?;

    let albums = load_albums(&conn)?;
    println!("{} albums trouvés.", albums.len());
    let total_tracks: i64 = albums
        .iter()
        .flat_map(|a| &a.discs)
        .map(|d| d.tracks.len() as i64)
        .sum();
    println!("{total_tracks} morceaux trouvés (dans les disques rattachés).");

    if let Some(csv_path) = &args.csv {
        write_csv(csv_path, &albums)?;
        println!("CSV écrit dans: {}", csv_path.display());
    }
    if let Some(json_path) = &args.json {
        let file = std::fs::File::create(json_path)
            .with_context(|| format!("écriture de {}", json_path.display()))?;
        serde_json::to_writer_pretty(file, &albums)?;
        println!("JSON écrit dans: {}", json_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_against_local_test_copy_if_present() {
        let package = std::path::Path::new("testdata/MyCollection.mucp");
        if !package.exists() {
            eprintln!(
                "testdata/MyCollection.mucp absent (copie personnelle, non committée) — test ignoré"
            );
            return;
        }
        let conn = Connection::open_with_flags(
            package.join("Music_data"),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
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
