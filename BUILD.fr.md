# Compiler MusicNojo depuis les sources

> 🇬🇧 [Read in English](BUILD.md)

Ce document explique comment compiler les outils en ligne de commande
et l'application macOS depuis le code source. Pour **utiliser** le
logiciel une fois compilé, voir [`MANUAL.fr.md`](MANUAL.fr.md)
(l'application — suffisant pour un usage courant) et
[`CLI.fr.md`](CLI.fr.md) (les outils en ligne de commande
`mc_export`/`mc_add`, facultatifs).

## Prérequis

- **Rust** (`cargo`) — installable via Homebrew si besoin :
  `brew install rust`.
- **Tauri CLI**, uniquement pour l'application macOS (pas nécessaire
  pour les seuls outils en ligne de commande) :
  ```bash
  cargo install tauri-cli --version "^2.0.0"
  ```

Tout le code et les manifestes se trouvent sous **`project/`** — la
racine du dépôt ne contient que de la documentation (`README.md`,
`BUILD.md`, `MANUAL.md`, `CLI.md`, `CHANGELOG.md`). Toutes les commandes
ci-dessous supposent que vous êtes dans `project/`.

## Outils en ligne de commande (`mc_export`, `mc_add`)

```bash
cd musicnojo/project
cargo build --release
```

Les binaires sont alors `target/release/mc_export` et
`target/release/mc_add` (donc `project/target/release/...` depuis la
racine du dépôt).

```bash
cargo test --release
```

## Application macOS

```bash
cd app
cargo tauri dev      # fenêtre de développement, rechargement à chaud
cargo tauri build    # construit une vraie .app (et un .dmg), dans
                      # app/src-tauri/target/release/bundle/
```

Contrairement à BookNojo, le frontend de l'app
(`app/src/{index.html,app.js,style.css}`) est édité directement — il
n'y a pas d'étape de génération depuis des templates partagés.

`cargo build --release` (depuis `project/`) ne produit **pas** une
application installable — seul `cargo tauri build` (dans
`project/app/`) le fait.

### Icône de l'application

Les icônes actuellement présentes sous `app/src-tauri/icons/` sont
celles par défaut du squelette Tauri — MusicNojo n'a pas encore
d'icône personnalisée. Une fois une image source 1024×1024 créée, elle
peut être transformée en jeu d'icônes complet avec :

```bash
cd project/app
cargo tauri icon chemin/vers/source.png
```

## Données de test (développement uniquement)

Ces copies et scripts ne sont nécessaires que pour contribuer au
projet — pas pour utiliser le logiciel sur sa propre collection. Depuis
`project/` :

```bash
./refresh-testdata.sh      # testdata/ : copie en lecture seule, pour mc_export
./refresh-testdata-rw.sh   # testdata-rw/ : copie modifiable, pour mc_add
```

`sample-collection/` (commitée dans le dépôt, entièrement fictive —
voir [`MANUAL.fr.md`](MANUAL.fr.md) § 1) peut être régénérée avec :

```bash
cargo run --release --example generate_sample_collection
```

Aucun de ces scripts, ni les outils eux-mêmes, n'écrit jamais dans le
vrai fichier de collection de l'utilisateur
(`~/Documents/Collectorz/Music Collector/...`) — une règle du projet,
vérifiée par des tests dédiés.

## Structure du dépôt

```
README.md, BUILD.md, MANUAL.md,     documentation (racine du dépôt)
CLI.md, CHANGELOG.md, LICENSE

project/
├─ src/              bibliothèque partagée + binaires CLI (mc_export, mc_add)
├─ app/              application macOS (Tauri) — réutilise src/ via une
│                     dépendance de chemin
├─ examples/          génère la collection d'exemple (sample-collection/)
├─ Cargo.toml         manifeste du crate racine (music_collector_export)
└─ refresh-testdata*.sh   scripts de données de test
```

## Annexe : modèle de données Music Collector

> ⚠️ **Donné à titre indicatif** — ce catalogue de champs peut être
> inexact ou incomplet. Il décrit l'export XML intégré de Music
> Collector (menu Export, format XML), pas le format de stockage réel
> utilisé par le reste de ce projet.

Chaque album exporté en XML déclare, en tête de fichier, la liste des
champs qu'il connaît (balise `<field name="..." id="..." label="...">`).
95 champs au total, dans l'ordre où Music Collector les déclare :

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

Chaque album, au-delà des champs simples ci-dessus, contient aussi des
listes répétables (un album peut avoir plusieurs artistes, plusieurs
crédits, etc.) et une hiérarchie disque/piste imbriquée :

- `<artists>` → `<artist>` (nom affiché, nom de tri, prénom, nom) —
  même structure pour `<composers>`, `<conductors>`, `<musicians>`,
  `<orchestras>`, `<choruses>`, chacun sa propre liste de rôle dédiée.
- `<credits>` → crédits génériques (rôle + personne), `<compositions>`
  → œuvres classiques.
- `<genres>`, `<studios>`, `<tags>`, `<sounds>` → listes de valeurs
  simples (nom affiché + nom de tri).
- `<links>`, `<onlinelinks>`, `<locallinks>`, `<otherlinks>`,
  `<playlistlinks>`, `<movielinks>`, `<imagelinks>` → liens fichier/URL.
- `<details>` → enregistrements imbriqués au niveau disque
  (`type="disc"`) et piste (`type="track"`), chacun contenant
  récursivement la même structure par élément (ses propres artistes/
  genres/etc., rarement utilisée en pratique au niveau piste).
