# Compiling MusicNojo from source

> 🇫🇷 [Lire en français](BUILD.fr.md)

This document explains how to compile the command-line tools and the
macOS application from source code. To **use** the software once
compiled, see [`MANUAL.md`](MANUAL.md) (the application — enough for
everyday use) and [`CLI.md`](CLI.md) (the `mc_export`/`mc_add`
command-line tools, optional).

## Prerequisites

- **Rust** (`cargo`) — installable via Homebrew if needed:
  `brew install rust`.
- **Tauri CLI**, only for the macOS application (not needed for the CLI
  tools alone):
  ```bash
  cargo install tauri-cli --version "^2.0.0"
  ```

All the code and manifests live under **`project/`** — the repo root
only contains documentation (`README.md`, `BUILD.md`, `MANUAL.md`,
`CLI.md`, `CHANGELOG.md`). All the commands below assume you are
inside `project/`.

## Command-line tools (`mc_export`, `mc_add`)

```bash
cd musicnojo/project
cargo build --release
```

The binaries are then `target/release/mc_export` and
`target/release/mc_add` (so `project/target/release/...` from the repo
root).

```bash
cargo test --release
```

## macOS application

```bash
cd app
cargo tauri dev      # development window, hot reload
cargo tauri build    # builds a real .app (and a .dmg), in
                      # app/src-tauri/target/release/bundle/
```

Unlike BookNojo, the app's frontend (`app/src/{index.html,app.js,style.css}`)
is edited directly — there is no shared-template/generation step to
run first.

`cargo build --release` (from `project/`) does **not** produce an
installable application — only `cargo tauri build` (inside
`project/app/`) does.

### Application icon

The icons currently bundled under `app/src-tauri/icons/` are the
Tauri scaffold's own defaults — MusicNojo has no custom icon yet. Once
a 1024×1024 source image exists, it can be turned into the full icon
set with:

```bash
cd project/app
cargo tauri icon path/to/source.png
```

## Test data (development only)

These copies and scripts are only needed to contribute to the project —
not to use the software on your own collection. From `project/`:

```bash
./refresh-testdata.sh      # testdata/: read-only copy, for mc_export
./refresh-testdata-rw.sh   # testdata-rw/: writable copy, for mc_add
```

`sample-collection/` (committed to the repo, entirely fictional — see
[`MANUAL.md`](MANUAL.md) § 1) can be regenerated with:

```bash
cargo run --release --example generate_sample_collection
```

None of these scripts, nor the tools themselves, ever write to the
user's real collection file (`~/Documents/Collectorz/Music Collector/...`)
— a project rule, checked by dedicated tests.

## Repository structure

```
README.md, BUILD.md, MANUAL.md,     documentation (repo root)
CLI.md, CHANGELOG.md, LICENSE

project/
├─ src/              shared library + CLI binaries (mc_export, mc_add)
├─ app/              macOS application (Tauri) — reuses src/ via a path
│                     dependency
├─ examples/          generates the sample collection (sample-collection/)
├─ Cargo.toml         root crate manifest (music_collector_export)
└─ refresh-testdata*.sh   test-data scripts
```

## Appendix: Music Collector data model

> ⚠️ **Given for information only** — this field catalogue may be
> inaccurate or incomplete. It describes Music Collector's built-in
> XML export (Export menu, XML format), not the actual storage format
> used by the rest of this project.

Every album exported to XML declares, at the top of the file, the
list of fields it knows about (`<field name="..." id="..." label="...">`
tag). 95 fields in total, in the order Music Collector declares them:

