//! Generates `sample-collection/MyCollection.mucp`, a tiny, entirely
//! fictional collection committed to the repo for the app's default/demo
//! view — never real personal data. Schema from `schema.sql` (structure
//! only); data and cover images (plain synthetic colors, no real
//! artwork) are invented by this file. Run with:
//!
//!   cargo run --release --example generate_sample_collection

use std::path::Path;

use anyhow::Result;
use image::{ImageBuffer, Rgb};
use music_collector_export::{build_plan, set_cover, AlbumFields, ManyToManyInputs, RoleInputs};
use rusqlite::Connection;

const SCHEMA: &str = include_str!("schema.sql");

// Z_ENT, Z_NAME, Z_SUPER — mirrors the real Z_PRIMARYKEY table's rows,
// all starting at Z_MAX=0 since this collection starts empty.
const ENTITIES: &[(i64, &str, i64)] = &[
    (1, "AlbumLookup1", 0),
    (2, "AlbumLookup2", 0),
    (3, "Artist", 0),
    (4, "ArtistPerson", 0),
    (5, "Boxset", 0),
    (6, "Chorus", 0),
    (7, "ChorusPerson", 0),
    (8, "Collectible", 0),
    (9, "MusicCollectible", 8),
    (10, "Album", 9),
    (11, "Disc", 9),
    (12, "Header", 9),
    (13, "SubTrack", 9),
    (14, "Track", 9),
    (15, "Collection", 0),
    (16, "Composer", 0),
    (17, "ComposerPerson", 0),
    (18, "Composition", 0),
    (19, "CompositionCollectible", 0),
    (20, "Condition", 0),
    (21, "Conductor", 0),
    (22, "ConductorPerson", 0),
    (23, "Country", 0),
    (24, "Credit", 0),
    (25, "CreditPerson", 0),
    (26, "Extra", 0),
    (27, "Format", 0),
    (28, "Genre", 0),
    (29, "Instrument", 0),
    (30, "ItemDelegate", 0),
    (31, "Key", 0),
    (32, "Label", 0),
    (33, "Link", 0),
    (34, "LocalLink", 33),
    (35, "OnlineLink", 33),
    (36, "Loan", 0),
    (37, "Loaner", 0),
    (38, "Location", 0),
    (39, "LookupItemDelegate", 0),
    (40, "Musician", 0),
    (41, "MusicianPerson", 0),
    (42, "Orchestra", 0),
    (43, "OrchestraPerson", 0),
    (44, "Owner", 0),
    (45, "Packaging", 0),
    (46, "PersonDelegate", 0),
    (47, "SearchString", 0),
    (48, "Slot", 0),
    (49, "Sound", 0),
    (50, "Spars", 0),
    (51, "StorageDevice", 0),
    (52, "Store", 0),
    (53, "Studio", 0),
    (54, "Tag", 0),
    (55, "TrackLookup1", 0),
    (56, "UserDefinedField", 0),
    (57, "UserDefinedLookupItem", 0),
    (58, "VinylColor", 0),
];

#[derive(Default)]
struct SampleAlbum {
    title: &'static str,
    artists: &'static [&'static str],
    composers: &'static [&'static str],
    conductors: &'static [&'static str],
    musicians: &'static [&'static str],
    orchestras: &'static [&'static str],
    choruses: &'static [&'static str],
    genres: &'static [&'static str],
    tags: &'static [&'static str],
    extras: &'static [&'static str],
    studios: &'static [&'static str],
    format: &'static str,
    label: &'static str,
    country: &'static str,
    condition: &'static str,
    packaging: &'static str,
    vinyl_color: &'static str,
    spars: &'static str,
    owner: &'static str,
    location: &'static str,
    release_year: &'static str,
    rating: Option<i64>,
    notes: &'static str,
    upc: &'static str,
    tracks: &'static [&'static str],
}

