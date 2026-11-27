# How PoB build codes and paste links decode

*Research for issue #2 · November 2026*

**Question:** How does a Path of Building export code decode, and what in the build XML do we need (tree specs, class and ascendancy, tree version, masteries, cluster jewel nodes)? Can a desktop app fetch pastebin.com and pobb.in links raw?

**Sources:** PathOfBuildingCommunity/PathOfBuilding at commit `343b054` (2026-11-19). Every file path below is in that repo. Three real public builds were downloaded and decoded to check the format (see [Verification](#6-verification-against-real-builds)).

---

## 1. Short answer

- **Code format:** URL-safe base64 of the zlib-compressed build XML. Decode: change `-` to `+` and `_` to `/`, base64-decode, then zlib-inflate (zlib header, not raw deflate). Real codes start with `eN`, which is the zlib header `78 da`.
- **Links:** PoB keeps a table of sites. Each one maps a share URL to a raw URL that returns the same code as `text/plain`. pobb.in: `https://pobb.in/<id>` → `https://pobb.in/pob/<id>`. pastebin: `https://pastebin.com/<id>` → `https://pastebin.com/raw/<id>`. Both worked from plain `curl` with no auth and no special headers. Neither sends CORS headers, so fetch them from the Rust side, not from the webview.
- **Tree data:** `<Tree activeSpec="N">` holds one or more `<Spec>` elements. Each Spec has `title` (it can be missing), `treeVersion` (e.g. `3_27`), `classId`, `ascendClassId`, `secondaryAscendClassId`, `nodes` (comma-separated node ids) and `masteryEffects` (`{masteryNodeId,effectId},...`).
- **No allocation order.** `nodes` is a set: current PoB sorts it numerically, and older saves use hash-table order. Next points have to be worked out from the tree graph, as GLOSSARY.md already expects.
- **Cluster jewel nodes** have ids of 65536 or more. They come from the socketed cluster jewel item and are not in the tree data. Older builds may have no `nodes` at all and only a legacy `<URL>` tree link, which we also have to decode.

## 2. Decoding a build code

Export (`src/Classes/ImportTab.lua:505`, same in `src/Modules/Build.lua:1598`):

```lua
common.base64.encode(Deflate(self.build:SaveDB("code"))):gsub("+","-"):gsub("/","_")
```

Import (`src/Classes/ImportTab.lua:632`, `src/Modules/Common.lua:1099-1104`):

```lua
Inflate(common.base64.decode(buf:gsub("-","+"):gsub("_","/")))
```

`Deflate`/`Inflate` are host functions from SimpleGraphic. The stub in `src/_SimpleGraphic.def.lua:362-376` documents them. `Deflate(data, isGzip)` writes "the default ZLib header" unless asked for gzip. `Inflate` accepts zlib or gzip headers and "Raw DEFLATE data is not supported".

So a decoder should:

1. Trim whitespace (pastes often end in a newline).
2. Map `-`→`+` and `_`→`/` (or use a URL-safe base64 decoder), and add `=` padding if it is missing.
3. Inflate with zlib (Rust: `flate2::read::ZlibDecoder`; Python: `zlib.decompress(data)`).
4. Parse the result as UTF-8 XML whose root is `<PathOfBuilding>`.

`SaveDB` (`src/Modules/Build.lua:2277`) writes `<Build>` first, then one element per saver in key order: `Config`, `Import`, `Items`, `Notes`, `Party`, `Skills`, `Tree`, `TreeView`, ... (`Build.lua:629-641`). Older builds use a different element order, so look elements up by name and never by position.

## 3. Fetching links

The site table is in `src/Modules/BuildSiteTools.lua:19-42`. `DownloadBuild` turns a share link into its raw URL with `link:gsub(regexURL, downloadURL)`, downloads it, and passes the body to the same base64+inflate step.

| Site | Share link pattern (Lua) | Raw URL fetched |
|---|---|---|
| pobb.in | `pobb%.in/(.+)%s*$` | `https://pobb.in/pob/%1` |
| pastebin.com | `pastebin%.com/(%w+)%s*$` | `https://pastebin.com/raw/%1` |
| pastebinp.com (proxy) | `pastebinp%.com/(%w+)` | `https://pastebinp.com/raw/%1` |
| poe.ninja | `poe%.ninja/?p?o?e?1?/pob/(%w+)` | `https://poe.ninja/poe1/pob/raw/%1` |
| pob.codes | `pob%.codes/b/([^?%s]+)` | `https://api.pob.codes/%1/raw` |
| maxroll | `maxroll%.gg/poe/pob/(.+)` | `https://maxroll.gg/poe/api/pob/%1` |
| rentry.co | `rentry%.co/(%w+)` | `https://rentry.co/paste/%1/raw` |
| poedb.tw | `poedb%.tw/pob/(.+)` | `https://poedb.tw/pob/%1/raw` |

`ImportBuild` (`Common.lua:1087`) also unwraps YouTube and Google redirect links (`?q=`) before matching. PoB also registers a `pob://<siteId>/<id>` protocol handler (`BuildSiteTools.lua:103-140`).

Checked on 2026-11-21 with plain `curl` and no User-Agent:

- `https://pobb.in/pob/gcUBkpbe0-3H` → `200 text/plain`, the code itself. `https://pobb.in/<id>/raw` also returns `200 text/plain`. An unknown id returns `404 application/json`.
- `https://pastebin.com/raw/pNx11mVg` → `200 text/plain; charset=utf-8` (served through Cloudflare). An unknown id returns `404 text/html`.
- Neither response has an `Access-Control-Allow-Origin` header, so a webview `fetch()` from the Tauri app would be blocked by CORS. Do the request in Rust (`reqwest`, or `tauri-plugin-http`, which runs in Rust).

For our app: support pobb.in and pastebin first, using the same table approach so other sites are a single row each. If the input matches no site, treat it as a raw code.

## 4. What the XML holds

### `<Build>` (`Build.lua:1017`, `buildMode:Save`)

Attributes: `level`, `className`, `ascendClassName` (for the active spec), `targetVersion` (game data version, `3_0`; this is not the tree version), `mainSocketGroup`, `bandit`, pantheon, `viewMode`, `characterLevelAutoMode`. Child elements `PlayerStat`/`FullDPSSkill` are cached stats. We can ignore them.

### `<Tree activeSpec="N">` and `<Spec>` (`src/Classes/TreeTab.lua:499-545`, `src/Classes/PassiveSpec.lua:103-267`)

`activeSpec` is a 1-based index into the Spec list. The spec list is just the order of the `<Spec>` children. Each Spec is written as:

```xml
<Spec title="Crit" treeVersion="3_27" clusterHashFormatVersion="2"
      classId="5" ascendClassId="2" secondaryAscendClassId="0"
      nodes="61525,7918,10893,..." masteryEffects="{13862,12119},{38921,23442}">
  <URL>https://www.pathofexile.com/passive-skill-tree/AAAABgUC...</URL>
  <Sockets><Socket nodeId="2311" itemId="24"/>...</Sockets>
  <Overrides><Override nodeId="..." dn="Tattoo name" icon="..." activeEffectImage="...">mod line</Override></Overrides>
</Spec>
```

| Attribute / child | Meaning and quirks |
|---|---|
| `title` | Spec name. **Optional.** PoB shows `"Default"` when it is missing (`TreeTab.lua:494`). If the spec's tree version is not the latest, PoB puts `[3.22]` in front of the name in the list. |
| `treeVersion` | Tree data key such as `3_27`, `3_27_ruthless`, `3_27_alternate`. The full list is `treeVersionList` in `src/GameVersions.lua:11`; the latest is `3_30` in this commit. **Missing means `3_6`** (`defaultTreeVersion`). If PoB doesn't know the version, it refuses to load the build (`TreeTab.lua:512`). Different specs in one build can have different versions (one sample has `3_22` and `3_25`). |
| `classId` | 0-based index into the tree's `classes` array: 0 Scion, 1 Marauder, 2 Ranger, 3 Witch, 4 Duelist, 5 Templar, 6 Shadow. The class start node is the tree node with `classStartIndex == classId`. |
| `ascendClassId` | 1-based index into that class's `ascendancies` (0 = none). For example Templar 2 = Hierophant. |
| `secondaryAscendClassId` | Bloodline/alternate ascendancy. Can be the literal string `"nil"` (from `tostring(nil)`) in real files, so parse it leniently. |
| `nodes` | Allocated node ids, separated by commas. This **includes the class start node** (e.g. 61525 TEMPLAR) and ascendancy nodes. PoB reads it with `gmatch("%d+")`. Ids that are not in the chosen tree version are kept aside as possible cluster nodes (`PassiveSpec.lua:308-318`). **Order means nothing.** |
| `masteryEffects` | `{masteryNodeId,effectId}` pairs. PoB allocates a Mastery node only if it has a selection (`PassiveSpec.lua:309-313`). Effect ids of 65536 or more are dropped (`PassiveSpec.lua:296`). Effect text is looked up in the tree's `masteryEffects`. |
| `clusterHashFormatVersion` | Missing on a spec that has `nodes` means `1` (legacy cluster ids). PoB then remaps cluster node ids when it rebuilds subgraphs (`PassiveSpec.lua:106`, `1759-1829`). New saves write `2`. |
| `<URL>` | The official tree link in its binary format (v6). This is the **legacy fallback**: PoB decodes it only if the `nodes` attribute is missing (`PassiveSpec.lua:191-193`). It is always written for compatibility. |
| `<Sockets>` | `nodeId` → `itemId`, linking jewel sockets to `<Items><Item id=...>`. `itemId="0"` means an empty socket and is ignored. |
| `<Overrides>` | Tattoo replacements for individual nodes (new name and stats). Useful later for showing node names correctly. |

Builds older than multi-spec support can have a lone `<Spec>` instead of `<Tree>`. PoB loads it with `defaultTreeVersion` (`TreeTab.lua:501`).

### Legacy `<URL>` format (`PassiveSpec.lua:498-596`)

URL-safe base64 of: 4-byte big-endian version (≤6) · classId byte · ascendancy byte (`secondary<<2 | ascend`, v≥4) · then, for v≥5, a node count byte followed by 16-bit big-endian ids, then a cluster count byte followed by 16-bit ids (add 65536 to each), then for v6 a mastery count byte followed by pairs of `effectId,nodeId` (16-bit each). For v<5 every remaining byte pair is a node id. Our pastebin sample is a v4 link with no `nodes` attribute, so the fallback is needed for older builds.

### Node data

Node ids are the `skill` hashes from GGG's tree export. PoB ships the converted data for each version in `src/TreeData/<treeVersion>/tree.lua`, with node fields `name`, `stats`, `group`, `orbit`, `orbitIndex`, `in`/`out` links, `classStartIndex`, and so on. To draw the Mini-tree and work out the Next points we need the tree for each Spec's `treeVersion`. Getting that data is a separate ticket.

## 5. Cluster jewel nodes

Cluster nodes are not part of the tree data. PoB builds them when a cluster jewel item sits in an expansion socket (`BuildClusterJewelGraphs`, `BuildSubgraph` at `PassiveSpec.lua:1831`, `2055`). Their ids follow this scheme (`PassiveSpec.lua:2066-2080`):

```
bit 16    : 1 (always set, so ids ≥ 65536 never clash with tree hashes)
bits 9-10 : medium socket index (0-2)
bits 6-8  : large socket index (0-5)
bits 4-5  : cluster size index (0 small, 1 medium, 2 large)
bits 0-3  : node index within the cluster (0-11)
```

Example from a real build: `65568` = `0x10020` → large cluster, node 0. `66585` = `0x10419` → medium index 2, medium cluster, node 9.

To name or draw these nodes we would need PoB's cluster logic: parse the jewel item text from `<Items>`, use the tree's `expansionJewel`/proxy group data, and use PoB's cluster jewel data files. **Recommendation for v1:** tell cluster nodes apart by `id >= 65536`. Leave them out of the Mini-tree and the Next points, or list them as "cluster jewel nodes". Don't rebuild subgraphs.

## 6. Verification against real builds

Decoder: a 20-line Python script (`base64.urlsafe_b64decode` + `zlib.decompress` + `ElementTree`) run with `python3 -I`. All three inputs decoded without trouble. Each started with the zlib header `78 da`.

| Source | Result |
|---|---|
| `pobb.in/gcUBkpbe0-3H` (a leveling PoB from forum thread 3533131) | Templar/Hierophant lvl 34. `activeSpec="2"`. **Two specs, both untitled**, `treeVersion` `3_22` (45 nodes) and `3_25` (58 nodes). Masteries `{63268,6216},{50071,46493}`. No `clusterHashFormatVersion`. `secondaryAscendClassId="nil"`. |
| `pobb.in/1dX2SJQBtPTK` (league-start PoB, same thread) | One spec `title="Crit"`, `treeVersion="3_27"`, 125 nodes **in unsorted order**, including the class start 61525 and cluster ids 65568, 66585, 65578, 65561, 65570. Six `<Socket>`s. Two SkillSets. |
| `pastebin.com/pNx11mVg` (3.2-era Gladiator, forum thread 1753413) | **Legacy:** `<Spec>` with no attributes and no `nodes`, only a v4 `<URL>`. So the URL fallback and the `3_6` default version are needed. Sockets with `itemId="0"`. |

## 7. What later tickets can rely on

1. Input is either a URL (match it against the site table, then fetch the raw text in Rust) or a raw code.
2. Code → bytes: URL-safe base64 with lenient padding. Bytes → XML: zlib inflate.
3. Build object = list of Tree specs `{index, title ?? "Default", treeVersion ?? "3_6", classId, ascendClassId, nodes: Set<u32>, masteries: Map<u16,u16>}` plus `activeSpec`. Use the `<URL>` fallback when `nodes` is missing.
4. Node ids ≥ 65536 are cluster nodes, out of scope for v1.
5. There is no allocation order in the data. Next points have to come from graph distance or path logic over the tree for that `treeVersion`.
6. We need tree data for every version a build might use, at least the current league (`3_27`+) and possibly older ones. PoB's own fallback when a version is unknown is to refuse the build, so the app can do the same.
