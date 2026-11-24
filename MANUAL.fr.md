# Manuel d'utilisation — application MusicNojo

> 🇬🇧 [Read in English](MANUAL.md)

Ce document explique comment utiliser l'application macOS
**MusicNojo** une fois installée. Pour la télécharger, voir
[`README.fr.md`](README.fr.md) ; pour la compiler depuis les sources,
voir [`BUILD.fr.md`](BUILD.fr.md).

## 1. Importer puis ouvrir une collection

MusicNojo garde sa propre copie indépendante de votre collection,
jamais votre base Music Collector d'origine — deux actions distinctes :

- **« File > Import Collection… »** — uniquement dans la barre de menu
  macOS (une action rare, à faire une seule fois, pas dans l'en-tête
  de l'app). Choisissez une vraie collection Music Collector `.mucp` ;
  MusicNojo la copie une seule fois dans une nouvelle base `.mnj`
  indépendante, à un emplacement fixe
  (`~/Documents/MusicNojo/<nom>.mnj`), puis l'ouvre automatiquement.
  Le `.mucp` d'origine n'est jamais modifié, seulement lu. Une
  collection `.mucp` créée avec une version de Music Collector
  différente de celle visée (20.3.2) est refusée, avec un message
  clair plutôt qu'un plantage. Importer à nouveau vers la même
  destination est refusé — ça n'écrase jamais un `.mnj` existant.
- Bouton **« 📂 Ouvrir une collection… »** dans l'en-tête (identique à
  **File > Open Collection…**) — l'action du quotidien. Ouvre le
  sélecteur de fichier natif macOS pour choisir un `.mnj` déjà
  importé ; un `.mucp` brut ne peut plus être sélectionné ici. Si
  aucun `.mnj` n'existe encore, le titre du sélecteur le signale et
  renvoie vers **File > Import Collection…**.
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
  pistes — une par ligne), avec en plus un sélecteur de pochette
  optionnel ("Choisir une image…") juste sous le titre — l'image
  choisie devient la pochette (avant) de l'album dès que celui-ci est
  créé. Avant toute écriture, un aperçu liste chaque valeur saisie
  (artiste, genre, format...) avec un badge "nouveau" sur celles qui
  n'existent pas encore dans la collection (pour repérer une faute de
  frappe avant qu'elle devienne une entrée dupliquée permanente) —
  c'est la confirmation de cet aperçu qui ajoute réellement l'album.
  Le nouvel album rejoint l'onglet de sous-collection actif au moment
  de l'ajout, et reçoit la date et l'heure actuelles comme date
  d'ajout — modifier l'album par la suite ne change jamais cette date.
  Si la pochette ne peut pas être enregistrée, l'album est quand même
  ajouté — un message le signale, et la pochette reste rattrapable
  ensuite (voir ci-dessous).
  Le formulaire d'ajout s'ouvre sur un bascule **« ✍️ Manuellement » /
  « 🔍 Recherche en ligne »**. Le panneau de recherche en ligne cherche
  par titre ou artiste, interroge en même temps 3 sources gratuites
  (iTunes, MusicBrainz, Deezer — jamais fusionnées entre elles, chaque
  résultat garde sa source visible), et choisir un résultat revient au
  formulaire manuel avec le titre, l'artiste, le label et l'année déjà
  remplis (iTunes et Deezer fournissent aussi une pochette, téléchargée
  automatiquement ; MusicBrainz n'en fournit pas) — à vérifier et
  compléter avant de valider, rien n'est écrit tant que le formulaire
  n'est pas soumis.
- **Modifier un album** : depuis sa fiche détail, le même formulaire
  s'ouvre pré-rempli avec les valeurs actuelles (sans sélecteur de
  pochette ici — modifier conserve la pochette existante), et écrit
  immédiatement (pas d'étape d'aperçu). La sous-collection d'un album
  ne peut pas encore être changée de cette façon.
- **Supprimer un album** : depuis sa fiche détail, après confirmation.
- Après tout ajout, modification ou suppression, la bibliothèque se
  recharge automatiquement : le changement apparaît immédiatement,
  sans action manuelle nécessaire.
- Pour poser la pochette d'un album qui n'en a pas, utiliser la liste
  "sans pochette" du panneau de statistiques. Il n'est pas encore
  possible de remplacer la pochette d'un album qui en a déjà une, ni
  de définir la pochette arrière, depuis l'application (`mc_add
  set-cover` peut faire les deux ; voir [`CLI.fr.md`](CLI.fr.md)).

## 4. Interface bilingue

- Une bascule **EN / FR** dans l'en-tête change toute l'interface —
  regroupement, liste, fiche détail, statistiques, formulaire
  d'ajout/modification — entre l'anglais et le français. Le choix est
  mémorisé d'une session à l'autre (tout comme la taille de la mise en
  page choisie).
- Les messages renvoyés par l'application elle-même après une action
  (confirmations, erreurs) restent en français quelle que soit la
  langue choisie — seule l'interface générée par le navigateur est
  traduite pour l'instant.

## 5. Sécurité des données

- La vraie collection Music Collector de l'utilisateur n'est jamais
  modifiée par accident : chaque écriture (ajout, modification,
  suppression, pochette) est précédée d'une sauvegarde horodatée du
  fichier, et est purement et simplement refusée — sans aucun
  contournement accessible depuis l'application — si le chemin
  ressemble à la vraie collection.
- Chaque écriture est aussi refusée purement et simplement si
  **Music Collector lui-même est actuellement lancé** — écrire dans le
  fichier pendant que l'application le garde ouvert risque un résultat
  incohérent, ou que l'application écrase le changement à sa prochaine
  sauvegarde. Fermer Music Collector d'abord.
- MusicNojo peut ouvrir la vraie collection en mode **lecture seule**
  pour la parcourir (l'ouverture se fait toujours en
  `SQLITE_OPEN_READ_ONLY` au niveau de la base), mais aucune des
  actions d'écriture ci-dessus n'a d'exception pour elle.

## 6. Ce que l'application ne fait pas encore

- Changer la sous-collection d'un album en le modifiant (seul l'ajout
  en définit une, celle de l'onglet actif).
- Recherche en ligne par code-barres/UPC — la recherche en ligne
  (section 3) cherche par titre/artiste, pas encore par code-barres.
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
