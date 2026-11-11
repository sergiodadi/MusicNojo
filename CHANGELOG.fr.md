# Changelog

> 🇬🇧 [Read in English](CHANGELOG.md)

Historique des **versions publiées sur GitHub**, une entrée par
tag/release — même convention que le [CHANGELOG.md de BookNojo](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md).

## [v1.0.0b4] - 2026-11-11

Nouveau `.dmg` (`MusicNojo_0.53.0_aarch64.dmg`). Corrige la version
affichée dans le badge d'en-tête et les deux panneaux « About
MusicNojo » (le dialogue personnalisé et le panneau natif macOS) — ils
affichaient la version de développement de Cargo.toml (ex. `0.51.0`)
au lieu de la véritable release publiée (`v1.0.0b3`).

## [v1.0.0b3] - 2026-11-11

Nouveau `.dmg` (`MusicNojo_0.51.0_aarch64.dmg`, non signé — même
contournement Gatekeeper qu'avant). Corrections de mise en page et de
données, toutes issues d'une comparaison directe avec Music Collector
et BookNojo :

- L'en-tête ne se replie plus sur une deuxième ligne à faible largeur
  de fenêtre ; la barre alphabétique A-Z occupe désormais toute la
  largeur entre les onglets et la disposition à trois panneaux, comme
  dans Music Collector.
- La fiche détail d'un album a été restructurée : artiste et titre
  au-dessus de la pochette, code-barres/nombre de pistes/durée/format
  à droite de la pochette, liste des pistes en dessous — puis
  restylée avec un fond noir, un titre en bleu, les champs clés en
  gras, et un bandeau bleu au-dessus de la liste des pistes, comme
  dans Music Collector.
- Correction d'un vrai bug : le nombre de pistes affiché partout
  (application, export CSV/JSON, export HTML statique) provenait d'un
  champ stocké qui peut valoir 0 même quand toutes les pistes d'un
  album sont bien présentes — il est désormais compté à partir des
  pistes réelles.

## [v1.0.0b2] - 2026-11-10

Premier `.dmg` téléchargeable pour MusicNojo
(`MusicNojo_0.44.0_aarch64.dmg`, non signé — voir
[`README.fr.md`](README.fr.md) pour contourner le message « endommagé »
du Gatekeeper).

## [v1.0.0b1] - 2026-11-10

Première version bêta de MusicNojo, taguée sur GitHub — sources
uniquement, pas encore de `.dmg` téléchargeable (voir
[`BUILD.fr.md`](BUILD.fr.md) pour la compiler).

L'historique de développement au quotidien (une fonctionnalité par
commit) vit dans les messages de commit du projet (`vX.Y.0 : ...`), à
l'intérieur du dépôt.
