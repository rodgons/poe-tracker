# How PoB stores items and Item sets

*Research for issue #65 (map #64) · November 2026*

**Question:** How does a Path of Building (PoE 1) export store items and Item sets? Covers the `<Items>` / `<Item>` text format (rarity, name, base, implicits, mod lines, `{…}` line tags, Corrupted, influences, variants), `<ItemSet>` and `<Slot>` names (gear, flasks, weapon swap, abyssal sockets), how tree jewels tie to sockets, how the active Item set is chosen, and how Item sets pair with Tree specs and Skill sets.

**Sources:** PathOfBuildingCommunity/PathOfBuilding branch `dev` at commit `5606cc6` (2026-11-24). Every file path below is in that repo. The latest release, v2.69.0 (2026-11-25), has the same `ItemsTab` Load/Save. Checked against the test build https://pobb.in/cYgy70giDjbo (Pohx RF Marauder, same build as `src-tauri/tests/pohx-rf.txt`), fetched from `/raw`, base64url-decoded and zlib-inflated with `python3 -I`. Pairing rules come from the Skill sets research (#32, branch `research/pob-skill-sets`, `docs/research/pob-skill-sets.md`).

---

## 1. Short answer

- **Shape:** `<Items activeItemSet="ID" useSecondWeaponSet showStatDifferences>` holds a flat **pool** of `<Item id>` elements, then `<ItemSet id title useSecondWeaponSet>` elements, then an optional `<TradeSearchWeights>`. An Item set is just a list of `<Slot name itemId active itemPbURL>` pointing into the pool. `itemId="0"` means the slot is empty. One item can sit in several sets, and in two slots of the same set.
- **Item text:** each `<Item>` has PoB's own item text as its text content (not the game's clipboard format), plus `<ModRange id range>` children (legacy roll positions). The order is fixed by `Item:BuildRaw()`: `Rarity:`, name, base (rare/unique only), header lines (`Crafted: true`, `Prefix:`/`Suffix:` affix ids, `Variant:` list, `Selected Variant:`, `Quality:`, `Sockets:`, `LevelReq:` …), `Implicits: N`, then N implicit-type lines, then explicit lines, then trailing flags (`Split`, `Mirrored`, `Fractured Item`, `Corrupted`). Influence lines (`Shaper Item`, `Searing Exarch Item` …) come in the header.
- **Line tags:** each mod line can start with `{…}` tags: `{variant:1,2}`, `{range:0.5}`, `{tags:fire,damage}`, `{crafted}`, `{fractured}`, `{custom}`, `{prefix}`, `{suffix}`, `{exarch}`, `{eater}`, `{mutated}` and others. Strip them all for display. `{variant:n}` lines only count when `n` is the item's selected variant.
- **Slots:** `Weapon 1`, `Weapon 2`, `Helmet`, `Body Armour`, `Gloves`, `Boots`, `Amulet`, `Ring 1`, `Ring 2`, `Ring 3`, `Belt`, `Graft 1`, `Graft 2`, `Flask 1`–`Flask 5`, plus `Weapon 1 Swap`, `Weapon 2 Swap` and `<parent> Abyssal Socket 1`–`6` for both weapons (and swaps), Helmet, Body Armour, Gloves, Boots and Belt. Flask slots carry `active="true"` when switched on.
- **Tree jewels are not in Item sets.** They live in each Tree spec: `<Spec>` › `<Sockets>` › `<Socket nodeId itemId>`. The item comes from the shared `<Items>` pool. A jewel only counts when its socket node is allocated in that spec.
- **Active set:** `activeItemSet` is an ItemSet **`id`** (not a position). If it is missing or unknown, PoB uses the first set in document order.
- **Pairing:** exactly the same rule as Skill sets (#32). PoB links Tree spec, Item set, Skill set and Config set by the first `{ids}` in each title, else by identical raw titles, else a lone set pairs with everything (`Build.lua` `SyncLoadouts()`). There is no `<Loadout>` element.
- **Trade links already exist in the format:** `itemPbURL` on each `<Slot>` (and `<SocketIdURL>` for tree sockets) stores a pathofexile.com trade search URL that PoB's Trader saved for that slot in that Item set. It is empty in the test build.
- **Test build:** 135 items (69 rare, 55 magic, 11 unique), 9 Item sets, `activeItemSet="3"` ("^4Level 1-30 {1}"). Item set ids `{1}`–`{5}`, `{6,7}`, `{8}`, `{9}`, `{10}` cover every Tree spec id, so in this build Item sets never block a loadout. Most rares are named "New Item": they are author-made templates to buy or craft, not real items.

## 2. XML shape

### `<Items>` (`src/Classes/ItemsTab.lua:1283-1413` Load, `:1415-1500` Save)

| Attribute | Meaning |
|---|---|
| `activeItemSet` | `id` of the selected ItemSet. `SetActiveItemSet` (`:1714-1747`) falls back to `itemSetOrderList[1]` (the first `<ItemSet>` in the document) if the id is unknown. |
| `useSecondWeaponSet` | Copy of the active set's flag. Only used if the file has no `<ItemSet>` at all (`:1403-1406`). |
| `showStatDifferences` | UI preference. |

Children, in the order Save writes them:

1. `<Item>`, in `itemOrderList` order (the item list order in the Items tab).
2. `<ItemSet>`, in `itemSetOrderList` order. Ids are not renumbered: the test build's sets appear in order with ids 3, 4, 6, 7, 8, 1, 2, 5, 9.
3. `<TradeSearchWeights>` › `<Stat label stat weightMult>`: Trader stat weights, not build content.

A bare `<Slot>` directly under `<Items>` is the pre-Item-set format. Load still reads it as "legacy compatibility" (`:1357-1367`).

Load keeps an `<Item>` only if PoB recognises its base (`if item.base then`, `:1351-1355`). An item with an unknown base is dropped silently, so a `<Slot>` can point to an id that PoB itself treats as empty.

### `<Item>`

| Attribute / child | Meaning |
|---|---|
| `id` | Integer, unique within the build. Referenced by `<Slot itemId>` and `<Socket itemId>`. |
| `variant` | Selected variant (1-based), same value as the `Selected Variant:` text line. |
| `variantAlt` … `variantAlt5` | Second to sixth variant picks, for uniques that choose several mods (Watcher's Eye, Forbidden Flame/Flesh, …). Match the `Selected Alt Variant[ Two…Five]:` lines. |
| text content | The item text (section 3). PoB indents the first line with tabs; trim every line. XML entities (`&apos;`) are normal XML escaping. |
| `<ModRange id range>` | Legacy roll position of mod line `id` (counted across enchant, scourge, implicit, explicit, crucible lines). The same value is also in the text as `{range:…}`. Ignore `<ModRange>` and read `{range:}`. |

### `<ItemSet>` and `<Slot>`

`<ItemSet id title useSecondWeaponSet>`. `title` may hold colour codes (`^4Level 1-30 {1}`). A missing title displays as "Default". `useSecondWeaponSet="true"` means the swap weapons are the ones in use.

`<Slot name itemId itemPbURL active>`:

| Attribute | Meaning |
|---|---|
| `name` | Slot name (table below). Unknown names are ignored (`:1373`). |
| `itemId` | Item `id`, or `0` for empty. Every set lists every slot, empty ones included: 74 `<Slot>`s per set in the test build, of which 14–16 are filled. |
| `active` | `"true"` when a flask (or tincture) is switched on in the calcs. Missing means off. In the test build only set 9 ("Lvl 90 Cogwork + Kalandra {10}") has flasks switched on. |
| `itemPbURL` | Trade search URL that PoB's Trader stored for this slot in this set (`TradeQuery.lua:1217-1232` accepts only `…/trade/search/<league>/<id>` URLs). Usually `""`. |

Current PoB also writes `<SocketIdURL name nodeId itemPbURL>` inside each ItemSet, one per tree jewel socket allocated in the active spec, only to keep the Trader URL for that socket (`:1477-1480`). It does not say which jewel is there. The test build has none and its slots are in hash order rather than PoB's slot order, so it was saved by an older PoB. A parser must not depend on slot order.

### Slot names (`ItemsTab.lua:58`, `:174-264`)

| Slot | Notes |
|---|---|
| `Weapon 1`, `Weapon 2` | Main hand / off hand. `Weapon 2` holds the shield or quiver. |
| `Weapon 1 Swap`, `Weapon 2 Swap` | Second weapon set. In use only when the set's `useSecondWeaponSet="true"`. |
| `Helmet`, `Body Armour`, `Gloves`, `Boots`, `Amulet`, `Ring 1`, `Ring 2`, `Belt` | Gear. |
| `Ring 3` | Shown only when something grants an extra ring slot (`AdditionalRingSlot` flag, `:203-206`). |
| `Graft 1`, `Graft 2` | Grafts (PoE 3.26+ league mechanic). |
| `Flask 1` … `Flask 5` | Flasks, with `active`. |
| `<X> Abyssal Socket 1` … `6` | X = `Weapon 1`, `Weapon 2`, `Weapon 1 Swap`, `Weapon 2 Swap`, `Helmet`, `Body Armour`, `Gloves`, `Boots`, `Belt`. Abyss jewels socketed in gear. Only the first *n* are live, where *n* is the parent item's abyssal socket count (`ItemSlotControl.lua:110-122`). Test build: set 5 has a corrupted Hypnotic Eye Jewel (item 161) in `Belt Abyssal Socket 1`. |
| `Jewel <nodeId>` | The Items tab's name for a tree socket. It appears in XML only in `<SocketIdURL name>`, never as a `<Slot>`. |

## 3. Item text format

Written by `Item:BuildRaw()` (`src/Classes/Item.lua:1801-2076`), read by `Item:ParseRaw()` (`:424` on). PoB's parser also accepts the game's Ctrl+C / Ctrl+Alt+C text, but a saved build always has PoB's own form.

### Line order (as written)

1. `Rarity: NORMAL|MAGIC|RARE|UNIQUE|RELIC`.
2. Name. Rare and unique items have two lines: the name (`Nebulis`, or `New Item` for an unnamed rare), then the base type (`Void Sceptre`). Normal and magic items have one line: the full name, with the base found by substring match against PoB's base list (`Smouldering Quartz Sceptre` → base `Quartz Sceptre`, `:976-1009`).
3. Armour block: `Armour: 1234`, `ArmourBasePercentile: 0.5`, also `Evasion`, `Energy Shield`, `Ward`. Base roll positions, not mods.
4. `Intangibility:`, `Unique ID: <hash>`, `League: Synthesis`, `Unreleased: true`.
5. Influence lines, one per influence: `Shaper Item`, `Elder Item`, `Warlord Item`, `Hunter Item`, `Crusader Item`, `Redeemer Item`, `Searing Exarch Item`, `Eater of Worlds Item` (`src/Modules/ItemTools.lua:85-95`, mapped at `Item.lua:108-111`).
6. If `Crafted: true`: one `Prefix: <modId>` / `Suffix: <modId>` line per affix slot, with optional `{fractured}` and `{range:x}` before the mod id, or `None` for an empty slot. These are PoB's crafting-UI picks (internal mod ids like `PercentIncreasedLifeJewel`), **not** the displayed mod text. The text lines further down are what counts.
7. `Catalyst: Fertile`, `CatalystQuality: 20`, `Cluster Jewel Skill: affliction_fire_damage`, `Cluster Jewel Node Count: 8`, `Talisman Tier:`, `Item Level: 87`, `Memory Strands:`.
8. Variants: `Version:` lines and `Selected Version:` (newer variant groups); `Variant: <name>` lines (one per variant, 1-based in order) then `Selected Variant: n`; or `Selected Variant Group: g=v` when the item uses variant groups. Then `Has Alt Variant: true` / `Selected Alt Variant: n` (and `Two` … `Five`), and `Allow Duplicate Variants: true`.
9. `Quality: 20`, `Sockets: R-R-R-R-R-R` (`-` = linked, space = new group; letters R G B W A), `LevelReq: 68`, `Radius: Large`, `Limited to: 1`, `Requires Class <Class>`.
10. `Implicits: N`.
11. N lines of implicit-type mods: enchants, then scourge, then implicits. Then all explicit lines, then crucible lines.
12. Trailing flags: `Split`, `Mirrored`, `Fractured Item`, `Corrupted` (also written for scourged items), `Foil Unique (<type>)`.

Other bare lines PoB understands anywhere: `Synthesised Item`, `Unidentified`, `Veiled Prefix` / `Veiled Suffix`.

### How `Implicits: N` splits the lines (`Item.lua:1320-1333`)

The first N mod lines after `Implicits:` are implicit-type. Inside those N, a line tagged `{crafted}` (or `{enchant}`) is filed as an **enchant**, a `{scourge}` line as scourge, others as implicit. Every line after the N is explicit (or crucible if tagged). So anoints and catalyst/quality-effect lines show up as `{crafted}` lines inside the implicit count. Test build: amulet 53 has `Implicits: 2` with `{crafted}Allocates Disciple of the Unyielding` (the anoint) then the real implicit. Nebulis (item 22) has `Implicits: 2` holding two `{crafted}` quality-effect lines (`Quality does not increase Physical Damage`, `Grants 1% increased Elemental Damage per 2% Quality`).

### Line tags (`Item.lua:113-135` `lineFlags`, `:885-950` parser, `:1871-1958` writer)

Tags sit at the start of a mod line, in braces, any number, any order. The parser removes every `{word}` or `{word:value}` and keeps the rest as the mod text.

| Tag | Meaning |
|---|---|
| `{variant:1,3}` | Line exists only when the selected variant is in the list. On multi-variant uniques, lines for the other variants are present in the text and must be dropped. |
| `{version:n}`, `{group:g}` | Newer variant-group selection (paired with `Version:` / `Selected Variant Group:`). |
| `{range:0.683}` | Roll position 0–1 within the `(min-max)` written in the line: `{range:0.5}(15-20)% increased Cast Speed`. Written only when the line has a `(min-max)` range. |
| `{corruptedRange:x}` | Roll position for corruption-scaled values. |
| `{tags:fire,damage}` | Mod tags (from the game's advanced copy). Display-irrelevant. |
| `{crafted}` | Bench-crafted mod (or, inside the implicit count, an enchant). |
| `{enchant}` | Enchant. Also marks the line crafted + implicit. |
| `{fractured}` | Fractured mod. |
| `{custom}` | A mod PoB could not match to a known affix (test build: Archdemon Crown 153's unveiled `+2 to Level of Socketed AoE Gems`). |
| `{prefix}`, `{suffix}` | Affix type from the game's advanced copy. |
| `{exarch}`, `{eater}` | Searing Exarch / Eater of Worlds implicits. |
| `{synthesis}`, `{scourge}`, `{crucible}` | Synthesis implicit, scourge mod, crucible tree mod. |
| `{mutated}` | Foulborn / mutated unique mod (test build: item 23 "Foulborn The Red Dream"). |
| `{modGroup:x}`, `{disabled}`, `{unscalable}`, `{vestigial}`, `{implicit}`, `{unveiled}` | Rarer flags. |

A trailing ` (implicit)`, ` (enchant)`, ` (crafted)`, ` (fractured)` etc. after the text is accepted as the same flag (`:951-956`), but PoB never writes that form.

Values: newer PoB writes ranged lines as `(min-max)` with `{range:}`. To show the planned value, compute `min + (max-min) * range` (PoB rounds per mod). For a trade search, the range itself is often the more useful number.

### Free text inside items

Authors sometimes type notes as extra lines. PoB keeps them as explicit mod lines that match no mod (shown in red in PoB). Test build, item 29 (Heavy Woodsplitter): `Weapon +Rustic Sash + Whetstone = High % Phys Weapon.` and `Vendor Recipe.  Higher Rarity Sash = Higher % Phys`. Item 78 has `^2Note: Wands or Sceptres both work here…` with colour codes. There is also a `Note:` header (`:861`). Showing explicit lines as-is will show these notes, which is probably fine (they are useful), but they are not mods.

## 4. Tree jewels and sockets

- **Storage:** `<Tree>` › `<Spec>` › `<Sockets>` › `<Socket nodeId="26725" itemId="23"/>` (`src/Classes/PassiveSpec.lua:117-132` load, `:229-245` save). `nodeId` is the passive tree jewel socket node (cluster sub-tree sockets use generated ids ≥ 65536). `itemId` points into the same `<Items>` pool as gear.
- **Per Tree spec, not per Item set.** Switching tree spec swaps jewels (`TreeTab.lua:555-570`). Switching Item set does not touch them. A new spec copies the current spec's jewels (`TreeTab.lua:605`).
- **Only allocated sockets count.** CalcSetup skips a jewel whose socket node is not allocated in the spec (`src/Modules/CalcSetup.lua:860-864`). But PoB keeps and saves the `<Socket>` entry anyway (Save only drops `itemId` 0). Test build: spec 10 "Lvl 100 Cogwork Ring {10}" lists 12 sockets, of which 6 (`14993`, `24970`, `55190`, `2311`, `33631`, `61419`) are not allocated. To list jewels a player needs, keep only `<Socket>`s whose `nodeId` is in the spec's `nodes`.
- **Placeholder jewels.** Item 1 ("New Item", Crimson Jewel, 6% life + 7% fire DoT multi) sits in up to six sockets per spec. Some are unallocated, but in specs 5 and 6 it fills two allocated sockets at once (`26725`, `28475`). The same item id in several sockets means "this kind of jewel, N copies", like the duplicate rings below.
- Test build sockets per spec: specs 1–5 have none. Specs 6–7 have 3 allocated (Crimson ×2 placeholder, item 24). Specs 8–10 hold a Large Cluster Jewel (2 or 21), Small Cluster Jewels (9, 20), Foulborn The Red Dream (23), Rational Doctrine (33, with `Unique ID`), Watcher's Eye (164) and plain rare jewels.
- Abyss jewels in gear are Item set `<Slot>`s (`Belt Abyssal Socket 1`), not tree sockets.

## 5. Active Item set and pairing

- `activeItemSet` = ItemSet `id`. Test build: `activeItemSet="3"` = "^4Level 1-30 {1}", `activeSpec="2"` (position) = "Lvl 01-30 {1}", `activeSkillSet="3"` = "^4Level 1-30 {1}". The four `active*` attributes are stored independently. Nothing records a chosen loadout.
- Pairing is `src/Modules/Build.lua:684` `SyncLoadouts()`. Its `identifyLinks` is shared by item, skill and config sets ("item, skill, and config sets have identical structure"). The rules from #32 apply unchanged: first `{ids}` brace group (alphanumerics and commas, e.g. `{6,7}`), else identical raw title, else a set type with only one set matches every tree. A loadout needs all four.
- Test build Item set titles: `^4Level 1-30 {1}`, `^6Level 31-40 {2}`, `^2Level 41-60 {3}`, `^8Level 61-80 {4}`, `Level 80-90 {5}`, `Level 90 Going Block Based {6,7}`, `Lvl 90 Setting up Archdemon Crown {8}`, `Lvl 90+ Annihilation Approach {9}`, `Lvl 90 Cogwork + Kalandra {10}`. Tree specs use `{1}`–`{6}`, `{8}`–`{10}`. Skill sets use `{1}`, `{2}`, `{3}`, `{4,5}`, `{6,7}`, `{8}`, `{9}`, `{10}`. So the Item set has its own id for tree 5 (`{5}`), where the Skill set shares `{4,5}`. And Item set `{6,7}` covers tree `{6}` (there is no `{7}` tree). Every tree id except spec 1 (no braces) finds an Item set, giving the same 9 loadouts as #32.
- Colour codes differ between paired sets (`^4Level 1-30 {1}` vs `Lvl 01-30 {1}`). The brace rule ignores the rest of the title, so this does not matter. Title-equality pairing would compare raw titles, colour codes included.

## 6. Odd items worth handling

| Case | Test build example | Handling |
|---|---|---|
| Multi-variant unique | Cloak of Flame (items 7, 45, 104): `Variant: Pre 3.19.0`, `Variant: Current`, `Selected Variant: 2`, lines tagged `{variant:1}` / `{variant:2}`. Immortal Flesh (10, 105) uses variant 4. | Drop lines whose `{variant:}` list excludes the selected variant. Untagged lines always apply. |
| Alt variants | Watcher's Eye (164): ~60 `Variant:` lines, `variant="46"`, `variantAlt="1"`, `variantAlt2="1"`, `Has Alt Variant: true` / `Selected Alt Variant: 1` / `… Two`. | A line applies if its variant list contains the selected variant or any selected alt variant (`Item.lua:2168-2174` `CheckModLineVariant`). Expect huge variant lists. |
| Corrupted | Cloak of Flame 45 and 104, Hypnotic Eye Jewel 161, Crimson Jewel 15: bare `Corrupted` as the last line. | Flag only. It matters for trade filters (corrupted state). |
| Mirrored | Kalandra's Touch (166): `League: Kalandra`, `Mirrored`. | Flag. |
| Influences | Shaper + Hunter tower shield (142), Shaper + Elder Archdemon Crown (153), Exarch + Eater Vaal Gauntlets (4, 26, 34, 99) with `{exarch}` / `{eater}` implicits. | Header lines; one per influence. |
| Fractured | `{fractured}+120 to maximum Life` (item 34), `{fractured}` before mod ids on `Prefix:` lines. | Line flag. `Fractured Item` trailer may be absent even when a line is fractured. |
| Unique ID / Item Level | Rational Doctrine (33): `Unique ID: 17301e…`, `Item Level: 87`. Imported from a real character. | Ignore. |
| Duplicate item in one set | Set 3: `Ring 1` and `Ring 2` both item 37. Set 2: both rings item 92. | Count copies; one Trade search, quantity 2. |
| Shared items across sets | 13 items appear in more than one set (e.g. Vaal Greaves 143 in sets 8 and 1, Nebulis 22 in sets 5 and 9). | Same `id` = same item; don't re-buy. |
| Unknown base | none in test build | PoB drops the `<Item>` at load; treat its slot as empty. |

## 7. Notes for poe-tracker

- `pob.rs` can read Item sets exactly like Skill sets: id, title, document order, `activeItemSet` by id, pairing by the existing title logic.
- For an "Items section with Trade links": rares named `New Item` are templates (base + desired mods with `(min-max)` ranges), uniques are bought by name + base (+ variant). Magic items are usually leveling bases.
- Gear for a loadout = filled `<Slot>`s of the paired Item set (skip `itemId="0"` and swap slots unless `useSecondWeaponSet`) + `<Socket>`s of the paired Tree spec whose node is allocated.
- `itemPbURL` is a ready-made place where PoB users already save trade search URLs per slot per set. It is a nice-to-have input, but it is usually empty.
