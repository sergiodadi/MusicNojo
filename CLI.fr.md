# Outils en ligne de commande — `mc_export` / `mc_add`

> 🇬🇧 [Read in English](CLI.md)

> Pour un usage quotidien, ces commandes ne sont **pas nécessaires** :
> l'application (voir [`MANUAL.fr.md`](MANUAL.fr.md)) suffit pour
> consulter, ajouter, modifier et supprimer des albums. Ce document
> s'adresse à qui veut exporter une collection (CSV/JSON/HTML) ou
> scripter des ajouts depuis le terminal — un usage plus technique et
> facultatif.

## 1. Vue d'ensemble

- **`mc_export`** lit une collection [Music Collector](https://www.collectorz.com/music)
  (Collectorz) en lecture seule et produit un export **CSV**, un export
  **JSON**, et/ou une page **HTML** autonome (§ 3).
- **`mc_add`** ajoute, modifie, supprime des albums et définit des
  pochettes depuis la ligne de commande (§ 4), avec les mêmes
  garde-fous que l'application.

`mc_export` ouvre toujours la base en lecture seule
(`SQLITE_OPEN_READ_ONLY`) : il ne peut jamais modifier la collection
d'origine.

Les commandes ci-dessous supposent que vous êtes dans le dossier
contenant les binaires compilés (`project/target/release/` si compilé
depuis les sources — voir [`BUILD.fr.md`](BUILD.fr.md)).

## 2. `mc_export` — export

```bash
./mc_export \
  --db "~/Documents/Collectorz/Music Collector/MyCollection.mucp" \
  --csv music_collection.csv \
  --json music_collection.json \
  --html music_collection.html
```

`--db` accepte soit le dossier `.mucp`, soit directement le fichier
`Music_data` qu'il contient. Les trois options de sortie sont
facultatives et indépendantes — ne passer que celles dont on a besoin.

Le **CSV** est un résumé plat, une ligne par album (id, titre, chaque
rôle et champ many-to-many joints par `; `, chaque champ scalaire,
nombre de pistes). Le **JSON** est la forme imbriquée complète
(albums → disques → pistes), les mêmes données que l'application
elle-même charge.

### Sous-collections

Un fichier `.mucp` peut contenir **plusieurs sous-collections**.

```bash
# Lister les sous-collections et leurs compteurs
./mc_export --db "<chemin>" --list-collections

# N'exporter qu'une seule sous-collection
./mc_export --db "<chemin>" --collection "<nom>" \
  --csv albums.csv --json albums.json
```

Le compteur affiché par l'application est la **somme des quantités**
de la sous-collection active, pas le nombre de fiches — les deux
valeurs peuvent légitimement différer.

## 3. La page HTML interactive

```bash
./mc_export --db "<chemin>" --html music_collection.html
```

S'ouvre dans n'importe quel navigateur (double-clic, ou `open`), sans
serveur — pochettes comprises, référencées comme des URL `file://`
locales. Trois zones, comme l'application :

- **à gauche** : regroupement (Artiste par défaut, ou Genre/Format/
  Pays/Label via le menu déroulant), avec une barre A→Z. Cliquer sur
  le nombre à côté de "All" bascule la liste des valeurs entre ordre
  alphabétique et ordre décroissant par nombre d'albums.
- **en haut à droite** : la liste des albums du groupe sélectionné —
  recherche, tri, pagination, bascule vue liste/grille.
- **en bas à droite** : la fiche détail (pochette, métadonnées,
  valeurs artiste/genre cliquables qui renvoient vers le panneau de
  regroupement).

Les onglets en haut changent de sous-collection sans relancer la
commande. Le bouton **Statistics** affiche une vue agrégée (genres,
formats, pays, labels, décennies, année d'ajout) pour la
sous-collection active, avec les mêmes listes déroulantes "sans
pochette"/"sans artiste" que l'application (en lecture seule ici —
pas de correction en un clic, cette page ne pouvant rien écrire).

Anglais uniquement (pas de bascule de langue), et il n'y a **pas de
formulaire d'ajout/modification/suppression** — contrairement à
l'export statique de BookNojo, qui a un formulaire d'aperçu
téléchargeant un JSON pour son outil en ligne de commande. `mc_add`
couvre déjà l'ajout/la modification/la suppression directement et
plus sûrement (un plan à relire avant toute écriture réelle), donc un
détour par un JSON téléchargé n'apporterait rien ici.

## 4. `mc_add` — ajout, modification, suppression, pochettes

> ⚠️ **Toujours tester sur une copie de sa collection d'abord, jamais
> directement sur l'original.** `mc_add` refuse purement et simplement
> d'écrire sur un chemin qui ressemble à la vraie collection Music
> Collector (`~/Documents/Collectorz/.../Music Collector.../`) — voir
> § "Garde-fous" ci-dessous. Il n'y a aucun contournement : toujours
> pointer vers une copie (voir [`BUILD.fr.md`](BUILD.fr.md) pour générer
> des copies de test dédiées).

### Planifier, puis valider

Ajouter, modifier ou supprimer un album se fait en deux temps : une
sous-commande `*-plan` calcule et écrit le SQL qui *serait* exécuté,
sans toucher à la base ; `commit` exécute ensuite le SQL d'un fichier
de plan, dans une seule transaction. Pouvoir relire le plan avant
`commit` est tout l'intérêt : rien n'est écrit avant.

```bash
# 1. Ajout — calcule le plan.
./mc_add add-plan --db "<chemin>" \
  --title "Un Album" \
  --artist "Un Artiste" \
  --genre "Rock" \
  --format "CD" --release-year "2024" \
  --track "Piste Un" --track "Piste Deux" \
  --out plan.sql

# 2. Relire plan.sql à la main si on le souhaite.

# 3. Valider — seule cette étape écrit réellement (et sauvegarde d'abord).
./mc_add commit --db "<chemin>" --plan plan.sql
```

`add-plan` accepte chaque rôle (répétable : `--artist`, `--composer`,
`--conductor`, `--musician`, `--orchestra`, `--chorus`), chaque champ
many-to-many (répétable : `--genre`, `--tag`, `--extra`, `--studio`),
chaque champ scalaire (`--format`, `--label`, `--country`,
`--condition`, `--packaging`, `--vinyl-color`, `--spars`, `--owner`,
`--location`, `--release-year`, `--rating`, `--notes`, `--upc`,
`--collection`), et `--track` (répétable, un par occurrence, dans
l'ordre). `--collection` doit nommer une sous-collection déjà
existante — elle n'est jamais créée. La date d'ajout de l'album est
horodatée automatiquement avec la date et l'heure actuelles — il n'y a
pas d'option pour ça, puisque modifier l'album par la suite ne la
change jamais non plus.

### Modifier — `edit-plan`

```bash
./mc_add edit-plan --db "<chemin>" --id <id de l'album> \
  --title "Nouveau Titre" \
  --replace-artists --artist "Nouvel Artiste" \
  --out edit.sql
./mc_add commit --db "<chemin>" --plan edit.sql
```

`--id` est le `Z_PK` de l'album (sortie JSON de `mc_export`, champ
`"id"`). Tous les paramètres scalaire/rôle/many-to-many de `add-plan`
sont disponibles ici aussi, y compris `--collection` (pour déplacer
l'album vers une autre sous-collection déjà existante). Un rôle ou un
champ many-to-many n'est **remplacé** que si son option `--replace-*`
correspondante est passée (`--replace-artists`, `--replace-genres`,
etc.) — sinon les valeurs passées pour ce champ sont ignorées et les
valeurs existantes restent inchangées.

### Supprimer — `delete-plan`

```bash
./mc_add delete-plan --db "<chemin>" --id <id de l'album> --out delete.sql
./mc_add commit --db "<chemin>" --plan delete.sql
```

Supprime l'album et tout ce qui lui est spécifique (ses lignes de
liaison rôle/many-to-many, ses disques, ses pistes) — jamais les
lignes de référence partagées qu'il utilisait (un format, un label, un
genre, un artiste...), puisque d'autres albums peuvent encore les
utiliser.

