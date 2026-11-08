# music_collector_export (MusicNojo)

Command-line tools for Music Collector (Collectorz) collections,
following the same approach as the BookNojo project (for Book
Collector): read/write a copy of the collection's own SQLite file
directly, never the live file.

**Status: early, private, CLI only.** No app yet — see `mc_export`/
`mc_add` below.

## Binaries

- **`mc_export`** — exports the collection to CSV, JSON, and/or a
  standalone HTML page.
  ```sh
  mc_export --db path/to/MyCollection.mucp --csv out.csv --json out.json --html out.html
  ```
- **`mc_add`** — a plan-then-commit workflow to add, edit, or delete an
  album. A `*-plan` subcommand only computes and prints/writes the SQL
  that would run; `commit` is the only subcommand that actually
  writes, and only after you've reviewed the plan.
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
  ```
  Both tools refuse to operate on a path that looks like your real
  collection (`~/Documents/Collectorz/.../Music Collector.../`) — only
  ever point them at a copy.

## Build and test

```sh
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
