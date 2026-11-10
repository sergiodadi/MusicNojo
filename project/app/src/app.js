const { invoke } = window.__TAURI__.core;

let albums = [];
let sortKey = "title";
let sortAsc = true;
let imagesDir = "";
let currentPath = null;
let currentDetailId = null;
let groupField = "artists";
let selectedGroup = null;
let selectedLetter = null;
let panel = "library"; // 'library' | 'stats'
let viewMode = "list"; // 'list' | 'grid'
let collCounts = new Map();
let collectionsList = [];
let currentCollection = null;

// --- i18n ---------------------------------------------------------------
// Mirrors BookNojo's own t()/setLang() pattern: everything user-visible
// goes through t() so toggling the language re-renders consistently
// instead of leaving some parts translated and others not.
const I18N = {
  en: {
    nav_open: "Open Collection…",
    nav_add: "Add Album…",
    nav_stats: "📊 Statistics",
    nav_back_to_collection: "← Back to collection",
    search_placeholder: "Search by title or artist…",
    count_albums: "{n} album(s)",
    group_all: "All",
    azbar_all: "All",
    collection_default: "My Collection",
    opt_group_artists: "Artist",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Country",
    opt_group_label: "Label",
    opt_sort_title: "Title (A→Z)",
    opt_sort_artist: "Artist (A→Z)",
    opt_sort_year_desc: "Year (newest first)",
    opt_sort_label: "Label (A→Z)",
    opt_sort_date_added: "Date added (newest first)",
    list_view_list: "☰ List",
    list_view_grid: "▦ Grid",
    list_prev: "‹ Prev.",
    list_next: "Next ›",
    list_page_info: "Page {page} of {total}",
    col_title: "Title",
    col_artists: "Artist(s)",
    col_genres: "Genre(s)",
    col_format: "Format",
    col_year: "Year",
    col_tracks: "Tracks",
    detail_select: "Select an album.",
    detail_edit: "Edit…",
    detail_delete: "Delete…",
    detail_field_artists: "Artist(s)",
    detail_field_composers: "Composer(s)",
    detail_field_conductors: "Conductor(s)",
    detail_field_musicians: "Musician(s)",
    detail_field_orchestras: "Orchestra(s)",
    detail_field_choruses: "Chorus(es)",
    detail_field_genres: "Genre(s)",
    detail_field_tags: "Tag(s)",
    detail_field_label: "Label",
    detail_field_format: "Format",
    detail_field_country: "Country",
    detail_field_year: "Year",
    detail_field_rating: "Rating",
    detail_field_notes: "Notes",
    detail_field_upc: "UPC",
    detail_field_date_added: "Date added",
    detail_disc: "Disc {n}",
    detail_tracks: "Tracks",
    delete_confirm: 'Delete "{title}"? This cannot be undone (a backup is made first).',
    delete_failed: "Delete failed: {err}",
    cover_set_failed: "Setting the cover failed: {err}",
    form_title_add: "Add Album",
    form_title_edit: "Edit Album",
    form_label_title: "Title",
    form_label_artists: "Artist(s), comma-separated",
    form_label_composers: "Composer(s)",
    form_label_conductors: "Conductor(s)",
    form_label_musicians: "Musician(s)",
    form_label_orchestras: "Orchestra(s)",
    form_label_choruses: "Chorus(es)",
    form_label_genres: "Genre(s)",
    form_label_tags: "Tag(s)",
    form_label_extras: "Extra(s)",
    form_label_studios: "Studio(s)",
    form_label_format: "Format",
    form_label_label: "Label",
    form_label_country: "Country",
    form_label_year: "Year",
    form_label_rating: "Rating",
    form_label_upc: "UPC",
    form_label_notes: "Notes",
    form_label_tracks: "Tracks, one per line",
    form_tracks_placeholder: "One track title per line",
    form_cancel: "Cancel",
    form_save: "Save",
    stats_title: "Statistics — {name}",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} track(s)",
    stats_no_cover: "{n} with no cover",
    stats_add_cover_hint: "Click to add a cover",
    stats_no_artist: "{n} with no artist",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Countries",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "By release decade",
    stats_by_added_year: "By year added",
    stats_no_data: "No data.",
  },
  fr: {
    nav_open: "Ouvrir une collection…",
    nav_add: "Ajouter un album…",
    nav_stats: "📊 Statistiques",
    nav_back_to_collection: "← Retour à la collection",
    search_placeholder: "Rechercher par titre ou artiste…",
    count_albums: "{n} album(s)",
    group_all: "Tous",
    azbar_all: "Tout",
    collection_default: "Ma collection",
    opt_group_artists: "Artiste",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Pays",
    opt_group_label: "Label",
    opt_sort_title: "Titre (A→Z)",
    opt_sort_artist: "Artiste (A→Z)",
    opt_sort_year_desc: "Année (plus récent d'abord)",
    opt_sort_label: "Label (A→Z)",
    opt_sort_date_added: "Date d'ajout (plus récent d'abord)",
    list_view_list: "☰ Liste",
    list_view_grid: "▦ Grille",
    list_prev: "‹ Préc.",
    list_next: "Suiv. ›",
    list_page_info: "Page {page} / {total}",
    col_title: "Titre",
    col_artists: "Artiste(s)",
    col_genres: "Genre(s)",
    col_format: "Format",
    col_year: "Année",
    col_tracks: "Pistes",
    detail_select: "Sélectionnez un album.",
    detail_edit: "Modifier…",
    detail_delete: "Supprimer…",
    detail_field_artists: "Artiste(s)",
    detail_field_composers: "Compositeur(s)",
    detail_field_conductors: "Chef(s) d'orchestre",
    detail_field_musicians: "Musicien(s)",
    detail_field_orchestras: "Orchestre(s)",
    detail_field_choruses: "Chœur(s)",
    detail_field_genres: "Genre(s)",
    detail_field_tags: "Étiquette(s)",
    detail_field_label: "Label",
    detail_field_format: "Format",
    detail_field_country: "Pays",
    detail_field_year: "Année",
    detail_field_rating: "Note",
    detail_field_notes: "Notes",
    detail_field_upc: "UPC",
    detail_field_date_added: "Date d'ajout",
    detail_disc: "Disque {n}",
    detail_tracks: "Pistes",
    delete_confirm: 'Supprimer « {title} » ? Cette action est irréversible (une sauvegarde est faite au préalable).',
    delete_failed: "Échec de la suppression : {err}",
    cover_set_failed: "Échec de la mise à jour de la pochette : {err}",
    form_title_add: "Ajouter un album",
    form_title_edit: "Modifier l'album",
    form_label_title: "Titre",
    form_label_artists: "Artiste(s), séparés par des virgules",
    form_label_composers: "Compositeur(s)",
    form_label_conductors: "Chef(s) d'orchestre",
    form_label_musicians: "Musicien(s)",
    form_label_orchestras: "Orchestre(s)",
    form_label_choruses: "Chœur(s)",
    form_label_genres: "Genre(s)",
    form_label_tags: "Étiquette(s)",
    form_label_extras: "Supplément(s)",
    form_label_studios: "Studio(s)",
    form_label_format: "Format",
    form_label_label: "Label",
    form_label_country: "Pays",
    form_label_year: "Année",
    form_label_rating: "Note",
    form_label_upc: "UPC",
    form_label_notes: "Notes",
    form_label_tracks: "Pistes, une par ligne",
    form_tracks_placeholder: "Un titre de piste par ligne",
    form_cancel: "Annuler",
    form_save: "Enregistrer",
    stats_title: "Statistiques — {name}",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} piste(s)",
    stats_no_cover: "{n} sans pochette",
    stats_add_cover_hint: "Cliquer pour ajouter une pochette",
    stats_no_artist: "{n} sans artiste",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Pays",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "Par décennie de sortie",
    stats_by_added_year: "Par année d'ajout",
    stats_no_data: "Aucune donnée.",
  },
};

