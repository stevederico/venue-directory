use crate::json::{stringify, Value};
use crate::venue::Venue;

pub fn html_home(venues: &[Venue]) -> String {
    let payload = Value::Array(venues.iter().map(Venue::to_json).collect());
    let json = stringify(&payload).replace('<', "\\u003c");
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Venue Directory</title>
<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css">
<style>{css}</style>
</head>
<body class="view-map">
<a class="skip" href="#list">Skip To List</a>
<header class="bar">
  <h1 class="skip">Venue Directory</h1>
  <form class="search" role="search" action="/" method="get" onsubmit="return false">
    <label class="skip" for="q">Search Venues</label>
    <input id="q" name="q" type="search" placeholder="Search Venues" autocomplete="off">
  </form>
</header>
<nav class="filters" aria-label="Status">
  <button type="button" class="chip on" data-bucket="all">All</button>
  <button type="button" class="chip" data-bucket="Ask">Ask</button>
  <button type="button" class="chip" data-bucket="Contacted">Contacted</button>
  <button type="button" class="chip" data-bucket="Low">Low</button>
  <button type="button" class="chip" data-bucket="Out">Out</button>
  <button type="button" class="chip" data-bucket="Needs Review">Needs Review</button>
  <p id="count" class="count"></p>
  <div class="views" role="group" aria-label="View Mode">
    <button type="button" class="chip on" data-view="map">Map</button>
    <button type="button" class="chip" data-view="list">List</button>
  </div>
</nav>
<div class="stage">
<main id="list">
  <ol id="rows" class="cards"></ol>
</main>
<div id="map" role="region" aria-label="Map of venues"></div>
<aside id="detail" aria-label="Venue Details">
  <button type="button" id="detail-close" class="chip">Close</button>
  <div id="detail-body"></div>
</aside>
</div>
<footer class="foot">
  <p>Map tiles © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>. API: <a href="/api/venues">/api/venues</a>.</p>
</footer>
<script>const VENUES = {json};</script>
<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
<script>{js}</script>
</body>
</html>
"##,
        css = CSS,
        js = JS,
        json = json
    )
}

pub fn html_venue(v: &Venue) -> String {
    let contact = html_contact(v);
    let geo = match (v.lat, v.lng) {
        (Some(lat), Some(lng)) => format!("<p class=\"meta\">{lat:.5}, {lng:.5}</p>"),
        _ => String::new(),
    };
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{place} · Venue Directory</title>
<style>{css}</style>
</head>
<body class="detail">
<a class="skip" href="#main">Skip To Content</a>
<header class="top">
  <p><a href="/">Venue Directory</a></p>
  <h1>{place}</h1>
  <p class="tag">{address}</p>
  {geo}
</header>
<main id="main" class="sheet">
  {status_row}
  {maps}
  {row_screen}
  {row_cap}
  {row_section}
  {row_notes}
  {contact}
  <p class="agents">Agents: <a href="/v/{slug}.md">Markdown</a> · <a href="/api/venues/{slug}">JSON</a> · <a href="/api/venues/{slug}/contact">Contact</a></p>
</main>
</body>
</html>
"##,
        css = CSS,
        place = esc(&v.place),
        address = esc(&v.address),
        geo = geo,
        status_row = status_row(v),
        maps = maps_link(v),
        row_screen = field("Screen", &v.screen),
        row_cap = field("Capacity", &v.capacity),
        row_section = field("Section", &v.section),
        row_notes = field("Notes", &v.notes),
        contact = contact,
        slug = esc(&v.slug),
    )
}

fn html_contact(v: &Venue) -> String {
    let c = &v.contact;
    if c.raw.is_empty() {
        return "<section><h2>Contact</h2><p>None on file.</p></section>".into();
    }
    let mut s = String::from("<section><h2>Contact</h2>");
    s.push_str(&format!("<p class=\"raw\">{}</p>", esc(&c.raw)));
    s.push_str(&links("Emails", "mailto:", &c.emails));
    s.push_str(&links("Phones", "tel:", &c.phones));
    s.push_str(&links("X", "", &c.x));
    s.push_str(&links("URLs", "", &c.urls));
    s.push_str("</section>");
    s
}

