# What pathofexile.com's get-items returns for socketed gems

*Research for issue #33 (map #31) · November 2026*

**Question:** What does the public `character-window/get-items` endpoint (no login, Characters tab public) return for a PoE 1 Character's socketed gems, and how does that map to a Path of Building Build's gems?

**Sources:**
- A live request on 2026-11-24 (UTC) for a public level 100 Standard Character (`PeakingDuck#4718` / `TheRealSlimDucky`, found through the public ladder API), sent with the app's User-Agent. Six requests in total to pathofexile.com, no credentials. All the response details below come from these captures unless a line names another source.
- GGG developer docs: [Type reference](https://www.pathofexile.com/developer/docs/reference) (`Item`, `ItemSocket`, `ItemProperty`) and [Introduction](https://www.pathofexile.com/developer/docs/index) (errors, rate limits). These describe the OAuth APIs. The `character-window` endpoints are undocumented, but their items use the same `Item` type.
- PathOfBuildingCommunity/PathOfBuilding at `dev` commit `f4a77c5` (2026-11-23): `src/Classes/ImportTab.lua` (`DownloadItems`, `ImportItem`, `ImportSocketedItems`), `src/Modules/Data.lua` (`setupGem`), `src/Data/Gems.lua`, `src/Classes/SkillsTab.lua` (save and load). Also tag `v2.35.0` for the old alternate-quality handling, and PRs [#7053](https://github.com/PathOfBuildingCommunity/PathOfBuilding/pull/7053), [#7189](https://github.com/PathOfBuildingCommunity/PathOfBuilding/pull/7189) and [#7043](https://github.com/PathOfBuildingCommunity/PathOfBuilding/pull/7043).

---

## 1. Short answer

- **Shape:** `{"items": [Item…], "character": {name, realm, class, league, level, lastLoginTime}}`. Each equipped item has `inventoryId` (`Weapon`, `Offhand`, `Weapon2`, `Offhand2`, `Helm`, `BodyArmour`, `Gloves`, `Boots`, `Belt`, `Amulet`, `Ring`, `Ring2`, `Flask`), `sockets` and `socketedItems`. Each gem is a full `Item` with a `socket` index.
- **Links:** `socketedItems[i].socket` is a 0-based index into the parent's `sockets` array. `sockets[n].group` is the link group, so gems with the same `group` on one item are linked. Empty sockets are left out of `socketedItems`. This is exactly how PoB builds its socket groups (`ImportTab.lua:1861`).
- **Names:** `typeLine` and `baseType` are the in-game display name, with " Support" on supports (`"Awakened Brutality Support"`). PoB's `nameSpec` drops " Support" (`"Awakened Brutality"`). There is **no metadata id** in the response, so PoB's `gemId`/`skillId`/`variantId` can only be found by matching names. Vaal gems carry `hybrid: {isVaalGem: true, baseTypeName: "Molten Shell"}`.
- **Level and quality** are strings in `properties`: `{"name":"Level","values":[["21 (Max)",0]]}` and `{"name":"Quality","values":[["+23%",1]]}`. Read the leading digits. When quality is 0 the Quality property is missing.
- **Weapon swap** gems come back on `Weapon2`/`Offhand2`. **Inventory** items did not appear in the public response, and PoB ignores them anyway. **Abyssal sockets** have `sColour`/`attr` `"A"` and hold an `abyssJewel: true` item instead of a gem.
- **Size:** 16 items, 26 gems and 1 abyss jewel came to 67.5 kB of JSON (15.6 kB gzipped on the wire).
- **Rate limits are NOT shared with get-passive-skills.** get-items uses policy `backend-item-request-limit`, `30:60:120,90:1800:600,180:7200:3600` per IP. get-passive-skills and get-characters share `backend-character-request-limit`, `15:60:120,90:1800:600,180:7200:3600`.
- **Errors:** private profile → **403**, unknown character → **404**. Both come back as the site's **HTML** page (`text/html`), not JSON. Both still count against the item rate limit.

## 2. Response shape (trimmed live sample)

Top level:

```json
{
  "items": [ /* 16 equipped items */ ],
  "character": {"name":"TheRealSlimDucky","realm":"pc","class":"Juggernaut","league":"Standard","level":100,"lastLoginTime":1637139787}
}
```

Boots with a 4-link holding a Vaal gem (icons, mods and description text cut):

```json
{
  "inventoryId": "Boots", "x": 0, "y": 0,
  "name": "Ghoul Track", "typeLine": "Two-Toned Boots", "baseType": "Two-Toned Boots",
  "sockets": [
    {"group":0,"attr":"S","sColour":"R"}, {"group":0,"attr":"S","sColour":"R"},
    {"group":0,"attr":"S","sColour":"R"}, {"group":0,"attr":"S","sColour":"R"}
  ],
  "requirements": [{"name":"Level","values":[["72",0]],"displayMode":0,"type":62,"suffix":"(gem)"}, "…"],
  "socketedItems": [
    {"socket":0,"colour":"S","support":true, "typeLine":"Cast when Damage Taken Support", "baseType":"Cast when Damage Taken Support", "…":"…"},
    {"socket":1,"colour":"S","support":false,"typeLine":"Immortal Call", "…":"…"},
    {"socket":2,"colour":"S","support":true, "typeLine":"More Duration Support", "…":"…"},
    {
      "socket": 3, "colour": "S", "support": false,
      "name": "", "typeLine": "Vaal Molten Shell", "baseType": "Vaal Molten Shell",
      "frameType": 4, "frameTypeId": "Gem", "corrupted": true, "ilvl": 0,
      "properties": [
        {"name":"Spell, AoE, Duration, Fire, Physical, Vaal, Guard","values":[],"displayMode":0},
        {"name":"Level","values":[["21 (Max)",0]],"displayMode":0,"type":5},
        {"name":"Cost","values":[["13 Mana",0]],"displayMode":0},
        {"name":"Quality","values":[["+23%",1]],"displayMode":0,"type":6}
      ],
      "hybrid": {"isVaalGem": true, "baseTypeName": "Molten Shell", "properties": ["…"], "explicitMods": ["…"]}
    }
  ]
}
```

These are the keys seen on gem entries: `abyssJewel, additionalProperties, baseType, colour, corrupted, descrText, explicitMods, frameType, frameTypeId, h, hybrid, icon, id, identified, ilvl, implicitMods, league, name, nextLevelRequirements, properties, rarity, requirements, secDescrText, socket, support, typeLine, verified, w`. Gems have no `inventoryId`, `x` or `y`. They do have `socket`, plus a `colour` (S/D/I/G per the docs).

## 3. Sockets, links and colours

From GGG's `ItemSocket` type and the live data:

| Field | Values | Meaning |
|---|---|---|
| `group` | 0, 1, 2… | Link group. Sockets that share a number are linked. |
| `attr` | `S`, `D`, `I`, `G`, `A`, `DV` | The socket's attribute (G = white/generic, A = abyss, DV = Delve resonator) |
| `sColour` | `R`, `G`, `B`, `W`, `A`, `DV` | The socket's colour |

Gem entries add:

| Field | Meaning |
|---|---|
| `socket` | 0-based index into the parent item's `sockets` |
| `colour` | The **gem's** attribute (S/D/I/G), not the socket's. A Dash gem in a white socket reads `"colour":"D"`. |
| `support` | `true` for supports, `false` for actives |

Live examples:
- Gloves `sockets` = `W0 W1 R2 R3`: four unlinked sockets, and only three of them hold gems. Each gem is its own group.
- Bow (`Weapon2`) `sockets` = `G G G R`, all group 0. `socketedItems` has sockets 0, 1 and 3. Socket 2 is empty and **not listed**, so don't assume the gems fill the sockets in order.

How to group gems the way PoB does (`ImportSocketedItems`, `ImportTab.lua:1831-1875`): for each non-abyss socketed item, `groupID = item.sockets[socketedItem.socket + 1].group` (Lua is 1-based). Gems are added to that group in `socketedItems` order. Each group becomes one PoB socket group whose `slot` is the item's slot name (`slotMap`, `ImportTab.lua:1498`: `Weapon`→"Weapon 1", `Offhand`→"Weapon 2", `Weapon2`→"Weapon 1 Swap", `Offhand2`→"Weapon 2 Swap", `Helm`→"Helmet", `BodyArmour`→"Body Armour", and so on). An item with `builtInSupport` ("Supported by Level 1 X", per GGG's type reference) gives that group an `imbuedSupport`.

## 4. Gem names compared with PoB's nameSpec / skillId / gemId

The response gives **only display names**: `typeLine`, `baseType` and `hybrid.baseTypeName`. It has no metadata path or skill id. A PoB build saves each gem as (`SkillsTab.lua:602-607`):

```xml
<Gem nameSpec="…" skillId="<grantedEffectId>" gemId="<gameId>" variantId="<variantId>" level="…" quality="…" …/>
```

PoB turns the API name into those ids with `data.gemForBaseName`, a lowercase name → PoB gem key table built in `Data.lua` `setupGem` (lines 1090-1104). Each gem is stored under its `name`, with " Support" added for supports (Barrage Support excepted), and also under its `baseTypeName` when that differs.

| Case | get-items says | PoB gem data (`Gems.lua`) | How PoB matches it |
|---|---|---|---|
| Normal active | `typeLine: "Earthquake"` | `name = "Earthquake"` | lowercase lookup of `typeLine` |
| Support | `"Fortify Support"`, `support: true` | `name = "Fortify"` (nameSpec has no " Support") | " Support" is added at table-build time, so `typeLine` matches |
| Awakened | `"Awakened Brutality Support"` (seen live) | key `Metadata/Items/Gems/SkillGemSupportAwakenedBrutality`, `name = "Awakened Brutality"`, `gameId = "Metadata/Items/Gems/SupportGemBrutalityPlus"` | same as a support. Watch out: PoB's internal key ≠ the `gameId` it saves as `gemId`. |
| Vaal | `typeLine: "Vaal Molten Shell"`, `hybrid: {isVaalGem: true, baseTypeName: "Molten Shell"}` (seen live) | `SkillGemVaalMoltenShell`, `vaalGem = true`, `secondaryGrantedEffectId` = the base skill | when `hybrid` is present, look up `hybrid.baseTypeName` to get the **base** gem, then map to its Vaal gem with `gemGrantedEffectIdForVaalGemId` (`ImportTab.lua:1842-1848`) |
| Transfigured | the transfigured name, e.g. `"Cyclone of Tumult"` (not seen live) | `SkillGemCycloneAltX`, `name = "Cyclone of Tumult"`, `gameId = "Metadata/Items/Gems/SkillGemCyclone"` (**same gameId as Cyclone**), `variantId = "CycloneAltX"` | plain name lookup. `variantId` is what tells it apart from Cyclone in a save (`SkillsTab.lua:466-480`; PR #7043) |
| Transfigured Vaal | per PR #7189, `hybrid.baseTypeName` is the base skill's display name from `activeskills.dat` (e.g. "Cleave of Rage" for Vaal Cleave of Rage) | `baseTypeName` is exported so this lookup works | same `hybrid` path as Vaal, which is why PoB resolves Vaal through the base gem (PR #7189, #7053) |
| Alternate quality | **gone from current PoE 1 and current PoB.** Up to PoB v2.35.0, `typeLine` had a prefix ("Anomalous X", "Divergent X", "Phantasmal X") that `GetBaseNameAndQuality` stripped and saved as `qualityId` | n/a in `dev` | today's `ImportSocketedItems` has no prefix handling, so a leftover alt-quality typeLine would just fail to match |

What this means for the app: to match an imported gem against a Build's `<Gem>`, compare **names**. Strip " Support" from `typeLine`, or compare against PoB's `nameSpec`. For Vaal gems, use `typeLine` for the Vaal skill and `hybrid.baseTypeName` for its base. Comparing against `gemId` would need PoB's `Gems.lua` name → gameId table, and even then transfigured variants share a gameId.

## 5. Level and quality

- `properties[name="Level"].values[0][0]` is a string such as `"20 (Max)"`, `"21 (Max)"`, `"17"` or `"1"`. It is the **gem's own level**, without +levels from gear (an Awakened gem at 5 reads `"5 (Max)"`). PoB takes the first number (`ImportTab.lua:1855-1856`).
- `properties[name="Quality"].values[0][0]` is like `"+20%"`. It is **absent** at 0% (the live level 1 Ensnaring Arrow has no Quality entry). PoB defaults to 0 (`ImportTab.lua:1851`).
- The first property is the tag line (`"Physical, Support"`) with empty `values`, so match on `name`, not position. The value pair's second element is GGG's value type (0 = default, 1 = augmented) per the `ItemProperty` type.
- `additionalProperties` holds experience (`{"name":"Experience","values":[["26272845/26272845",0]],"progress":1}`). `nextLevelRequirements` lists the next level's requirements.
- The parent item's `requirements` entries carry `"suffix":"(gem)"` when a socketed gem raises them, which is why PoB ignores item requirements when gems are socketed (`ImportTab.lua:1669`).

## 6. Weapon swap, inventory, jewels

- **Weapon swap:** the live response had `Weapon2` (Short Bow with 3 gems) and `Offhand2` (quiver). PoB imports them as "Weapon 1 Swap"/"Weapon 2 Swap" unless "ignore weapon swap" is ticked (`ImportTab.lua:1508`).
- **Inventory:** the public response held only equipped slots plus `Flask` (x = 0-4). No `MainInventory` items showed up. I couldn't confirm whether the public view always leaves out the inventory or this Character's bag was just empty. PoB drops any `inventoryId` that isn't in `slotMap`, so it never imports inventory gems either way.
- **Tree jewels** are not in get-items. They come from **get-passive-skills** `items` (`inventoryId: "PassiveJewels"`) next to `hashes`, `hashes_ex`, `mastery_effects`, `jewel_data` and so on (live). The app's `Passives` struct reads only `character` and `hashes` today.
- **Abyssal sockets:** Belt `sockets: [{"group":0,"attr":"A","sColour":"A"}]`, and `socketedItems: [{"socket":0,"abyssJewel":true,"typeLine":"Murderous Eye Jewel","frameType":2,…}]`, with no `colour`. Skip `abyssJewel` entries when collecting gems (PoB imports them as jewels, `ImportTab.lua:1836`).

## 7. Size

| Request | Status | Body (JSON) | On the wire |
|---|---|---|---|
| get-items, 16 items, 26 gems + 1 abyss jewel | 200 | 67,552 B | 15,640 B (gzip) |
| get-passive-skills, same Character | 200 | 19,096 B | n/a |
| get-items, unknown character | 404 | 17.9 kB HTML | 3.9 kB (br) |

Each gem carries full description text, mods, the `hybrid` copy and an icon URL, so the body grows with gem count. Expect roughly 50-100 kB for an endgame Character.

## 8. Rate limits

GGG's docs: limits are dynamic, so parse `X-Rate-Limit-Policy`, `X-Rate-Limit-Rules` and `X-Rate-Limit-{rule}` (`hits:period:penalty`), plus `-State` (`current:period:active-penalty`) and `Retry-After` on 429. Any 4xx (401/403/429 included) also counts toward an "invalid requests threshold" that can restrict a client.

Live headers (rule `Ip` on all three):

| Endpoint | `X-Rate-Limit-Policy` | `X-Rate-Limit-Ip` |
|---|---|---|
| get-items | `backend-item-request-limit` | `30:60:120,90:1800:600,180:7200:3600` |
| get-passive-skills | `backend-character-request-limit` | `15:60:120,90:1800:600,180:7200:3600` |
| get-characters | `backend-character-request-limit` | `15:60:120,90:1800:600,180:7200:3600` |

The `-State` counters confirm it. After get-characters and get-passive-skills, the character policy read `2:1800`. The item policy counted only get-items calls (including the 404 and the 403) and reached `3:1800`. So **adding a get-items call does not eat into the get-passive-skills budget**, but each policy alone allows only 90 hits per 30 min and 180 per 2 h per IP. One get-items per area change would hit the 30-minute cap after about 90 zones.

The responses also set a `POESESSID` cookie. The app can ignore it.

## 9. Errors

| Case | Status | Body |
|---|---|---|
| Private profile (`Jin_melike#2945`, ladder entry without `"public": true`) | **403** | site HTML page, `text/html` |
| Unknown character on a real account | **404** | site HTML "Resource Not Found" page |
| Rate limited (from docs, not triggered) | **429** | `Retry-After` header |

None of these return the `{"error":{"code":…,"message":…}}` JSON that GGG documents for its OAuth APIs. PoB treats 401/403 as "private or does not exist" and also checks for a literal `false` body (`ImportTab.lua:863-869, 919-920`). The app's existing `get()` in `src-tauri/src/poeapi.rs` already maps 403/404/429 this way, and `serde_json` failing on an HTML body is the same signal it uses for "Account not found".

One more catch (`ImportTab.lua:945`): get-characters matches the account name case-insensitively, but get-passive-skills and get-items are **case-sensitive**. PoB scrapes the profile page to fix the case. A character from the ladder that wasn't in the account's get-characters list (`TaylorSwiftVEVO`) also came back 404 from get-items.

## 10. Other clients

- **Path of Building** is the only open-source client I found that reads get-items gems. Everything above about mapping comes from it.
- **Awakened PoE Trade** reads item text from the clipboard (Ctrl+C), not from this endpoint, so it doesn't help with gems here.
- GGG's OAuth "Account Characters" API returns the same `Item` type with `equipment`/`inventory`/`jewels`. PoB reshapes get-items into that form (`charData.equipment = responseLua.items`, `ImportTab.lua:877-879`), so the field names above carry over if the app ever moves to OAuth.
