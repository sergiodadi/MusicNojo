use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use music_collector_export::{load_albums, open_readonly, write_csv, write_html, Connection, ENT_ALBUM};

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

    /// Write a standalone, searchable HTML page to this file
    #[arg(long)]
    html: Option<PathBuf>,

    /// Only keep the named sub-collection (ZCOLLECTION table, e.g.
    /// "music-collector-serge"). A .mucp file can hold several
    /// sub-collections; without this option, ALL of them are exported.
    #[arg(long)]
    collection: Option<String>,

    /// Lists the available sub-collections (name, album count, copy
    /// count) and stops — writes no CSV/JSON/HTML.
    #[arg(long)]
    list_collections: bool,
}

/// Prints every sub-collection with its album count and the sum of
/// their quantities (the number the app's own counter would display)
/// — same shape as BookNojo's own `print_collections`.
fn print_collections(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT col.ZTITLE, COUNT(c.Z_PK), SUM(COALESCE(c.ZQUANTITY, 1))
         FROM ZCOLLECTION col
         LEFT JOIN ZCOLLECTIBLE c ON c.ZCOLLECTION = col.Z_PK AND c.Z_ENT = ?1
         GROUP BY col.Z_PK
         ORDER BY col.ZTITLE",
    )?;
    let mut rows = stmt.query([ENT_ALBUM])?;
    println!("{:<30} {:>10} {:>12}", "collection", "albums", "exemplaires");
    while let Some(row) = rows.next()? {
        let name: String = row.get(0)?;
        let albums: i64 = row.get(1)?;
        let qty: i64 = row.get(2)?;
        println!("{:<30} {:>10} {:>12}", name, albums, qty);
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    let conn = open_readonly(&args.db)?;

    if args.list_collections {
        return print_collections(&conn);
    }

    let mut albums = load_albums(&conn)?;
    if let Some(name) = &args.collection {
        albums.retain(|a| a.collection.as_deref() == Some(name.as_str()));
    }
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
    if let Some(html_path) = &args.html {
        write_html(html_path, &albums)?;
        println!("HTML écrit dans: {}", html_path.display());
    }

    Ok(())
}
