# MusicNojo

> 🇫🇷 [Lire en français](README.fr.md)

## Why this project

Music collection manager. Music Collector for Mac is an Intel app
that depends on Rosetta 2. Apple has announced the planned end of
Rosetta 2 (full support only through macOS 27, largely removed
starting with macOS 28, ~fall 2027) — so Music Collector has a
limited remaining lifespan on an up-to-date Mac. MusicNojo imports a
[Music Collector](https://www.collectorz.com/music) (Collectorz)
collection's database into its own tools, and lets you browse,
export, and edit it without being limited to what the vendor app
itself offers — reading and writing a copy of the existing collection
with its own independent tools (export, browse, add, edit, delete,
cover art), so it keeps working, and keeps growing, without depending
on Music Collector for either, the day it no longer runs.

MusicNojo is a sibling project to [BookNojo](https://github.com/sergiodadi/BookNojo)
(the same approach and the same underlying Rosetta 2 problem, applied
to Book Collector instead of Music Collector).

> Actively developed project, publicly available since 2026-11-17.

## What it does today

- **macOS application** (icon, window, native menus) to view, add, edit
  and delete albums in a collection — with automatic backups before
  any write, and an outright refusal to write to anything that looks
  like your real collection.
- Grouping by artist/genre/format/country/label, with an A→Z bar;
  search; sort (dropdown or column click); list/grid view; pagination.
- Multiple sub-collections in one file — tabs switch between them.
- Statistics panel (genres, formats, countries, labels, decades, year
  added), with the "no cover"/"no artist" counts clickable to see
  which albums they refer to — and the "no cover" list doubles as a
  one-click way to add the missing cover.
- Bilingual French/English interface.
- Command-line export to **CSV**, **JSON**, and a standalone **HTML**
  page.

➡️ Full guide to using the application: [`MANUAL.md`](MANUAL.md).
Command-line tools also exist (CSV/JSON/HTML export, scripting) —
optional, not needed for everyday use: see [`CLI.md`](CLI.md).

## Installation

Download the `.dmg` from the [latest release](https://github.com/sergiodadi/MusicNojo/releases/latest),
open it, and drag MusicNojo.app into Applications.

> ⚠️ **"MusicNojo is damaged and can't be opened"?** MusicNojo isn't
> signed with an Apple Developer certificate, so macOS Gatekeeper
> quarantines it and shows this misleading message — the app isn't
> actually damaged. Fix it by running this once in Terminal, then open
> the app normally:
> ```bash
> xattr -cr /Applications/MusicNojo.app
> ```
> If that doesn't work, right-click (or Control-click) the app and
> choose **Open** instead of double-clicking.

To compile from source instead of using the `.dmg`, see
[`BUILD.md`](BUILD.md).

> Tested only on **macOS Tahoe (26)** — not verified on other macOS
> versions.

## Version history

See [`CHANGELOG.md`](CHANGELOG.md).

## License

[MIT](LICENSE).

---

These packages were built, tested and published with the help of Claude, the AI assistant by Anthropic.

Music Collector and Collectorz/CLZ are trademarks of their respective owners. MusicNojo is an independent project, not affiliated with these companies.
