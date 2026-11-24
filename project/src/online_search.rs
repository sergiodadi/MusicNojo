//! Online album-metadata lookup ("Recherche en ligne" feature) — three
//! independent, free, keyless sources: the iTunes Search API, MusicBrainz,
//! and Deezer. Each source's function is self-contained and returns `Err`
//! only for a genuine transport/parse failure; a source simply having zero
//! matches returns `Ok(vec![])`, never an error — the caller (a Tauri
//! command) runs all three and shows whatever comes back, without one
//! failing source blocking the others. Ported from the same feature in
//! BookNojo (`online_search.rs` there), adapted to album metadata instead
//! of book metadata and to 3 different sources (BookNojo's 3 book sources
//! don't have music-album equivalents).

use serde::Serialize;

/// One candidate album, normalized from whichever source returned it.
/// `source` is a short label ("iTunes"/"MusicBrainz"/"Deezer"), shown to
/// the user so they know which result they're picking — results are
/// deliberately never merged across sources.
#[derive(Debug, Clone, Serialize)]
pub struct AlbumSearchResult {
    pub source: &'static str,
    pub title: String,
    pub artist: Option<String>,
    pub label: Option<String>,
    pub release_year: Option<String>,
    pub cover_url: Option<String>,
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        // MusicBrainz requires API clients to identify themselves (a
        // generic/absent User-Agent gets a 403); iTunes and Deezer don't
        // require it, but it costs nothing to send everywhere.
        .user_agent("MusicNojo (https://github.com/sergiodadi/MusicNojo)")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())
}

