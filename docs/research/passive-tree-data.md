# Passive tree data: ids, hashes and positions

*Path of Exile 1 · research for issue #3 · checked 2026-11-20 against game patch 3.30.0*

**Question.** PoB stores passive nodes as numeric hashes. `Client.txt` logs a string id plus a display name, for example `Successfully allocated passive skill id: life_regeneration1087, name: Life Regeneration`. Where does the tree data live, what does it give per node, and can we turn the logged string id into a PoB hash?

## 1. Short answer

| Answer | Claim |
|---|---|
| **Yes** | GGG's `grindinggear/skilltree-export` `data.json` has every node keyed by numeric hash, with name, group/orbit/orbitIndex (for x,y), `out`/`in` edges, ascendancy, and mastery/notable/keystone flags. It is versioned with one git tag per patch. |
| **No** | `data.json` has **no** string id. PoB's `tree.lua` is the same data converted to Lua and has no string id either. |
| **Yes, exact** | The game's own `PassiveSkills.dat64` table has both `Id` (the logged string) and `PassiveSkillGraphId` (the hash). RePoE publishes it as JSON: `life_regeneration1087` → hash **55906**. All 3,059 string ids are unique, and every node that has a tree position is covered. |
| **Fallback only** | Display-name matching is ambiguous for 58% of nodes (mostly small nodes like "Life" or "Strength"). It works for keystones and almost all notables. Narrowed to the build's Next points it is usually enough, but it shouldn't be the main path. The numeric suffix in the id is **not** the hash (0 of 2,821 match). |

Recommendation: at build time, join GGG `data.json` (positions, edges) with RePoE `passive_skill_trees/Default.json` (string id → hash) into one bundled table per patch. At runtime, look up `id:` from the log. If the id is unknown, fall back to matching the name against the remaining Next points.

## 2. GGG's tree export (`grindinggear/skilltree-export`)

Source: <https://github.com/grindinggear/skilltree-export>. It has `data.json` (main tree), `alternate.json`, `ruthless.json`, `ruthless-alternate.json` and `assets/`. A separate repo, `atlastree-export`, holds the atlas tree.

**Versioning.** There is one git tag per release: `3.30.0`, `3.29.1`, `3.29.0`, `3.28.0`… plus preview commits (latest on `master`: `550fb34a "3.30.0 preview"`). Format changes are listed under "Notable Changes" in the repo README. For example, 3.17.0 changed orbits 2 and 3 from 12 to 16 slots, 3.18.1 moved to spritesheets, and 3.20.0 added group `background`. Node hashes are mostly stable between leagues, but nodes do get added and removed, so pin data to a tag and match it to the PoB build's `treeVersion` (e.g. `3_30`).

**Top-level keys** in the current `data.json` (6.7 MB): `tree` (`"Default"`), `classes` (with ascendancies), `alternate_ascendancies`, `groups`, `nodes`, `extraImages`, `jewelSlots`, `min_x/min_y/max_x/max_y`, `constants`, `sprites`, `imageZoomLevels`, `points` (`totalPoints: 123, ascendancyPoints: 8`).

**Nodes.** There are 3,462 entries keyed by the hash as a string, including a synthetic `root` whose `out` lists the class starts. Here is how often each field appears:

| Field | Count | Meaning |
|---|---|---|
| `skill` | 3461 | numeric hash (same as the key; same number PoB stores) |
| `name`, `icon`, `stats` | 3461 | display name, icon path, stat lines |
| `group`, `orbit`, `orbitIndex` | 3060 | placement (missing = node not drawn on the tree, see below) |
| `out`, `in` | 3060 | edges as hash strings (draw an edge for each pair once) |
| `isNotable` / `isKeystone` / `isMastery` / `isJewelSocket` | 1021 / 57 / 353 / 60 | node type |
| `ascendancyName` | 558 | ascendancy nodes (drawn in their own cluster, skip for the Mini-tree) |
| `masteryEffects` | 315 | mastery choices (PoB stores the chosen effect separately) |
| `classStartIndex`, `isAscendancyStart`, `isProxy`, `isBloodline`, `expansionJewel`, `grantedPassivePoints`, `recipe`, `reminderText`, `flavourText`, … | various | extras |

The 402 nodes without a `group` are cluster-jewel and timeless-jewel notables, such as "Disciple of Kitava" and "Veteran's Awareness". None of them sit on the drawn tree.

Example (`57283`):

```json
{"skill": 57283, "name": "Life Regeneration", "group": 146, "orbit": 2, "orbitIndex": 0,
 "in": ["26270", "33943"], "out": [], "stats": ["Regenerate 0.6% of Life per second"]}
```

**Groups**: `{"x": -14159.2, "y": 8413.88, "orbits": [0], "nodes": ["28609"], "background": {...}}`.

**Constants**: `skillsPerOrbit: [1, 6, 16, 16, 40, 72, 72]`, `orbitRadii: [0, 82, 162, 335, 493, 662, 846]`, `PSSCentreInnerRadius: 130`.

## 3. Computing node x,y

This is taken from PoB, `src/Classes/PassiveTree.lua` (`CalcOrbitAngles`, and the node-position block):

```
angle = orbitAngles[skillsPerOrbit[orbit]][orbitIndex]   (degrees)
r     = orbitRadii[orbit]
x     = group.x + r * sin(angle)
y     = group.y - r * cos(angle)      // y grows downward; angle 0 = straight up, clockwise
```

Angle tables:

