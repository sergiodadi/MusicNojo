# Command-line tools — `mc_export` / `mc_add`

> 🇫🇷 [Lire en français](CLI.fr.md)

> For everyday use, these commands are **not needed**: the application
> (see [`MANUAL.md`](MANUAL.md)) is enough to view, add, edit and
> delete albums. This document is for anyone who wants to export a
> collection (CSV/JSON/HTML) or script additions from the terminal — a
> more technical, optional use.

## 1. Overview

- **`mc_export`** reads a [Music Collector](https://www.collectorz.com/music)
  (Collectorz) collection read-only and produces a **CSV** export, a
  **JSON** export, and/or a standalone **HTML page** (§ 3).
- **`mc_add`** adds, edits, deletes albums and sets covers from the
  command line (§ 4), with the same safety gates as the application.

`mc_export` always opens the database read-only
(`SQLITE_OPEN_READ_ONLY`): it can never modify the original collection.

The commands below assume you are in the folder containing the compiled
binaries (`project/target/release/` if compiled from source — see
[`BUILD.md`](BUILD.md)).

## 2. `mc_export` — export

```bash
./mc_export \
  --db "~/Documents/Collectorz/Music Collector/MyCollection.mucp" \
  --csv music_collection.csv \
  --json music_collection.json \
  --html music_collection.html
```

`--db` accepts either the `.mucp` folder, or directly the `Music_data`
file it contains. All three output flags are optional and independent —
pass only the ones you need.

The **CSV** is a flat, one-row-per-album summary (id, title, every
role and many-to-many field joined with `; `, every scalar field,
track count). The **JSON** is the full nested shape (albums → discs →
tracks), the same data the application itself loads.

## 3. The HTML export

```bash
./mc_export --db "<path>" --html music_collection.html
```

Opens in any browser (double-click, or `open`), no server needed. A
single page: a search box (title/artist) and a sortable table (title,
artists, genres, format, country, release year, track count) — a
simple, static summary. It does **not** reproduce the application's
full interface (no grouping sidebar, no tabs, no statistics, no
add/edit form, no cover images) — for that, use the application itself
(see [`MANUAL.md`](MANUAL.md)).

## 4. `mc_add` — adding, editing, deleting, covers

> ⚠️ **Always test on a copy of your collection first, never directly
> on the original.** `mc_add` refuses outright to write to a path that
> looks like the real Music Collector collection
> (`~/Documents/Collectorz/.../Music Collector.../`) — see § "Safety
> gates" below. There is no override: point it at a copy, always (see
> [`BUILD.md`](BUILD.md) for how to generate dedicated test copies).

### Plan, then commit

Adding, editing or deleting an album is a two-step process: a
`*-plan` subcommand computes and writes the SQL that *would* run,
without touching the database; `commit` then executes a plan file's
SQL, inside a single transaction. Reviewing the plan before `commit`
is the point — nothing is written until you do.

```bash
# 1. Add — compute the plan.
./mc_add add-plan --db "<path>" \
  --title "Some Album" \
  --artist "Some Artist" \
  --genre "Rock" \
  --format "CD" --release-year "2024" \
  --track "Track One" --track "Track Two" \
  --out plan.sql

# 2. Review plan.sql by hand if you want to.

# 3. Commit — this is the only step that actually writes (and backs up first).
./mc_add commit --db "<path>" --plan plan.sql
```

`add-plan` accepts every role (repeatable: `--artist`, `--composer`,
`--conductor`, `--musician`, `--orchestra`, `--chorus`), every
many-to-many field (repeatable: `--genre`, `--tag`, `--extra`,
`--studio`), every scalar field (`--format`, `--label`, `--country`,
`--condition`, `--packaging`, `--vinyl-color`, `--spars`, `--owner`,
`--location`, `--release-year`, `--rating`, `--notes`, `--upc`), and
`--track` (repeatable, one per occurrence, in order).

### Editing — `edit-plan`

```bash
./mc_add edit-plan --db "<path>" --id <album id> \
  --title "New Title" \
  --replace-artists --artist "New Artist" \
  --out edit.sql
./mc_add commit --db "<path>" --plan edit.sql
```

`--id` is the album's `Z_PK` (`mc_export`'s JSON output, field `"id"`).
Every scalar/role/many-to-many flag from `add-plan` is available here
too. A role or many-to-many field is only **replaced** if you pass its
matching `--replace-*` flag (`--replace-artists`, `--replace-genres`,
etc.) — otherwise the values you pass are ignored for that field and
the existing ones are left untouched. There is no way yet to set an
album's sub-collection or its date-added from the command line.

### Deleting — `delete-plan`

```bash
./mc_add delete-plan --db "<path>" --id <album id> --out delete.sql
./mc_add commit --db "<path>" --plan delete.sql
```

Removes the album and everything specific to it (its role/many-to-many
link rows, discs, tracks) — never the shared lookup rows it referenced
(a format, a label, a genre, an artist...), since other albums may
still use them.

### Covers — `set-cover`

```bash
./mc_add set-cover --db "<path>" --id <album id> --front cover.jpg
./mc_add set-cover --db "<path>" --id <album id> --front front.jpg --back back.jpg
```

Writes immediately — unlike the plan/commit split above, a cover is
just a file copy into the package's `Images/` folder (named
`"<id>_f_<slug>.<ext>"` / `"<id>_b_<slug>.<ext>"`, matching the
application's own naming) plus one column each, nothing to review
first.

### Safety gates

Before writing, `commit` and `set-cover` both check, in order:

1. the database path **does not look like** the real Music Collector
   collection (`~/Documents/Collectorz/...`) — refusal otherwise, with
   no override;
2. a timestamped backup of `Music_data` is created in `backups/`, next
   to the database;
3. the SQL runs in a single transaction (`commit`) or the file copy
   plus column updates happen (`set-cover`).

## 5. Troubleshooting

- **`--db "~/Documents/..."` fails**: if the path is quoted, the shell
  doesn't expand `~` itself — the tool does it for you.
- **SQLite lock error**: close Music Collector before running an export
  or an add.

## 6. Going further

- [`MANUAL.md`](MANUAL.md): using the application — enough for everyday
  use, without these commands.
- [`BUILD.md`](BUILD.md): compiling the tools and the application from
  source.
- [`CHANGELOG.md`](CHANGELOG.md): history of versions published on
  GitHub.
- `./mc_export --help` / `./mc_add <subcommand> --help`: full list of
  options.
