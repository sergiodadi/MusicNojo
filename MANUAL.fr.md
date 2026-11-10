# Manuel d'utilisation — application MusicNojo

> 🇬🇧 [Read in English](MANUAL.md)

Ce document explique comment utiliser l'application macOS
**MusicNojo** une fois installée. Pas encore de version
téléchargeable — pour la compiler depuis les sources, voir
[`BUILD.fr.md`](BUILD.fr.md).

## 1. Ouvrir une collection

- **"Ouvrir une collection…"** ouvre le sélecteur de fichier natif
  macOS pour choisir une collection `.mucp`. Le sélecteur n'accepte que
  les fichiers `.mucp`.
- Au tout premier lancement, avant d'avoir rien ouvert, MusicNojo
  affiche une collection d'exemple intégrée, entièrement fictive (12
  albums inventés, aucune donnée personnelle) — il y a donc toujours
  quelque chose à voir, sans jamais exposer de vraies données par
  défaut.
- Chaque lancement suivant rouvre la même collection, exactement là où
  on l'avait laissée — onglet actif, regroupement, recherche, tri,
  vue, page et album sélectionné sont tous mémorisés.
- L'en-tête affiche toujours le nom du fichier ouvert, un badge de
  version, et une bascule de langue EN/FR.

## 2. Parcourir la bibliothèque

- **Trois zones** : le panneau de regroupement à gauche, la liste des
  albums en haut à droite, la fiche détail en bas à droite. Les
  séparateurs entre les trois zones se redimensionnent à la souris.
- **Regroupement** par Artiste (par défaut), Genre, Format, Pays ou
  Label, avec une barre alphabétique A→Z qui réduit la liste des
  valeurs affichées (choisir une lettre, puis une valeur dans la liste
  réduite, pour filtrer la liste des albums).
- **Vue liste ou grille** (vignettes de pochettes) — une bascule
  au-dessus de la liste.
- **Tri** : un menu déroulant (Titre, Artiste, Année — plus récent
  d'abord, Label) ou un clic sur n'importe quel en-tête de colonne pour
  trier selon cette colonne (recliquer pour inverser le sens).
- **Recherche** par titre ou artiste.
- **Pagination**, 60 albums par page.
- **Plusieurs sous-collections** dans un même fichier — des onglets en
  haut permettent de basculer entre elles, en limitant la liste, les
  groupes, la barre A-Z et les statistiques à celle qui est active.
- **Statistiques** : nombre d'albums/pistes, répartitions par genre/
  format/pays/label/décennie/année d'ajout, pour l'onglet actif. Les
  compteurs "N sans pochette" et "N sans artiste" sont cliquables, et
  déroulent une liste Artiste — Titre des albums concernés en dessous.
  Dans la liste "sans pochette", chaque album est lui-même cliquable :
  il ouvre un sélecteur de fichier natif pour choisir une image, qui
  devient immédiatement la pochette (avant) de cet album.
- **Fiche détail d'un album** : pochette, titre, chaque rôle
  (artistes, compositeurs, chefs d'orchestre, musiciens, orchestres,
  chœurs), genres, étiquettes, label, format, pays, année, note,
  notes, UPC, date d'ajout, et la liste des pistes (regroupées par
  disque, s'il y en a plusieurs).

## 3. Ajouter, modifier, supprimer un album

- **Ajouter un album** : un formulaire couvrant chaque champ (chaque
  rôle, chaque champ many-to-many, chaque champ scalaire, et les
  pistes — une par ligne). Avant toute écriture, un aperçu liste
  chaque valeur saisie (artiste, genre, format...) avec un badge
  "nouveau" sur celles qui n'existent pas encore dans la collection
  (pour repérer une faute de frappe avant qu'elle devienne une entrée
  dupliquée permanente) — c'est la confirmation de cet aperçu qui
  ajoute réellement l'album. Le nouvel album rejoint l'onglet de
  sous-collection actif au moment de l'ajout.
- **Modifier un album** : depuis sa fiche détail, le même formulaire
  s'ouvre pré-rempli avec les valeurs actuelles, et écrit
  immédiatement (pas d'étape d'aperçu). La sous-collection d'un album
  ne peut pas encore être changée de cette façon.
- **Supprimer un album** : depuis sa fiche détail, après confirmation.
- Après tout ajout, modification ou suppression, la bibliothèque se
  recharge automatiquement : le changement apparaît immédiatement,
  sans action manuelle nécessaire.
- Il n'est pas encore possible, depuis l'application, de choisir une
  pochette au moment de l'*ajout* — seul `set-cover` ensuite (via la
  liste "sans pochette" du panneau de statistiques, ou
  `mc_add set-cover` en ligne de commande) en définit une.

## 4. Interface bilingue

- Une bascule **EN / FR** dans l'en-tête change toute l'interface —
  regroupement, liste, fiche détail, statistiques, formulaire
  d'ajout/modification — entre l'anglais et le français. Le choix est
  mémorisé d'une session à l'autre (tout comme la taille de la mise en
  page choisie).

## 5. Sécurité des données

- La vraie collection Music Collector de l'utilisateur n'est jamais
  modifiée par accident : chaque écriture (ajout, modification,
  suppression, pochette) est précédée d'une sauvegarde horodatée du
  fichier, et est purement et simplement refusée — sans aucun
  contournement accessible depuis l'application — si le chemin
  ressemble à la vraie collection.
- MusicNojo peut ouvrir la vraie collection en mode **lecture seule**
  pour la parcourir (l'ouverture se fait toujours en
  `SQLITE_OPEN_READ_ONLY` au niveau de la base), mais aucune des
  actions d'écriture ci-dessus n'a d'exception pour elle.

## 6. Ce que l'application ne fait pas encore

- Changer la sous-collection d'un album en le modifiant (seul l'ajout
  en définit une, celle de l'onglet actif), ou définir sa date
  d'ajout, depuis l'application ou la ligne de commande.
- Recherche en ligne pour pré-remplir une fiche à partir d'un
  code-barres/UPC ou d'une correspondance Discogs/MusicBrainz — pas
  commencé.
- Pas encore signée/notarisée avec un compte développeur Apple — sans
  conséquence pour un usage personnel, mais nécessaire avant de
  distribuer l'application à qui que ce soit d'autre.

## 7. Pour aller plus loin

- [`BUILD.fr.md`](BUILD.fr.md) : compiler l'application depuis les
  sources.
- [`CHANGELOG.fr.md`](CHANGELOG.fr.md) : historique des versions
  publiées sur GitHub.

Pour un usage courant, l'application seule suffit — aucune commande de
terminal n'est nécessaire. Pour les outils en ligne de commande
(`mc_export`/`mc_add`), destinés à un usage plus technique (export
CSV/JSON/HTML, scripts), voir [`CLI.fr.md`](CLI.fr.md).