| Internal ID | Label |
|---|---|
| `dfTitleSort` | Title Sort |
| `dfArtist` | Artist |
| `dfSubTitle` | SubTitle |
| `dfReleaseDate` | Release Date |
| `dfMyRating` | My Rating |
| `dfLabel` | Label |
| `dfOrigReleaseDate` | Orig. Rel. Date |
| `dfReleaseYear` | Release Year |
| `dfTourname` | Tourname |
| `dfLabelNumber` | Cat. Number |
| `dfPosition` | Position |
| `dfComposer` | Composer |
| `dfOrigReleaseYear` | Original Release Year |
| `dfSpars` | Spars |
| `dfUPC` | UPC (Barcode) |
| `dfComposition` | Composition |
| `dfInstrument` | Instrument |
| `dfCountry` | Country |
| `dfFormat` | Format |
| `dfCondition` | Package/Sleeve Condition |
| `dfConductor` | Conductor |
| `dfRecordingYear` | Recording Year |
| `dfSound` | Sound |
| `dfPackaging` | Packaging |
| `dfCurrentValue` | Current Value |
| `dfOrchestra` | Orchestra |
| `dfPurchaseYear` | Purchase Year |
| `dfExtras` | Extras |
| `dfGenre` | Genre |
| `dfRare` | Rare |
| `dfChorus` | Chorus |
| `dfNotes` | Notes |
| `dfImageLinks` | Image Files |
| `dfOwner` | Owner |
| `dfArtistFirstLetter` | Artist First Letter |
| `dfStudio` | Studio |
| `dfOnlineLinks` | Online Links |
| `dfMovieLinks` | Movie Files |
| `dfLocation` | Location |
| `dfVinylColor` | Vinyl Color |
| `dfRecordingDate` | Rec. Date |
| `dfLocalLinks` | Local Links |
| `dfOtherLinks` | Other Files |
| `dfPurchaseDate` | Purchase Date |
| `dfTrackText1` | Track Text1 |
| `dfLoaner` | Loaned To |
| `dfVinylWeight` | Vinyl Weight |
| `dfIndex` | Index |
| `dfPlaylistLinks` | Playlist Files |
| `dfPrice` | Purchase Price |
| `dfLoanDate` | Loan Date |
| `dfRPM` | RPM |
| `dfQuantity` | Quantity |
| `dfStore` | Store |
| `dfMusicFile` | Music File |
| `dfDueDate` | Due Date |
| `dfTag` | Tags |
| `dfCollectionStatus` | Collection Status |
| `dfMediaCondition` | Media Condition |
| `dfRating` | My Rating |
| `dfLyricsFile` | Lyrics File |
| `dfIsOverdue` | isOverdue |
| `dfMusician` | Musician |
| `dfLinks` | Online Links |
| `dfClzAlbumID` | Clz Album ID |
| `dfStorageDevice` | Storage Device |
| `dfReturnDate` | Return Date |
| `dfCredit` | Credits |
| `dfLive` | Live |
| `dfLastModified` | Last Modified |
| `dfSlot` | Slot |
| `dfLoanNotes` | Loan Notes |
| `dfSyncFrontCover` | Custom Image |
| `dfLength` | Length |
| `dfSongwriter` | Songwriter |
| `dfCoverFront` | Front Cover |
| `dfLoanEmail` | Loaned To Email |
| `dfAddedDate` | Added Date |
| `dfBPM` | BPM |
| `dfProducer` | Producer |
| `dfCoverBack` | Back Cover |
| `dfLoanAddress` | Loaned To Address |
| `dfCollection` | Collection |
| `dfEngineer` | Engineer |
| `dfBoxSet` | Box Set |
| `dfLoanTitle` | Loan Title |
| `dfSyncBackCover` | Custom Back Cover |
| `dfUserCredit1` | User Credit 1 |
| `dfBackdrop` | Backdrop |
| `dfUserCredit2` | User Credit 2 |
| `dfTitle` | Title |
| `dfKey` | Key |
| `dfThumbFilePath` | Thumbnail |
| `dfBPAlbumID` | ClzAlbumID |
| `dfBPMusicLastReceivedRevision` | BPMusicLastReceivedVersion |

Beyond the simple fields above, each album also contains repeatable
lists (an album can have several artists, several credits, etc.) and
a nested disc/track hierarchy:

- `<artists>` → `<artist>` (display name, sort name, first name, last
  name) — same shape for `<composers>`, `<conductors>`, `<musicians>`,
  `<orchestras>`, `<choruses>`, each its own dedicated role list.
- `<credits>` → generic credits (role + person), `<compositions>` →
  classical compositions.
- `<genres>`, `<studios>`, `<tags>`, `<sounds>` → lists of plain
  values (display name + sort name).
- `<links>`, `<onlinelinks>`, `<locallinks>`, `<otherlinks>`,
  `<playlistlinks>`, `<movielinks>`, `<imagelinks>` → file/URL links.
- `<details>` → nested disc-level (`type="disc"`) and track-level
  (`type="track"`) records, each recursively containing the same
  per-item shape (its own artists/genres/etc., rarely used at the
  track level in practice).
