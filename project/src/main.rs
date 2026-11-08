use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use music_collector_export::{load_albums, open_readonly, write_csv, write_html};

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let conn = open_readonly(&args.db)?;
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
    if let Some(html_path) = &args.html {
        write_html(html_path, &albums)?;
        println!("HTML écrit dans: {}", html_path.display());
    }

    Ok(())
}
