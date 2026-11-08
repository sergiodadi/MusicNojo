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

#[derive(Serialize)]
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

#[derive(Serialize)]
struct Album {
    id: i64,
    title: String,
    artists: Vec<String>,
    genres: Vec<String>,
    label: Option<String>,
    format: Option<String>,
    country: Option<String>,
    release_year: Option<String>,
    rating: Option<i64>,
    track_count: i64,
    discs: Vec<Disc>,
}

fn resolve_simple_lookup(conn: &Connection, table: &str, pk: i64) -> Result<Option<String>> {
    let sql = format!(
        "SELECT l.ZDISPLAYNAME FROM {table} t \
         JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = t.ZDELEGATE \
         WHERE t.Z_PK = ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let name: Option<String> = stmt.query_row([pk], |row| row.get(0)).ok();
    Ok(name)
}

fn load_artists(conn: &Connection, album_pk: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT p.ZDISPLAYNAME FROM ZARTIST a \
         JOIN ZARTISTPERSON ap ON ap.Z_PK = a.ZARTISTPERSON \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = ap.ZDELEGATE \
         WHERE a.ZALBUM = ?1 ORDER BY a.ZRANK",
    )?;
    let names = stmt
        .query_map([album_pk], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

fn load_genres(conn: &Connection, album_pk: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT l.ZDISPLAYNAME FROM Z_10GENRES j \
         JOIN ZGENRE g ON g.Z_PK = j.Z_28GENRES \
         JOIN ZLOOKUPITEMDELEGATE l ON l.Z_PK = g.ZDELEGATE \
         WHERE j.Z_10ALBUMS2 = ?1",
    )?;
    let names = stmt
        .query_map([album_pk], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

fn load_discs(conn: &Connection, album_pk: i64) -> Result<Vec<Disc>> {
    let mut disc_stmt = conn.prepare(
        "SELECT Z_PK, ZDISCINDEX FROM ZCOLLECTIBLE \
         WHERE Z_ENT = ?1 AND ZALBUM = ?2 ORDER BY ZDISCINDEX",
    )?;
    let discs_raw: Vec<(i64, Option<String>)> = disc_stmt
        .query_map([ENT_DISC, album_pk], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut track_stmt = conn.prepare(
        "SELECT ZPOSITION, ZTITLE, ZLENGTHMINS, ZLENGTHSECS FROM ZCOLLECTIBLE \
         WHERE Z_ENT = ?1 AND ZDISC1 = ?2 ORDER BY ZPOSITION",
    )?;

    let mut discs = Vec::new();
    for (disc_pk, index) in discs_raw {
        let tracks = track_stmt
            .query_map([ENT_TRACK, disc_pk], |row| {
                let position: Option<i64> = row.get(0)?;
                let title: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
                let mins: Option<String> = row.get(2)?;
                let secs: Option<String> = row.get(3)?;
                let length = match (mins, secs) {
                    (Some(m), Some(s)) if !m.is_empty() || !s.is_empty() => Some(format!("{m}:{s}")),
                    _ => None,
                };
                Ok(Track {
                    position,
                    title,
                    length,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        discs.push(Disc { index, tracks });
    }
    Ok(discs)
}

fn load_albums(conn: &Connection) -> Result<Vec<Album>> {
    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZFORMAT, ZCOUNTRY, ZLABEL, ZRELEASEYEAR, ZMYRATING, ZNRTRACKS \
         FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 ORDER BY Z_PK",
    )?;
    let rows: Vec<(
        i64,
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        i64,
    )> = stmt
        .query_map([ENT_ALBUM], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut albums = Vec::with_capacity(rows.len());
    for (id, title, format_pk, country_pk, label_pk, release_year, rating, track_count) in rows {
        let format = format_pk
            .map(|pk| resolve_simple_lookup(conn, "ZFORMAT", pk))
            .transpose()?
            .flatten();
        let country = country_pk
            .map(|pk| resolve_simple_lookup(conn, "ZCOUNTRY", pk))
            .transpose()?
            .flatten();
        let label = label_pk
            .map(|pk| resolve_simple_lookup(conn, "ZLABEL", pk))
            .transpose()?
            .flatten();
        let artists = load_artists(conn, id)?;
        let genres = load_genres(conn, id)?;
        let discs = load_discs(conn, id)?;

        albums.push(Album {
            id,
            title,
            artists,
            genres,
            label,
            format,
            country,
            release_year,
            rating,
            track_count,
            discs,
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
        "genres",
        "label",
        "format",
        "country",
        "release_year",
        "rating",
        "track_count",
    ])?;
    for album in albums {
        writer.write_record([
            album.id.to_string(),
            album.title.clone(),
            album.artists.join("; "),
            album.genres.join("; "),
            album.label.clone().unwrap_or_default(),
            album.format.clone().unwrap_or_default(),
            album.country.clone().unwrap_or_default(),
            album.release_year.clone().unwrap_or_default(),
            album.rating.map(|r| r.to_string()).unwrap_or_default(),
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
