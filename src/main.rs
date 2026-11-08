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
    label: Option<String>,
    format: Option<String>,
    country: Option<String>,
    condition: Option<String>,
    packaging: Option<String>,
    vinyl_color: Option<String>,
    spars: Option<String>,
    release_year: Option<String>,
    rating: Option<i64>,
    notes: Option<String>,
    upc: Option<String>,
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

// Artist/Composer/Conductor/Musician/Orchestra/Chorus each have their own
// dedicated entity + person-delegate pair (unlike the generic Credit
// entity below), so none of these roles need an id-to-label mapping.
fn load_role_names(
    conn: &Connection,
    role_table: &str,
    person_table: &str,
    person_fk: &str,
    album_pk: i64,
) -> Result<Vec<String>> {
    let sql = format!(
        "SELECT p.ZDISPLAYNAME FROM {role_table} r \
         JOIN {person_table} pt ON pt.Z_PK = r.{person_fk} \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = pt.ZDELEGATE \
         WHERE r.ZALBUM = ?1 ORDER BY r.ZRANK"
    );
    let mut stmt = conn.prepare(&sql)?;
    let names = stmt
        .query_map([album_pk], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(names)
}

fn load_credits(conn: &Connection, album_pk: i64) -> Result<Vec<Credit>> {
    let mut stmt = conn.prepare(
        "SELECT p.ZDISPLAYNAME, c.ZROLEID FROM ZCREDIT c \
         JOIN ZCREDITPERSON cp ON cp.Z_PK = c.ZCREDITPERSON \
         JOIN ZPERSONDELEGATE p ON p.Z_PK = cp.ZDELEGATE \
         WHERE c.ZALBUM = ?1 ORDER BY c.ZRANK",
    )?;
    let credits = stmt
        .query_map([album_pk], |row| {
            Ok(Credit {
                name: row.get(0)?,
                role_id: row.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(credits)
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

#[allow(clippy::type_complexity)]
fn load_albums(conn: &Connection) -> Result<Vec<Album>> {
    let mut stmt = conn.prepare(
        "SELECT Z_PK, ZTITLE, ZFORMAT, ZCOUNTRY, ZLABEL, ZCONDITION, ZPACKAGING, \
                ZVINYLCOLOR, ZSPARS, ZRELEASEYEAR, ZMYRATING, ZNOTES, ZUPC, ZNRTRACKS \
         FROM ZCOLLECTIBLE WHERE Z_ENT = ?1 ORDER BY Z_PK",
    )?;
    let rows: Vec<(
        i64,
        String,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<i64>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<String>,
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
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
                row.get(13)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut albums = Vec::with_capacity(rows.len());
    for (
        id,
        title,
        format_pk,
        country_pk,
        label_pk,
        condition_pk,
        packaging_pk,
        vinyl_color_pk,
        spars_pk,
        release_year,
        rating,
        notes,
        upc,
        track_count,
    ) in rows
    {
        let resolve = |table: &str, pk: Option<i64>| -> Result<Option<String>> {
            pk.map(|pk| resolve_simple_lookup(conn, table, pk))
                .transpose()
                .map(|o| o.flatten())
        };
        let format = resolve("ZFORMAT", format_pk)?;
        let country = resolve("ZCOUNTRY", country_pk)?;
        let label = resolve("ZLABEL", label_pk)?;
        let condition = resolve("ZCONDITION", condition_pk)?;
        let packaging = resolve("ZPACKAGING", packaging_pk)?;
        let vinyl_color = resolve("ZVINYLCOLOR", vinyl_color_pk)?;
        let spars = resolve("ZSPARS", spars_pk)?;

        let artists = load_role_names(conn, "ZARTIST", "ZARTISTPERSON", "ZARTISTPERSON", id)?;
        let composers =
            load_role_names(conn, "ZCOMPOSER", "ZCOMPOSERPERSON", "ZCOMPOSERPERSON", id)?;
        let conductors = load_role_names(
            conn,
            "ZCONDUCTOR",
            "ZCONDUCTORPERSON",
            "ZCONDUCTORPERSON",
            id,
        )?;
        let musicians =
            load_role_names(conn, "ZMUSICIAN", "ZMUSICIANPERSON", "ZMUSICIANPERSON", id)?;
        let orchestras = load_role_names(
            conn,
            "ZORCHESTRA",
            "ZORCHESTRAPERSON",
            "ZORCHESTRAPERSON",
            id,
        )?;
        let choruses = load_role_names(conn, "ZCHORUS", "ZCHORUSPERSON", "ZCHORUSPERSON", id)?;
        let credits = load_credits(conn, id)?;
        let genres = load_genres(conn, id)?;
        let discs = load_discs(conn, id)?;

        albums.push(Album {
            id,
            title,
            artists,
            composers,
            conductors,
            musicians,
            orchestras,
            choruses,
            credits,
            genres,
            label,
            format,
            country,
            condition,
            packaging,
            vinyl_color,
            spars,
            release_year,
            rating,
            notes,
            upc,
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
        "composers",
        "conductors",
        "musicians",
        "orchestras",
        "choruses",
        "credits",
        "genres",
        "label",
        "format",
        "country",
        "condition",
        "packaging",
        "vinyl_color",
        "spars",
        "release_year",
        "rating",
        "notes",
        "upc",
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
            album.label.clone().unwrap_or_default(),
            album.format.clone().unwrap_or_default(),
            album.country.clone().unwrap_or_default(),
            album.condition.clone().unwrap_or_default(),
            album.packaging.clone().unwrap_or_default(),
            album.vinyl_color.clone().unwrap_or_default(),
            album.spars.clone().unwrap_or_default(),
            album.release_year.clone().unwrap_or_default(),
            album.rating.map(|r| r.to_string()).unwrap_or_default(),
            album.notes.clone().unwrap_or_default(),
            album.upc.clone().unwrap_or_default(),
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
