const { invoke } = window.__TAURI__.core;

let albums = [];
let sortKey = "title";
let sortAsc = true;
let imagesDir = "";
let currentPath = null;
let currentDetailId = null;
let groupField = "artists";
let selectedGroup = null;
let panel = "library"; // 'library' | 'stats'

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
    opt_group_artists: "Artist",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Country",
    opt_group_label: "Label",
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
    detail_disc: "Disc {n}",
    detail_tracks: "Tracks",
    delete_confirm: 'Delete "{title}"? This cannot be undone (a backup is made first).',
    delete_failed: "Delete failed: {err}",
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
    stats_title: "Statistics",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} track(s)",
    stats_no_cover: "{n} with no cover",
    stats_no_artist: "{n} with no artist",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Countries",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "By release decade",
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
    opt_group_artists: "Artiste",
    opt_group_genres: "Genre",
    opt_group_format: "Format",
    opt_group_country: "Pays",
    opt_group_label: "Label",
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
    detail_disc: "Disque {n}",
    detail_tracks: "Pistes",
    delete_confirm: 'Supprimer « {title} » ? Cette action est irréversible (une sauvegarde est faite au préalable).',
    delete_failed: "Échec de la suppression : {err}",
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
    stats_title: "Statistiques",
    stats_albums: "{n} album(s)",
    stats_tracks: "{n} piste(s)",
    stats_no_cover: "{n} sans pochette",
    stats_no_artist: "{n} sans artiste",
    stats_genres: "Genres",
    stats_formats: "Formats",
    stats_countries: "Pays",
    stats_labels_top10: "Labels (top 10)",
    stats_by_decade: "Par décennie de sortie",
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

// Scalar fields (format/country/label) and array fields (artists/
// genres) both need to behave like "this album belongs to these
// group values" — scalars are wrapped as a single-element list.
function groupValues(album, field) {
  const v = album[field];
  if (Array.isArray(v)) return v;
  return v ? [v] : [];
}

function renderGroups() {
  const counts = new Map();
  for (const a of albums) {
    for (const v of groupValues(a, groupField)) {
      counts.set(v, (counts.get(v) ?? 0) + 1);
    }
  }
  const sorted = [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]));

  const list = document.getElementById("group-list");
  list.innerHTML =
    `<div class="${selectedGroup === null ? "selected" : ""}" data-group="">
       <span>${escapeHtml(t("group_all"))}</span><span class="group-count">${albums.length}</span>
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
      renderGroups();
      render(document.getElementById("search").value);
    })
  );
}

document.getElementById("group-field").addEventListener("change", (e) => {
  groupField = e.target.value;
  selectedGroup = null;
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

function render(filterText) {
  const needle = filterText.trim().toLowerCase();
  const rows = albums.filter((a) => {
    const matchesSearch =
      !needle ||
      a.title.toLowerCase().includes(needle) ||
      a.artists.join(" ").toLowerCase().includes(needle);
    const matchesGroup = selectedGroup === null || groupValues(a, groupField).includes(selectedGroup);
    return matchesSearch && matchesGroup;
  });
  rows.sort((a, b) => {
    const va = Array.isArray(a[sortKey]) ? a[sortKey].join(", ") : a[sortKey];
    const vb = Array.isArray(b[sortKey]) ? b[sortKey].join(", ") : b[sortKey];
    const cmp =
      typeof va === "number" ? va - vb : String(va ?? "").localeCompare(String(vb ?? ""));
    return sortAsc ? cmp : -cmp;
  });

  document.getElementById("count").textContent = t("count_albums", { n: rows.length });
  const body = document.querySelector("#albums tbody");
  body.innerHTML = rows
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

document.getElementById("search").addEventListener("input", (e) => render(e.target.value));
document.querySelectorAll("th").forEach((th) =>
  th.addEventListener("click", () => {
    const k = th.dataset.k;
    sortAsc = sortKey === k ? !sortAsc : true;
    sortKey = k;
    render(document.getElementById("search").value);
  })
);

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

// --- Statistics ----------------------------------------------------------
// Mirrors BookNojo's own computeStats()/renderStats()/barList() pattern,
// adapted to Album's fields instead of Book's.
function computeStats() {
  const tally = (getter) => {
    const m = new Map();
    for (const a of albums) {
      for (const v of getter(a)) {
        if (v) m.set(v, (m.get(v) || 0) + 1);
      }
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  };
  const decades = new Map();
  for (const a of albums) {
    const y = parseInt(a.release_year, 10);
    if (!y) continue;
    const d = Math.floor(y / 10) * 10;
    decades.set(d, (decades.get(d) || 0) + 1);
  }
  return {
    totalAlbums: albums.length,
    totalTracks: albums.reduce((s, a) => s + (a.track_count || 0), 0),
    noCover: albums.filter((a) => !a.front_cover).length,
    noArtist: albums.filter((a) => !(a.artists && a.artists.length)).length,
    genres: tally((a) => a.genres || []),
    formats: tally((a) => (a.format ? [a.format] : [])),
    countries: tally((a) => (a.country ? [a.country] : [])),
    labels: tally((a) => (a.label ? [a.label] : [])),
    decades: [...decades.entries()].sort((a, b) => a[0] - b[0]).map(([d, c]) => [`${d}s`, c]),
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

function renderStats() {
  const s = computeStats();
  document.getElementById("stats-pane").innerHTML = `
    <div class="stats-wrap">
      <h2>${escapeHtml(t("stats_title"))}</h2>
      <div class="stat-summary">
        <span>${t("stats_albums", { n: `<strong>${s.totalAlbums}</strong>` })}</span>
        <span>${t("stats_tracks", { n: `<strong>${s.totalTracks}</strong>` })}</span>
        <span>${t("stats_no_cover", { n: `<strong>${s.noCover}</strong>` })}</span>
        <span>${t("stats_no_artist", { n: `<strong>${s.noArtist}</strong>` })}</span>
      </div>
      <div class="stat-grid">
        ${barList(t("stats_genres"), s.genres, 15)}
        ${barList(t("stats_formats"), s.formats)}
        ${barList(t("stats_countries"), s.countries)}
        ${barList(t("stats_labels_top10"), s.labels, 10)}
        ${barList(t("stats_by_decade"), s.decades)}
      </div>
    </div>`;
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
