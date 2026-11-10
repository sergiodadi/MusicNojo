# MusicNojo

> 🇫🇷 [Lire en français](README.fr.md)

Music collection manager — imports a [Music Collector](https://www.collectorz.com/music)
(Collectorz) collection's database into its own tools, and lets you
browse, export, and edit it without being limited to what the vendor
app itself offers.

> Private, early-stage project — not yet published on GitHub.

## Why this project

MusicNojo is a sibling project to [BookNojo](https://github.com/sergiodadi/BookNojo)
(the same approach applied to Book Collector instead of Music
Collector): read and write a copy of the collection's own SQLite file
directly, with your own tools, rather than depending solely on the
vendor application to view or grow your collection.

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

No downloadable release yet — compile from source, see
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
