# User manual — MusicNojo application

> 🇫🇷 [Lire en français](MANUAL.fr.md)

This document explains how to use the macOS **MusicNojo** application
once installed. No downloadable release yet — to compile it from
source, see [`BUILD.md`](BUILD.md).

## 1. Opening a collection

- **"Open Collection…"** opens the native macOS file picker to choose
  a `.mucp` collection. The picker only accepts `.mucp` files.
- On the very first launch, before opening anything, MusicNojo shows
  an entirely fictional bundled sample collection (12 invented albums,
  no personal data) — so there's always something to see, without ever
  exposing real data by default.
- The header always displays the name of the open file, a version
  badge, and an EN/FR language toggle.

## 2. Browsing the library

- **Three zones**: the grouping panel on the left, the album list at
  top right, the detail card at bottom right. The dividers between the
  three zones can be resized with the mouse.
- **Grouping** by Artist (default), Genre, Format, Country or Label,
  with an A→Z alphabetical bar that narrows which group values are
  listed (pick a letter, then a value from the narrowed list, to
  filter the album list).
- **List or grid view** (cover thumbnails) — a toggle above the list.
- **Sort**: a dropdown (Title, Artist, Year — newest first, Label) or
  click any column header to sort by that column (click again to
  reverse direction).
- **Search** by title or artist.
- **Pagination**, 60 albums per page.
- **Multiple sub-collections** in a single file — tabs at the top
  switch between them, scoping the list, groups, A-Z bar and
  statistics to whichever is active.
- **Statistics**: album/track counts, genre/format/country/label/
  decade/year-added breakdowns, for the currently active tab. The "N
  with no cover" and "N with no artist" counts are clickable, and
  expand an Artist — Title list of the matching albums below. In the
  "no cover" list, each album is itself clickable: it opens a native
  file picker to choose an image, which becomes that album's front
  cover immediately.
- **Album detail card**: cover, title, every role (artists, composers,
  conductors, musicians, orchestras, choruses), genres, tags, label,
  format, country, year, rating, notes, UPC, date added, and the
  track listing (grouped by disc, when there's more than one).

## 3. Adding, editing, deleting an album

- **Add an album**: a form covering every field (every role, every
  many-to-many field, every scalar field, and tracks — one per line).
- **Edit an album**: from its detail card, the same form opens
  pre-filled with the current values.
- **Delete an album**: from its detail card, after a confirmation
  dialog.
- After any add, edit or delete, the library is automatically
  reloaded: the change appears immediately, with no manual action
  needed.
- There is no way yet, from the application, to set an album's
  sub-collection or to pick which cover to use when *adding* an album
  — only `set-cover` afterwards (via the statistics panel's "no
  cover" list, or `mc_add set-cover` from the command line) sets one.

## 4. Bilingual interface

- An **EN / FR** toggle in the header switches the whole interface —
  grouping, list, detail card, statistics, add/edit form — between
  English and French. The choice is remembered from one session to
  the next (and so is the chosen layout size).

## 5. Data safety

- The user's real Music Collector collection is never modified by
  accident: every write (add, edit, delete, cover) is preceded by a
  timestamped backup of the file, and is refused outright — with no
  override reachable from the application — if the path looks like the
  real collection.
- MusicNojo can open the real collection in **read-only** mode to
  browse it (opening it is always `SQLITE_OPEN_READ_ONLY` at the
  database level), but none of the write actions above have any
  exception for it.

## 6. What the application doesn't do yet

- Setting an album's sub-collection or date-added, from the
  application or the command line.
- Online lookup to pre-fill a record from a barcode/UPC or a Discogs/
  MusicBrainz match — not started.
- Not yet code-signed/notarized with an Apple developer account — no
  consequence for personal use, but needed before distributing the
  application to anyone else.

## 7. Going further

- [`BUILD.md`](BUILD.md): compiling the application from source.
- [`CHANGELOG.md`](CHANGELOG.md): history of versions published on
  GitHub.

For everyday use, the application alone is enough — you don't need to
know any terminal commands. For the command-line tools
(`mc_export`/`mc_add`), meant for more technical use (CSV/JSON/HTML
export, scripts), see [`CLI.md`](CLI.md).
