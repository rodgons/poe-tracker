# PoE1 campaign layout sources, and whether we may reuse them

*Research for [#73](https://github.com/rodgons/poe-tracker/issues/73), map [#72](https://github.com/rodgons/poe-tracker/issues/72) · 2026-11-25*

**Question.** Which sources of Path of Exile 1 campaign zone layouts exist? For each one: is it a minimap screenshot (GGG art) or an original schematic, how many zones and variants does it cover, what does it mark, what format is it in, and does its licence let us bundle it in an in-game panel? Which source should our own schematics start from?

## 1. Short answer

None of the layout sources can be bundled as-is. Two problems keep coming up:

1. **Most layout pictures are GGG art, or are traced from it.** Cyclon's guide uses annotated in-game screenshots. Exile-UI's Act-Decoder pictures are pixel-traced minimap outlines, and they carry GGG's own icons (waypoint, bosses, quest items). PRODUCT.md keeps GGG art out of the in-game panels, so both are ruled out whatever their licence says.
2. **The only fully original schematics (Engineering Eternity's) have no licence and are old.** They were drawn around 2018. A licence never covered them: JusKillmeQik's repo is MIT, but it credits the images to "_Treb/ Engineering Eternity" separately, and the only permission on record is a Google Doc compiler's "asked for permission" to share them.

**Recommendation.** Draw our own schematics in our own simple style: entrance, exit, waypoint, objectives and route as lines and dots, stored as vector data keyed by area id. Use the community sources only as **reference for facts**, such as how many variants a zone has, where exits and waypoints can be, and which minimap clue tells variants apart. Do not copy their pictures. The best references, in order:

- **Exile-UI Act-Decoder** (MIT code, actively maintained, Acts 1–10, 138 area keys, 467 images). It is the most complete catalogue of variants and their clues, and its files are already keyed by the same area ids our log gives us.
- **Cyclon's Definitiv Guide** (Oct 2025, all ten acts, named variants such as Coast 1–8, with written tips per variant). It is the best current source of route wisdom.
- **exile-leveling**. We already bundle its route under MIT. It has 86 `dir` hints (e.g. "Go 270°") that make the text fallback.

Before drawing, ask Lailloken whether we may use the Act-Decoder's variant catalogue (names and order, not pictures) as reference, and credit Exile-UI in `THIRD_PARTY_LICENSES.md` / the README. That is courtesy, not a legal requirement for facts. Do the same if we follow Cyclon's variant naming.

## 2. Source by source

| Source | Picture type | Coverage | Marks | Format | Licence / reuse |
|---|---|---|---|---|---|
| Exile-UI Act-Decoder | Pixel-traced minimap outlines + GGG icons (derived from GGG art) | PoE1 Acts 1–10: 138 area-id keys, 467 JPGs (292 top-level variant pictures plus deeper sub-pictures); PoE2 in a separate folder | Points of interest, suggested order, alt/shared locations, bosses, "mini-map clues", sometimes route arrows | 600×600 JPG, keyed `<areaId> <variant>[_<sub>].jpg` | Repo is MIT (© Lailloken). MIT covers his work, not GGG's minimap shapes or icons. Ruled out for in-game panels by the GGG-art rule |
| Engineering Eternity (Treb) layouts, as bundled in JusKillmeQik's PoE-Leveling-Guide | **Original schematics**: flat lines and coloured dots on grey | 85 zone names, 194 images across Acts 1–10 (`<Zone>_Seed_<n>.jpg`) | Entrance (white dot), route (green line), exit (green dot), waypoint (blue dot), side branches (grey) | ~480×260 JPG, Photoshop, dated 2018 | Repo is MIT, but the images are credited to "_Treb/ Engineering Eternity" with no licence. All rights reserved by default. The README warns "Some of these zones are outdated" |
| Engineering Eternity Google Doc (compiled by a fan) | Same images plus text notes | Acts 1–10 | Quest item clusters, waypoint/boss numbers, text like "exits spawn in a random corner" | Google Doc | No licence. The compiler shared it "after asking for permission", for personal use |
| Cyclon's Advanced Campaign Guide V3 (definitivguide.com) | **In-game screenshots** with the map overlay and pink arrows (GGG art) | All ten acts, ~150 zone pages, many variants (e.g. The Coast: 8 named variants) | Route arrows, waypoint, plus written rules per variant ("Waypoints and Exits have to be on Water Level") | PNG on a Docusaurus site | "Copyright © 2026 CyclonDefinitiv". No reuse terms. Pictures ruled out; written tips are facts we may restate in our own words |
| poewiki.net zone pages | One area screenshot per zone (tagged `{{Copyright Grinding Gear Games}}`), no layout maps | Every area | Text only: connections, sometimes a one-line layout note ("from the Lioneye's Watch entrance on the left side of the map to the Mud Flats exit on the right or bottom") | Wiki pages; MediaWiki API works, the HTML site sits behind Anubis bot protection | Text is CC BY-NC-SA 3.0. NonCommercial + ShareAlike, so only short facts restated in our own words |
| exile-leveling (HeartofPhos) | No layout pictures (the only images are step icons) | Acts 1–10 route | `#sub` hints and `{dir\|N}` compass headings (e.g. Tidal Island: "Go {dir\|270}", "If you find a ledge, turn around and go {dir\|90}"). Our `route.json` holds 86 `dir` entries | Text route files, which we already convert to `route.json` | MIT, already bundled and credited in `THIRD_PARTY_LICENSES.md` |
| Maxroll PoE1 campaign guide | One overview image per act, plus text layout tips for chosen zones (Ledge, Chamber of Sins, Control Blocks…) | Acts 1–10, prose | Text tips | Web article | No reuse terms seen. Reference only |
| Game files (datamined tile graphs) | Would be GGG data | All zones | N/A | Binary game data | Campaign zones are put together from tiles at random per seed, so there is no fixed picture to extract. Drawing straight from GGG data also runs into the no-GGG-art rule. Not pursued |

Not covered: Mobalytics's layout guides found by search are for PoE2 (their PoE1 page returned 403). The PoE2 Campaign Codex is out of scope.

## 3. Details that matter for the build

### Exile-UI Act-Decoder

- **Where the pictures come from.** Lailloken made them himself. The commit history of `img/GUI/act-decoder/zones` shows only his commits: the folder was moved from `img/GUI/leveling tracker/zones` in May 2025 (`c46516da`), and most acts were drawn in Jan–Mar 2026 ("implemented A4" … "implemented A10", "PoE1 clean-up" 2026-03-02). The wiki and README credit no outside source.
- **What the pictures are.** Looking at samples shows white outlines traced from the minimap (the Lower Prison picture keeps the isometric minimap grid). GGG's waypoint, boss-skull and quest icons are pasted on. The legend (`zones/explanation.jpg`) lists: point of interest, optional point, suggested order, alt. location, shared location, mini-map clues, boss, optional boss.
- **How variants are organised.** File `<areaId> <n>.jpg` is variant *n*, `<n>_<m>.jpg` is a finer pick inside it, and `x` marks a decoded or sample picture. The module (`modules/act-decoder.ahk`) reads `file-list.json` to know which files are valid, and checks a `layouts` branch for updates. Pictures are keyed by the area id from `Generating level N area "<id>"`, the same key our clientlog uses.
- **Rotate / flip.** The overlay lets the player rotate in 90° steps and flip each picture (commit `9316a844`, Apr 2025). The wiki explains this as a PoE2 need: "many can spawn rotated and/or mirrored". It gives no such reason for PoE1. **For PoE1 we found no evidence that campaign layouts mirror or rotate.** By the map's rule, skip those controls unless the owner sees it happen in game.
- **Licence.** `LICENSE.md` is MIT, © 2022 Lailloken. That would let us bundle *his* expression with notice. But the pictures embed GGG icons and shapes traced from GGG minimaps, and our PRODUCT rule bans GGG art in panels whatever its licence.

### Engineering Eternity schematics

- They are the closest thing to what we want: abstract route diagrams with entrance, exit, waypoint and branches, and no game art.
- They are old (Photoshop EXIF dated 2018-05-31). Their author never licensed them for reuse. JusKillmeQik's own README says they are outdated. Coverage is one or a few "seeds" per zone, not the full set of variants.
- Use their *style* (dots and lines) as proof that a minimal schematic works. Do not copy the files.

### Cyclon's guide

- It is the most current human-checked list of variants (Oct 2025, maintained into 2026), with a name and a written tell for each variant. For example, The Coast's 8 variants: Simple North-East, Simple East, Handgun go South, Handgun Go Past, South Variant, Walk North Variant, Stick to Wall, Long Coast.
- The pictures are full screenshots (GGG art). The written rules are useful facts for the route text and for checking our schematics.

### What "facts vs expression" means here

How many variants a zone has, which corner the exit can be in, and "the waypoint is on water level" are facts about the game. Anyone may state them in their own words and pictures. What the licences protect is each author's picture or text. So a schematic we draw ourselves, from our own play and checked against these sources, is ours to ship under GPL-3.0.

## 4. Suggested basis for our schematics

1. **Data model.** One record per area id (`1_1_2`), holding a list of variants. Each variant is a small vector drawing in a normalised box: outline polyline(s), entrance, exits by target area id, waypoint, objective points, plus a precomputed route polyline per Guide step. Keep it next to `route.json` as bundled data.
2. **Variant catalogue.** Seed the list of variants per zone from Exile-UI's file names (count and order) and Cyclon's named variants. Then confirm each one against the owner's own minimap screenshots, which stay private and are never bundled.
3. **Drawing.** Trace by hand in a small authoring tool, or as hand-written SVG, from the owner's screenshots. Keep the shapes coarse: a recognisable silhouette plus key points, not a pixel tracing. That keeps the result clearly our own work, and avoids recreating GGG's minimap art.
4. **Fallback.** Where no schematic exists yet, show exile-leveling's `#sub` / `dir` text (86 `dir` hints already in `route.json`).
5. **Order.** Start with the zones every guide calls hardest or most varied: Coast, Mud Flats, Submerged Passage, Prisoner's Gate, Ship Graveyard, Lower Prison (Act 1), then Cyclon's "Top 10 Zones to learn".
6. **Credit.** Add a line to `THIRD_PARTY_LICENSES.md` / README thanking Exile-UI's Act-Decoder and Cyclon's guide as references, even though no licence requires it.

## 5. Sources

- Exile-UI repo, MIT: https://github.com/Lailloken/Exile-UI (`LICENSE.md`, `modules/act-decoder.ahk`, `img/GUI/act-decoder/zones/`, `img/GUI/act-decoder/file-list.json`)
- Exile-UI commit history for the zones folder: https://github.com/Lailloken/Exile-UI/commits/main/img/GUI/act-decoder/zones
- Act-Decoder wiki: https://github.com/Lailloken/Exile-UI/wiki/Act%E2%80%90Decoder
- JusKillmeQik PoE-Leveling-Guide, MIT, credits in README: https://github.com/JusKillmeQik/PoE-Leveling-Guide (`images/Act N/`)
- Engineering Eternity layout doc: https://docs.google.com/document/d/1sExA-AnTbroJ-HN2neZiij5G4X9u2ENlC7m_zf1tqP8/edit
- Cyclon's forum post: https://www.pathofexile.com/forum/view-thread/3869230 and guide: https://www.definitivguide.com/docs/category/path-of-exile-1 (e.g. https://www.definitivguide.com/docs/PoE1/Act%201/Coast)
- poewiki The Coast (Act 1), via API: https://www.poewiki.net/w/api.php?action=parse&page=The_Coast_(Act_1)&prop=wikitext ; site licence via `meta=siteinfo&siprop=rightsinfo` (CC BY-NC-SA 3.0); screenshot file page tagged `{{Copyright Grinding Gear Games}}`
- exile-leveling, MIT: https://github.com/HeartofPhos/exile-leveling (`common/data/routes/act-1.txt` … `act-10.txt`)
- Maxroll PoE1 campaign guide: https://maxroll.gg/poe/getting-started/campaign-guide
- PRODUCT.md (no GGG art in the in-game panels) and THIRD_PARTY_LICENSES.md, this repo
