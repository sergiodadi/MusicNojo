# Changelog

> 🇬🇧 [Read in English](CHANGELOG.md)

Historique des **versions publiées sur GitHub**, une entrée par
tag/release — même convention que le [CHANGELOG.md de BookNojo](https://github.com/sergiodadi/BookNojo/blob/main/CHANGELOG.md).

## [v1.0.0b8] - 2026-11-22

- « MusicNojo Help > About MusicNojo » (une simple boîte nom+version
  qui dupliquait le vrai panneau natif « About ») remplacé par une
  vraie fenêtre d'aide bilingue « User Guide ».

## [v1.0.0b7] - 2026-11-22

- Nouvelle icône d'application : un carré plat en dégradé violet avec un
  monogramme blanc « MN », à la place de l'icône générique par défaut de
  Tauri utilisée jusqu'ici.

## [v1.0.0b6] - 2026-11-21

Nouveau `.dmg` (`MusicNojo_1.0.0-b6_aarch64.dmg` — le nom du `.dmg` suit
désormais le tag de release publié au lieu de la version de dev par
commit de Cargo.toml, corrigé dans `tauri.conf.json` ; les releases
précédentes affichaient à tort un numéro de version de dev interne dans
le nom du fichier). Également :

- Le dépôt est désormais **public**.
- README : ajout d'une section « Copie d'écran » (deux captures de
  l'app) juste après Installation.
- README : reformulation de l'intro pour commencer par « Pourquoi ce
  projet », fusionnée avec le paragraphe d'ouverture.

## [v1.0.0b5] - 2026-11-16

Nouveau `.dmg` (`MusicNojo_0.56.0_aarch64.dmg`). Deux ajouts, pour
homogénéité avec BookNojo :

- L'export HTML statique (`mc_export --html`) a maintenant la même
  bascule **EN / FR** que l'application — il restait auparavant en
  anglais uniquement.
- `mc_add commit`/`set-cover` ont gagné un drapeau `--force`, qui lève
  le refus automatique sur un chemin qui ressemble à la vraie
  collection Music Collector. Une confirmation `OUI` tapée dans le
  terminal reste exigée avant toute écriture réelle.

## [v1.0.0b4] - 2026-11-11

Nouveau `.dmg` (`MusicNojo_0.53.0_aarch64.dmg`). Corrige la version
affichée dans le badge d'en-tête et les deux panneaux « About
MusicNojo » (le dialogue personnalisé et le panneau natif macOS) — ils
affichaient la version de développement de Cargo.toml (ex. `0.51.0`)
au lieu de la véritable release publiée (`v1.0.0b3`).

## [v1.0.0b3] - 2026-11-11

Nouveau `.dmg` (`MusicNojo_0.51.0_aarch64.dmg`, non signé — même
contournement Gatekeeper qu'avant). Corrections de mise en page et de
données :

- L'en-tête ne se replie plus sur une deuxième ligne à faible largeur
  de fenêtre ; la barre alphabétique A-Z occupe désormais toute la
  largeur entre les onglets et la disposition à trois panneaux.
- La fiche détail d'un album a été restructurée : artiste et titre
  au-dessus de la pochette, code-barres/nombre de pistes/durée/format
  à droite de la pochette, liste des pistes en dessous — puis
  restylée avec un fond noir, un titre en bleu, les champs clés en
  gras, et un bandeau bleu au-dessus de la liste des pistes.
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
