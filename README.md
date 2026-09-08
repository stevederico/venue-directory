<div align="center">
  <h1 align="center" style="border-bottom: none; margin-bottom: 0;">Venue Directory</h1>
  <h3 align="center" style="margin-top: 0; font-weight: normal;">
    sf meetup rooms — map, list, and an agent contact api in one zero-crate rust binary
  </h3>
</div>

<br />

## 🚀 Quick Start

Needs Rust (edition 2021) and system SQLite (`libsqlite3`).

```bash
git clone https://github.com/stevederico/venue-directory
cd venue-directory
cargo run
```

Open http://127.0.0.1:18790

Fresh DB from the checked-in CSV + coords:

```bash
python3 scripts/import_sqlite.py
cargo run
```

<br />

## ✨ What's Included

### 🗺️ **Map + List**
- **Leaflet + OpenStreetMap** pins for every geocoded room (no map API key)
- **List view** with search across place, street, and contact
- **Status chips** — Ask / Contacted / Low / Out / Needs Review (local review workflow)
- **Detail sheet** with address, maps link, capacity, screen notes, and parsed contact

### 🤖 **Agent API**
- **Contact first** — `GET /api/venues/{slug}/contact` returns emails, phones, and X URLs parsed from the raw contact string
- **Search / list** — `GET /api/search?q=` and `GET /api/venues`
- **CRUD** — create, patch, delete, and mark Ask / Out / Low; writes SQLite live (no restart)
- **Machine index** — `GET /llms.txt`; Accept `application/json` or `text/markdown` on `/` and `/v/{slug}`

Full request / success / error examples: [AGENTS.md](AGENTS.md).

```bash
curl -sS http://127.0.0.1:18790/api/venues/cloudflare/contact
```

Empty `emails` / `phones` / `x` means none parsed — read `contact.raw`. Do not invent addresses.

### 📦 **Data**
- **Runtime store** — `data/venues.db` (SQLite). Override with `VENUES_DB`
- **Import-only CSV** — `data/omarchy-sf-venues.csv` plus `data/coords.tsv`
- **Geocode helper** — `python3 scripts/geocode.py` (Nominatim) then re-import
- **Source notes** — curated from [omarchy-meetup](https://github.com/stevederico/omarchy-meetup) and public venue pages

Venue contact strings (emails, phones, X) are intentional public booking info. Personal outreach status/notes are stripped from the published dataset.

<br />

## 📖 Configuration

| Var | Default | Purpose |
|---|---|---|
| `PORT` | `18790` | Listen port |
| `HOST` | `127.0.0.1` | Bind address |
| `VENUES_DB` | `data/venues.db` | SQLite path |

No `.env` required. No auth — local tool. Do not expose the write API on a public network without your own gate.

<br />

## 🏗️ Tech Stack

| Technology | Version | Purpose |
|---|---|---|
| **Rust** | edition 2021 | HTTP server + JSON (stdlib) |
| **SQLite** | system `libsqlite3` | Venue store |
| **crates** | **none** | Zero Cargo dependencies |
| **Leaflet** | 1.9.4 | Map UI (CDN) |
| **OpenStreetMap** | tiles + Nominatim | Map tiles and geocode |

Same product shape as [SF Schools](https://github.com/stevederico/preschool) (map / list / filters / detail sheet), implemented as a single Rust binary instead of skateboard.

```bash
cargo test && cargo run
```

<br />

## 🤝 Contributing

```bash
git clone https://github.com/stevederico/venue-directory
cd venue-directory
cargo test
cargo run
```

Keep the zero-crate rule: justify any new dependency against stdlib or a system library first. Prefer patches that improve public venue metadata over personal outreach tracking.

<br />

## 💬 Community & Support

- X: [@stevederico](https://x.com/stevederico)
- Issues: [github.com/stevederico/venue-directory/issues](https://github.com/stevederico/venue-directory/issues)

<br />

## 🙏 Acknowledgements

- [Leaflet](https://leafletjs.com/) — interactive maps
- [OpenStreetMap](https://www.openstreetmap.org/) — tiles and Nominatim geocoding (keep attribution)
- [omarchy-meetup](https://github.com/stevederico/omarchy-meetup) — source notes for this list

<br />

## 🔗 Related

- [SF Schools](https://github.com/stevederico/preschool) — same directory UX for SF schools
- [omarchy-meetup](https://github.com/stevederico/omarchy-meetup) — meetup notes this list grew from

<br />

## 🎯 Get Started

```bash
cargo run
```

http://127.0.0.1:18790

<br />

## 📄 License

[MIT License](LICENSE)

<br />

<div align="center">
  <p>Built with Rust, system SQLite, and OpenStreetMap.</p>
  <p>If this helps you book a room, <a href="https://github.com/stevederico/venue-directory">star the repo</a>.</p>
</div>
