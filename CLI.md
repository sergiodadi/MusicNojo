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

### Sub-collections

A `.mucp` file can contain **several sub-collections**.

```bash
# List the sub-collections and their counters
./mc_export --db "<path>" --list-collections

# Export only one sub-collection
./mc_export --db "<path>" --collection "<name>" \
  --csv albums.csv --json albums.json
```

The counter shown by the application is the **sum of quantities** for
the active sub-collection, not the number of records — the two values
can legitimately differ.

## 3. The interactive HTML page

```bash
./mc_export --db "<path>" --html music_collection.html
```

Opens in any browser (double-click, or `open`), no server needed —
including cover images, referenced as local `file://` URLs. Three
zones, like the application:

- **on the left**: grouping (Artist by default, or Genre/Format/
  Country/Label via the dropdown menu), with an A→Z bar. Clicking the
  number next to "All" toggles the value list between alphabetical
  order and descending album-count order.
- **top right**: the list of albums in the selected group — search,
  sort, pagination, list/grid view toggle.
- **bottom right**: the detail card (cover, metadata, clickable
  artist/genre values that jump back into the groups panel).

The tabs at the top switch sub-collection without re-running the
command. The **Statistics** button shows an aggregated view (genres,
formats, countries, labels, decades, year added) for the active
sub-collection, including the same "no cover"/"no artist" expandable
lists the application has (read-only here — no click-to-fix, since
this page can't write anything).

English only (no language toggle), and there is **no add/edit/delete
form** — unlike BookNojo's own static export, which has a
preview-only form that downloads a JSON file for its CLI tool to
read. `mc_add` already covers adding/editing/deleting directly and
more safely (a reviewable plan before any real write), so a
JSON-download detour wouldn't add anything here.

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
`--location`, `--release-year`, `--rating`, `--notes`, `--upc`,
`--collection`), and `--track` (repeatable, one per occurrence, in
order). `--collection` must name an already-existing sub-collection —
it is never created. The album's date-added is stamped automatically
with the current date and time — there is no flag for it, since
editing an album never changes it afterward either.

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
too, including `--collection` (to move the album to a different
already-existing sub-collection). A role or many-to-many field is
only **replaced** if you pass its matching `--replace-*` flag
(`--replace-artists`, `--replace-genres`, etc.) — otherwise the values
you pass are ignored for that field and the existing ones are left
untouched.

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
