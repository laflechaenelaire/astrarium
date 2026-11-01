#!/usr/bin/env python3
"""Rebuild the bundled catalogue from d3-celestial (network needed only here)."""
import json
from pathlib import Path
from urllib.request import urlopen

UPSTREAM_REVISION = "7e720a3de062059d4c5400a379146a601d9010e0"
BASE = f"https://raw.githubusercontent.com/ofrohn/d3-celestial/{UPSTREAM_REVISION}/"
ROOT = Path(__file__).resolve().parents[1] / "data"


def download(path):
    with urlopen(BASE + path, timeout=60) as response:
        return response.read().decode()


def main():
    ROOT.mkdir(exist_ok=True)
    stars = json.loads(download("data/stars.6.json"))["features"]
    names = json.loads(download("data/starnames.json"))
    lines = json.loads(download("data/constellations.lines.json"))["features"]
    output = []
    for star in stars:
        info = names.get(str(star["id"]), {})
        ra, dec = star["geometry"]["coordinates"]
        designation = " ".join(filter(None, [info.get("desig"), info.get("c")]))
        output.append({
            "id": star["id"], "name": info.get("name") or designation or f'HIP {star["id"]}',
            "named": bool(info.get("name")), "con": info.get("c", ""),
            "ra": ra % 360, "dec": dec, "mag": star["properties"]["mag"],
        })
    output.sort(key=lambda item: item["mag"])
    catalogue = {"stars": output, "constellations": [
        {"name": line["id"], "lines": line["geometry"]["coordinates"]} for line in lines
    ]}
    (ROOT / "sky.json").write_text(json.dumps(catalogue, ensure_ascii=False, separators=(",", ":")) + "\n")
    (ROOT / "LICENSE-d3-celestial").write_text(download("LICENSE"))
    print(f'Bundled {len(output)} stars and {len(lines)} constellations.')


if __name__ == "__main__":
    main()
