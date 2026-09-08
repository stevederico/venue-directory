#!/usr/bin/env python3
"""Import data/omarchy-sf-venues.csv + coords.tsv into data/venues.db."""

from __future__ import annotations

import csv
import re
import sqlite3
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSV_PATH = ROOT / "data" / "omarchy-sf-venues.csv"
COORDS_PATH = ROOT / "data" / "coords.tsv"
DB_PATH = ROOT / "data" / "venues.db"

SCHEMA = """
CREATE TABLE venues (
  slug TEXT PRIMARY KEY,
  place TEXT NOT NULL,
  address TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT '',
  screen TEXT NOT NULL DEFAULT '',
  capacity TEXT NOT NULL DEFAULT '',
  contact TEXT NOT NULL DEFAULT '',
  notes TEXT NOT NULL DEFAULT '',
  section TEXT NOT NULL DEFAULT '',
  maps TEXT NOT NULL DEFAULT '',
  lat REAL,
  lng REAL
);
"""


def slugify(place: str) -> str:
    out = []
    dash = False
    for c in place.lower():
        if c.isalnum():
            out.append(c)
            dash = False
        elif out and not dash:
            out.append("-")
            dash = True
    s = "".join(out).strip("-")
    return s or "venue"


def load_coords() -> dict[str, tuple[float, float]]:
    coords: dict[str, tuple[float, float]] = {}
    if not COORDS_PATH.exists():
        return coords
    for i, line in enumerate(COORDS_PATH.read_text().splitlines()):
        if i == 0 or not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        try:
            coords[parts[0]] = (float(parts[1]), float(parts[2]))
        except ValueError:
            continue
    return coords


def main() -> None:
    coords = load_coords()
    if DB_PATH.exists():
        DB_PATH.unlink()
    con = sqlite3.connect(DB_PATH)
    con.execute(SCHEMA)
    seen: dict[str, int] = {}
    n = 0
    with CSV_PATH.open(newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            place = (row.get("place") or "").strip()
            if not place:
                continue
            base = slugify(place)
            if base in seen:
                slug = f"{base}-{seen[base]}"
                seen[base] += 1
            else:
                slug = base
                seen[base] = 1
            latlng = coords.get(slug)
            con.execute(
                """INSERT INTO venues
                (slug, place, address, status, screen, capacity, contact, notes, section, maps, lat, lng)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    slug,
                    place,
                    (row.get("address") or "").strip(),
                    (row.get("status") or "").strip(),
                    (row.get("screen") or "").strip(),
                    (row.get("capacity") or "").strip(),
                    (row.get("contact") or "").strip(),
                    (row.get("notes") or "").strip(),
                    (row.get("section") or "").strip(),
                    (row.get("maps") or "").strip(),
                    None if latlng is None else latlng[0],
                    None if latlng is None else latlng[1],
                ),
            )
            n += 1
    con.commit()
    print(f"wrote {n} venues -> {DB_PATH}")
    # sanity
    out = con.execute(
        "SELECT slug, status FROM venues WHERE slug IN ('595-pacific','commonwealth-club')"
    ).fetchall()
    for slug, status in out:
        print(f"  {slug}: {status}")
    con.close()


if __name__ == "__main__":
    main()
