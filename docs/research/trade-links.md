# How a pathofexile.com/trade link encodes a search

*Research for issue #66 (map #64) · November 2026*

**Question:** How can the app build a pathofexile.com/trade search URL for (a) a unique by name and base, and (b) a rare by base type or item category plus stat filters with minimums? The URL is opened in the user's browser, and the app makes no trade API call. Covers the URL and query format, length limits, how PoB mod lines map to trade stat ids, whether that mapping can be bundled (source, stability, licensing), and how PoB and Awakened PoE Trade solve the same problem.

**Sources:**
- GGG: [3.29.3b patch notes](https://www.pathofexile.com/forum/view-thread/4001117) (Kieren_GGG, 2026-09-04), the trade site's public data endpoints (`/api/trade/data/stats`, `/data/filters`, `/data/leagues`), and six `POST /api/trade/search/Covens` requests sent on 2026-11-25 to validate the example queries. Total: 9 requests to pathofexile.com, no credentials. The live captures are the source for every response detail below unless a line names another source.
- PathOfBuildingCommunity/PathOfBuilding at `dev` commit `5606cc6` (2026-11-24): `src/Classes/CompareBuySimilar.lua`, `TradeHelpers.lua`, `TradeQuery.lua`, `TradeQueryGenerator.lua`, `TradeQueryRequests.lua`, `src/Modules/Common.lua`, `src/Classes/Item.lua`, `src/Data/QueryMods.lua`, `src/Data/TradeSiteStats.lua`. PRs [PoB #10368](https://github.com/PathOfBuildingCommunity/PathOfBuilding/pull/10368) and [PoB-PoE2 #2502](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2/pull/2502).
- SnosMe/awakened-poe-trade at commit `db34867` (2026-11-22): `renderer/src/web/price-check/trade/pathofexile-trade.ts`, `CheckedItem.vue`, `TradeListing.vue`, `filters/create-stat-filters.ts`, `filters/pseudo/index.ts`, `renderer/src/assets/data/interfaces.ts`, `renderer/public/data/en/stats.ndjson`, `LICENSE`.
- Wayback Machine snapshots of `/api/trade/data/stats` from 2021-01-30 and 2024-10-02, used for the stability comparison.
- Cloudflare [connection limits](https://developers.cloudflare.com/fundamentals/reference/connection-limits/) (the trade site sits behind Cloudflare).

---

## 1. Short answer

- **Two URL forms work. Prefer the new one.** Since 3.29.3b (2026-09-04) the site encodes the whole search in the path: `https://www.pathofexile.com/trade/search/{league}/{base64url(gzip(JSON of the query object))}`. The older `…/trade/search/{league}?q={"query":{…}}` form is what Awakened PoE Trade still uses for "open in browser". But GGG says the old short-id URLs are being "phased out", and the PoB-PoE2 author warned that `?q=` may go too. PoB (both games) switched to the gzip path in Sept–Oct 2026.
- **The path segment is exactly what the search API returns.** `POST /api/trade/search/{league}` now answers with `id` = base64url, no padding, of the gzipped query JSON. For all three examples it decoded to the same JSON the app would build. So the app can build the URL offline, with no API call.
- **Query schema** (the part that matters here): `query.status.option` (`securable` = Instant Buyout, `available`, `online`, `any`), `query.name` (unique name), `query.type` (base type), `query.stats[]` groups `{type: "and"|"count"|"not"|"if"|"weight", value?: {min,max}, filters: [{id, value?: {min,max}|{option}, disabled?}]}`, and `query.filters.{type_filters, misc_filters, armour_filters, socket_filters, req_filters, trade_filters, …}.filters.{…}`. The trade site's own `/api/trade/data/filters` lists every filter id and option value.
- **Length is not a concern.** A 3-stat rare is 365 chars of JSON, 661 chars as `?q=` and 320 chars as a gzip path. Cloudflare allows 16 KB URLs, and the site itself caps a query at "complexity 200" (PoB's comment), which is reached long before 16 KB.
- **Stat ids are stable and computable.** `explicit.stat_3299347043` is `{type}.stat_{MurmurHash2 of the game's internal stat ids}`, with fixed seeds (PoB `Common.lua: HashStats`). I re-derived 4 live ids from the stat names (`base_maximum_life` → 3299347043, etc.). So an id only changes when GGG changes the underlying game stat, not between leagues or when the display text is reworded. Of the explicit ids live in Jan 2021, 92% still exist today, and those that vanished belong to removed mechanics.
- **A bundled mapping is fine.** PoB does exactly this. A maintainer step downloads `/api/trade/data/stats` and commits `TradeSiteStats.lua` (2.8 MB) and `QueryMods.lua`, and at runtime PoB only reads those files. APT ships `public/data/en/stats.ndjson` (9,923 entries) the same way. The app can generate its own mapping at build time from the public stats endpoint (2.1 MB JSON, 18,453 entries in 14 groups) and never touch the trade API at runtime. PoB and APT code are both MIT. The stat text itself is GGG game data ("Game data (c) Grinding Gear Games", as PoB's generated files say).
- **Mapping a mod line:** replace the numbers with `#` and look the template up in the stats list for the right group: PoB `findTradeHash` / APT `matchers`. Unmatched lines (about 80 explicit templates map to more than one id, plus PoB-only or legacy text) have to be dropped or put in a `count` group. PoB "Buy similar" puts an ambiguous line into a `count` group with `min: 1` over all candidate ids.
- **Pseudo stats** (`pseudo.pseudo_total_life`, `pseudo.pseudo_total_elemental_resistance`, `pseudo.pseudo_total_resistance`, per-element and attribute totals, 298 in all) sum a stat across implicit, explicit and crafted mods and convert "all resistances"/hybrid mods. That makes them the robust choice for life and resistances on rares. PoB's weighted generator maps single-resist and attribute hashes to the pseudo totals and drops hybrid and all-res stats for this reason.
- **`explicit.*` also matches crafted mods.** A live search for boots with `crafted.stat_3299347043` gave 10,000+ results. Adding `not explicit.stat_3299347043` gave 0. So a PoB `{crafted}` line can be searched with its `explicit.` id, which is what PoB does. `crafted.` only narrows the search.
- **Unique search:** `{"query":{"name":"Tabula Rasa","type":"Simple Robe", …}}`. An invalid name makes the site error, which is why PoB checks the name against its unique DB first. Strip PoB-only decorations ("Foulborn " prefix and trailing numbers) and add `misc_filters.foulborn_item` when it applies.

## 2. URL formats

### 2.1 The gzip path (current)

GGG, 3.29.3b patch notes (2026-09-04):

> Trade Site search URLs will no longer use short ids, and will instead encode the entire search. This means that these links will no longer expire, and you can share URLs without actually performing the search. Existing search URLs will still work for some time, but we will be phasing these out.

The encoding, from PoB `TradeHelpers.lua:649` (`B64GzipEncode`): gzip the JSON (`Deflate(str, true)`, gzip header), then base64 with `+`→`-` and `/`→`_`. The JSON is the **`query` object only**, not `{query, sort}`. PoB-PoE2 #2502: "This is similar to the `?q=` parameter, with the exception that it only has the query field. I.e. you can't set the sort order with it."

URL build, PoB `CompareBuySimilar.lua:196-206` / `TradeQueryRequests.lua:504`:

```
https://www.pathofexile.com/trade/search[/{realm} if not pc]/{league percent-encoded}/{b64gzip}
```

The league is percent-encoded (`Hardcore Covens` → `Hardcore%20Covens`). Realms other than PC are `xbox`/`sony`. Decoding works the other way (`B64GzipDecode`, `SearchWithURL`), which is how PoB reads a pasted trade URL.

Live check: the `id` the search API returned for each example query was the same base64url gzip stream as the one built locally. Only the gzip header's OS byte differed (Python writes `0xff`, the server `0x00`), plus trailing `=` padding, which the server leaves out. Both decode to the same JSON. Use unpadded base64url to match what the site produces.

### 2.2 The `?q=` form (legacy, still emitted by APT)

APT `CheckedItem.vue:259` and `TradeListing.vue:240`:

```ts
`https://${getTradeEndpoint()}/trade/search/${league}?q=${JSON.stringify(createTradeRequest(filters, stats))}`
```

Here the JSON is the full request (`{query, sort}`), and APT relies on the browser to percent-encode it. APT is actively maintained (last commit 2026-11-22) and still emits this form, so it presumably still loads. GGG has not promised to keep it, though. PoB-PoE2 #2502 (2026-09-29): "The query parameter still seems to exist, but I'm not convinced it will stay there forever." **Not verified in a browser** (see §7).

### 2.3 Length

| Example | JSON | `?q=` URL (percent-encoded) | gzip-path URL |
|---|---|---|---|
| Unique (name + type) | 123 | 273 | 196 |
| Rare, 3 stats + category | 365 | 661 | 320 |
| Rare, 1 `and` + 3-stat `count` | 465 | 831 | 352 |

Cloudflare's documented URL limit is 16 KB. The tighter limit is the trade site's query complexity: PoB `TradeQueryGenerator.lua:990` says "the trade site has a maximum complexity of 200 for each query", and its budget gives roughly 2 points per stat filter. A Build item has at most about 6 explicits, 2 implicits and an enchant, so it is far below both.

## 3. Query schema

APT's TypeScript type for the request (`pathofexile-trade.ts:91-140`) and PoB's builders agree. The pieces this feature needs:

```jsonc
{
  "query": {
    "status": { "option": "securable" },          // securable | available | online | onlineleague | any
    "name": "Tabula Rasa",                          // uniques only; or {discriminator, option}
    "type": "Simple Robe",                          // base type; optional for rares
    "stats": [
      { "type": "and",   "filters": [ { "id": "pseudo.pseudo_total_life", "value": { "min": 70 } } ] },
      { "type": "count", "value": { "min": 2 }, "filters": [ /* any 2 of these */ ] },
      { "type": "not",   "filters": [ { "id": "…" } ] }
      // also "if" and "weight" (weighted sum, value.min = minimum total; PoB's "Find best")
    ],
    "filters": {
      "type_filters": { "filters": { "category": { "option": "armour.boots" }, "rarity": { "option": "nonunique" } } },
      "misc_filters": { "filters": { "ilvl": { "min": 84 }, "corrupted": { "option": "false" }, "foulborn_item": { "option": "true" } } },
      "armour_filters": { "filters": { "ar": { "min": 500 }, "ev": {}, "es": {}, "ward": {} } },
      "socket_filters": { "filters": { "links": { "min": 6 } } },
      "trade_filters": { "filters": { "price": { "option": "divine" }, "collapse": { "option": "true" } } }
    }
  },
  "sort": { "price": "asc" }   // only in ?q= / API body; the gzip path carries no sort
}
```

- **Status values** come from `/api/trade/data/filters` → `status_filters.status`: `available`, `securable`, `onlineleague`, `online`, `any`. PoB "Buy similar" labels them "Instant Buyout" (`securable`), "Instant Buyout & In Person" (`available`), "In Person (Online)" (`online`) and "Any" (`CompareBuySimilar.lua:34-39`).
- **Category ids** (live `type_filters.category`): `armour.helmet`, `armour.chest`, `armour.gloves`, `armour.boots`, `armour.shield`, `armour.quiver`, `accessory.amulet`, `accessory.ring`, `accessory.belt`, `jewel`, `jewel.abyss`, `jewel.cluster`, `flask`, `weapon.bow`, `weapon.claw`, `weapon.dagger`, `weapon.runedagger`, `weapon.oneaxe`, `weapon.onemace`, `weapon.onesword`, `weapon.sceptre`, `weapon.staff`, `weapon.warstaff`, `weapon.twoaxe`, `weapon.twomace`, `weapon.twosword`, `weapon.wand`, `weapon.rod`, plus the broader parents `weapon`, `weapon.one`, `armour`, `accessory`, etc. APT's `CATEGORY_TO_TRADE_ID` and PoB's `getTradeCategory(slotName, item)` map item classes to these.
- **Rarity:** `normal`, `magic`, `rare`, `unique`, `uniquefoil`, `nonunique`. Rares should use `nonunique` (PoB and APT both do), so magic and normal items with the stats are also found.
- **Filter value:** `{min, max}` for numeric stats, `{option}` for option stats (e.g. timeless jewel seeds, "Allocates #" notables). A filter with no `value` just requires the stat to be present.
- **Invalid ids are errors.** A stat id the site does not know returns `400 {"error":{"code":2,"message":"Unknown stat provided: explicit.stat_1"}}`. A bundled mapping that goes stale (a stat GGG removed) therefore breaks the whole search, not just one filter. Generate the mapping and validate it at build time, and keep it fresh.

## 4. Stat ids: where they come from and how stable they are

### 4.1 The endpoint

`GET https://www.pathofexile.com/api/trade/data/stats` is public, unauthenticated, gzip-encoded on the wire and cached by Cloudflare (`cache-control: public, max-age=300`). It returns `{"result":[{"id","label","entries":[{"id","text","type", "option"?}]}]}`. Live on 2026-11-25:

| group | entries | | group | entries |
|---|---|---|---|---|
| pseudo | 298 | | crafted | 288 |
| explicit | 7,909 | | mercenary | 539 |
| implicit | 2,039 | | veiled | 20 |
| imbued | 175 | | delve | 81 |
| fractured | 1,778 | | ultimatum | 63 |
| enchant | 2,523 | | sanctum | 240 |
| scourge | 409 | | crucible | 2,311 |

Examples: `explicit.stat_3299347043` "+# to maximum Life", `explicit.stat_983749596` "#% increased maximum Life", `explicit.stat_3372524247` "+#% to Fire Resistance", `explicit.stat_2250533757` "#% increased Movement Speed", `pseudo.pseudo_total_life` "+# total maximum Life", `pseudo.pseudo_total_elemental_resistance` "+#% total Elemental Resistance", `pseudo.pseudo_total_resistance` "+#% total Resistance". The same number appears under every group (`implicit.stat_3299347043`, `crafted.stat_3299347043`, `fractured.stat_…`), so the group prefix chooses where on the item the stat must be.

Local vs global: 20 explicit entries carry a `(Local)` suffix (e.g. `explicit.stat_210067635` "#% increased Attack Speed (Local)"), which is a different stat from the global one. 80 explicit text templates map to more than one id (e.g. "+#% chance to Suppress Spell Damage", "#% increased Reservation Efficiency of Skills").

### 4.2 The hash

PoB `src/Modules/Common.lua:1120-1137`:

```lua
local GGG_STAT_HASH32_SEED = 0xC58F1A7B
local GGG_TRADE_SEED = 0x02312233
function HashStats(stats, extraStat)
	…
	for _, statName in ipairs(stats) do
		statHashes = statHashes .. intToBytes(murmurHash2(statName, GGG_STAT_HASH32_SEED))  -- little-endian u32
	end
	return murmurHash2(statHashes, GGG_TRADE_SEED)
end
```

`stats` is the list of internal stat ids on one stat-description entry (most have one). `murmurHash2` is standard 32-bit MurmurHash2 (m = `0x5BD1E995`, r = 24). Re-implemented in Python and checked against the live endpoint:

| internal stat id(s) | computed | live id |
|---|---|---|
| `base_maximum_life` | 3299347043 | `explicit.stat_3299347043` "+# to maximum Life" |
| `maximum_life_+%` | 983749596 | `explicit.stat_983749596` "#% increased maximum Life" |
| `base_fire_damage_resistance_%` | 3372524247 | `explicit.stat_3372524247` |
| `base_movement_velocity_+%` | 2250533757 | `explicit.stat_2250533757` |

So the id is a function of game data, not something the trade site assigns. It changes only when GGG renames or removes the internal stat.

### 4.3 Measured stability

Comparing archived copies of the endpoint with today's:

| snapshot | ids then | still present now | same text | explicit ids then → still present |
|---|---|---|---|---|
| 2021-01-30 | 7,031 | 6,459 (92%) | 4,759 | 4,204 → 3,874 (92%) |
| 2024-10-02 | 13,789 | 13,051 (95%) | 12,575 | 5,487 → 5,196 (95%) |

The explicit ids that disappeared belong to removed content (old map mods such as "Rare Monsters each have a Nemesis Mod", removed "Socketed Gems are Supported by Level # …" support mods, reworked jewels). Texts get reworded while the id stays: `explicit.stat_3917489142` went from "#% increased Rarity of Items found" to "+#% to Item Rarity". **Key the bundled mapping on the id and treat the text as a display/matching label that has to be refreshed each league.** The life, resistance, attribute and movement-speed ids used for gear are unchanged since at least 2021.

## 5. How PoB and APT map a mod line to an id

### 5.1 PoB "Buy similar" (closest to this feature)

`CompareBuySimilar.lua` opens a popup for one item and builds a gzip-path URL.

- **Unique** (`:73-94`): `name` = the item title with "Foulborn " and any trailing number stripped, then **verified against `main.uniqueDB`** ("the trade site will error on an invalid name"). `type` = `item.baseName`. Foulborn items add `misc_filters.foulborn_item`. No stat filters by default.
- **Rare/other** (`:95-145`): `type_filters.category` from `getTradeCategory(slotName, item)`. `type` (base) only if the user ticks "base type". Optional `misc_filters.ilvl`. Optional `armour_filters` ar/ev/es/ward, whose min defaults to the item's own value.
- **Mods** (`addModEntries`, `:214-298`): for each enchant, implicit, explicit and scourge line (crucible is disabled "due to matching difficulty"), it resolves `{range:x}` to the concrete value, then:
  1. `findTradeIdOption(line, type)` for option-type stats;
  2. otherwise `findTradeHash(line)` (`TradeHelpers.lua:249`) matches the line against **PoB's bundled game stat descriptions** (`Data/StatDescriptions/stat_descriptions`) with a full-line pattern. It returns the `HashStats` of the matching entry, the value (the canonical stat, which is not always the first number), and whether the value must be negated ("reduced" vs "increased");
  3. it builds `"{type}.{prefix}{hash}"` and **keeps it only if that id exists in `TradeSiteStats.lua`**.
  Lines with identical ids and type are summed into one entry (e.g. two flat-damage lines).
- **Filters** (`:147-190`): an entry with exactly one id goes into the `and` group. An entry with several candidate ids becomes its own `{type:"count", value:{min:1}}` group over all of them. Each row's Min defaults to the item's value (`:508`). **Rows start unchecked**, so the user opts in to each stat, and `invert` flips min/max and the sign.
- **Crafted lines** live in `item.explicitModLines` with a `{crafted}` flag (`Item.lua:1893`). They are searched as `explicit.` (`source.type`), which §6.3 confirms works.

### 5.2 PoB "Find best" (weighted)

`TradeQueryGenerator.lua` scores every candidate mod for the slot by its effect on the Build's DPS, EHP and so on, then emits a `{type:"weight", value:{min:minWeight}, filters:[{id, value:{weight}}]}` group. Single-resist and attribute stats are mapped to the pseudo totals (`pseudoMap`, `:995-1003`), and hybrid and "all" resist and attribute stats are dropped (`ignoredStats`). Its data is `Data/QueryMods.lua`, a 1.4 MB table of game mod → `tradeMod` id per item category. A maintainer regenerates it: `InitMods` downloads `/api/trade/data/stats` with libcurl, writes `TradeSiteStats.lua`, crosses it with PoB's mod data and commits the result (`:369-414`). At runtime PoB only `require`s the committed files. That is the pattern this app would copy, but weighted search needs PoB's calc engine, which is out of scope.

### 5.3 Awakened PoE Trade

- **Data:** `renderer/public/data/{en,ru,ko,cmn-Hant}/stats.ndjson`, committed to the repo. Each line is `{"ref":"+# to maximum Life","better":1,"matchers":[{"string":"# to maximum Life"}],"trade":{"ids":{"explicit":["explicit.stat_3299347043"],"implicit":[…],"fractured":[…],"crafted":[…]}}}`. Pseudo stats are entries too (`{"ref":"+# total maximum Life", "trade":{"ids":{"pseudo":["pseudo.pseudo_total_life"]}}}`). Matching swaps the numbers for `#` and looks the template up by FNV-1a hash of the matcher string (`make-index-files.mjs`, `assets/data/index.ts`). I found no public generator for these files, so they are prebuilt and committed. APT also generates them from game files rather than from the trade API alone (it includes `better`, mod families, reminder text).
- **Loosening minimums:** APT does not search exact values by default. `searchStatRange` defaults to **10** (`PriceCheckWindow.vue:115`, user-configurable, with "exact" = 0). For rares the default min is `floor(value − |value| × 10%)` (`percentRoll`). For uniques it is `value − 10% × (roll.max − roll.min)`, i.e. 10% of the roll range, and 0 when the roll is perfect. Bounds are clamped to the mod's possible range (`create-stat-filters.ts:271-330`).
- **Pseudo:** `filters/pseudo/index.ts` defines rules that fold matching explicit and implicit stats into pseudo totals (total elemental resistance, per-element, chaos, attributes, life, mana, ES, attack and cast speed, etc.). It then offers the pseudo filter in place of the individual lines.
- **License:** MIT, "Copyright (c) 2020 Alexander Drozdov" (`LICENSE`). PoB is MIT too. Copying either project's data or code needs their copyright notice. The stat text in both is GGG game data.

## 6. Worked examples (League "Covens", 2026-11-25)

All three queries were sent once to `POST /api/trade/search/Covens` (the endpoint the trade page itself calls) and all returned **200** with results. The returned `id` is the URL path segment.

### 6.1 Unique by name and base

```json
{"status":{"option":"securable"},"name":"Tabula Rasa","type":"Simple Robe","stats":[{"type":"and","filters":[]}]}
```

- gzip path: <https://www.pathofexile.com/trade/search/Covens/H4sIAAAAAAAAAzXMOwqAMBBF0a3Iq11BlqF2YjHREQKTD2ZSSMjeTQrbc-FWZCUtGaYiJnUxwCDzWR6ywmgzAnnutpEtQtNCmTBD3zRwdT4JT0u03HGc-mivf6Zwdb6dKD8jHO1oH6fdKUlxAAAA> (the server's own id. Live: total 3,274.)
- `?q=`: <https://www.pathofexile.com/trade/search/Covens?q=%7B%22query%22%3A%7B%22status%22%3A%7B%22option%22%3A%22securable%22%7D%2C%22name%22%3A%22Tabula%20Rasa%22%2C%22type%22%3A%22Simple%20Robe%22%2C%22stats%22%3A%5B%7B%22type%22%3A%22and%22%2C%22filters%22%3A%5B%5D%7D%5D%7D%7D>

### 6.2 Rare boots, 3 stat filters

Total life ≥ 70, total elemental resistance ≥ 60, movement speed ≥ 25%, any non-unique boots:

```json
{"status":{"option":"securable"},
 "stats":[{"type":"and","filters":[
   {"id":"pseudo.pseudo_total_life","value":{"min":70}},
   {"id":"pseudo.pseudo_total_elemental_resistance","value":{"min":60}},
   {"id":"explicit.stat_2250533757","value":{"min":25}}]}],
 "filters":{"type_filters":{"filters":{"category":{"option":"armour.boots"},"rarity":{"option":"nonunique"}}}}}
```

- gzip path: <https://www.pathofexile.com/trade/search/Covens/H4sIAAAAAAAAA31Q2wqDMAz9lzyLiOIEf2WIdJqNQG1cm44N6b8vzhdFWF9yOck56VkgiJEYoF2AZyF20ELAIXpzswgp--EKXxeQz4yKGjdCBneygn4DaNT2HDCOnG-hFxZje0t31NmXsRFXhYmUvilSyv4socUJ3Zp5DKTybjiTXHYk-J4tDST5empflnVRV1VTN6elsk6pS93u-O1T_a7epYMRfLD_HLwxfuLo8xuzuqL2eONJjiOOXXT0VOG0vi-SVq_tYwEAAA> (live: total 10,000, the API's cap)
- `?q=`: <https://www.pathofexile.com/trade/search/Covens?q=%7B%22query%22%3A%7B%22status%22%3A%7B%22option%22%3A%22securable%22%7D%2C%22stats%22%3A%5B%7B%22type%22%3A%22and%22%2C%22filters%22%3A%5B%7B%22id%22%3A%22pseudo.pseudo_total_life%22%2C%22value%22%3A%7B%22min%22%3A70%7D%7D%2C%7B%22id%22%3A%22pseudo.pseudo_total_elemental_resistance%22%2C%22value%22%3A%7B%22min%22%3A60%7D%7D%2C%7B%22id%22%3A%22explicit.stat_2250533757%22%2C%22value%22%3A%7B%22min%22%3A25%7D%7D%5D%7D%5D%2C%22filters%22%3A%7B%22type_filters%22%3A%7B%22filters%22%3A%7B%22category%22%3A%7B%22option%22%3A%22armour.boots%22%7D%2C%22rarity%22%3A%7B%22option%22%3A%22nonunique%22%7D%7D%7D%7D%7D%7D>

### 6.3 Same idea with a base type and a `count` group

Two-Toned Boots, total life ≥ 70, and **at least 2 of** {movement speed ≥ 25, total ele res ≥ 60, total chaos res ≥ 20}:

```json
{"status":{"option":"securable"},"type":"Two-Toned Boots",
 "stats":[{"type":"and","filters":[{"id":"pseudo.pseudo_total_life","value":{"min":70}}]},
          {"type":"count","value":{"min":2},"filters":[
            {"id":"explicit.stat_2250533757","value":{"min":25}},
            {"id":"pseudo.pseudo_total_elemental_resistance","value":{"min":60}},
            {"id":"pseudo.pseudo_total_chaos_resistance","value":{"min":20}}]}],
 "filters":{"type_filters":{"filters":{"rarity":{"option":"nonunique"}}}}}
```

- gzip path: <https://www.pathofexile.com/trade/search/Covens/H4sIAAAAAAAAA31RzYrCMBB-Fcm5K6VSCx73GbxJKdl0xIE4000m7krJu5tYdqlUzSUzk-9vyKi8aAle7UbFgyCT2ikPJjj9ZUHFQsl1gDTb__DHngn61SezeFXciYl3GP8gmvo0PqIVcNMD9mk8eAg9r6erExZtO4tHSNiLtgGy9RmTb1PG2MbiX89wIFmgqvjEA34HiwZlnUN1VVWX9WbT1M2SXcfs8DIYWDgD5cqBx6RGZhl0W74XMSfN_p1Add-0nS0yLd3N-lnptEO5PnwRMQXC76QZ87kBdi9D4scBAAA> (live: total 1,245)

### 6.4 Two further live checks

- An unknown id (`explicit.stat_1`) → `400 Unknown stat provided` (see §3).
- `crafted.stat_3299347043 ≥ 1` on boots → 10,000+ results. The same search plus `{"type":"not","filters":[{"id":"explicit.stat_3299347043"}]}` → **0**. So every item with crafted life also matches the explicit life filter: `explicit.*` covers crafted mods, and `crafted.*` is a narrower subset.

Rate limit headers on search: `X-Rate-Limit-Ip: 5:10:60,15:60:300,30:300:1800,600:21600:3600`. The app never calls this endpoint. Opening a link in the browser makes the *user's* browser do the search.

### 6.5 Building the gzip path (reference)

```python
import json, gzip, base64, urllib.parse
q = json.dumps(query, separators=(",", ":"))           # the query object, not {query, sort}
seg = base64.urlsafe_b64encode(gzip.compress(q.encode())).decode().rstrip("=")
url = f"https://www.pathofexile.com/trade/search/{urllib.parse.quote(league)}/{seg}"
```

In Rust: `flate2::write::GzEncoder` + `base64::engine::general_purpose::URL_SAFE_NO_PAD`. Both crates are pure Rust, with no network.

## 7. Not verified / open

- **The browser render was not checked.** The trade *page* (`/trade/search/...`) sits behind a Cloudflare JS challenge ("Just a moment…") for curl and headless Chromium, and no real browser was available to this session. The evidence that the links load is: the search API returns the gzip path segment as the search's own id; GGG's patch note says URLs now encode the whole search; and PoB (gzip path) and APT (`?q=`) ship these links to users today. Click one of the §6 links in a real browser before building on it, especially the `?q=` form.
- **How long `?q=` keeps working** is unknown. GGG only promised to phase out the old *short ids*. Build the gzip path.
- **Mapping coverage for PoB lines** was not measured. Neither was the share of a typical Build's rare mod lines that a template match finds a unique id for. PoB's own matcher notes known misses (Death's Oath, life flask immunity lines, `TradeHelpers.lua:13-15`).
- The query-complexity limit of 200 is from a PoB comment, not from a GGG document.

## 8. What this means for the app

- Build the URL as `https://www.pathofexile.com/trade/search/{league}/{base64url-nopad(gzip(query JSON))}`, from data bundled at build time. Opening it in the browser is not an app network request, so PRODUCT.md's constraint holds.
- League: the Character's league from the public profile the app already reads (`character.league`), percent-encoded.
- Bundle a mapping **template text → trade id(s) per group**, generated at build time from `/api/trade/data/stats` (and validated against it), keyed on the id. Refresh it each league. A stale id makes the whole search fail with a 400, so a failing build check beats a broken link.
- Uniques: `name` + `type` (base). Make sure the name is a real unique (strip PoB decorations).
- Rares: `type_filters.category` (or `type` when the base matters) + `rarity: nonunique` + stat filters. The strictness decision (next ticket) chooses between these options:
  - exact minimums vs loosened ones (APT: −10% of the value for rares, −10% of the roll range for uniques; PoB: exact, user-edited)
  - `and` vs `count` groups (PoB uses `count min 1` for ambiguous lines; a `count` of N−1 or N−2 over the item's mods is the usual "close enough" search)
  - pseudo totals for life/resistances/attributes (robust across implicit, explicit and crafted, and across hybrid and "all res" mods) vs exact explicit ids
  - what to do with lines that don't match (drop them and say so)