### Pochettes — `set-cover`

```bash
./mc_add set-cover --db "<chemin>" --id <id de l'album> --front pochette.jpg
./mc_add set-cover --db "<chemin>" --id <id de l'album> --front avant.jpg --back arriere.jpg
```

Écrit immédiatement — contrairement au découpage plan/commit
ci-dessus, une pochette n'est qu'une copie de fichier dans le dossier
`Images/` du paquet (nommée `"<id>_f_<slug>.<ext>"` /
`"<id>_b_<slug>.<ext>"`, comme le fait l'application elle-même) plus
une colonne chacune, rien à relire avant.

### Garde-fous

Avant d'écrire, `commit` et `set-cover` vérifient tous deux, dans
l'ordre :

1. le chemin de la base **ne ressemble pas** à la vraie collection
   Music Collector (`~/Documents/Collectorz/...`) — refus sinon, sans
   contournement possible ;
2. **Music Collector lui-même n'est pas actuellement lancé** (vérifié
   via `pgrep`) — refus sinon, car écrire dans `Music_data` pendant
   que l'application le garde ouvert risque un WAL incohérent, ou que
   l'application écrase le changement à sa prochaine sauvegarde.
   Fermer l'application d'abord, puis réessayer ;
3. une sauvegarde horodatée de `Music_data` est créée dans
   `backups/`, à côté de la base ;
