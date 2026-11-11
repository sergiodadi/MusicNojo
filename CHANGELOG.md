# Changelog

> 🇫🇷 [Lire en français](CHANGELOG.fr.md)

History of **versions published on GitHub**, one entry per
tag/release — same convention as [BookNojo's CHANGELOG.md](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md).

## [v1.0.0b4] - 2026-11-11

New `.dmg` (`MusicNojo_0.53.0_aarch64.dmg`). Fixes the version shown
in the header badge and both "About MusicNojo" surfaces (the custom
dialog and the native macOS panel) — they displayed Cargo.toml's own
per-commit development version (e.g. `0.51.0`) instead of the actual
published release (`v1.0.0b3`).

## [v1.0.0b3] - 2026-11-11

New `.dmg` (`MusicNojo_0.51.0_aarch64.dmg`, unsigned — same Gatekeeper
workaround as before). Layout and data fixes, all from direct
comparison with Music Collector and BookNojo:

- Header no longer wraps to a second row at a narrow window width; the
  A-Z letter bar now spans the full width between the tabs and the
  three-pane layout, matching Music Collector's own placement.
- The album detail card was restructured: artist and title above the
  cover, UPC/track count/duration/format to its right, track listing
  below — then restyled with a dark background, a blue title, bold
  key fields, and a blue banner above the track list, matching Music
  Collector's own look.
- Fixed a real bug: the track count shown everywhere (app, CSV/JSON
  export, static HTML export) came from a stored field that can be 0
  even when an album's tracks are all present — it's now counted from
  the actual track rows instead.

## [v1.0.0b2] - 2026-11-10

First downloadable `.dmg` for MusicNojo (`MusicNojo_0.44.0_aarch64.dmg`,
unsigned — see [`README.md`](README.md) for the Gatekeeper "damaged"
workaround).

## [v1.0.0b1] - 2026-11-10

First beta version of MusicNojo, tagged on GitHub — source only, no
downloadable `.dmg` yet (see [`BUILD.md`](BUILD.md) to compile it).

Day-to-day development history (one feature per commit) lives in the
project's own commit messages (`vX.Y.0: ...`), inside the
repository.
