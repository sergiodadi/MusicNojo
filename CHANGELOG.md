# Changelog

> 🇫🇷 [Lire en français](CHANGELOG.fr.md)

History of **versions published on GitHub**, one entry per
tag/release — same convention as [BookNojo's CHANGELOG.md](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md).

## [v1.0.0] - 2026-11-26

**First stable release** — MusicNojo exits beta (previous releases were
v1.0.0b1 through v1.0.0b9).

- **Online search**: prefill a new album's fields and cover from
  iTunes, MusicBrainz or Deezer, by title/artist.
- **Import Collection** (File menu) now bootstraps an independent
  `.mnj` database from a Music Collector `.mucp`, kept separate from
  the everyday **Open Collection…** button — the original `.mucp` is
  never modified, and an existing `.mnj` is never overwritten.
- The "destination already exists" message now explains what to do
  next.

## [v1.0.0b9] - 2026-11-22

- New cover picker in the "Add Album…" form (optional) — no longer
  necessary to add the album first and set its cover afterwards via
  Statistics' "no cover" list, though that still works too.
- Header title and buttons gained the same emoji already used by
  BookNojo, and the Help window's language toggle now matches the
  main header's button order.

## [v1.0.0b8] - 2026-11-22

- "MusicNojo Help > About MusicNojo" (a plain name+version dialog
  duplicating the real native About panel) replaced with a real,
  bilingual "User Guide" help window.

## [v1.0.0b7] - 2026-11-22

- New app icon: a flat violet-gradient square with a white "MN" monogram,
  replacing the generic default Tauri icon used until now.

## [v1.0.0b6] - 2026-11-21

New `.dmg` (`MusicNojo_1.0.0-b6_aarch64.dmg` — the `.dmg` filename now
tracks the published release tag instead of Cargo.toml's per-commit dev
version, fixed in `tauri.conf.json`; previous releases wrongly showed an
internal dev version number in the filename). Also:

- The repository is now **public**.
- README: added a "Screenshots" section (two screenshots of the app)
  right after Installation.
- README: reworded the intro to lead with "Why this project", merging it
  with the opening paragraph.

## [v1.0.0b5] - 2026-11-16

New `.dmg` (`MusicNojo_0.56.0_aarch64.dmg`). Two additions, for
homogeneity with BookNojo:

- The static HTML export (`mc_export --html`) now has the same **EN /
  FR** toggle as the application — it previously stayed in English
  only.
- `mc_add commit`/`set-cover` gained a `--force` flag, lifting the
  automatic refusal on a path that looks like the real Music Collector
  collection. A typed `OUI` confirmation in the terminal is still
  required before any real write happens.

## [v1.0.0b4] - 2026-11-11

New `.dmg` (`MusicNojo_0.53.0_aarch64.dmg`). Fixes the version shown
in the header badge and both "About MusicNojo" surfaces (the custom
dialog and the native macOS panel) — they displayed Cargo.toml's own
per-commit development version (e.g. `0.51.0`) instead of the actual
published release (`v1.0.0b3`).

## [v1.0.0b3] - 2026-11-11

New `.dmg` (`MusicNojo_0.51.0_aarch64.dmg`, unsigned — same Gatekeeper
workaround as before). Layout and data fixes:

- Header no longer wraps to a second row at a narrow window width; the
  A-Z letter bar now spans the full width between the tabs and the
  three-pane layout.
- The album detail card was restructured: artist and title above the
  cover, UPC/track count/duration/format to its right, track listing
  below — then restyled with a dark background, a blue title, bold
  key fields, and a blue banner above the track list.
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
