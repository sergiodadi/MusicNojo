const { invoke } = window.__TAURI__.core;

let albums = [];
let sortKey = "title";
let sortAsc = true;

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
      (a) => `<tr>
        <td>${escapeHtml(a.title)}</td>
        <td>${escapeHtml(a.artists.join(", "))}</td>
        <td>${escapeHtml(a.genres.join(", "))}</td>
        <td>${escapeHtml(a.format ?? "")}</td>
        <td>${escapeHtml(a.release_year ?? "")}</td>
        <td>${a.track_count}</td>
      </tr>`
    )
    .join("");
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
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

invoke("list_albums")
  .then((result) => {
    albums = result;
    render("");
  })
  .catch((err) => {
    document.getElementById("count").textContent = `Error: ${err}`;
  });