let currentLang = "en";
try {
  const savedLang = localStorage.getItem("musicnojo_lang");
  if (savedLang === "fr" || savedLang === "en") currentLang = savedLang;
} catch {
  // Private window / blocked site data — keep the default.
}

function t(key, vars) {
  let s = (I18N[currentLang] && I18N[currentLang][key]) ?? I18N.en[key] ?? key;
  if (vars) for (const k of Object.keys(vars)) s = s.replaceAll(`{${k}}`, vars[k]);
  return s;
}

// Every static label that lives in the HTML (via data-i18n/data-i18n-placeholder)
// rather than being built by a render function — run once at load and again
// on every setLang().
function applyStaticLabels() {
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((el) => {
    el.placeholder = t(el.dataset.i18nPlaceholder);
  });
  statsBtnEl.textContent = panel === "stats" ? t("nav_back_to_collection") : t("nav_stats");
}

function initLangToggle() {
  const toggle = document.createElement("div");
  toggle.id = "lang-toggle";
  for (const lang of ["en", "fr"]) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.dataset.lang = lang;
    btn.textContent = lang.toUpperCase();
    btn.addEventListener("click", () => setLang(lang));
    toggle.appendChild(btn);
  }
  document.getElementById("app-version").insertAdjacentElement("afterend", toggle);
  updateLangToggleUI();
}

function updateLangToggleUI() {
  const toggle = document.getElementById("lang-toggle");
  if (!toggle) return;
  toggle.querySelectorAll("button").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.lang === currentLang);
  });
}

function setLang(lang) {
  if ((lang !== "en" && lang !== "fr") || lang === currentLang) return;
  currentLang = lang;
  try {
    localStorage.setItem("musicnojo_lang", lang);
  } catch {
    // Ignore — the choice just won't survive a relaunch.
  }
  updateLangToggleUI();
  applyStaticLabels();
  renderTabs();
  renderGroups();
  render(document.getElementById("search").value);
  if (currentDetailId !== null) showDetail(currentDetailId);
  if (panel === "stats") renderStats();
}

