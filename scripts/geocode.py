#!/usr/bin/env python3
"""One-shot Nominatim geocode. 1 req/sec. Writes data/coords.tsv."""

from __future__ import annotations

import csv
import json
import re
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSV_PATH = ROOT / "data" / "omarchy-sf-venues.csv"
OUT_PATH = ROOT / "data" / "coords.tsv"
UA = "venue-directory/0.1 (steve@dottie.ai; OSM geocode for SF venue map)"
SKIP = {"", "—", "-", "sf", "south bay"}

# Bay Area box. Drop hits outside it.
LAT_MIN, LAT_MAX = 37.2, 38.3
LNG_MIN, LNG_MAX = -123.1, -121.5


def slugify(place: str) -> str:
    s = place.lower()
    s = re.sub(r"[^a-z0-9]+", "-", s).strip("-")
    return s or "venue"


def clean_address(addr: str) -> str | None:
    a = (addr or "").strip()
    if a.lower() in SKIP:
        return None
    a = re.sub(r"\([^)]*\)", " ", a)
    a = re.sub(r"(?i)\b\d+(st|nd|rd|th)\s+floors?\b", " ", a)
    a = re.sub(r"(?i)\b\d+(st|nd|rd|th)\s+fl\.?\b", " ", a)
    a = re.sub(r"(?i)\bfl(?:oor)?\.?\s*\d+(?:\s*[-–]\s*\d+)?", " ", a)
    a = re.sub(r"#\s*\d+", " ", a)
    a = re.sub(r"(?i)\bsuite\s+\d+\w*", " ", a)
    a = re.sub(r"(?i)\bste\.?\s+\d+\w*", " ", a)
    a = re.sub(r"\bSSF\b", "South San Francisco, CA", a)
    m = re.search(r"^(.*?\b\d{5}\b)", a)
    if m:
        a = m.group(1)
    a = re.sub(r"\s+", " ", a).strip(" ,/")
    if not a or a.lower() in SKIP:
        return None
    if not re.search(r"\d", a):
        return None
    if " and " in a.lower():
        first = a.split(" and ")[0].strip(" ,")
        if re.search(r"\d", first):
            a = first
    if not re.search(r"(san francisco|oakland|south san francisco)", a, re.I):
        a = a + ", San Francisco, CA"
    return a


def nominatim(q: str) -> tuple[float, float] | None:
    params = {
        "format": "jsonv2",
        "limit": "1",
        "countrycodes": "us",
        "q": q,
    }
    url = "https://nominatim.openstreetmap.org/search?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode())
    if not data:
        return None
    lat = float(data[0]["lat"])
    lng = float(data[0]["lon"])
    if not (LAT_MIN <= lat <= LAT_MAX and LNG_MIN <= lng <= LNG_MAX):
        return None
    return lat, lng


def load_done() -> dict[str, tuple[str, str]]:
    done: dict[str, tuple[str, str]] = {}
    if not OUT_PATH.exists():
        return done
    for line in OUT_PATH.read_text().splitlines()[1:]:
        parts = line.split("\t")
        if len(parts) >= 3 and parts[0]:
            done[parts[0]] = (parts[1], parts[2])
    return done


def main() -> None:
    done = load_done()
    rows = []
    with CSV_PATH.open(newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            place = (row.get("place") or "").strip()
            if not place:
                continue
            rows.append((slugify(place), place, row.get("address") or ""))

    seen: set[str] = set()
    unique = []
    for slug, place, addr in rows:
        if slug in seen:
            continue
        seen.add(slug)
        unique.append((slug, place, addr))

    out_lines = ["slug\tlat\tlng"]
    # keep previous hits first
    for slug, (lat, lng) in done.items():
        out_lines.append(f"{slug}\t{lat}\t{lng}")

    n_ok = len(done)
    n_skip = 0
    n_fail = 0
    for i, (slug, place, addr) in enumerate(unique, 1):
        if slug in done:
            continue
        q = clean_address(addr)
        queries = []
        if q:
            queries.append(q)
        hit = None
        used = None
        for qtry in queries:
            try:
                hit = nominatim(qtry)
            except Exception as e:
                print(f"ERR {slug}: {e}")
                hit = None
            time.sleep(1.1)
            if hit:
                used = qtry
                break
        if hit:
            lat, lng = hit
            done[slug] = (f"{lat:.6f}", f"{lng:.6f}")
            out_lines.append(f"{slug}\t{lat:.6f}\t{lng:.6f}")
            n_ok += 1
            print(f"OK  {i}/{len(unique)} {slug} {lat:.5f},{lng:.5f}  ({used})")
        else:
            if q is None:
                n_skip += 1
                print(f"SKIP {i}/{len(unique)} {slug} (no address)")
            else:
                n_fail += 1
                print(f"FAIL {i}/{len(unique)} {slug} ({q})")
        OUT_PATH.write_text("\n".join(out_lines) + "\n")

    print(f"done ok={n_ok} skip={n_skip} fail={n_fail} file={OUT_PATH}")


if __name__ == "__main__":
    main()
