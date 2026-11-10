use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
pub use rusqlite::Connection;
use serde::Serialize;

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
    track_count: i64,
}

pub fn load_albums(conn: &Connection) -> Result<Vec<Album>> {
    let lookups = load_lookups(conn)?;

    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZFORMAT, ZCOUNTRY, ZLABEL, ZCONDITION, ZPACKAGING, \
                ZVINYLCOLOR, ZSPARS, ZOWNER, ZLOCATION, ZCOLLECTION, ZRELEASEYEAR, \
                ZMYRATING, ZNOTES, ZUPC, ZFRONTCOVER, ZBACKCOVER, ZTHUMBFILEPATH, \
                ZBACKDROP, ZNRTRACKS, \
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
                track_count: row.get(20)?,
                date_added: row.get(21)?,
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
            discs: lookups.discs(id),
        });
    }
    Ok(albums)
}

pub fn write_html(path: &Path, albums: &[Album]) -> Result<()> {
    #[derive(Serialize)]
    struct Row<'a> {
        title: &'a str,
        artists: String,
        genres: String,
        format: &'a str,
        country: &'a str,
        release_year: &'a str,
        track_count: i64,
    }

    let rows: Vec<Row> = albums
        .iter()
        .map(|a| Row {
            title: &a.title,
            artists: a.artists.join(", "),
            genres: a.genres.join(", "),
            format: a.format.as_deref().unwrap_or(""),
            country: a.country.as_deref().unwrap_or(""),
            release_year: a.release_year.as_deref().unwrap_or(""),
            track_count: a.track_count,
        })
        .collect();
    // Escaped so a title/artist containing "</script>" can't break out of
    // the embedding <script> tag.
    let data_json = serde_json::to_string(&rows)?.replace("</", "<\\/");

    let html = format!(
        r##"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<title>MusicNojo — Collection</title>
<style>
  body {{ font-family: -apple-system, system-ui, sans-serif; margin: 2rem; color: #222; }}
  h1 {{ font-size: 1.3rem; }}
  input {{ font-size: 1rem; padding: 0.4rem; width: 100%; max-width: 24rem; margin-bottom: 1rem; }}
  table {{ border-collapse: collapse; width: 100%; }}
  th, td {{ text-align: left; padding: 0.3rem 0.6rem; border-bottom: 1px solid #ddd; }}
  th {{ cursor: pointer; user-select: none; position: sticky; top: 0; background: #fafafa; }}
  #count {{ color: #666; font-size: 0.9rem; margin-bottom: 0.5rem; }}
</style>
</head>
<body>
<h1>MusicNojo — Collection ({count} albums)</h1>
<input id="search" type="search" placeholder="Rechercher par titre ou artiste…" autofocus>
<div id="count"></div>
<table id="t">
<thead><tr>
  <th data-k="title">Titre</th>
  <th data-k="artists">Artiste(s)</th>
  <th data-k="genres">Genre(s)</th>
  <th data-k="format">Format</th>
  <th data-k="country">Pays</th>
  <th data-k="release_year">Année</th>
  <th data-k="track_count">Pistes</th>
</tr></thead>
<tbody></tbody>
</table>
<script>
const ALBUMS = {data_json};
let sortKey = "title", sortAsc = true;

function render(filterText) {{
  const needle = filterText.trim().toLowerCase();
  let rows = ALBUMS.filter(a =>
    !needle || a.title.toLowerCase().includes(needle) || a.artists.toLowerCase().includes(needle)
  );
  rows.sort((a, b) => {{
    const va = a[sortKey], vb = b[sortKey];
    const cmp = typeof va === "number" ? va - vb : String(va).localeCompare(String(vb));
    return sortAsc ? cmp : -cmp;
  }});
  document.getElementById("count").textContent = rows.length + " résultat(s)";
  const body = document.querySelector("#t tbody");
  body.innerHTML = rows.map(a => `<tr>
    <td>${{a.title}}</td><td>${{a.artists}}</td><td>${{a.genres}}</td>
    <td>${{a.format}}</td><td>${{a.country}}</td><td>${{a.release_year}}</td><td>${{a.track_count}}</td>
  </tr>`).join("");
}}

document.getElementById("search").addEventListener("input", e => render(e.target.value));
document.querySelectorAll("th").forEach(th => th.addEventListener("click", () => {{
  const k = th.dataset.k;
  sortAsc = (sortKey === k) ? !sortAsc : true;
  sortKey = k;
  render(document.getElementById("search").value);
}}));
render("");
</script>
</body>
</html>
"##,
        count = albums.len(),
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
pub fn ensure_not_real_collection(path: &Path) -> Result<()> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase();
    if text.contains("collectorz") && (text.contains("music collector") || text.contains("music_collector")) {
        bail!(
            "{} ressemble au chemin d'une vraie collection Music Collector \
             (~/Documents/Collectorz/...) — refusé. N'utiliser que des copies de test.",
            canonical.display()
        );
    }
    Ok(())
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

pub fn set_cover(db: &Path, album_id: i64, front: Option<&Path>, back: Option<&Path>) -> Result<()> {
    ensure_not_real_collection(db)?;
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
    Ok(ids)
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

pub fn build_plan(
    conn: &Connection,
    title: &str,
    roles: &RoleInputs,
    many: &ManyToManyInputs,
    fields: &AlbumFields,
    tracks: &[String],
) -> Result<String> {
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
    ];
    let mut values = vec![
        album_id.to_string(),
        ENT_ALBUM.to_string(),
        "1".to_string(),
        "2".to_string(),
        tracks.len().to_string(),
        sql_quote(title),
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
    Ok(sql.join("\n"))
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