fn sample_albums() -> Vec<SampleAlbum> {
    vec![
        // Fully filled, every field — the other albums below only
        // use the common subset, same as a real collection's mix of
        // thoroughly-catalogued and quickly-added entries.
        SampleAlbum {
            title: "Lanterns Over the Harbor",
            artists: &["The Paper Boats"],
            genres: &["Folk-Rock"],
            tags: &["Road Trip"],
            extras: &["Lyrics Booklet"],
            studios: &["Harborlight Studios"],
            format: "CD",
            label: "Tidewater Records",
            country: "Canada",
            condition: "Mint",
            packaging: "Jewel Case",
            owner: "J. Harrow",
            location: "Shelf A1",
            release_year: "2014",
            rating: Some(4),
            notes: "Signed by the band at the 2014 release show.",
            upc: "600753000001",
            tracks: &["Harbor Light", "Low Tide", "Paper Boats", "Morning Fog"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Driftwood Radio",
            artists: &["The Paper Boats"],
            genres: &["Folk-Rock"],
            format: "CD",
            country: "Canada",
            release_year: "2016",
            tracks: &["Driftwood Radio", "Static Lullaby", "Northbound"],
            ..Default::default()
        },
        SampleAlbum {
            title: "The Paper Boats Live in Halifax",
            artists: &["The Paper Boats"],
            genres: &["Folk-Rock", "Live"],
            format: "Vinyl",
            country: "Canada",
            release_year: "2018",
            tracks: &["Harbor Light (Live)", "Low Tide (Live)", "Encore: Morning Fog"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Concrete Garden",
            artists: &["Nine Static Clouds"],
            genres: &["Electronic"],
            format: "Vinyl",
            country: "Germany",
            release_year: "2019",
            tracks: &["Concrete Garden", "Glass Orbit", "Signal Drift"],
            ..Default::default()
        },
        // Fully filled, classical-specific roles (composer/conductor/
        // orchestra/chorus) exercised here since the other albums are
        // all simple band/artist entries.
        SampleAlbum {
            title: "Symphony No. 2 \"Riverlands\"",
            composers: &["Elena Vasko"],
            conductors: &["Hugo Falkenrath"],
            orchestras: &["Riverlands Philharmonic"],
            choruses: &["Riverlands Festival Chorus"],
            genres: &["Classical"],
            tags: &["Reference Recording"],
            studios: &["Riverlands Concert Hall"],
            format: "CD",
            label: "Meridian Classics",
            country: "France",
            condition: "Near Mint",
            packaging: "Digipak",
            spars: "DDD",
            owner: "J. Harrow",
            location: "Shelf C3",
            release_year: "1998",
            rating: Some(5),
            notes: "Live recording, Riverlands Festival, autumn 1997.",
            upc: "600753000002",
            tracks: &["I. Allegro", "II. Andante", "III. Scherzo", "IV. Finale"],
            ..Default::default()
        },
        // Fully filled, a vinyl-specific field (vinyl_color) exercised
        // here since it's meaningless for the CD/Digital entries above.
        SampleAlbum {
            title: "Dust and Neon",
            artists: &["Marlowe Creed"],
            musicians: &["Reiko Ashworth"],
            genres: &["Rock"],
            tags: &["Desert Rock"],
            extras: &["Poster"],
            format: "Vinyl",
            label: "Dustbowl Records",
            country: "USA",
            condition: "Very Good",
            packaging: "Gatefold",
            vinyl_color: "Translucent Red",
            owner: "J. Harrow",
            location: "Shelf B2",
            release_year: "2007",
            rating: Some(4),
            notes: "First pressing; sleeve has light corner wear.",
            upc: "600753000003",
            tracks: &["Dust and Neon", "Last Exit", "Eleven Red Lights", "Static"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Neon Afterglow",
            artists: &["Marlowe Creed"],
            genres: &["Rock"],
            format: "Vinyl",
            country: "USA",
            release_year: "2011",
            tracks: &["Neon Afterglow", "Borrowed Time", "The Long Drive Back"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Quiet Machinery",
            artists: &["Hinoki Room"],
            genres: &["Jazz"],
            format: "Digital",
            country: "Japan",
            release_year: "2021",
            tracks: &["Quiet Machinery", "Rust Garden", "Paper Lantern"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Field Recordings, Vol. 1",
            artists: &["Various Artists"],
            genres: &["Folk"],
            format: "CD",
            country: "UK",
            release_year: "1985",
            tracks: &["Market Day", "The Long Walk Home", "Harvest Song"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Borrowed Light",
            artists: &["Sable & Finch"],
            genres: &["Pop"],
            format: "Vinyl",
            country: "Sweden",
            release_year: "2016",
            tracks: &["Borrowed Light", "Half Awake", "Blue Hour", "Say It Twice"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Second Wind",
            artists: &["Sable & Finch"],
            genres: &["Pop"],
            format: "CD",
            country: "Sweden",
            release_year: "2019",
            tracks: &["Second Wind", "Paper Crown", "Afterthought"],
            ..Default::default()
        },
        SampleAlbum {
            title: "Underneath the Underpass",
            artists: &["The Loose Fittings"],
            genres: &["Punk"],
            format: "CD",
            country: "Australia",
            release_year: "1991",
            tracks: &["Underpass", "No Fixed Address", "Spare Change"],
            ..Default::default()
        },
    ]
}

fn opt(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn list(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

/// A plain solid-color placeholder cover (no real artwork), color
/// derived deterministically from the title so each album gets a
/// distinct, stable color across regenerations.
fn placeholder_cover(title: &str, lightness: u8) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let hash = title.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let r = ((hash & 0xFF) as u16 * lightness as u16 / 255) as u8;
    let g = (((hash >> 8) & 0xFF) as u16 * lightness as u16 / 255) as u8;
    let b = (((hash >> 16) & 0xFF) as u16 * lightness as u16 / 255) as u8;
    ImageBuffer::from_pixel(400, 400, Rgb([r.max(40), g.max(40), b.max(40)]))
}

fn write_cover(dir: &Path, name: &str, img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<std::path::PathBuf> {
    let path = dir.join(name);
    img.save(&path)?;
    Ok(path)
}

fn main() -> Result<()> {
    let root = Path::new("sample-collection/MyCollection.mucp");
    if root.exists() {
        std::fs::remove_dir_all(root)?;
    }
    std::fs::create_dir_all(root.join("Images"))?;

    let db_path = root.join("Music_data");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(SCHEMA)?;

    for (ent, name, super_ent) in ENTITIES {
        conn.execute(
            "INSERT INTO Z_PRIMARYKEY (Z_ENT, Z_NAME, Z_SUPER, Z_MAX) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![ent, name, super_ent],
        )?;
    }

    let scratch = std::env::temp_dir().join("musicnojo_sample_covers");
    std::fs::create_dir_all(&scratch)?;

    let albums = sample_albums();
    let count = albums.len();
    for album in albums {
        let roles = RoleInputs {
            artist: list(album.artists),
            composer: list(album.composers),
            conductor: list(album.conductors),
            musician: list(album.musicians),
            orchestra: list(album.orchestras),
            chorus: list(album.choruses),
        };
        let many = ManyToManyInputs {
            genre: list(album.genres),
            tag: list(album.tags),
            extra: list(album.extras),
            studio: list(album.studios),
        };
        let fields = AlbumFields {
            format: opt(album.format),
            label: opt(album.label),
            country: opt(album.country),
            condition: opt(album.condition),
            packaging: opt(album.packaging),
            vinyl_color: opt(album.vinyl_color),
            spars: opt(album.spars),
            owner: opt(album.owner),
            location: opt(album.location),
            release_year: opt(album.release_year),
            rating: album.rating,
            notes: opt(album.notes),
            upc: opt(album.upc),
        };
        let tracks: Vec<String> = album.tracks.iter().map(|s| s.to_string()).collect();
        let plan = build_plan(&conn, album.title, &roles, &many, &fields, &tracks)?;
        conn.execute_batch(&plan)?;

        let id: i64 = conn.query_row(
            // Z_ENT=10 (Album) is required: several albums reuse their
            // own title as a track title too (e.g. "Driftwood Radio"),
            // and without this filter the track's higher Z_PK would
            // win over the album's.
            "SELECT Z_PK FROM ZCOLLECTIBLE WHERE ZTITLE = ?1 AND Z_ENT = 10 ORDER BY Z_PK DESC LIMIT 1",
            [album.title],
            |row| row.get(0),
        )?;
        let front = write_cover(&scratch, "front.jpg", &placeholder_cover(album.title, 200))?;
        let back = write_cover(&scratch, "back.jpg", &placeholder_cover(album.title, 110))?;
        set_cover(root, id, Some(&front), Some(&back))?;
    }

    let _ = std::fs::remove_dir_all(&scratch);
    let _ = std::fs::remove_dir_all(root.join("backups"));

    println!("{count} albums fictifs générés dans {}", root.display());
    Ok(())
}
