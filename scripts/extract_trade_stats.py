#!/usr/bin/env python3
"""Builds src-tauri/data/trade-stats.json: pathofexile.com/trade stat ids by mod text,
and the base types, so a magic item's name ("Flagellant's Quartz Flask of the Curlew")
can be searched by its base ("Quartz Flask").

The Items section's Trade links turn a Build's mod lines into trade stat filters. The
app never calls the trade site itself, so the mapping ships with it. Regenerate each
patch / league:

    curl -A "poe-tracker (github.com/rodgons/poe-tracker)" \
        https://www.pathofexile.com/api/trade/data/stats -o stats.json
    curl -A "poe-tracker (github.com/rodgons/poe-tracker)" \
        https://www.pathofexile.com/api/trade/data/items -o items.json
    python3 -I scripts/extract_trade_stats.py stats.json items.json src-tauri/data/trade-stats.json

Keys are the trade site's own texts ("+# to maximum Life", "... (Local)"), values the
stat id. A text with several ids keeps the first, as the trade site lists them.
"""
import json
import sys

src, items_src, out = sys.argv[1], sys.argv[2], sys.argv[3]
data = json.load(open(src, encoding="utf-8"))
groups = {g["id"]: g["entries"] for g in data["result"]}
table = {}
for kind in ("explicit", "enchant"):
    m = {}
    dupes = 0
    for e in groups[kind]:
        text = e["text"].replace("\n", " ")
        if text in m:
            dupes += 1
            continue
        m[text] = e["id"]
    table[kind] = m
    print(f"{kind}: {len(m)} texts, {dupes} duplicate texts kept first", file=sys.stderr)
items = json.load(open(items_src, encoding="utf-8"))
bases = set()
for g in items["result"]:
    if g["id"] in ("accessory", "armour", "flask", "jewel", "weapon", "tincture", "graft"):
        bases.update(e["type"] for e in g["entries"] if "name" not in e)
table["bases"] = sorted(bases)
print(f"bases: {len(bases)}", file=sys.stderr)
json.dump(table, open(out, "w", encoding="utf-8"), ensure_ascii=False, separators=(",", ":"), sort_keys=True)
