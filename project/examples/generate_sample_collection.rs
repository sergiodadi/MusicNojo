//! Generates `sample-collection/MyCollection.mucp`, a tiny, entirely
//! fictional collection committed to the repo for the app's default/demo
//! view — never real personal data. Schema from `schema.sql` (structure
//! only); data and cover images (plain synthetic colors, no real
//! artwork) are invented by this file. Run with:
//!
//!   cargo run --release --example generate_sample_collection

use std::path::Path;

use ab_glyph::{FontRef, PxScale};
use anyhow::Result;
use image::{ImageBuffer, Rgb};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut, text_size,
};
use imageproc::rect::Rect;
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

/// Credit line for a cover: the first name in whichever role list this
/// album actually has one in, since a purely classical entry (see
/// "Symphony No. 2") has conductors/orchestras but no plain "artists".
fn primary_credit(album: &SampleAlbum) -> &'static str {
    album
        .artists
        .first()
        .or(album.composers.first())
        .or(album.conductors.first())
        .or(album.orchestras.first())
        .copied()
        .unwrap_or("")
}

fn hash_u32(s: &str) -> u32 {
    s.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32))
}

fn bg_color(title: &str, lightness: u8) -> Rgb<u8> {
    let hash = hash_u32(title);
    let r = ((hash & 0xFF) as u16 * lightness as u16 / 255) as u8;
    let g = (((hash >> 8) & 0xFF) as u16 * lightness as u16 / 255) as u8;
    let b = (((hash >> 16) & 0xFF) as u16 * lightness as u16 / 255) as u8;
    Rgb([r.max(40), g.max(40), b.max(40)])
}

/// Perceived-luminance check so title/credit text stays legible against
/// whichever background color this title happened to hash to.
fn readable_text_color(bg: Rgb<u8>) -> Rgb<u8> {
    let [r, g, b] = bg.0;
    let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    if luminance > 150.0 {
        Rgb([20, 20, 20])
    } else {
        Rgb([245, 245, 245])
    }
}

fn rgba(c: Rgb<u8>, alpha: u8) -> image::Rgba<u8> {
    image::Rgba([c.0[0], c.0[1], c.0[2], alpha])
}

type Canvas = ImageBuffer<image::Rgba<u8>, Vec<u8>>;

/// A simple geometric motif keyed off the album's first genre — not real
/// artwork, just a visual cue so covers for a Rock album and a Classical
/// album don't look interchangeable beyond their background color.
fn draw_genre_motif(img: &mut Canvas, genre: &str, accent: Rgb<u8>) {
    let genre = genre.to_lowercase();
    if genre.contains("classical") || genre.contains("jazz") {
        // Concentric rings.
        for radius in (40..=180).step_by(35) {
            draw_hollow_circle(img, (200, 170), radius, accent);
        }
    } else if genre.contains("electronic") {
        // A grid.
        for x in (40..360).step_by(40) {
            draw_line_segment_mut(img, (x as f32, 40.0), (x as f32, 300.0), rgba(accent, 90));
        }
        for y in (40..300).step_by(40) {
            draw_line_segment_mut(img, (40.0, y as f32), (360.0, y as f32), rgba(accent, 90));
        }
    } else if genre.contains("folk") {
        // Soft overlapping waves.
        for i in 0..3 {
            let y0 = 100.0 + i as f32 * 50.0;
            let mut prev = (20.0, y0);
            for step in 1..=18 {
                let x = 20.0 + step as f32 * 20.0;
                let y = y0 + (step as f32 * 0.9).sin() * 18.0;
                draw_line_segment_mut(img, prev, (x, y), rgba(accent, 110));
                prev = (x, y);
            }
        }
    } else if genre.contains("rock") || genre.contains("punk") {
        // Jagged lightning-bolt-ish zigzag.
        let points = [(40.0, 60.0), (160.0, 140.0), (90.0, 180.0), (320.0, 300.0)];
        for w in points.windows(2) {
            draw_line_segment_mut(img, w[0], w[1], rgba(accent, 160));
        }
    } else {
        // Default: a few scattered dots (also used for Pop).
        let hash = hash_u32(&genre).max(1);
        for i in 0..8 {
            let cx = 40 + ((hash.wrapping_mul(i + 1)) % 320) as i32;
            let cy = 40 + ((hash.wrapping_mul(i + 7)) % 260) as i32;
            draw_filled_circle_mut(img, (cx, cy), 6, rgba(accent, 200));
        }
    }
}

fn draw_hollow_circle(img: &mut Canvas, center: (i32, i32), radius: i32, color: Rgb<u8>) {
    let steps = 120;
    let mut prev = None;
    for i in 0..=steps {
        let angle = i as f32 / steps as f32 * std::f32::consts::TAU;
        let x = center.0 as f32 + radius as f32 * angle.cos();
        let y = center.1 as f32 + radius as f32 * angle.sin();
        if let Some(p) = prev {
            draw_line_segment_mut(img, p, (x, y), rgba(color, 80));
        }
        prev = Some((x, y));
    }
}

