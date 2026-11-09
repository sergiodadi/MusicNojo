const { invoke } = window.__TAURI__.core;

let albums = [];
let sortKey = "title";
let sortAsc = true;
let currentPath = null;

function escapeHtml(s) {
  return String(s ?? "").replace(
    /[&<>"']/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c])
  );
}

function render(filterText) {
  const needle = filterText.trim().toLowerCase();
  const rows = albums.filter(
    (a) =>
      !needle ||
      a.title.toLowerCase().includes(needle) ||
      a.artists.join(" ").toLowerCase().includes(needle)
  );
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

  document.getElementById("detail-content").innerHTML = `
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
    albums = await invoke("list_albums", { path });
    currentPath = path;
    document.getElementById("detail").hidden = true;
    render(document.getElementById("search").value);
  } catch (err) {
    document.getElementById("count").textContent = `Error: ${err}`;
  }
}

document.getElementById("open-collection").addEventListener("click", async () => {
  const path = await invoke("pick_collection");
  if (path) {
    await loadCollection(path);
  }
});

loadCollection(null);