4. le SQL s'exécute en une seule transaction (`commit`), ou la copie
   de fichier plus les mises à jour de colonnes ont lieu (`set-cover`).

### Pourquoi une base indépendante, séparée de Music Collector

Music Collector pour Mac est une application Intel qui dépend de
Rosetta 2 pour tourner sur Apple Silicon. Apple a annoncé la fin
programmée de Rosetta 2 : support complet jusqu'à macOS 27 "Golden
Gate" (automne 2026), largement retiré à partir de macOS 28 (automne
2027). Music Collector a donc une espérance de vie limitée sur un Mac
à jour, sauf si son éditeur sort une mise à jour native Apple Silicon.

L'objectif à long terme est une base **indépendante** de
l'application — amorcée une fois depuis la vraie collection (§
"`mc_add import`"), puis alimentée uniquement par MusicNojo, sans plus
dépendre de Music Collector pour grandir. L'extension choisie pour
cette base persistante est **`.mnc`** — elle fonctionne exactement
comme un `.mucp` (seule la présence d'un fichier `Music_data` dedans
compte, jamais le nom du dossier). Pour continuer à alimenter aussi la
vraie application Music Collector pendant une période de transition,
relancer la même séquence `add-plan`/`edit-plan`/`delete-plan` +
`commit` une seconde fois avec `--db` pointant vers une copie de la
vraie collection — chaque base est ainsi résolue et planifiée
**séparément** (jamais le même `plan.sql` réutilisé sur les deux),
puisque les clés primaires et les valeurs de référence déjà connues
divergent dès que les deux bases évoluent indépendamment.

### `mc_add import` — le transfert de Music Collector vers MusicNojo

Amorce une nouvelle base indépendante `.mnc` à partir d'une collection
existante (base + dossier `Images/`). Conçu pour n'être exécuté
**qu'une seule fois** : refuse d'écraser une destination qui existe
déjà.

```bash
./mc_add import --source "~/Documents/Collectorz/Music Collector/MyCollection.mucp"
# -> collection/MyCollection.mnc (destination par défaut)

# Ou avec une destination explicite :
./mc_add import \
  --source "~/Documents/Collectorz/Music Collector/MyCollection.mucp" \
  --dest ~/MyCollectionMusicNojo.mnc
```

## 5. Dépannage

- **`--db "~/Documents/..."` échoue** : si le chemin est entre
  guillemets, le shell n'étend pas lui-même le `~` — l'outil s'en
  charge.
- **Erreur de verrou SQLite** : fermer Music Collector avant de lancer
  un export ou un ajout.

## 6. Pour aller plus loin

- [`MANUAL.fr.md`](MANUAL.fr.md) : utiliser l'application — suffisant
  pour un usage courant, sans ces commandes.
- [`BUILD.fr.md`](BUILD.fr.md) : compiler les outils et l'application
  depuis les sources.
- [`CHANGELOG.fr.md`](CHANGELOG.fr.md) : historique des versions
  publiées sur GitHub.
- `./mc_export --help` / `./mc_add <sous-commande> --help` : liste
  complète des options.