fn links(title: &str, prefix: &str, items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut s = format!("<h3>{title}</h3><ul>");
    for item in items {
        let href = if prefix.is_empty() {
            esc(item)
        } else {
            format!("{prefix}{}", esc(item).replace(' ', ""))
        };
        s.push_str(&format!("<li><a href=\"{href}\">{}</a></li>", esc(item)));
    }
    s.push_str("</ul>");
    s
}

fn status_row(v: &Venue) -> String {
    if v.status.is_empty() {
        return String::new();
    }
    format!(
        r#"<p><span class="pill pill-{pill}">{bucket}</span> {status}</p>"#,
        pill = esc(&v.bucket.replace(' ', "-")),
        bucket = esc(&v.bucket),
        status = esc(&v.status),
    )
}

fn maps_link(v: &Venue) -> String {
    let href = if !v.maps.is_empty() {
        esc(&v.maps)
    } else if !v.address.is_empty() {
        let q = v.address.replace(' ', "+");
        format!("https://www.google.com/maps/search/?api=1&query={q},+San+Francisco,+CA")
    } else {
        return String::new();
    };
    format!(
        r#"<p class="sheet-actions"><a class="maps-btn" href="{href}" target="_blank" rel="noopener">Google Maps</a></p>"#
    )
}

fn field(label: &str, val: &str) -> String {
    if val.is_empty() {
        String::new()
    } else {
        format!("<h2>{label}</h2><p>{}</p>", esc(val))
    }
}

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