// --- Header badges --------------------------------------------------------
// Shows which collection is actually open, since currentPath starts out
// null (the bundled sample-collection) and only becomes concrete after
// list_albums's first response.
function updateFileBadge() {
  const el = document.getElementById("file-badge");
  if (!currentPath) {
    el.textContent = "";
    return;
  }
  const parts = currentPath.replace(/\/+$/, "").split("/").filter(Boolean);
  const file = parts[parts.length - 1] || currentPath;
  const dir = parts.length > 1 ? parts[parts.length - 2] : "";
  el.textContent = dir ? `📂 ${file} — ${dir}` : `📂 ${file}`;
  el.title = currentPath;
}

async function updateVersionBadge() {
  try {
    const version = await invoke("app_version");
    document.getElementById("app-version").textContent = `Ⓥ MusicNojo v${version}`;
  } catch {
    // Non-critical — leave the static placeholder label in place.
  }
}

// A stable (non-translated) key to filter/group by — an album with no
// Music Collector sub-collection set (ZCOLLECTION is nullable) still
// needs one to live under. Kept separate from the *displayed* label
// (collectionLabel below) so a language switch can't desync
// currentCollection from the just-retranslated label it used to match.
const DEFAULT_COLLECTION_KEY = "__default__";
function collectionKey(album) {
  return album.collection || DEFAULT_COLLECTION_KEY;
}
function collectionLabel(key) {
  return key === DEFAULT_COLLECTION_KEY ? t("collection_default") : key;
}

// One tab per sub-collection (Music Collector's own "Collections"
// feature — e.g. "My Music" plus any the user created), largest first,
// ties broken alphabetically — same ordering as BookNojo's own tabs.
// Recomputed once per load, not per render, since the set of
// sub-collections doesn't change just from filtering/sorting.
function computeCollections() {
  collCounts = new Map();
  for (const a of albums) {
    const key = collectionKey(a);
    collCounts.set(key, (collCounts.get(key) ?? 0) + 1);
  }
  collectionsList = [...collCounts.keys()].sort(
    (a, b) => collCounts.get(b) - collCounts.get(a) || collectionLabel(a).localeCompare(collectionLabel(b))
  );
  currentCollection = collectionsList[0] ?? null;
}

function renderTabs() {
  const el = document.getElementById("tabs");
  el.innerHTML = "";
  for (const key of collectionsList) {
    const tab = document.createElement("div");
    tab.className = "tab" + (key === currentCollection ? " active" : "");
    const label = document.createElement("span");
    label.textContent = collectionLabel(key);
    const n = document.createElement("span");
    n.className = "n";
    n.textContent = collCounts.get(key);
    tab.appendChild(label);
    tab.appendChild(n);
    tab.addEventListener("click", () => switchCollection(key));
    el.appendChild(tab);
  }
}

function switchCollection(name) {
  if (name === currentCollection) return;
  currentCollection = name;
  selectedGroup = null;
  selectedLetter = null;
  page = 1;
  const searchEl = document.getElementById("search");
  searchEl.value = "";
  renderTabs();
  renderGroups();
  render("");
  if (panel === "stats") renderStats();
}

// Scalar fields (format/country/label) and array fields (artists/
// genres) both need to behave like "this album belongs to these
// group values" — scalars are wrapped as a single-element list.
function groupValues(album, field) {
  const v = album[field];
  if (Array.isArray(v)) return v;
  return v ? [v] : [];
}

// First letter used by the A-Z quick-jump bar — non-letters (numbers,
// symbols, accented-but-not-matched edge cases) all bucket under "#".
function letterOf(value) {
  const ch = (value || "").trim().charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : "#";
}

function computeAvailableLetters() {
  const set = new Set();
  for (const a of albums) {
    if (collectionKey(a) !== currentCollection) continue;
    for (const v of groupValues(a, groupField)) {
      if (v) set.add(letterOf(v));
    }
  }
  return [...set].sort();
}

function renderAzBar() {
  const el = document.getElementById("azbar");
  el.innerHTML = "";
  const all = document.createElement("button");
  all.textContent = t("azbar_all");
  all.className = selectedLetter === null ? "active" : "";
  all.addEventListener("click", () => {
    selectedLetter = null;
    renderGroups();
  });
  el.appendChild(all);
  for (const letter of computeAvailableLetters()) {
    const btn = document.createElement("button");
    btn.textContent = letter;
    btn.className = selectedLetter === letter ? "active" : "";
    btn.addEventListener("click", () => {
      selectedLetter = letter;
      renderGroups();
    });
    el.appendChild(btn);
  }
}

// Narrows which group values are listed below (not the album list itself —
// picking a specific group from the narrowed list is still what filters
// the albums), same two-step behavior as BookNojo's own A-Z bar.
function renderGroups() {
  renderAzBar();
  const counts = new Map();
  let totalInCollection = 0;
  for (const a of albums) {
    if (collectionKey(a) !== currentCollection) continue;
    totalInCollection++;
    for (const v of groupValues(a, groupField)) {
      if (selectedLetter && letterOf(v) !== selectedLetter) continue;
      counts.set(v, (counts.get(v) ?? 0) + 1);
    }
  }
  const sorted = [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));

  const list = document.getElementById("group-list");
  list.innerHTML =
    `<div class="${selectedGroup === null ? "selected" : ""}" data-group="">
       <span>${escapeHtml(t("group_all"))}</span><span class="group-count">${totalInCollection}</span>
     </div>` +
    sorted
      .map(
        ([value, count]) => `<div class="${value === selectedGroup ? "selected" : ""}" data-group="${escapeHtml(value)}">
          <span>${escapeHtml(value)}</span><span class="group-count">${count}</span>
        </div>`
      )
      .join("");
  list.querySelectorAll("div").forEach((div) =>
    div.addEventListener("click", () => {
      selectedGroup = div.dataset.group || null;
      page = 1;
      renderGroups();
      render(document.getElementById("search").value);
    })
  );
}

