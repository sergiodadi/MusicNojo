//! Generates `sample-collection/MyCollection.mucp`, a tiny, entirely
//! fictional collection committed to the repo for the app's default/demo
//! view — never real personal data. Schema from `schema.sql` (structure
//! only); data is invented by this file. Run with:
//!
//!   cargo run --release --example generate_sample_collection

use std::path::Path;

use anyhow::Result;
use music_collector_export::{build_plan, AlbumFields, ManyToManyInputs, RoleInputs};
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

struct SampleAlbum {
    title: &'static str,
    artists: &'static [&'static str],
    genres: &'static [&'static str],
    format: &'static str,
    country: &'static str,
    release_year: &'static str,
    tracks: &'static [&'static str],
}

const ALBUMS: &[SampleAlbum] = &[
    SampleAlbum {
        title: "Lanterns Over the Harbor",
        artists: &["The Paper Boats"],
        genres: &["Folk-Rock"],
        format: "CD",
        country: "Canada",
        release_year: "2014",
        tracks: &["Harbor Light", "Low Tide", "Paper Boats", "Morning Fog"],
    },
    SampleAlbum {
        title: "Driftwood Radio",
        artists: &["The Paper Boats"],
        genres: &["Folk-Rock"],
        format: "CD",
        country: "Canada",
        release_year: "2016",
        tracks: &["Driftwood Radio", "Static Lullaby", "Northbound"],
    },
    SampleAlbum {
        title: "The Paper Boats Live in Halifax",
        artists: &["The Paper Boats"],
        genres: &["Folk-Rock", "Live"],
        format: "Vinyl",
        country: "Canada",
        release_year: "2018",
        tracks: &["Harbor Light (Live)", "Low Tide (Live)", "Encore: Morning Fog"],
    },
    SampleAlbum {
        title: "Concrete Garden",
        artists: &["Nine Static Clouds"],
        genres: &["Electronic"],
        format: "Vinyl",
        country: "Germany",
        release_year: "2019",
        tracks: &["Concrete Garden", "Glass Orbit", "Signal Drift"],
    },
    SampleAlbum {
        title: "Symphony No. 2 \"Riverlands\"",
        artists: &["Elena Vasko"],
        genres: &["Classical"],
        format: "CD",
        country: "France",
        release_year: "1998",
        tracks: &["I. Allegro", "II. Andante", "III. Scherzo", "IV. Finale"],
    },
    SampleAlbum {
        title: "Dust and Neon",
        artists: &["Marlowe Creed"],
        genres: &["Rock"],
        format: "CD",
        country: "USA",
        release_year: "2007",
        tracks: &["Dust and Neon", "Last Exit", "Eleven Red Lights", "Static"],
    },
    SampleAlbum {
        title: "Neon Afterglow",
        artists: &["Marlowe Creed"],
        genres: &["Rock"],
        format: "Vinyl",
        country: "USA",
        release_year: "2011",
        tracks: &["Neon Afterglow", "Borrowed Time", "The Long Drive Back"],
    },
    SampleAlbum {
        title: "Quiet Machinery",
        artists: &["Hinoki Room"],
        genres: &["Jazz"],
        format: "Digital",
        country: "Japan",
        release_year: "2021",
        tracks: &["Quiet Machinery", "Rust Garden", "Paper Lantern"],
    },
    SampleAlbum {
        title: "Field Recordings, Vol. 1",
        artists: &["Various Artists"],
        genres: &["Folk"],
        format: "CD",
        country: "UK",
        release_year: "1985",
        tracks: &["Market Day", "The Long Walk Home", "Harvest Song"],
    },
    SampleAlbum {
        title: "Borrowed Light",
        artists: &["Sable & Finch"],
        genres: &["Pop"],
        format: "Vinyl",
        country: "Sweden",
        release_year: "2016",
        tracks: &["Borrowed Light", "Half Awake", "Blue Hour", "Say It Twice"],
    },
    SampleAlbum {
        title: "Second Wind",
        artists: &["Sable & Finch"],
        genres: &["Pop"],
        format: "CD",
        country: "Sweden",
        release_year: "2019",
        tracks: &["Second Wind", "Paper Crown", "Afterthought"],
    },
    SampleAlbum {
        title: "Underneath the Underpass",
        artists: &["The Loose Fittings"],
        genres: &["Punk"],
        format: "CD",
        country: "Australia",
        release_year: "1991",
        tracks: &["Underpass", "No Fixed Address", "Spare Change"],
    },
];

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

    for album in ALBUMS {
        let roles = RoleInputs {
            artist: album.artists.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let many = ManyToManyInputs {
            genre: album.genres.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let fields = AlbumFields {
            format: Some(album.format.to_string()),
            country: Some(album.country.to_string()),
            release_year: Some(album.release_year.to_string()),
            ..Default::default()
        };
        let tracks: Vec<String> = album.tracks.iter().map(|s| s.to_string()).collect();
        let plan = build_plan(&conn, album.title, &roles, &many, &fields, &tracks)?;
        conn.execute_batch(&plan)?;
    }

    println!(
        "{} albums fictifs générés dans {}",
        ALBUMS.len(),
        root.display()
    );
    Ok(())
}
