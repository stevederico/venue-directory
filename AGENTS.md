# Venue Directory

SF rooms with parsed contact.

Default: `http://127.0.0.1:18790`

## Get contact for a venue

This is the agent path. Use it before emailing or DMing anyone.

```bash
curl -sS http://127.0.0.1:18790/api/venues/cloudflare/contact
```

Success `200`:

```json
{
  "slug": "cloudflare",
  "place": "Cloudflare",
  "address": "101 Townsend St, 94107",
  "maps": "https://www.google.com/maps/search/?api=1&query=101+Townsend+St,+94107,+San+Francisco,+CA",
  "status": "",
  "contact": {
    "raw": "cloudflareusergroup@cloudflare.com. usergroup-support@cloudflare.com. …",
    "emails": ["cloudflareusergroup@cloudflare.com", "usergroup-support@cloudflare.com"],
    "phones": [],
    "x": ["https://x.com/lucataco", "https://x.com/cloudflare"],
    "urls": []
  }
}
```

Empty `emails` / `phones` / `x` means none parsed. Read `contact.raw`. Do not invent addresses.

Error `404`:

```json
{ "error": "venue not found" }
```

Slug is kebab-case of `place` (`SHACK15` → `shack15`, `Founders, Inc.` → `founders-inc`).

```bash
curl -sS http://127.0.0.1:18790/api/search?q=shack
curl -sS http://127.0.0.1:18790/api/venues/shack15
curl -sS http://127.0.0.1:18790/v/shack15.md
```

## List

`GET /api/venues` and `GET /api/search?q=`
`GET /api/venues?bucket=Contacted` or `?bucket=Ask,Contacted` filters status buckets.

## Mark Ask, Out, or Low

Review action. Writes `venues.status` in SQLite.

```bash
curl -sS -X POST http://127.0.0.1:18790/api/venues/aws-builder-loft/status \
  -H 'content-type: application/json' \
  -d '{"status":"Ask"}'
```

Success `200`: the full venue JSON (`status` and `bucket` updated).

Error `400`:

```json
{ "error": "status must be Ask, Out, or Low" }
```

Error `404`: `{ "error": "venue not found" }`

Only `Ask`, `Out`, and `Low`. Restart not required. The process updates memory after the write. Full field writes use PATCH below.

## Create a venue

`POST /api/venues`

`place` is required. `slug` is optional (kebab of place). `status` defaults to `Ask`. Other fields default empty. `lat`/`lng` are numbers or omitted.

```bash
curl -sS -X POST http://127.0.0.1:18790/api/venues \
  -H 'content-type: application/json' \
  -d '{"place":"Test Loft","address":"1 Market St, 94105","status":"Ask","contact":"desk@testloft.example","lat":37.793,"lng":-122.396}'
```

Success `201`: the full venue JSON (same shape as `GET /api/venues/{slug}`).

Error `400`: `{ "error": "place required" }` (or `json object required`, `bad slug`).

Error `409`: `{ "error": "slug exists" }` when you send a slug that is already taken.

## Update a venue

`PATCH /api/venues/{slug}` (PUT is the same). Partial. Omitted keys stay. `null` clears a string or coord.

```bash
curl -sS -X PATCH http://127.0.0.1:18790/api/venues/test-loft \
  -H 'content-type: application/json' \
  -d '{"status":"Contacted","notes":"Emailed Mon"}'
```

Success `200`: the full venue JSON.

Error `400`: `{ "error": "empty patch" }`

Error `404`: `{ "error": "venue not found" }`

`contact` is a string (raw). Nested `{ "raw": "..." }` is also accepted. Slug does not change.

## Delete a venue

`DELETE /api/venues/{slug}`

```bash
curl -sS -X DELETE http://127.0.0.1:18790/api/venues/test-loft
```

Success `200`:

```json
{ "ok": true, "slug": "test-loft" }
```

Error `404`: `{ "error": "venue not found" }`

No auth. Local tool. Writes SQLite and the live process. Restart not required.

UI query params on `/`: `q`, `bucket`, `view=list`, `highlight={slug}`. Same idea as SF Schools (`preschool`).

```json
{
  "count": 2,
  "venues": [
    {
      "slug": "cloudflare",
      "place": "Cloudflare",
      "address": "101 Townsend St, 94107",
      "status": "",
      "bucket": "Other",
      "lat": 37.78,
      "lng": -122.39,
      "has_contact": true
    }
  ]
}
```

## Machine index

`GET /llms.txt`

Accept `application/json` or `text/markdown` on `/` and `/v/{slug}`.

## Data

Runtime is SQLite: `data/venues.db`. Edit rows there. CSV is import-only.

```bash
python3 scripts/import_sqlite.py   # one-shot: data/omarchy-sf-venues.csv + coords.tsv -> venues.db
```

Status is one column: `venues.status`. Filter chips roll that up to `bucket`. Pins: `lat`/`lng` on the same row.

Zero crates. System `libsqlite3`. `cargo test && cargo run`. `VENUES_DB` overrides the path.

Do not commit `.env`. Do not force-push. Map tiles are OSM; keep the attribution.
