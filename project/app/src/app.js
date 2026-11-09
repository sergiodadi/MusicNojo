const { invoke } = window.__TAURI__.core;

let albums = [];
let sortKey = "title";
let sortAsc = true;
let imagesDir = "";
let currentPath = null;
let currentDetailId = null;
let groupField = "artists";
let selectedGroup = null;

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
       <span>All</span><span class="group-count">${albums.length}</span>
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

  document.getElementById("count").textContent = `${rows.length} album(s)`;
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
        ? `Disc ${escapeHtml(disc.index)}`
        : a.discs.length > 1
        ? `Disc ${i + 1}`
        : "Tracks";
      const tracks = disc.tracks
        .map((t) => `<li>${escapeHtml(t.title)}${t.length ? ` — ${escapeHtml(t.length)}` : ""}</li>`)
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
      ${roleList("Artist(s)", a.artists)}
      ${roleList("Composer(s)", a.composers)}
      ${roleList("Conductor(s)", a.conductors)}
      ${roleList("Musician(s)", a.musicians)}
      ${roleList("Orchestra(s)", a.orchestras)}
      ${roleList("Chorus(es)", a.choruses)}
      ${roleList("Genre(s)", a.genres)}
      ${roleList("Tag(s)", a.tags)}
      ${field("Label", a.label)}
      ${field("Format", a.format)}
      ${field("Country", a.country)}
      ${field("Year", a.release_year)}
      ${field("Rating", a.rating != null ? String(a.rating) : "")}
      ${field("Notes", a.notes)}
      ${field("UPC", a.upc)}
    </dl>
    ${discs}
  `;
  document.getElementById("detail").hidden = false;
}

document.getElementById("detail-close").addEventListener("click", () => {
  document.getElementById("detail").hidden = true;
});

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
    document.getElementById("detail").hidden = true;
    selectedGroup = null;
    renderGroups();
    render(document.getElementById("search").value);
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
  document.getElementById("album-form-title").textContent = "Add Album";
  form.dataset.mode = "add";
  formError.textContent = "";
  dialog.showModal();
}

function openEditForm(a) {
  form.reset();
  document.getElementById("album-form-title").textContent = "Edit Album";
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
  form.elements.tracks.value = a.discs.flatMap((d) => d.tracks.map((t) => t.title)).join("\n");
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
  if (!confirm(`Delete "${a.title}"? This cannot be undone (a backup is made first).`)) return;
  try {
    await invoke("delete_album", { path: currentPath, id: a.id });
    document.getElementById("detail").hidden = true;
    await loadCollection(currentPath);
  } catch (err) {
    alert(`Delete failed: ${err}`);
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
    document.getElementById("detail").hidden = true;
    await loadCollection(currentPath);
  } catch (err) {
    formError.textContent = String(err);
  }
});

loadCollection(null);