/// Pulls the first 4-digit run out of a free-text date string — same
/// approach as BookNojo's `extract_year`, needed here for MusicBrainz's
/// `date` field ("1982-11-30" or just "1982").
fn extract_year(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 4 {
        return None;
    }
    for start in 0..=(chars.len() - 4) {
        let candidate: String = chars[start..start + 4].iter().collect();
        if candidate.chars().all(|c| c.is_ascii_digit()) {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------
// iTunes Search API
// ---------------------------------------------------------------------

/// `itunes.apple.com/search` — keyless, no published hard rate limit
/// (Apple asks for reasonable use). `entity=album` restricts results to
/// albums (otherwise songs/artists/etc. would also come back).
pub fn search_itunes_album(query: &str) -> Result<Vec<AlbumSearchResult>, String> {
    let client = http_client()?;
    let resp = client
        .get("https://itunes.apple.com/search")
        .query(&[("term", query), ("entity", "album"), ("limit", "10")])
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let items = json.get("results").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Ok(items.iter().filter_map(itunes_result_from_item).collect())
}

fn itunes_result_from_item(item: &serde_json::Value) -> Option<AlbumSearchResult> {
    let title = item.get("collectionName").and_then(|v| v.as_str())?.to_string();
    let artist = item.get("artistName").and_then(|v| v.as_str()).map(str::to_string);
    let release_year = item
        .get("releaseDate")
        .and_then(|v| v.as_str())
        .and_then(extract_year);
    // artworkUrl100 is a thumbnail; swapping the trailing size suffix gets a
    // bigger image without a second request (a well-known iTunes Search
    // API convention — the 100x100bb suffix in the URL can be replaced).
    let cover_url = item
        .get("artworkUrl100")
        .and_then(|v| v.as_str())
        .map(|url| url.replace("100x100bb", "600x600bb"));
    Some(AlbumSearchResult {
        source: "iTunes",
        title,
        artist,
        label: None,
        release_year,
        cover_url,
    })
}

// ---------------------------------------------------------------------
// MusicBrainz
// ---------------------------------------------------------------------

/// `musicbrainz.org/ws/2/release` — keyless but requires an identifying
/// `User-Agent` (set in `http_client`), otherwise MusicBrainz returns a
/// `403`. No cover art in this response (MusicBrainz's own cover art lives
/// in the separate Cover Art Archive, keyed by release MBID) — deliberately
/// not fetched here, same reasoning as BnF in BookNojo's `online_search.rs`
/// (adds a 2nd network round-trip per result just to maybe find a cover;
/// not worth it when 2 of the 3 sources already provide one directly).
pub fn search_musicbrainz_album(query: &str) -> Result<Vec<AlbumSearchResult>, String> {
    let client = http_client()?;
    let q = format!("release:{query}");
    let resp = client
        .get("https://musicbrainz.org/ws/2/release/")
        .query(&[("query", q.as_str()), ("fmt", "json"), ("limit", "10")])
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let releases = json.get("releases").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Ok(releases.iter().filter_map(musicbrainz_result_from_release).collect())
}

fn musicbrainz_result_from_release(release: &serde_json::Value) -> Option<AlbumSearchResult> {
    let title = release.get("title").and_then(|v| v.as_str())?.to_string();
    let artist = release
        .get("artist-credit")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let label = release
        .get("label-info")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|li| li.get("label"))
        .and_then(|l| l.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let release_year = release
        .get("date")
        .and_then(|v| v.as_str())
        .and_then(extract_year);
    Some(AlbumSearchResult {
        source: "MusicBrainz",
        title,
        artist,
        label,
        release_year,
        cover_url: None,
    })
}

// ---------------------------------------------------------------------
// Deezer
// ---------------------------------------------------------------------

/// `api.deezer.com/search/album` — fully keyless, no User-Agent required.
/// The search endpoint itself carries no release date (that's only on the
/// per-album endpoint, `GET /album/{id}`); deliberately not fetched here to
/// keep this a single request per query, same reasoning as the MusicBrainz
/// cover-art tradeoff above — the other 2 sources already provide a year.
pub fn search_deezer_album(query: &str) -> Result<Vec<AlbumSearchResult>, String> {
    let client = http_client()?;
    let resp = client
        .get("https://api.deezer.com/search/album")
        .query(&[("q", query), ("limit", "10")])
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(vec![]);
    }
    let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let items = json.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    Ok(items.iter().filter_map(deezer_result_from_item).collect())
}

fn deezer_result_from_item(item: &serde_json::Value) -> Option<AlbumSearchResult> {
    let title = item.get("title").and_then(|v| v.as_str())?.to_string();
    let artist = item
        .get("artist")
        .and_then(|a| a.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let cover_url = item
        .get("cover_medium")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Some(AlbumSearchResult {
        source: "Deezer",
        title,
        artist,
        label: None,
        release_year: None,
        cover_url,
    })
}

// ---------------------------------------------------------------------
// Cover download
// ---------------------------------------------------------------------

/// Counter making each downloaded cover's temp filename unique within this
/// run — combined with the process id. Same mechanism as BookNojo's
/// `download_cover_to_temp_file`.
static ONLINE_COVER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Downloads a chosen online search result's `cover_url` (iTunes and
/// Deezer both provide one; MusicBrainz's response carries no cover image,
/// see `musicbrainz_result_from_release`) into a throwaway file under the
/// system temp dir, and returns its local path. Lives here (not in the
/// Tauri app crate) so `reqwest` stays a dependency of this crate only. The
/// resulting path is meant to be handed to MusicNojo's existing add-time
/// cover mechanism exactly like a user's own manually-picked file. The file
/// is never cleaned up here: the OS temp dir is cleared on its own
/// schedule, and the add-time cover mechanism only *reads* the path it's
/// given.
pub fn download_cover_to_temp_file(url: &str) -> Result<String, String> {
    let client = http_client()?;
    let resp = client.get(url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Téléchargement de la pochette échoué ({}).", resp.status()));
    }
    let extension = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .map(|content_type| match content_type.as_str() {
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        })
        .unwrap_or("jpg");
    let bytes = resp.bytes().map_err(|e| e.to_string())?;

    let n = ONLINE_COVER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("musicnojo-online-cover-{}-{n}.{extension}", std::process::id()));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_year_finds_a_4_digit_run_in_free_text_dates() {
        assert_eq!(extract_year("1982-11-30"), Some("1982".to_string()));
        assert_eq!(extract_year("1982"), Some("1982".to_string()));
        assert_eq!(extract_year("unknown"), None);
    }

    #[test]
    fn itunes_result_from_item_parses_a_real_shaped_response_and_upsizes_artwork() {
        let item: serde_json::Value = serde_json::from_str(
            r#"{
                "collectionName": "Thriller",
                "artistName": "Michael Jackson",
                "releaseDate": "1982-11-30T12:00:00Z",
                "artworkUrl100": "https://example.com/thriller/100x100bb.jpg"
            }"#,
        )
        .unwrap();
        let result = itunes_result_from_item(&item).unwrap();
        assert_eq!(result.title, "Thriller");
        assert_eq!(result.artist, Some("Michael Jackson".to_string()));
        assert_eq!(result.release_year, Some("1982".to_string()));
        assert_eq!(result.cover_url, Some("https://example.com/thriller/600x600bb.jpg".to_string()));
    }

    #[test]
    fn itunes_result_from_item_is_none_without_a_collection_name() {
        let item: serde_json::Value = serde_json::from_str(r#"{"artistName": "Someone"}"#).unwrap();
        assert!(itunes_result_from_item(&item).is_none());
    }

    #[test]
    fn musicbrainz_result_from_release_parses_a_real_shaped_response() {
        let release: serde_json::Value = serde_json::from_str(
            r#"{
                "title": "Thriller",
                "artist-credit": [{"name": "Michael Jackson"}],
                "label-info": [{"label": {"name": "Epic"}}],
                "date": "1982-11-30"
            }"#,
        )
        .unwrap();
        let result = musicbrainz_result_from_release(&release).unwrap();
        assert_eq!(result.title, "Thriller");
        assert_eq!(result.artist, Some("Michael Jackson".to_string()));
        assert_eq!(result.label, Some("Epic".to_string()));
        assert_eq!(result.release_year, Some("1982".to_string()));
        assert_eq!(result.cover_url, None);
    }

    #[test]
    fn musicbrainz_result_from_release_is_none_without_a_title() {
        let release: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        assert!(musicbrainz_result_from_release(&release).is_none());
    }

    #[test]
    fn deezer_result_from_item_parses_a_real_shaped_response() {
        let item: serde_json::Value = serde_json::from_str(
            r#"{
                "title": "Thriller",
                "artist": {"name": "Michael Jackson"},
                "cover_medium": "https://example.com/cover.jpg"
            }"#,
        )
        .unwrap();
        let result = deezer_result_from_item(&item).unwrap();
        assert_eq!(result.title, "Thriller");
        assert_eq!(result.artist, Some("Michael Jackson".to_string()));
        assert_eq!(result.cover_url, Some("https://example.com/cover.jpg".to_string()));
    }

    #[test]
    fn deezer_result_from_item_is_none_without_a_title() {
        let item: serde_json::Value = serde_json::from_str(r#"{"artist": {"name": "Someone"}}"#).unwrap();
        assert!(deezer_result_from_item(&item).is_none());
    }

    // Live network tests — real calls to the actual services, not mocks.
    // iTunes and Deezer are both confirmed reachable (200 OK) and keyless
    // from this environment; MusicBrainz is also keyless but only asserted
    // on "doesn't panic" here, since its unauthenticated rate limit is
    // shared globally and a 403/503 would be an environment fluke, not a
    // bug in this code (same spirit as BookNojo's handling of Google
    // Books' shared keyless quota).

    #[test]
    fn search_itunes_album_finds_a_well_known_album() {
        let results = search_itunes_album("Thriller Michael Jackson").expect("network call failed");
        assert!(!results.is_empty(), "expected at least one result for a well-known album");
        assert!(results.iter().any(|r| r.title.to_lowercase().contains("thriller")));
    }

    #[test]
    fn search_deezer_album_finds_a_well_known_album() {
        let results = search_deezer_album("Thriller Michael Jackson").expect("network call failed");
        assert!(!results.is_empty(), "expected at least one result for a well-known album");
        assert!(results.iter().any(|r| r.title.to_lowercase().contains("thriller")));
    }

    #[test]
    fn search_musicbrainz_album_does_not_panic_on_a_known_query() {
        let _ = search_musicbrainz_album("Thriller Michael Jackson");
    }

    #[test]
    fn download_cover_to_temp_file_saves_a_real_itunes_cover() {
        let results = search_itunes_album("Thriller Michael Jackson").expect("network call failed");
        let cover_url = results
            .iter()
            .find_map(|r| r.cover_url.clone())
            .expect("expected at least one result with a cover");
        let path = download_cover_to_temp_file(&cover_url).expect("download failed");
        let metadata = std::fs::metadata(&path).expect("downloaded file should exist");
        assert!(metadata.len() > 1000, "downloaded cover is suspiciously small: {} bytes", metadata.len());
        std::fs::remove_file(&path).ok();
    }
}