const CSS: &str = r#"
:root { --bg:#0c0c0c; --fg:#f4f4f4; --muted:#b0b0b0; --line:#2a2a2a; --card:#161616; }
* { box-sizing: border-box; }
html { color-scheme: dark; }
html, body { margin:0; height:100%; background:var(--bg); color:var(--fg); font: 16px/1.45 ui-sans-serif, system-ui, sans-serif; }
a { color: var(--fg); }
.skip { position:absolute; left:-999px; top:0; }
.skip:focus { left:8px; background:#fff; color:#000; padding:8px; z-index:9; }
body { display:flex; flex-direction:column; }
.bar { padding: 10px 16px 8px; }
.search { width: 100%; }
.search input {
  width: 100%;
  height: 44px;
  padding: 0 14px;
  border: 1px solid #8a8a8a;
  background: #2c2c2c;
  color: #fff;
  border-radius: 8px;
  font: inherit;
}
.search input::placeholder { color: #d0d0d0; }
.search input:focus { outline: 2px solid #f4f4f4; outline-offset: 2px; border-color: #f4f4f4; }
.views { display:flex; gap:8px; margin-left: auto; }
.filters { display:flex; flex-wrap:wrap; gap:8px; padding: 4px 16px 10px; align-items:center; }
.chip { height:36px; padding:0 12px; border:1px solid var(--line); background:transparent; color:var(--fg); border-radius:999px; cursor:pointer; }
.chip.on { background:var(--fg); color:var(--bg); }
.count { color:var(--muted); margin:0; font-size:14px; }
.stage { flex:1; display:flex; min-height:0; border-top:1px solid var(--line); }
#list { display:none; width: 320px; flex-shrink:0; overflow:auto; padding: 8px; border-right:1px solid var(--line); background: var(--bg); scrollbar-color: #5a5a5a var(--bg); }
#list::-webkit-scrollbar { width: 10px; }
#list::-webkit-scrollbar-track { background: var(--bg); }
#list::-webkit-scrollbar-thumb { background: #5a5a5a; border-radius: 8px; }
body.view-list #list { display:block; }
#map { flex:1; min-width:0; min-height: 240px; }
.foot { display:none; padding: 8px 20px 16px; color:var(--muted); font-size:13px; }
.cards { list-style:none; margin:0; padding:0; display:flex; flex-direction:column; gap:8px; }
.card { display:flex; flex-direction:column; gap:8px; padding:10px 12px; border:1px solid var(--line); background:var(--card); border-radius:8px; color:inherit; text-align:left; cursor:pointer; min-height:44px; width:100%; font: inherit; }
.card:hover, .card:focus-within, .card.on { border-color:#888; }
.card.on { background: #1f1f1f; }
.card-top { display:flex; align-items:flex-start; justify-content:space-between; gap:8px; }
.card-top b { flex:1; min-width:0; }
.card .addr { color:var(--muted); font-size:14px; }
.card-bottom { display:flex; align-items:center; justify-content:space-between; gap:8px; }
.card-icons { display:flex; flex-direction:row; gap:6px; flex-shrink:0; margin-left:auto; }
.card-icon { display:inline-flex; align-items:center; justify-content:center; width:36px; height:36px; border:1px solid var(--line); border-radius:6px; color:var(--fg); background:transparent; }
.card-icon:hover { border-color:#888; }
.card-icon svg { width:16px; height:16px; display:block; }
.maps-btn { display:inline-flex; align-items:center; justify-content:center; height:36px; padding:0 12px; border:1px solid var(--line); border-radius:6px; color:var(--fg); text-decoration:none; font-size:13px; align-self:flex-start; background:transparent; cursor:pointer; font: inherit; }
.maps-btn:hover { border-color:#888; }
.act-ask { background:#3d2e08; color:#fbbf24; border-color:#854d0e; }
.act-out { background:#450a0a; color:#fca5a5; border-color:#b91c1c; }
.act-low { background:#431407; color:#fdba74; border-color:#c2410c; }
.pill { display:inline-block; border:1px solid var(--line); padding:2px 8px; border-radius:999px; font-size:12px; flex-shrink:0; white-space:nowrap; }
.pill-Ask { background:#3d2e08; color:#fbbf24; border-color:#854d0e; }
.pill-Contacted { background:#172554; color:#93c5fd; border-color:#1d4ed8; }
.pill-Low { background:#431407; color:#fdba74; border-color:#c2410c; }
.pill-Out { background:#450a0a; color:#fca5a5; border-color:#b91c1c; }
.pill-Needs-Review { background:#1c1917; color:#d6d3d1; border-color:#78716c; }
.pill-Other { background:#1f2937; color:#e5e7eb; border-color:#4b5563; }
#detail { display:none; width: 380px; flex-shrink:0; overflow:auto; padding: 12px 16px 32px; border-left:1px solid var(--line); background: var(--bg); scrollbar-color: #5a5a5a var(--bg); }
body.sheet-open #detail { display:block; }
#detail-close { margin-bottom: 12px; }
#detail h2 { margin: 16px 0 6px; font-size:15px; color:var(--muted); font-weight:600; }
#detail h2:first-child { margin-top: 0; color: var(--fg); font-size: 22px; }
#detail h3 { margin: 12px 0 6px; font-size:14px; }
#detail ul { margin: 0; padding-left: 1.2em; }
.sheet-actions { display:flex; flex-wrap:wrap; gap:8px; margin: 12px 0; }
.detail .sheet { max-width: 42rem; padding: 16px 20px 48px; }
.detail .top { padding: 16px 20px 8px; }
.detail h2 { margin: 20px 0 6px; font-size:15px; color:var(--muted); font-weight:600; }
.detail h3 { margin: 12px 0 6px; font-size:14px; }
.raw { white-space: pre-wrap; }
.agents { margin-top:24px; color:var(--muted); }
.meta { color:var(--muted); }
.tag { margin: 0; }
@media (max-width: 700px) {
  .views { margin-left: auto; }
  body.view-list .stage { flex-direction: column; }
  body.view-list #list { width: 100%; max-height: 38%; border-right: none; border-bottom: 1px solid var(--line); }
  body.sheet-open .stage { flex-direction: column; }
  body.sheet-open #detail { width: 100%; max-height: 48%; border-left: none; border-top: 1px solid var(--line); }
}
"#;

const JS: &str = r#"
const rowsEl = document.getElementById('rows');
const countEl = document.getElementById('count');
const qEl = document.getElementById('q');
let buckets = ['all'];
let view = 'map';
let highlight = '';
let markers = {};
const map = L.map('map').setView([37.7749, -122.4194], 13);
L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
  maxZoom: 19,
  attribution: '&copy; OpenStreetMap'
}).addTo(map);

function esc(s) {
  return String(s).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
}
function mapsHref(v) {
  if (v.maps) return v.maps;
  const q = [v.address, v.place, 'San Francisco, CA'].filter(Boolean).join(', ');
  return 'https://www.google.com/maps/search/?api=1&query=' + encodeURIComponent(q);
}
function pill(bucket, status) {
  if (!status) return '';
  const cls = String(bucket || '').replace(/\s+/g, '-');
  return '<span class="pill pill-' + esc(cls) + '">' + esc(bucket) + '</span>';
}
const ICON_MAIL = '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="none" stroke="currentColor" stroke-width="2" d="M3 6h18v12H3z"/><path fill="none" stroke="currentColor" stroke-width="2" d="M3 6l9 7 9-7"/></svg>';
const ICON_X = '<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M14.2 10.3 22 3h-2.2l-6.8 6.3L8 3H2.3l8.2 9.5L2 21h2.2l7.2-6.6L16.2 21H22l-7.8-10.7zm-1.1 1.3-.8-.9-6.5-7.5h2.2l5.3 6.1.8.9 6.9 8h-2.2l-5.7-6.6z"/></svg>';
function contactIcons(v) {
  const c = v.contact || {};
  const emails = c.emails || [];
  const xs = c.x || [];
  let s = '';
  if (emails.length) {
    s += '<a class="card-icon" href="mailto:' + esc(emails[0]) + '" aria-label="Email">' + ICON_MAIL + '</a>';
  }
  if (xs.length) {
    s += '<a class="card-icon" href="' + esc(xs[0]) + '" target="_blank" rel="noopener" aria-label="X">' + ICON_X + '</a>';
  }
  return s ? '<span class="card-icons">' + s + '</span>' : '';
}
function field(label, val) {
  if (!val) return '';
  return '<h2>' + esc(label) + '</h2><p>' + esc(val) + '</p>';
}
function links(title, prefix, items) {
  if (!items || !items.length) return '';
  return '<h3>' + esc(title) + '</h3><ul>' + items.map(item => {
    const href = prefix ? prefix + String(item).replace(/ /g, '') : item;
    return '<li><a href="' + esc(href) + '">' + esc(item) + '</a></li>';
  }).join('') + '</ul>';
}
function detailHtml(v) {
  const c = v.contact || {};
  const geo = (v.lat != null && v.lng != null)
    ? '<p class="meta">' + Number(v.lat).toFixed(5) + ', ' + Number(v.lng).toFixed(5) + '</p>'
    : '';
  let contact = '<section><h2>Contact</h2>';
  if (!c.raw) {
    contact += '<p>None on file.</p></section>';
  } else {
    contact += '<p class="raw">' + esc(c.raw) + '</p>';
    contact += links('Emails', 'mailto:', c.emails);
    contact += links('Phones', 'tel:', c.phones);
    contact += links('X', '', c.x);
    contact += links('URLs', '', c.urls);
    contact += '</section>';
  }
  const slug = encodeURIComponent(v.slug);
  return '<h2>' + esc(v.place) + '</h2>' +
    '<p class="tag">' + esc(v.address || 'No street') + '</p>' +
    geo +
    (v.status ? '<p>' + pill(v.bucket, v.status) + ' ' + esc(v.status) + '</p>' : '') +
    '<p class="sheet-actions">' +
      '<a class="maps-btn" href="' + esc(mapsHref(v)) + '" target="_blank" rel="noopener">Google Maps</a>' +
      '<button type="button" class="maps-btn act-ask" data-status="Ask">Ask</button>' +
      '<button type="button" class="maps-btn act-out" data-status="Out">Out</button>' +
      '<button type="button" class="maps-btn act-low" data-status="Low">Low</button>' +
    '</p>' +
    field('Screen', v.screen) +
    field('Capacity', v.capacity) +
    field('Section', v.section) +
    field('Notes', v.notes) +
    contact +
    '<p class="agents">Agents: <a href="/v/' + slug + '.md">Markdown</a> · <a href="/api/venues/' + slug + '">JSON</a> · <a href="/api/venues/' + slug + '/contact">Contact</a></p>';
}
function resizeMap() {
  setTimeout(() => map.invalidateSize(), 0);
}
function openDetail(slug) {
  const v = VENUES.find(x => x.slug === slug);
  if (!v) return;
  highlight = slug;
  markCard(slug);
  document.getElementById('detail-body').innerHTML = detailHtml(v);
  document.body.classList.add('sheet-open');
  const m = markers[slug];
  if (m) map.panTo(m.getLatLng());
  resizeMap();
  writeUrl();
}
function closeDetail() {
  document.body.classList.remove('sheet-open');
  resizeMap();
}
for (const v of VENUES) {
  if (v.lat == null || v.lng == null) continue;
  const m = L.marker([v.lat, v.lng]);
  m.on('click', () => openDetail(v.slug));
  m.addTo(map);
  markers[v.slug] = m;
}
function match(v, q) {
  if (!q) return true;
  const blob = [v.place, v.address, v.status, v.bucket, v.section, v.notes, v.slug, v.contact && v.contact.raw].join(' ').toLowerCase();
  return blob.includes(q);
}
function isAll() {
  return buckets.length === 1 && buckets[0] === 'all';
}
function visible() {
  const q = (qEl.value || '').trim().toLowerCase();
  return VENUES.filter(v => (isAll() || buckets.includes(v.bucket)) && match(v, q));
}
function fitList(list) {
  const pts = list.filter(v => v.lat != null).map(v => [v.lat, v.lng]);
  const sf = pts.filter(([lat, lng]) => lat >= 37.70 && lat <= 37.82 && lng >= -122.52 && lng <= -122.35);
  const fit = sf.length ? sf : pts;
  if (fit.length) map.fitBounds(fit, { padding: [40, 40], maxZoom: 14 });
}
function writeUrl() {
  const p = new URLSearchParams();
  const q = (qEl.value || '').trim();
  if (q) p.set('q', q);
  if (!isAll()) p.set('bucket', buckets.join(','));
  if (view === 'list') p.set('view', 'list');
  if (highlight) p.set('highlight', highlight);
  const s = p.toString();
  history.replaceState(null, '', s ? ('?' + s) : location.pathname);
}
function readUrl() {
  const p = new URLSearchParams(location.search);
  qEl.value = p.get('q') || '';
  const raw = p.get('bucket');
  if (!raw || raw === 'all') buckets = ['all'];
  else buckets = raw.split(',').map(s => s.trim()).filter(Boolean);
  view = p.get('view') === 'list' ? 'list' : 'map';
  highlight = p.get('highlight') || '';
}
function setView(next) {
  view = next;
  document.body.classList.toggle('view-list', view === 'list');
  document.body.classList.toggle('view-map', view === 'map');
  document.querySelectorAll('[data-view]').forEach(b => b.classList.toggle('on', b.getAttribute('data-view') === view));
  setTimeout(() => { map.invalidateSize(); fitList(visible()); }, 0);
}
function markCard(slug) {
  document.querySelectorAll('.card').forEach(c => {
    const on = c.getAttribute('data-slug') === slug;
    c.classList.toggle('on', on);
    if (on) c.scrollIntoView({ block: 'nearest' });
  });
}
function render() {
  const list = visible();
  countEl.textContent = list.length + ' venues';
  rowsEl.innerHTML = list.map(v => {
    const on = v.slug === highlight ? ' on' : '';
    return '<li><div class="card' + on + '" data-slug="' + esc(v.slug) + '" role="button" tabindex="0">' +
      '<div class="card-top"><b>' + esc(v.place) + '</b>' + pill(v.bucket, v.status) + '</div>' +
      '<span class="addr">' + esc(v.address || 'No street') + '</span>' +
      '<div class="card-bottom">' +
      '<a class="maps-btn" href="' + esc(mapsHref(v)) + '" target="_blank" rel="noopener">Google Maps</a>' +
      contactIcons(v) +
      '</div>' +
      '</div></li>';
  }).join('');
  for (const [slug, m] of Object.entries(markers)) {
    const v = VENUES.find(x => x.slug === slug);
    const show = list.includes(v);
    if (show) { if (!map.hasLayer(m)) m.addTo(map); }
    else { map.removeLayer(m); }
  }
  document.querySelectorAll('[data-bucket]').forEach(b => {
    const id = b.getAttribute('data-bucket');
    const on = isAll() ? id === 'all' : id !== 'all' && buckets.includes(id);
    b.classList.toggle('on', on);
  });
  if (view === 'map' || view === 'list') fitList(list);
  writeUrl();
}
qEl.addEventListener('input', render);
document.querySelector('.filters').addEventListener('click', (e) => {
  const btn = e.target.closest('[data-bucket]');
  if (!btn) return;
  const id = btn.getAttribute('data-bucket');
  if (id === 'all') {
    buckets = ['all'];
  } else if (isAll()) {
    buckets = [id];
  } else if (buckets.includes(id)) {
    if (buckets.length === 1) return;
    buckets = buckets.filter(b => b !== id);
  } else {
    buckets = buckets.concat(id);
  }
  render();
});
document.querySelector('.views').addEventListener('click', (e) => {
  const btn = e.target.closest('[data-view]');
  if (!btn) return;
  setView(btn.getAttribute('data-view'));
  writeUrl();
});
rowsEl.addEventListener('click', (e) => {
  if (e.target.closest('.maps-btn, .card-icon')) return;
  const card = e.target.closest('[data-slug]');
  if (!card) return;
  openDetail(card.getAttribute('data-slug'));
});
rowsEl.addEventListener('keydown', (e) => {
  if (e.key !== 'Enter' && e.key !== ' ') return;
  if (e.target.closest('.maps-btn, .card-icon')) return;
  const card = e.target.closest('[data-slug]');
  if (!card) return;
  e.preventDefault();
  openDetail(card.getAttribute('data-slug'));
});
document.getElementById('detail-close').addEventListener('click', closeDetail);
document.getElementById('detail').addEventListener('click', (e) => {
  const btn = e.target.closest('[data-status]');
  if (!btn || !highlight) return;
  setStatus(highlight, btn.getAttribute('data-status'));
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeDetail();
});
function setStatus(slug, status) {
  fetch('/api/venues/' + encodeURIComponent(slug) + '/status', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ status: status })
  }).then(res => {
    if (!res.ok) return res.json().then(j => Promise.reject(j));
    return res.json();
  }).then(next => {
    const i = VENUES.findIndex(x => x.slug === slug);
    if (i >= 0) VENUES[i] = next;
    render();
    openDetail(slug);
  }).catch(() => {});
}
readUrl();
setView(view);
render();
if (highlight) openDetail(highlight);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_html() {
        assert_eq!(esc("A & B <c>"), "A &amp; B &lt;c&gt;");
    }
}