- **16 slots** (orbits 2 and 3): `0,30,45,60,90,120,135,150,180,210,225,240,270,300,315,330` (from the GGG README, 3.17.0).
- **40 slots** (orbit 4): `0,10,20,30,40,45,50,60,…` (every 10° plus 45/135/225/315; PoB hardcodes this list, GGG's README doesn't document it).
- **Other orbits**: uniform spacing, `360 * i / n`.

Checked against the data: running this over all 3,059 placed nodes gives x in [-14159, 12748] and y in [-10775, 10900]. That is close to `min_x..max_y` (those bounds look like group centres). Node 57283 lands at (-7403.3, -3172.8). For the Mini-tree, take the bounding box of the Next points plus nearby Allocated nodes and scale it to fit.

## 4. PoB Community's data

- PoB ships `src/TreeData/<ver>/tree.lua` (latest `3_30`, `3_30_ruthless`), which is GGG's `data.json` converted to Lua, plus spritesheets. A node looks like `[57283] = { group=146, name="Life Regeneration", orbit=2, orbitIndex=0, skill=57283, … }`. There is **no string id**: `life_regeneration` appears 0 times in the 2 MB file.
- PoB builds store allocations as hashes. `PassiveSpec.lua` reads `<Spec nodes="…">` (a comma list of hashes), `masteryEffects="{mastery,effect},…"` and `treeVersion`.
- PoB's dat spec (`src/Export/spec.lua`, table `passiveskills`) shows the game table's columns: `Id` (String), `Icon`, `Stats`, `Stat1..4`, `PassiveSkillNodeId` (UInt16; this is the hash), `Name`, `ClassStart`, `Keystone`, `Notable`, …. PoB doesn't export the `Id` column into its tree data.

## 5. The mapping: `PassiveSkills.dat64` via RePoE

The string in `Client.txt` is the `Id` column of the game's `PassiveSkills.dat64`, and the same row holds the hash. RePoE (maintained fork <https://github.com/repoe-fork/repoe>, published at <https://repoe-fork.github.io/>) exports it. Its `RePoE/parser/modules/passives.py` maps `"Id" → "id"` and `"PassiveSkillGraphId" → "hash"`, and writes `passive_skill_trees/<TreeId>.json`.

The data as checked: `https://repoe-fork.github.io/passive_skill_trees/Default.json` is 2.4 MB, its `version.txt` says `3.30.0.4`, and it was last modified 2026-11-20. Results:

| Check | Result |
|---|---|
| Entries in `passives` | 3,059, keyed by hash |
| `life_regeneration1087` | → hash **55906**, "Life Regeneration" (group 442 in GGG data) |
| String ids unique | yes |
| Overlap with GGG hashes | 3,059 / 3,059 present in `data.json` |
| GGG nodes not in RePoE | 402, all of them the groupless jewel nodes above |
| Name disagreements on shared hashes | 0 |
| Id formats | `twin_terrors1030`, `mastery_life135`, `shield_defences_2792`, `AscendancyAscendant19`, `AscendancyGuardianStart` |

RePoE's file also carries its own `groups` (x, y, `radius`, `position_clockwise`, `connections`) read straight from the game's `.psg` graph. It could replace `data.json` entirely, but GGG's export is the documented first-party source and is what PoB uses, so keep it for geometry.

**Risks.** RePoE is community-run, not GGG. It lags patches by hours to days, and a hotfix could rename ids. For a backup that doesn't depend on RePoE, read `PassiveSkills.dat64` from the game install ourselves with a dat reader (the column layout is in PoB's `spec.lua` or poe-tool-dev's dat-schema). That reads game files but doesn't change them, which fits the ground rules, but it is more work. Check the bundled table against the patch tag at build time.

## 6. Fallbacks, quantified (GGG `data.json`, 3,461 real nodes)

**Display name.** There are 1,911 distinct names, and 1,446 of them belong to exactly one node. In all, 2,015 nodes (58%) share their name with at least one other node.

| Node kind | Nodes | Share a name |
|---|---|---|
| Keystone | 57 | 0 |
| Notable | 1,021 | 34 |
| Mastery | 353 | 315 |
| Small (not notable/keystone/mastery/socket) | 1,970 | 1,606 |

The most repeated names are Intelligence (73), Strength (68), Dexterity (67), Position Proxy (42), Life (30), Energy Shield (24) and Mana (19). "Life Regeneration" alone covers 14 nodes. Name matching is fine for keystones and notables. For small nodes it only works if you narrow it to the build's Next points, where two Next points rarely share a name, and adjacency to Allocated nodes can break the tie.

**Numeric suffix.** It is not the hash: 0 of 2,821 suffixed ids match their hash. It looks like an internal row or serial number. Don't use it.

## Sources

- GGG tree export: <https://github.com/grindinggear/skilltree-export> (`data.json` at `master` = 3.30.0 preview; README "Notable Changes"; tags per patch)
- PoB Community: `src/TreeData/3_30/tree.lua`, `src/Classes/PassiveTree.lua` (`CalcOrbitAngles`, node position), `src/Classes/PassiveSpec.lua` (Spec XML), `src/Export/spec.lua` (`passiveskills` columns): <https://github.com/PathOfBuildingCommunity/PathOfBuilding> (branch `dev`)
- RePoE fork: <https://github.com/repoe-fork/repoe> (`RePoE/parser/modules/passives.py`); data <https://repoe-fork.github.io/passive_skill_trees/Default.json>, <https://repoe-fork.github.io/version.txt>
- Counts in this note come from local analysis of the files above, downloaded 2026-11-20.