/// Greedy word-wrap against the font's own measured widths, so long
/// titles don't run off the 400px cover.
fn wrap_text(font: &FontRef, scale: PxScale, text: &str, max_width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        let (w, _) = text_size(scale, font, &candidate);
        if w as u32 > max_width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Loads a system font so the generator needs no bundled/licensed font
/// file in the repo — only the rendered pixels (the .jpg covers) are
/// committed, never the font itself. macOS-only, same as the rest of
/// this developer-local tool (refresh-testdata*.sh etc.).
fn load_font(bold: bool) -> Result<Vec<u8>> {
    let path = if bold {
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf"
    } else {
        "/System/Library/Fonts/Supplemental/Arial.ttf"
    };
    Ok(std::fs::read(path)?)
}

/// A placeholder cover: a title-hash-derived background color, a small
/// geometric motif keyed off the album's first genre, and the album's
/// own title/artist (front) or format/year/genre (back) rendered as
/// text — never real artwork, but no two albums look the same and each
/// cover is identifiable at a glance, which a flat color swatch wasn't.
fn placeholder_cover(album: &SampleAlbum, front: bool) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>> {
    let lightness = if front { 200 } else { 110 };
    let bg = bg_color(album.title, lightness);
    let mut img: Canvas = ImageBuffer::from_pixel(400, 400, rgba(bg, 255));

    let text_color = readable_text_color(bg);
    let text_rgba = rgba(text_color, 255);
    let accent = Rgb([
        text_color.0[0].min(bg.0[0].saturating_add(60)),
        text_color.0[1].min(bg.0[1].saturating_add(60)),
        text_color.0[2].min(bg.0[2].saturating_add(60)),
    ]);
    let genre = album.genres.first().copied().unwrap_or("");
    draw_genre_motif(&mut img, genre, accent);

    // An opaque panel behind the text keeps it legible over the motif
    // regardless of where the lines/dots happened to land.
    draw_filled_rect_mut(&mut img, Rect::at(0, 295).of_size(400, 105), rgba(bg, 255));

    let title_bytes = load_font(true)?;
    let title_font = FontRef::try_from_slice(&title_bytes)?;
    let body_bytes = load_font(false)?;
    let body_font = FontRef::try_from_slice(&body_bytes)?;

    if front {
        // Fixed title/credit positions (not stacked relative to each
        // other) so a 2-line-wrapped title can never push the credit
        // line below the 400px canvas edge.
        let title_scale = PxScale::from(26.0);
        let lines = wrap_text(&title_font, title_scale, album.title, 370);
        for (i, line) in lines.iter().take(2).enumerate() {
            draw_text_mut(&mut img, text_rgba, 15, 300 + i as i32 * 28, title_scale, &title_font, line);
        }
        let credit = primary_credit(album);
        if !credit.is_empty() {
            draw_text_mut(&mut img, text_rgba, 15, 368, PxScale::from(18.0), &body_font, credit);
        }
    } else {
        let info = [album.format, album.release_year, genre]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("  ·  ");
        draw_text_mut(&mut img, text_rgba, 15, 325, PxScale::from(18.0), &body_font, &info);
        let credit = primary_credit(album);
        if !credit.is_empty() {
            draw_text_mut(&mut img, text_rgba, 15, 355, PxScale::from(18.0), &body_font, credit);
        }
    }

    // JPEG has no alpha channel — flatten to RGB8 for write_cover/save().
    Ok(image::DynamicImage::ImageRgba8(img).to_rgb8())
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
            collection: None,
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
        // mc_add's write path never sets ZDATEADDED (not asked for yet) —
        // the real app does, and it's always populated in a real
        // collection. Backfilled here, purely for this fictional demo
        // data, spread across the last ~2 years (deterministic from the
        // title hash) so "sort by date added" and the stats panel's
        // "by year added" breakdown have something to show.
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let days_ago = (hash_u32(album.title) % 700) as i64;
        let date_added_core_data = (now_unix - 978_307_200 - days_ago * 86_400) as f64;
        conn.execute(
            "UPDATE ZCOLLECTIBLE SET ZDATEADDED = ?1 WHERE Z_PK = ?2",
            rusqlite::params![date_added_core_data, id],
        )?;

        let front = write_cover(&scratch, "front.jpg", &placeholder_cover(&album, true)?)?;
        let back = write_cover(&scratch, "back.jpg", &placeholder_cover(&album, false)?)?;
        set_cover(root, id, Some(&front), Some(&back))?;
    }

    let _ = std::fs::remove_dir_all(&scratch);
    let _ = std::fs::remove_dir_all(root.join("backups"));

    println!("{count} albums fictifs générés dans {}", root.display());
    Ok(())
}