document.getElementById("group-field").addEventListener("change", (e) => {
  groupField = e.target.value;
  selectedGroup = null;
  selectedLetter = null;
  page = 1;
  renderGroups();
  render(document.getElementById("search").value);
});

// The WebView's security sandbox blocks raw file:// loads, so cover
// images go through Tauri's asset protocol instead (only serves paths
// explicitly granted via asset_protocol_scope — see lib.rs).
function coverSrc(filename) {
  if (!filename || !imagesDir) return null;
  return window.__TAURI__.core.convertFileSrc(`${imagesDir}/${filename}`);
}

function escapeHtml(s) {
  return String(s ?? "").replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

const PAGE_SIZE = 60;
let page = 1;

function render(filterText) {
  const needle = filterText.trim().toLowerCase();
  const rows = albums.filter((a) => {
    const matchesCollection = collectionKey(a) === currentCollection;
    const matchesSearch =
      !needle ||
      a.title.toLowerCase().includes(needle) ||
      a.artists.join(" ").toLowerCase().includes(needle);
    const matchesGroup = selectedGroup === null || groupValues(a, groupField).includes(selectedGroup);
    return matchesCollection && matchesSearch && matchesGroup;
  });
  rows.sort((a, b) => {
    const va = Array.isArray(a[sortKey]) ? a[sortKey].join(", ") : a[sortKey];
    const vb = Array.isArray(b[sortKey]) ? b[sortKey].join(", ") : b[sortKey];
    const cmp =
      typeof va === "number" ? va - vb : String(va ?? "").localeCompare(String(vb ?? ""));
    return sortAsc ? cmp : -cmp;
  });

  const totalPages = Math.max(1, Math.ceil(rows.length / PAGE_SIZE));
  page = Math.min(Math.max(1, page), totalPages);
  const start = (page - 1) * PAGE_SIZE;
  const pageRows = rows.slice(start, start + PAGE_SIZE);

  document.getElementById("count").textContent = t("count_albums", { n: rows.length });
  const body = document.querySelector("#albums tbody");
  body.innerHTML = pageRows
    .map(
      (a) => `<tr data-id="${a.id}">
        <td>${escapeHtml(a.title)}</td>
        <td>${escapeHtml(a.artists.join(", "))}</td>
        <td>${escapeHtml(a.genres.join(", "))}</td>
        <td>${escapeHtml(a.format)}</td>
        <td>${escapeHtml(a.release_year)}</td>
        <td>${a.track_count}</td>
      </tr>`
    )
    .join("");
  body.querySelectorAll("tr").forEach((tr) =>
    tr.addEventListener("click", () => showDetail(Number(tr.dataset.id)))
  );

  renderGridView(pageRows);
  renderPager(totalPages);
}

function renderPager(totalPages) {
  document.getElementById("page-info").textContent = t("list_page_info", { page, total: totalPages });
  document.getElementById("prev-page").disabled = page <= 1;
  document.getElementById("next-page").disabled = page >= totalPages;
}

function renderGridView(rows) {
  const grid = document.getElementById("grid-view");
  grid.innerHTML = rows
    .map((a) => {
      const src = coverSrc(a.front_cover);
      const thumb = src
        ? `<img class="thumb" loading="lazy" src="${src}" alt="">`
        : `<div class="thumb"></div>`;
      return `<div class="grid-tile" data-id="${a.id}">
        ${thumb}
        <div class="t">${escapeHtml(a.title)}</div>
        <div class="a">${escapeHtml(a.artists.join(", "))}</div>
      </div>`;
    })
    .join("");
  grid.querySelectorAll(".grid-tile").forEach((tile) =>
    tile.addEventListener("click", () => showDetail(Number(tile.dataset.id)))
  );
}

function roleList(label, names) {
  return names && names.length ? `<dt>${label}</dt><dd>${escapeHtml(names.join(", "))}</dd>` : "";
}

function field(label, value) {
  return value ? `<dt>${label}</dt><dd>${escapeHtml(value)}</dd>` : "";
}

function showDetail(id) {
  currentDetailId = id;
  const a = albums.find((x) => x.id === id);
  if (!a) return;

  const discs = a.discs
    .map((disc, i) => {
      const heading = disc.index
        ? t("detail_disc", { n: escapeHtml(disc.index) })
        : a.discs.length > 1
        ? t("detail_disc", { n: i + 1 })
        : t("detail_tracks");
      const tracks = disc.tracks
        .map((tr) => `<li>${escapeHtml(tr.title)}${tr.length ? ` — ${escapeHtml(tr.length)}` : ""}</li>`)
        .join("");
      return `<div class="disc-heading">${heading}</div><ol>${tracks}</ol>`;
    })
    .join("");

  const front = coverSrc(a.front_cover);
  const cover = front ? `<img class="cover" src="${front}" alt="">` : "";

  document.getElementById("detail-content").innerHTML = `
    ${cover}
    <h2>${escapeHtml(a.title)}</h2>
    <dl>
      ${roleList(t("detail_field_artists"), a.artists)}
      ${roleList(t("detail_field_composers"), a.composers)}
      ${roleList(t("detail_field_conductors"), a.conductors)}
      ${roleList(t("detail_field_musicians"), a.musicians)}
      ${roleList(t("detail_field_orchestras"), a.orchestras)}
      ${roleList(t("detail_field_choruses"), a.choruses)}
      ${roleList(t("detail_field_genres"), a.genres)}
      ${roleList(t("detail_field_tags"), a.tags)}
      ${field(t("detail_field_label"), a.label)}
      ${field(t("detail_field_format"), a.format)}
      ${field(t("detail_field_country"), a.country)}
      ${field(t("detail_field_year"), a.release_year)}
      ${field(t("detail_field_rating"), a.rating != null ? String(a.rating) : "")}
      ${field(t("detail_field_notes"), a.notes)}
      ${field(t("detail_field_upc"), a.upc)}
      ${field(t("detail_field_date_added"), a.date_added)}
    </dl>
    ${discs}
  `;
  document.getElementById("detail-actions").hidden = false;
}

function clearDetail() {
  currentDetailId = null;
  document.getElementById("detail-content").innerHTML =
    `<p id="detail-empty">${escapeHtml(t("detail_select"))}</p>`;
  document.getElementById("detail-actions").hidden = true;
}

document.getElementById("search").addEventListener("input", (e) => {
  page = 1;
  render(e.target.value);
});

// A handful of named presets for the toolbar dropdown — a quicker path to
// the sort orders people actually reach for, without giving up the
// column-header click/toggle that already covers every field.
const SORT_PRESETS = {
  title: { key: "title", asc: true },
  artist: { key: "artists", asc: true },
  year_desc: { key: "release_year", asc: false },
  label: { key: "label", asc: true },
  date_added_desc: { key: "date_added", asc: false },
};

const sortEl = document.getElementById("sort");

function syncSortDropdown() {
  const match = Object.entries(SORT_PRESETS).find(
    ([, p]) => p.key === sortKey && p.asc === sortAsc
  );
  if (match) sortEl.value = match[0];
}

sortEl.addEventListener("change", () => {
  const preset = SORT_PRESETS[sortEl.value];
  sortKey = preset.key;
  sortAsc = preset.asc;
  page = 1;
  render(document.getElementById("search").value);
});

document.querySelectorAll("th").forEach((th) =>
  th.addEventListener("click", () => {
    const k = th.dataset.k;
    sortAsc = sortKey === k ? !sortAsc : true;
    sortKey = k;
    page = 1;
    syncSortDropdown();
    render(document.getElementById("search").value);
  })
);

document.getElementById("prev-page").addEventListener("click", () => {
  page--;
  render(document.getElementById("search").value);
});
document.getElementById("next-page").addEventListener("click", () => {
  page++;
  render(document.getElementById("search").value);
});

const viewListBtn = document.getElementById("view-list");
const viewGridBtn = document.getElementById("view-grid");
const listPaneForView = document.getElementById("list-pane");

function setViewMode(mode) {
  viewMode = mode;
  listPaneForView.classList.toggle("view-grid", mode === "grid");
  viewListBtn.classList.toggle("active", mode === "list");
  viewGridBtn.classList.toggle("active", mode === "grid");
}

viewListBtn.addEventListener("click", () => setViewMode("list"));
viewGridBtn.addEventListener("click", () => setViewMode("grid"));

async function loadCollection(path) {
  try {
    const resp = await invoke("list_albums", { path });
    albums = resp.albums;
    imagesDir = resp.images_dir;
    // The command's own echoed-back path, never null — add_album/
    // edit_album/delete_album require a concrete path, and the initial
    // load passes path=null (meaning "use the bundled sample-collection"),
    // which resp.path resolves to a real path.
    currentPath = resp.path;
    updateFileBadge();
    clearDetail();
    selectedGroup = null;
    selectedLetter = null;
    page = 1;
    computeCollections();
    renderTabs();
    renderGroups();
    render(document.getElementById("search").value);
    if (panel === "stats") renderStats();
  } catch (err) {
    document.getElementById("count").textContent = `Error: ${err}`;
  }
}

window.__TAURI__.event.listen("collection-picked", (event) => {
  loadCollection(event.payload);
});

document.getElementById("open-collection").addEventListener("click", () => {
  // Fire-and-forget: the dialog is async on the Rust side, and its
  // result arrives later as a "collection-picked" event (see above),
  // not as this invoke()'s return value.
  invoke("pick_collection");
});

window.__TAURI__.event.listen("cover-set", () => {
  // Reloads the whole collection (simplest way to pick up the new
  // front_cover everywhere it's shown — table/grid thumbnails, detail
  // panel, the stats "no cover" list/count itself) rather than patching
  // just the one album in local state.
  loadCollection(currentPath);
});

window.__TAURI__.event.listen("cover-set-error", (event) => {
  alert(t("cover_set_failed", { err: event.payload }));
});

// --- Statistics ----------------------------------------------------------
// Mirrors BookNojo's own computeStats()/renderStats()/barList() pattern,
// adapted to Album's fields instead of Book's.
function computeStats() {
  const scoped = albums.filter((a) => collectionKey(a) === currentCollection);
  const tally = (getter) => {
    const m = new Map();
    for (const a of scoped) {
      for (const v of getter(a)) {
        if (v) m.set(v, (m.get(v) || 0) + 1);
      }
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  };
  const decades = new Map();
  for (const a of scoped) {
    const y = parseInt(a.release_year, 10);
    if (!y) continue;
    const d = Math.floor(y / 10) * 10;
    decades.set(d, (decades.get(d) || 0) + 1);
  }
  const addedYears = new Map();
  for (const a of scoped) {
    if (!a.date_added) continue;
    const y = a.date_added.slice(0, 4);
    addedYears.set(y, (addedYears.get(y) || 0) + 1);
  }
  const noCoverAlbums = scoped
    .filter((a) => !a.front_cover)
    .sort((a, b) => a.title.localeCompare(b.title));
  const noArtistAlbums = scoped
    .filter((a) => !(a.artists && a.artists.length))
    .sort((a, b) => a.title.localeCompare(b.title));
  return {
    totalAlbums: scoped.length,
    totalTracks: scoped.reduce((s, a) => s + (a.track_count || 0), 0),
    noCover: noCoverAlbums.length,
    noArtist: noArtistAlbums.length,
    noCoverAlbums,
    noArtistAlbums,
    genres: tally((a) => a.genres || []),
    formats: tally((a) => (a.format ? [a.format] : [])),
    countries: tally((a) => (a.country ? [a.country] : [])),
    labels: tally((a) => (a.label ? [a.label] : [])),
    decades: [...decades.entries()].sort((a, b) => a[0] - b[0]).map(([d, c]) => [`${d}s`, c]),
    addedYears: [...addedYears.entries()].sort((a, b) => a[0].localeCompare(b[0])),
  };
}

function barList(title, entries, limit) {
  const top = limit ? entries.slice(0, limit) : entries;
  const max = top.reduce((m, [, c]) => Math.max(m, c), 1);
  const rows = top
    .map(
      ([label, count]) => `
    <div class="stat-row">
      <span class="label">${escapeHtml(label)}</span>
      <span class="bar-wrap"><span class="bar" style="width:${((count / max) * 100).toFixed(1)}%"></span></span>
      <span class="val">${count}</span>
    </div>`
    )
    .join("");
  return `<div class="stat-block"><h3>${escapeHtml(title)}</h3>${rows || `<p class="stat-empty">${escapeHtml(t("stats_no_data"))}</p>`}</div>`;
}

// Which "N with no cover"/"N with no artist" summary items are
// currently expanded in the stats panel — a plain module-level Set
// rather than per-call state, so it survives the full-innerHTML
// re-render renderStats() does on every toggle/language switch/tab
// change.
const statsExpanded = new Set();

// `clickableForCover`: the "no cover" list's entries double as a
// shortcut into pick_and_set_cover (user asked for exactly this — "ça
// serait pratique pour ajouter facilement la pochette manquante"); the
// "no artist" list reuses the same renderer but stays plain, since
// there's no equivalent one-click action for a missing artist.
function albumListHtml(albums, clickableForCover) {
  if (!albums.length) return `<p class="stat-empty">${escapeHtml(t("stats_no_data"))}</p>`;
  return `<ul class="stat-detail-list">${albums
    .map((a) => {
      const label = `${escapeHtml(a.artists.join(", ") || "—")} — ${escapeHtml(a.title)}`;
      return clickableForCover
        ? `<li class="clickable" data-album-id="${a.id}" title="${escapeHtml(t("stats_add_cover_hint"))}">${label}</li>`
        : `<li>${label}</li>`;
    })
    .join("")}</ul>`;
}

// A summary item whose count can be clicked to expand/collapse an
// Artist — Title list of the matching albums, right below the summary
// row (user asked specifically for "sans pochette" to do this; "sans
// artiste" gets the same treatment for consistency, same mechanism).
function statLink(key, labelKey, n) {
  const expanded = statsExpanded.has(key);
  return `<span class="stat-link${expanded ? " expanded" : ""}" data-stat-key="${key}">${t(labelKey, { n: `<strong>${n}</strong>` })}</span>`;
}

function renderStats() {
  const s = computeStats();
  const detailBlock = (key, labelKey, n, albums, clickableForCover) => {
    if (!statsExpanded.has(key)) return "";
    return `<div class="stat-detail"><h4>${escapeHtml(t(labelKey, { n }))}</h4>${albumListHtml(albums, clickableForCover)}</div>`;
  };
  document.getElementById("stats-pane").innerHTML = `
    <div class="stats-wrap">
      <h2>${escapeHtml(t("stats_title", { name: collectionLabel(currentCollection) }))}</h2>
      <div class="stat-summary">
        <span>${t("stats_albums", { n: `<strong>${s.totalAlbums}</strong>` })}</span>
        <span>${t("stats_tracks", { n: `<strong>${s.totalTracks}</strong>` })}</span>
        ${statLink("noCover", "stats_no_cover", s.noCover)}
        ${statLink("noArtist", "stats_no_artist", s.noArtist)}
      </div>
      ${detailBlock("noCover", "stats_no_cover", s.noCover, s.noCoverAlbums, true)}
      ${detailBlock("noArtist", "stats_no_artist", s.noArtist, s.noArtistAlbums, false)}
      <div class="stat-grid">
        ${barList(t("stats_genres"), s.genres, 15)}
        ${barList(t("stats_formats"), s.formats)}
        ${barList(t("stats_countries"), s.countries)}
        ${barList(t("stats_labels_top10"), s.labels, 10)}
        ${barList(t("stats_by_decade"), s.decades)}
        ${barList(t("stats_by_added_year"), s.addedYears)}
      </div>
    </div>`;
  document.querySelectorAll(".stat-link").forEach((el) =>
    el.addEventListener("click", () => {
      const key = el.dataset.statKey;
      if (statsExpanded.has(key)) statsExpanded.delete(key);
      else statsExpanded.add(key);
      renderStats();
    })
  );
  document.querySelectorAll(".stat-detail-list li.clickable").forEach((el) =>
    el.addEventListener("click", () => {
      // Fire-and-forget: the picker is async on the Rust side, result
      // arrives later as a "cover-set"/"cover-set-error" event.
      invoke("pick_and_set_cover", { path: currentPath, id: Number(el.dataset.albumId) });
    })
  );
}

const statsBtnEl = document.getElementById("stats-btn");
function setPanel(next) {
  panel = next;
  document.body.classList.toggle("panel-stats", panel === "stats");
  statsBtnEl.classList.toggle("active", panel === "stats");
  statsBtnEl.textContent = panel === "stats" ? t("nav_back_to_collection") : t("nav_stats");
  if (panel === "stats") renderStats();
}
statsBtnEl.addEventListener("click", () => {
  setPanel(panel === "stats" ? "library" : "stats");
});

// --- Add/Edit/Delete -------------------------------------------------

const dialog = document.getElementById("album-form-dialog");
const form = document.getElementById("album-form");
const formError = document.getElementById("album-form-error");

function splitList(value) {
  return value
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
}

function emptyRoleInputs() {
  return {
    artist: [],
    composer: [],
    conductor: [],
    musician: [],
    orchestra: [],
    chorus: [],
  };
}

function emptyManyToManyInputs() {
  return { genre: [], tag: [], extra: [], studio: [] };
}

function emptyAlbumFields() {
  return {
    format: null,
    label: null,
    country: null,
    condition: null,
    packaging: null,
    vinyl_color: null,
    spars: null,
    owner: null,
    location: null,
    release_year: null,
    rating: null,
    notes: null,
    upc: null,
  };
}

function openAddForm() {
  form.reset();
  document.getElementById("album-form-title").textContent = t("form_title_add");
  form.dataset.mode = "add";
  formError.textContent = "";
  dialog.showModal();
}

function openEditForm(a) {
  form.reset();
  document.getElementById("album-form-title").textContent = t("form_title_edit");
  form.dataset.mode = "edit";
  form.dataset.id = a.id;
  formError.textContent = "";
  form.elements.title.value = a.title;
  form.elements.artists.value = a.artists.join(", ");
  form.elements.composers.value = a.composers.join(", ");
  form.elements.conductors.value = a.conductors.join(", ");
  form.elements.musicians.value = a.musicians.join(", ");
  form.elements.orchestras.value = a.orchestras.join(", ");
  form.elements.choruses.value = a.choruses.join(", ");
  form.elements.genres.value = a.genres.join(", ");
  form.elements.tags.value = a.tags.join(", ");
  form.elements.extras.value = a.extras.join(", ");
  form.elements.studios.value = a.studios.join(", ");
  form.elements.format.value = a.format ?? "";
  form.elements.label.value = a.label ?? "";
  form.elements.country.value = a.country ?? "";
  form.elements.release_year.value = a.release_year ?? "";
  form.elements.rating.value = a.rating ?? "";
  form.elements.upc.value = a.upc ?? "";
  form.elements.notes.value = a.notes ?? "";
  form.elements.tracks.value = a.discs.flatMap((d) => d.tracks.map((tr) => tr.title)).join("\n");
  dialog.showModal();
}

document.getElementById("add-album").addEventListener("click", openAddForm);
document.getElementById("album-form-cancel").addEventListener("click", () => dialog.close());

document.getElementById("detail-edit").addEventListener("click", () => {
  const a = albums.find((x) => x.id === currentDetailId);
  if (a) openEditForm(a);
});

document.getElementById("detail-delete").addEventListener("click", async () => {
  const a = albums.find((x) => x.id === currentDetailId);
  if (!a) return;
  if (!confirm(t("delete_confirm", { title: a.title }))) return;
  try {
    await invoke("delete_album", { path: currentPath, id: a.id });
    clearDetail();
    await loadCollection(currentPath);
  } catch (err) {
    alert(t("delete_failed", { err }));
  }
});

form.addEventListener("submit", async (e) => {
  // method="dialog" closes the dialog natively on submit unless this
  // runs first — we want to stay open on failure (to show the error).
  e.preventDefault();
  formError.textContent = "";
  const data = new FormData(form);
  const roles = emptyRoleInputs();
  roles.artist = splitList(data.get("artists"));
  roles.composer = splitList(data.get("composers"));
  roles.conductor = splitList(data.get("conductors"));
  roles.musician = splitList(data.get("musicians"));
  roles.orchestra = splitList(data.get("orchestras"));
  roles.chorus = splitList(data.get("choruses"));

  const many = emptyManyToManyInputs();
  many.genre = splitList(data.get("genres"));
  many.tag = splitList(data.get("tags"));
  many.extra = splitList(data.get("extras"));
  many.studio = splitList(data.get("studios"));

  const fields = emptyAlbumFields();
  fields.format = data.get("format") || null;
  fields.label = data.get("label") || null;
  fields.country = data.get("country") || null;
  fields.release_year = data.get("release_year") || null;
  fields.rating = data.get("rating") ? Number(data.get("rating")) : null;
  fields.upc = data.get("upc") || null;
  fields.notes = data.get("notes") || null;

  const tracks = data
    .get("tracks")
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  try {
    if (form.dataset.mode === "edit") {
      const id = Number(form.dataset.id);
      await invoke("edit_album", {
        path: currentPath,
        id,
        title: data.get("title"),
        fields,
        replace: {
          replace_artists: true,
          replace_composers: true,
          replace_conductors: true,
          replace_musicians: true,
          replace_orchestras: true,
          replace_choruses: true,
        },
        roles,
        replaceMany: {
          replace_genres: true,
          replace_tags: true,
          replace_extras: true,
          replace_studios: true,
        },
        many,
      });
    } else {
      await invoke("add_album", {
        path: currentPath,
        title: data.get("title"),
        roles,
        many,
        fields,
        tracks,
      });
    }
    dialog.close();
    clearDetail();
    await loadCollection(currentPath);
  } catch (err) {
    formError.textContent = String(err);
  }
});

// --- Draggable splitters (groups|rest, and list|detail) ---------------
// Same approach as BookNojo's own app: a drag handle tracks mousemove
// on the window (not just the handle) so a fast drag doesn't lose the
// pointer, and persists sizes to localStorage as a per-viewer
// convenience only (never read back by anything else).
const MIN_GROUPS_W = 120;
const MAX_GROUPS_W = 420;
const MIN_LIST_H = 100;
const MIN_DETAIL_H = 120;

const mainEl = document.getElementById("main");
const groupsEl = document.getElementById("groups");
const rightColEl = document.getElementById("right-col");
const listPaneEl = document.getElementById("list-pane");
const splitterV = document.getElementById("splitter-v");
const splitterH = document.getElementById("splitter-h");

function makeDraggable(handle, onDrag) {
  let dragging = false;
  handle.addEventListener("mousedown", (e) => {
    dragging = true;
    handle.classList.add("dragging");
    document.body.style.userSelect = "none";
    e.preventDefault();
  });
  window.addEventListener("mousemove", (e) => {
    if (dragging) onDrag(e);
  });
  window.addEventListener("mouseup", () => {
    if (!dragging) return;
    dragging = false;
    handle.classList.remove("dragging");
    document.body.style.userSelect = "";
    saveLayout();
  });
}

makeDraggable(splitterV, (e) => {
  const rect = mainEl.getBoundingClientRect();
  const w = Math.max(MIN_GROUPS_W, Math.min(MAX_GROUPS_W, e.clientX - rect.left));
  groupsEl.style.width = `${w}px`;
});

makeDraggable(splitterH, (e) => {
  const rect = rightColEl.getBoundingClientRect();
  const maxH = rect.height - MIN_DETAIL_H - splitterH.offsetHeight;
  const h = Math.max(MIN_LIST_H, Math.min(maxH, e.clientY - rect.top));
  listPaneEl.style.flex = `0 0 ${h}px`;
});

function saveLayout() {
  try {
    localStorage.setItem(
      "musicnojo_layout",
      JSON.stringify({ groupsWidth: groupsEl.style.width, listHeight: listPaneEl.style.flex })
    );
  } catch {
    // Private window / blocked site data — keep defaults, not fatal.
  }
}

function restoreLayout() {
  try {
    const saved = JSON.parse(localStorage.getItem("musicnojo_layout") || "{}");
    if (saved.groupsWidth) groupsEl.style.width = saved.groupsWidth;
    if (saved.listHeight) listPaneEl.style.flex = saved.listHeight;
  } catch {
    // Ignore a corrupt/missing value — defaults from CSS still apply.
  }
}

applyStaticLabels();
initLangToggle();
updateVersionBadge();
restoreLayout();
loadCollection(null);
