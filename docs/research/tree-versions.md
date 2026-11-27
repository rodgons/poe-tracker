# Tree Versions: Build Tree vs Current Patch

*Path of Exile 1 · research for [issue #11](https://github.com/rodgons/poe-tracker/issues/11) · 20 November 2026*

**Question.** A Path of Building (PoB) build names the tree it was made on (the test build https://pobb.in/cYgy70giDjbo says `treeVersion="3_29"`). The game and RePoE follow the current patch (3.30). Which tree data should drive the next-point paths and the whole-spec mini-tree drawing? Should we bundle every tagged `grindinggear/skilltree-export` version and pick by `treeVersion`, or ship only the current one?

---

## 1. Short answer

**Use only the current tree** to drive paths and drawing. Treat a build's spec as a list of node hashes, and map those hashes onto the current tree when the build is imported. Then report any hashes the current tree no longer has. Don't bundle old versions.

Why:

- **The player allocates in the current game.** Client.txt names current-patch string ids, and only current-tree nodes can be allocated. A path drawn on an old tree could show a route the game no longer has.
- **Hashes are stable.** Across 3.27 → 3.28 → 3.29 → 3.29.1 → 3.30, no node hash was ever removed. 3.29.1 → 3.30.0 removed 0 edges and moved no connected tree node. The RePoE string id → hash mapping didn't change for any of the 2,987 shared nodes.
- **On real data, the result is the same.** All 127 tree nodes in the test build exist in 3.30.0, at identical positions. Shortest-path distances from each spec to the next are identical on both trees (895 of 895 targets).
- **This is what PoB does too.** PoB keeps every version, but it labels an old spec "older tree version, may not be fully compatible with the current game version" and offers to convert it. Converting replays the hash list onto the new tree and silently drops hashes that aren't there. We can do that conversion automatically at import, because we only care about the live game.

Keep a fallback: a build *can* contain a hash the current tree lacks (a future patch may remove nodes, though none of the last four did). The importer should list such hashes as "not in 3.30 tree" and leave them out of the plan, as PoB's convert does. It shouldn't fail.

## 2. The real 3.29.1 → 3.30.0 diff

Data: `data.json` at skilltree-export tags `3.29.1` (6,666,935 bytes) and `3.30.0` (6,709,327 bytes). The root node is excluded. A node's position is computed from its group `x,y` plus `orbit` and `orbitIndex`.

| Measure | 3.29.1 → 3.30.0 |
|---|---|
| Nodes | 3,389 → 3,461 |
| Added | **72** (48 small, 24 notable) |
| Removed | **0** |
| Moved (position changed by more than 0.5 units) | **5**, none in the connected tree that a build paths through (details below) |
| `orbit`/`orbitIndex` changed | 0 |
| Edges | 3,359 → 3,431: **72 added, 0 removed** |
| Nodes whose edge set changed | 6, all Large Jewel Sockets that gained a new branch |
| Renamed | 23, all Reliquarian ascendancy nodes (they rotate unique-item names each league) |
| Stats text changed | 104 (mostly ascendancy rebalancing) |
| Node type changed (notable, keystone, mastery, jewel, ascendancy) | 0 |
| Mastery effect ids (node, effect) | 1,837 → 1,837, none added or removed |
| Group ids | **2,718 of 3,389 surviving nodes have a different `group` number.** The coordinates are the same, but GGG renumbered the groups. |
| Tree bounds (`min_x` … `max_y`) | unchanged |

**Added nodes.** The 72 new nodes are the 3.30 league's "Mycelial Growth" branches (RePoE ids `blight_grown_notable_*`). They form 12 small clusters of 3 to 12 nodes. Each cluster is a dead end that hangs off a single Large Jewel Socket (2491, 7960, 21984, 32763, 46882, 55190). None of them links two old nodes, so **no shortest path between pre-existing nodes can change.**

**Moved nodes.** Of the 5 moved nodes:
- Two are blighted (anoint-only) notables with no edges: 53759 Cleansed Thoughts moved 1,468 units and 40849 Persistence moved 689. The tree never draws or paths them as regular nodes.
- Three moved 66 units each: Small Jewel Socket 24970 and its two Position Proxy nodes (3854, 25441).

**Other patches, for scale.** These used the same script:

| Diff | Added | Removed | Moved | Edges +/− |
|---|---|---|---|---|
| 3.27.0 → 3.28.0 | 57 | 0 | 7 | +57 / −0 |
| 3.28.0 → 3.29.0 | 52 | 0 | 55 (mostly Occultist ascendancy rework plus some small nodes) | +59 / −2 |
| 3.29.0 → 3.29.1 | 0 | 0 | 0 | 0 / 0 (5 stat-text changes) |
| 3.29.1 → 3.30.0 | 72 | 0 | 5 | +72 / −0 |

GGG grows the tree and keeps old hashes. Moves and edge removals do happen (3.29.0 had 55 moves and 2 removed edges), so the code must not *assume* they never happen.

## 3. The test build against 3.30.0

The PoB code from https://pobb.in/pob/cYgy70giDjbo is URL-safe base64 of zlib-compressed XML. It decodes to 269,159 bytes of XML holding 11 `<Spec>` elements, all `treeVersion="3_29"`, `classId="1"` (Marauder):

| Spec | Nodes | Tree hashes | Cluster hashes (≥ 65536) | Missing in 3.30.0 | Moved |
|---|---|---|---|---|---|
| CHECK POB NOTES | 1 | 1 | 0 | 0 | 0 |
| Lvl 01-30 | 34 | 34 | 0 | 0 | 0 |
| Lvl 31-40 RF Start | 51 | 51 | 0 | 0 | 0 |
| Lvl 41-60 | 80 | 80 | 0 | 0 | 0 |
| Lvl 61-80 | 110 | 110 | 0 | 0 | 0 |
| Lvl 81-90 | 122 | 122 | 0 | 0 | 0 |
| Lvl 90 Going Block Based | 122 | 122 | 0 | 0 | 0 |
| Lvl 95+ Small Cluster | 128 | 122 | 6 | 0 | 0 |
| Lvl 100 25% Effect Cluster | 133 | 127 | 6 | 0 | 0 |
| Lvl 100 Cogwork Ring | 133 | 127 | 6 | 0 | 0 |

- **Every tree hash in every spec exists in 3.30.0, at the same position.** All 21 distinct `masteryEffects` pairs `{node,effect}` also exist in 3.30.0.
- One build node changed stat text: 61355 Ramako, Sun's Light (Chieftain). That doesn't matter for paths or drawing.
- Hashes of 65536 and above are cluster-jewel subgraph nodes. They aren't in `data.json` in any version. PoB generates them from the socketed jewel (PoB puts unknown hashes in `allocSubgraphNodes`). The overlay must skip or handle them in every case; this has nothing to do with tree versions.
- **Path check.** For each spec *i*, we ran a breadth-first search (BFS) from spec *i−1*'s nodes over the main tree. Ascendancy and mastery nodes were excluded, and the search never crossed another class start. We then compared the hop distance to each node of spec *i*. Over 895 targets, the distances on 3.29.1 and 3.30.0 differ **0** times.

## 4. Client.txt string ids → hash (RePoE)

Client.txt prints `Successfully allocated passive skill id: life_regeneration1087, name: Life Regeneration`. skilltree-export `data.json` has **no** string id field. The id → hash map has to come from RePoE `passive_skill_trees/Default.json` (each entry in `passives` has `hash` and `id`).

We compared the RePoE exports for each patch: `Export 3.29.1.2.2` (commit `5dd4bfc`, 2,349,455 bytes) against `Export 3.30.0.2` (commit `0071545`, 2,405,026 bytes) in `repoe-fork/repoe-fork.github.io`.

| Measure | Result |
|---|---|
| Passives | 2,987 → 3,059 |
| String ids in both | 2,987 |
| Ids only in 3.29.1 (removed or renamed) | **0** |
| Ids only in 3.30.0 | 72 (the `blight_grown_*` nodes above) |
| Same id, different hash | **0** |
| Same hash, different id | **0** |
| Duplicate ids in 3.30 | none |

**The mapping is stable for this patch.** One caveat: RePoE's Default.json has every hash that has edges, but not 402 of the hashes in `data.json`. All 402 have no edges: cluster-jewel notables and keystones (Disciple of Kitava, Lone Messenger, …) and 38 masteries. They never show up as a normal allocation path. Because the RePoE ids follow the current patch, using the current tree keeps the id map, the paths and the drawing on **one** version.

## 5. What breaks when versions differ

| Change | If we use only the current tree | If we used the build's old tree |
|---|---|---|
| Node removed | That hash is missing from the plan. Report it ("1 node not in 3.30"); the remaining nodes still path. | The old tree draws a node and a path the player can't allocate. Client.txt never ticks it. |
| Node moved or edges rerouted | The path is correct for the game. The mini-tree draws the current layout, which matches the in-game tree. | The drawing and the route disagree with the in-game tree. The "next point" could be a node that isn't adjacent anymore. |
| Node added (3.30's 72) | Invisible to an old build. It's a dead end, so it never appears on a path. | Not present at all. Harmless either way. |
| Node renamed (Reliquarian) | Shows the current name, which is what Client.txt `name:` prints. | Shows a stale name. |
| String id renamed in RePoE | Not seen in 3.29.1 → 3.30.0. Hash lookup is unaffected. If it ever happens, a Client.txt line can still be matched through the current RePoE file. | A mismatch would be possible, because the id map (current) and the tree (old) would be different versions. |
| Group ids renumbered (2,718 nodes this patch) | Irrelevant if positions are computed when the data loads and nothing stores group ids. | Same. **Never persist group ids.** Store hashes only. |

## 6. Bundle cost per version

| Artifact | Size per version |
|---|---|
| skilltree-export `data.json` (raw) | 6.67 MB (3.29.1), 6.71 MB (3.30.0) |
| same, gzip -9 | 450 KB / 453 KB |
| Slim extract (hash → `[x, y, orbit, orbitIndex, out[], name]` plus orbit constants) | 172 KB / 176 KB |
| same, gzip -9 | **48 KB / 49 KB** |
| skilltree-export `assets/` sprite sheets (76 files) | 41.8 MB (3.29.1), 42.2 MB (3.30.0) |
| skilltree-export also has `ruthless.json`, `alternate.json`, `ruthless-alternate.json` | about 6.6 MB each |
| PoB `TreeData/3_30/tree.lua` | 2.10 MB (3_29: 2.92 MB), plus about 12 MB of images per version |
| RePoE `Default.json` (3.30.0) | 2.41 MB raw, 202 KB gzip |

There are about 30 skilltree-export tags (3.8.0 … 3.30.0). Bundling all of them as slim extracts would cost roughly 1.5 MB gzipped. That's affordable, but it buys nothing for a live-game overlay. As full `data.json` files it would cost about 200 MB raw. A mini-tree drawn as dots and lines needs no sprite sheets.

## 7. How Path of Building handles an older `treeVersion`

From PoB source, `PathOfBuildingCommunity/PathOfBuilding` at `dev` commit `343b054`:

- **Ships every version.** `src/GameVersions.lua` defines `treeVersionList` from `"2_6"` through `"3_30"`, plus ruthless and alternate variants, with `latestTreeVersion = treeVersionList[#treeVersionList]`. `src/TreeData/` has one folder per version (`3_29/`, `3_30/`, …).
- **Loads the spec's own version, lazily.** `PassiveSpec:Init(treeVersion)` calls `main:LoadTree(treeVersion)`. `Main.lua` `LoadTree` builds a `PassiveTree` the first time a version is used and caches it in `self.tree[treeVersion]`.
- **Warns and offers to convert.** `TreeTab.lua` sets `self.showConvert = not curSpec.treeVersion:match("^" .. latestTreeVersion)`. It then shows the label *"This is an older tree version, which may not be fully compatible with the current game version."* next to the buttons *"Convert to 3.30"* and *"Convert all trees to 3.30"*. The spec dropdown prefixes old specs with `[3.29]`. Picking a version in the version dropdown opens `OpenVersionConvertPopup`: *"Warning: some or all of the passives may be de-allocated due to changes in the tree."* with the choices Convert, Copy + Convert and Cancel.
- **What convert does.** `TreeTab:ConvertToVersion` creates a new `PassiveSpec` on the target version and calls `RestoreUndoState(oldSpec:CreateUndoState(), version)`, which leads to `ImportFromNodeList`. For each hash: `if node then node.alloc = true … else t_insert(self.allocSubgraphNodes, id)`. Hashes missing from the new tree are simply not allocated. Masteries are allocated only if their selection still exists. Afterwards: *"Note that some or all of the passives may have been de-allocated due to changes in the tree."*

So PoB keeps old trees because it's a planner: you may want to view an old build exactly as it was written. Its own answer for "play it now" is to convert the build to the current tree by hash, which is the approach recommended here.

## 8. Recommendation for poe-tracker

1. Bundle **one** tree: the current skilltree-export tag as a slim extract (about 50 KB gzipped), plus the current RePoE `Default.json` id → hash map (or a slim `{id: hash}` extract). Bump both together when a patch lands.
2. At import, read each `<Spec>`'s `nodes` and `masteryEffects`. Skip hashes of 65536 and above (cluster subgraphs). Look up each remaining hash in the current tree, then keep three lists: present, missing, and moved (compare the stored position only if we ever keep one). If the spec's `treeVersion` isn't current, show a PoB-style notice: "Built on 3.29, shown on the 3.30 tree; N nodes not in this tree."
3. Run pathing (next points) and the mini-tree drawing on the current tree only. Compute positions from the groups when the data loads, and never persist group ids.
4. Don't bundle old versions. If a "view as authored" mode is ever wanted, fetch that tag's `data.json` on demand. Don't ship it.

## 9. Method and sources

- [grindinggear/skilltree-export](https://github.com/grindinggear/skilltree-export) — `data.json` at tags `3.27.0`, `3.28.0`, `3.29.0`, `3.29.1`, `3.30.0`. Tag list from the GitHub API. Sizes from the files and the git tree API (`assets/`).
- [repoe-fork/repoe-fork.github.io](https://github.com/repoe-fork/repoe-fork.github.io) — `data/passive_skill_trees/Default.json` at commits `5dd4bfc` ("Export 3.29.1.2.2", 2026-08-04) and `0071545` ("Export 3.30.0.2", 2026-11-16).
- [PathOfBuildingCommunity/PathOfBuilding](https://github.com/PathOfBuildingCommunity/PathOfBuilding) at `343b054` — `src/GameVersions.lua`, `src/Classes/TreeTab.lua` (lines about 325–342, 572, 593–617, 654–673), `src/Classes/PassiveSpec.lua` (`Init`, `ImportFromNodeList` lines about 274–330, `RestoreUndoState`), `src/Modules/Main.lua` (`LoadTree`), and the `src/TreeData/` listing.
- Test build: https://pobb.in/pob/cYgy70giDjbo (raw PoB code), decoded locally.
- `Client.txt` line format: [overlay-field-guide.md](overlay-field-guide.md) §Source 1.
- Analysis scripts were throwaway Python run on the downloaded files, covering the diff, spec membership, id map comparison, BFS distance comparison and slim-extract sizing. The position formula assumed uniform orbit angles. That only matters for movement detection, and since no surviving node changed `orbit` or `orbitIndex`, it doesn't affect the moved count.
