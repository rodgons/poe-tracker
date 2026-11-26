#!/usr/bin/env python3
"""Builds src-tauri/data/trade-stats.json: pathofexile.com/trade stat ids by mod text,
and the base types, so a magic item's name ("Flagellant's Quartz Flask of the Curlew")
can be searched by its base ("Quartz Flask"), and each base's trade category, for Trade links
that leave the base out ("any gloves"). Categories come from Path of Building's base data
(MIT), fetched at the pinned commit below.

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
import re
import sys
import urllib.request

# Path of Building Community's base item data, by commit; bump it with the league
POB_REF = "5606cc64fa9f15694e69001a52b90aa968d01a86"
POB_BASES = "https://raw.githubusercontent.com/PathOfBuildingCommunity/PathOfBuilding/{ref}/src/Data/Bases/{name}.lua"
# PoB's (type, subType) → trade's type_filters category
CATEGORY = {
    ("One Handed Sword", None): "weapon.onesword", ("Thrusting One Handed Sword", None): "weapon.onesword",
    ("Two Handed Sword", None): "weapon.twosword", ("One Handed Axe", None): "weapon.oneaxe",
    ("Two Handed Axe", None): "weapon.twoaxe", ("One Handed Mace", None): "weapon.onemace",
    ("Sceptre", None): "weapon.sceptre", ("Two Handed Mace", None): "weapon.twomace", ("Bow", None): "weapon.bow",
    ("Claw", None): "weapon.claw", ("Dagger", None): "weapon.dagger", ("Dagger", "Rune Dagger"): "weapon.runedagger",
    ("Staff", None): "weapon.staff", ("Staff", "Warstaff"): "weapon.warstaff", ("Wand", None): "weapon.wand",
    ("Shield", None): "armour.shield", ("Quiver", None): "armour.quiver", ("Helmet", None): "armour.helmet",
    ("Body Armour", None): "armour.chest", ("Gloves", None): "armour.gloves", ("Boots", None): "armour.boots",
    ("Amulet", None): "accessory.amulet", ("Ring", None): "accessory.ring", ("Belt", None): "accessory.belt",
    ("Flask", None): "flask", ("Jewel", None): "jewel", ("Jewel", "Abyss"): "jewel.abyss", ("Jewel", "Cluster"): "jewel.cluster",
}
FILES = ["amulet", "axe", "belt", "body", "boots", "bow", "claw", "dagger", "flask", "gloves", "helmet", "jewel",
         "mace", "quiver", "ring", "shield", "staff", "sword", "wand"]


def pob_categories():
    out = {}
    for name in FILES:
        req = urllib.request.Request(POB_BASES.format(ref=POB_REF, name=name), headers={"User-Agent": "poe-tracker"})
        lua = urllib.request.urlopen(req, timeout=30).read().decode("utf-8")
        for base, body in re.findall(r'itemBases\["([^"]+)"\] = \{(.*?)\n\}', lua, re.S):
            t = re.search(r'\btype = "([^"]+)"', body)
            sub = re.search(r'\bsubType = "([^"]+)"', body)
            if not t:
                continue
            key = (t.group(1), sub.group(1) if sub else None)
            cat = CATEGORY.get(key) or CATEGORY.get((key[0], None))
            if cat:
                out[base] = cat
    return out

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
table["categories"] = pob_categories()
print(f"categories: {len(table['categories'])} bases", file=sys.stderr)
print(f"bases: {len(bases)}", file=sys.stderr)
json.dump(table, open(out, "w", encoding="utf-8"), ensure_ascii=False, separators=(",", ":"), sort_keys=True)
