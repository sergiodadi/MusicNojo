# music_collector_export (MusicNojo)

Command-line tools for Music Collector (Collectorz) collections,
following the same approach as the BookNojo project (for Book
Collector): read/write a copy of the collection's own SQLite file
directly, never the live file.

**Status: early, private.** CLI tools and the macOS app are both
functionally complete relative to each other: anything `mc_export`
reads, both `mc_add` and the app can write.

All code and manifests live under **`project/`** — the repo root only
holds documentation. Every command below assumes you are inside
`project/`.

## App

A Tauri app under `project/app/`: open a collection (native folder
picker), browse/search it, click an album for its full detail
(artists/composers/conductors/musicians/orchestras/choruses, genres/
tags/extras/studios, cover image, every scalar field, tracks with
lengths), and add/edit/delete albums through a form. Opens the bundled
fictional `sample-collection/` by default.

```sh
cd project/app
cargo tauri dev      # development window, hot reload
cargo tauri build    # builds a real .app (and a .dmg), in
                      # app/src-tauri/target/release/bundle/
```

Same safety rule as the CLI: refuses to write to a path that looks
like your real collection, and makes an automatic backup
(`backups/Music_data.<timestamp>.bak`, next to the collection) before
every write it does make.

## Binaries

- **`mc_export`** — exports the collection to CSV, JSON, and/or a
  standalone HTML page.
  ```sh
  mc_export --db path/to/MyCollection.mucp --csv out.csv --json out.json --html out.html
  ```
- **`mc_add`** — a plan-then-commit workflow to add, edit, or delete an
  album. A `*-plan` subcommand only computes and prints/writes the SQL
  that would run; `commit` is the only subcommand that actually
  writes, and only after you've reviewed the plan. `set-cover` is the
  one exception (plain file copy, no row allocation to review).
  ```sh
  mc_add add-plan --db path/to/MyCollection.mucp \
    --title "Some Album" --artist "Some Artist" --genre "Rock" \
    --track "Track One" --track "Track Two" \
    --out plan.sql
  mc_add commit --db path/to/MyCollection.mucp --plan plan.sql

  mc_add edit-plan --db path/to/MyCollection.mucp --id <album id> \
    --title "New Title" --out edit.sql
  mc_add delete-plan --db path/to/MyCollection.mucp --id <album id> \
    --out delete.sql
  mc_add set-cover --db path/to/MyCollection.mucp --id <album id> \
    --front cover.jpg
  ```
  Both tools refuse to operate on a path that looks like your real
  collection (`~/Documents/Collectorz/.../Music Collector.../`) — only
  ever point them at a copy.

## Build and test

```sh
cd project
cargo build --release
cargo test --release
```

Tests run against local, gitignored test copies and are skipped
gracefully if those copies aren't present:

```sh
./refresh-testdata.sh     # testdata/: read-only copy, for mc_export
./refresh-testdata-rw.sh  # testdata-rw/: writable copy, for mc_add
```

Neither script, nor the tools themselves, ever write to the real
collection file.

## License

[MIT](LICENSE).
