# How PoB stores Skill sets and pairs them with Tree specs

*Research for issue #32 (map #31) · November 2026*

**Question:** How does a Path of Building (PoE 1) export store Skill sets, and how does a Skill set pair with a Tree spec? Covers the `<Skills>/<SkillSet>/<Skill>/<Gem>` XML, label-only and disabled groups, Vaal / awakened / transfigured / alt-quality gems, the `{n}` title convention vs "Loadouts", `activeSkillSet`, and colour codes.

**Sources:** PathOfBuildingCommunity/PathOfBuilding branch `dev` at commit `f4a77c5` (2026-11-23), unless a tag is named. Every file path below is in that repo. Older tags (`v2.37.0`–`v2.68.1`) were fetched to date format changes. Checked against the test build `src-tauri/tests/pohx-rf.txt` (Pohx RF Marauder, pobb.in/cYgy70giDjbo), decoded with `python3 -I`.

---

## 1. Short answer

- **Shape:** `<Skills activeSkillSet="ID">` › `<SkillSet id title>` › `<Skill>` (one socket group) › `<Gem>`. Document order is display order. `activeSkillSet` is a SkillSet **`id`**, not a position. (`<Tree activeSpec>` is a 1-based **position**, because `<Spec>` has no id.)
- **Gem identity:** `gemId` is the game's metadata path (`Metadata/Items/Gems/...`). It is **shared** by a gem and its transfigured versions. `variantId` (since v2.38.0, PoE 3.23) picks the exact variant. `skillId` is the granted-effect id. `nameSpec` is the display name and the fallback. Match on `(gemId, variantId)`; use `nameSpec` for display and name matching.
- **Not every `<Skill>` is gems to socket:** label-only groups (no `<Gem>` children) are guide notes and headers. Groups with a `source` attribute are made by PoB from items, the tree or config (e.g. `source="Explode"`, `source="Item:25:..."`). Their gems have no `gemId` and must not count as gems to acquire.
- **Pairing:** PoB has **no `<Loadout>` XML element** in any version, PoE 1 or PoE 2. A "Loadout" is worked out at runtime in `Build.lua` `SyncLoadouts()` from set **titles**: (1) the same alphanumeric id in braces, e.g. `{1}` or a list `{4,5}`, anywhere in the title; or (2) identical titles, colour codes included; or (3) a set type with only one set pairs with every loadout. A loadout needs a Tree spec **and** a matching Item set, Skill set and **Config set**.
- **Versions:** brace linking arrived in v2.43.0 (2024-07-23, PR #7672). v2.45.0 (#7847) allowed the braces "at any spot" in the title. Before v2.43.0 nothing linked the sets; the `{n}` text was only a convention. Skill sets themselves date from v2.19.0 (2022-07, PR #4447). Older builds have bare `<Skill>` elements directly under `<Skills>`.
- **Test build:** 10 Tree specs, 8 Skill sets, 9 Item sets, 2 Config sets. PoB resolves **9 loadouts** (`{1}`–`{6}`, `{8}`–`{10}`). Spec 1 ("CHECK POB NOTES…") pairs with nothing. Skill set `{4,5}` serves Tree specs `{4}` and `{5}`. Skill set `{6,7}` has no `{7}` tree.

## 2. XML shape

### `<Skills>` (`src/Classes/SkillsTab.lua:524-630`, Load and Save)

| Attribute | Meaning |
|---|---|
| `activeSkillSet` | `id` of the selected SkillSet. If missing or unknown, falls back to the first in order (`SetActiveSkillSet`, `:1533-1551`). |
| `defaultGemLevel`, `defaultGemQuality`, `sortGemsByDPS`, `sortGemsByDPSField`, `showSupportGemTypes`, `showLegacyGems` | UI preferences, not build content. Legacy `matchGemLevelToCharacterLevel` is still read. |

Load (`:551-569`): a `<SkillSet>` child creates set `tonumber(id)` with `title` and appends it to `skillSetOrderList`. A bare `<Skill>` child (the pre-v2.19 format) goes into an implicit set id 1 with no title. Save (`:584-629`) writes only `<SkillSet>`s, in `skillSetOrderList` order. Ids are not renumbered: Pohx's sets appear in order with ids 3,4,5,6,7,1,2,8.

### `<SkillSet>`

`id` (integer string) and `title` (optional; PoB shows a missing title as "Default").

### `<Skill>`: one socket group (`LoadSkill`, `:441-522`; Save `:590-600`)

| Attribute | Meaning |
|---|---|
| `enabled` | `"true"`/`"false"`. Legacy `active="true"` is also accepted. A disabled group still sits in the list, shown as "(Disabled)" (`src/Classes/SkillListControl.lua:83-86`), and is left out of calcs. |
| `label` | Free text with optional colour codes. If empty, PoB displays the enabled active gems' names joined by ", ", or `<No active skills>` (`src/Modules/CalcSetup.lua:1987-1998`). |
| `slot` | Optional item slot the gems are socketed in: `Body Armour`, `Helmet`, `Gloves`, `Boots`, `Weapon 1`, `Weapon 2`, `Amulet`, `Ring 1`, ... (plus swap slots). Most leveling groups have none. |
| `source` | **Set only for groups PoB generates**: item-granted (`Item:<itemId>:<name>`), tree-granted (`Tree:<nodeId>`), `Config`, and `Explode` (On Kill Monster Explosion). They are rebuilt and removed on every calc pass (`CalcSetup.lua:1527-1575`, `:1611`). |
| `groupCount` | Only shown for `source` groups (`SkillsTab.lua:371-382`). |
| `includeInFullDPS`, `mainActiveSkill`, `mainActiveSkillCalcs` | Calc settings. |
| `imbuedSupport` | Name of an imbued support on this group's item (v2.64.0+, PR #9670). Only read when `slot` is set (`:456-460`). |

### `<Gem>` (`:462-516` load, `:601-625` save)

| Attribute | Meaning |
|---|---|
| `nameSpec` | Display name from PoB's gem data, e.g. `Righteous Fire`, `Vaal Righteous Fire`, `Righteous Fire of Arcane Devotion`, `Awakened Burning Damage`. Supports have **no** " Support" suffix (`Burning Damage`). Pre-1.4.20 builds may hold only a typed or abbreviated name, which PoB resolves with `FindSkillGem` (`:1113-1137`, `:1203-1211`). |
| `gemId` | Game metadata path = gem data `gameId`. **Shared by a base gem and its transfigured variants** (`src/Data/Gems.lua:1309-1331`: both `Righteous Fire` and `Righteous Fire of Arcane Devotion` have `gameId = "Metadata/Items/Gems/SkillGemRighteousFire"`). Missing for item- or tree-granted skills. |
| `variantId` | Exact variant, e.g. `RighteousFire` vs `RighteousFireAltX`. Written since v2.38.0 ("New save format from 3.23", `:470-472`). Loader: `gemsByGameId[gemId][variantId]`, else the variant whose `grantedEffectId == skillId` (old format), else `skillId` alone. |
| `skillId` | Granted-effect id (`RighteousFire`, `SupportBurningDamage`). Item skills have only this (`ApproachingFlames`, `EnemyExplode`). |
| `level`, `quality` | Integers as strings. Level may exceed 20 (Pohx plans several level 21 gems, and one item skill is level 30). |
| `qualityId` | Alt quality (`Default`/`Anomalous`/`Divergent`/`Phantasmal`). Written up to **v2.63.0**, removed in **v2.64.0** (PR #9758, "Remove alternate quality code"). Current PoB ignores it. Older loaders also stripped an alt-quality word off the front of `nameSpec` (`GetBaseNameAndQuality`, `v2.63.0` `SkillsTab.lua:282-300`). |
| `enabled` | `"false"` = gem is in the group but switched off. Missing means enabled. |
| `count` | Number of copies of this gem (default 1). |
| `enableGlobal1`, `enableGlobal2`, `skillPart*`, `skillStageCount*`, `skillMineCount*`, `skillMinion*` | Calc options, not needed by us. |

## 3. Gem variants

All come from `src/Data/Gems.lua` plus `src/Modules/Data.lua:1090-1140`:

- **Vaal:** a separate gem with its own `gemId` (`SkillGemVaalRighteousFire`), `vaalGem = true`, a `vaal` tag, and `secondaryGrantedEffectId` = the base skill. Pohx set `{9}` socketing `Vaal Righteous Fire` is a different gem from `Righteous Fire`. **Transfigured Vaal** gems are made at load time with `variantId = <base>AltX/AltY` and the *same* `gemId` and `grantedEffectId` as the plain Vaal gem. Only `variantId` tells them apart.
- **Awakened:** its own gem with an `awakened` tag, e.g. `Awakened Burning Damage`, `gameId = Metadata/Items/Gems/SupportGemBurningDamagePlus` (`Gems.lua:11135-11142`). Note that PoB's table key (`SkillGemSupportAwakenedBurningDamage`) differs from the gameId; the XML stores the gameId. Legacy awakened gems are hidden unless `showLegacyGems` (`GemSelectControl.lua:128-151`).
- **Transfigured:** same `gemId` as the base, different `variantId`/`skillId`, and a name of the form "<Base> of <X>". Gem data has 648 `AltX`/`AltY`/`AltZ` entries.
- **Alternate quality:** gone from current PoB (see `qualityId` above). Old builds may carry `qualityId="Anomalous"` etc. or an "Anomalous X" `nameSpec`. Treat it as the base gem.
- **On the character side** (for the matching ticket): PoB's own character import maps the API's `typeLine` through `gemForBaseName`, whose keys add " Support" to support names. Transfigured and Vaal-hybrid gems go through `socketedItem.hybrid.baseTypeName` / `isVaalGem` (`src/Classes/ImportTab.lua:1831-1852`).

## 4. Label-only and disabled groups

- **Label-only:** `<Skill label="..." enabled="true"/>` with no children. Guide authors use these as headings and notes: Pohx has 112 of its 178 `<Skill>`s like this ("^6<< Auras >>", "^4Level all gems to MAX level Except For", empty "^0" spacers). They are not gems and should not appear as missing. They could be shown as notes, but labels contain colour codes and are often blank after stripping them.
- **Disabled group:** `enabled="false"` on `<Skill>`. Pohx has none. PoB keeps it but excludes it from calcs, so it is not part of the plan.
- **Disabled gem:** `enabled="false"` on `<Gem>`. Pohx has 8, e.g. `[off]Enduring Cry` in "Punishment, Enduring Cry", `[off]Lifetap`/`[off]Swift Affliction` in the `{4,5}` RF group, and `[off]Blood Rage, [off]Immortal Call` in `{6,7}`. Authors use these for optional or alternative gems. Whether to show them as "optional" or skip them is a product decision.
- **Generated groups (`source`):** Pohx has 8, all with gems lacking a `gemId`: `On Kill Monster Explosion` (source `Explode`), `Approaching Flames` from Annihilation's Approach boots, and `Aspect of the Spider` from an amulet. Rule: **skip any `<Skill>` with a `source` attribute, and any `<Gem>` without a `gemId`.**
- **Duplicates are real:** the same gem can appear in several groups of one set (Pohx `{8}` has Lifetap ×3 and Swift Affliction ×2), so compare counts, not just presence.

## 5. Pairing: `{n}` titles and Loadouts

### No `<Loadout>` element

The build file's sections are exactly `Config, Notes, Party, Tree, TreeView, Items, Skills, Calcs, Import`, plus the legacy `Spec` (`src/Modules/Build.lua:542-553`). Nothing in the code reads or writes a `Loadout` element, and a GitHub code search of the PathOfBuildingCommunity org for `elem == "Loadout"` returns 0. Loadouts are a dropdown built from titles every time PoB loads or changes sets.

### How PoB resolves them (`Build.lua:684-818` `SyncLoadouts`, `:940-995` `GetLoadoutByName`, `:997-1021` `SetActiveLoadout`)

1. For each Tree spec (title, or "Default" if missing), it takes the **first** match of `%{([%w,]+)%}`: letters, digits and commas inside braces, anywhere in the title. Each comma-separated id maps to that spec. The loadout's display name is the title with the brace group removed and trimmed, plus " {id}". Colour codes are kept in the name.
2. Item, Skill and Config sets are read the same way into an `id → setId` map. **Last writer wins** if two sets share an id.
3. Spec link id `L` becomes a loadout if there is an item set, a skill set and a config set for `L`. A set type with exactly one set counts as a match for every `L`.
4. Specs **without** braces pair only by **identical full title**, colour codes included (with a `[3.xx] ` prefix added for non-latest tree versions). The one-set rule applies here too.
5. Choosing a loadout sets the active spec, item set, skill set and config set together. PoB does **not** store which loadout is chosen; it only stores the four `active*` attributes. On load, it highlights the dropdown entry if the active sets happen to match.

Help text (`help.txt:122-137`) says the braces go "at the end of the name". The code has accepted them anywhere since v2.45.0 (#7847). Pohx relies on this: spec `"Lvl 31-40 {2} RF Start"`.

### Test build, resolved

| Link | Tree spec (pos) | Skill set (id) | Item set (id) | Config set (id) |
|---|---|---|---|---|
| — | 1 `^1CHECK POB NOTES FOR INFORMATION` | — | — | — |
| {1} | 2 `Lvl 01-30 {1}` | 3 `^4Level 1-30 {1}` | 3 | 1 `Default {1,…,8}` |
| {2} | 3 `Lvl 31-40 {2} RF Start` | 4 `^4Level 31-40 {2}` | 4 | 1 |
| {3} | 4 `Lvl 41-60  {3}` | 5 | 6 | 1 |
| {4} | 5 `Lvl 61-80 {4}` | 6 `^4Level 61-80 {4,5}` | 7 | 1 |
| {5} | 6 `Lvl 81-90 {5}` | 6 (same) | 8 | 1 |
| {6} | 7 `Lvl 90 ^2Going Block Based ^7{6}` | 7 `{6,7}` | 1 `{6,7}` | 1 |
| {8} | 8 | 1 `{8}` | 2 | 1 |
| {9} | 9 | 2 `^4Level 95+  {9}` | 5 | 2 `{9,10}` |
| {10} | 10 | 8 | 9 | 2 |

Saved state: `activeSpec="2"` (position 2 = `{1}`), `activeSkillSet="3"` (id 3 = `{1}`), `activeItemSet="3"` (`{1}`). These are consistent, so PoB opens on loadout `{1}`.

### Implications for poe-tracker

- Pair a Skill set with the chosen Tree spec the way PoB does: the brace ids first, then the identical title. If there is only one Skill set, use it. Requiring item and config matches, as PoB does, would not help us; we only need Tree to Skill.
- No pair is a normal case: spec 1 in Pohx, any untitled build, or a pre-v2.43 build that uses other naming. The map's "Not yet specified" item needs a fallback, e.g. `activeSkillSet` or "choose a Skill set".
- Use `%{([A-Za-z0-9,]+)%}`, first match only, comma lists. Ids can be letters (`{a}`). Stripping colour codes first is harmless, since `^` is not `%w`.

## 6. Colour codes

PoB titles and labels use the SimpleGraphic escapes `^0`–`^9` and `^xRRGGBB`. Named colours are in `src/Data/Global.lua:7+` (`colorCodes.NORMAL = "^xC8C8C8"`, ...), and the same two patterns are parsed in `src/Modules/Common.lua:806-807`. They can appear anywhere and be stacked (`^0^1`, `^1^1When ^6Dropping`), and can end a string (`"Vitality ^6"`, `"^4"`). `pob.rs::clean_title` already strips both forms. The same function serves Skill set titles and group labels. Equality pairing (rule 4) compares the *raw* title, so strip codes only for display.

## 7. Verification

Decoding `src-tauri/tests/pohx-rf.txt` and walking it with Python `xml.etree`:

- `<Skills activeSkillSet="3" ... showLegacyGems="false">`, 8 `<SkillSet>`s, 178 `<Skill>`s, 173 `<Gem>`s.
- Gem attributes seen: `nameSpec, skillId, gemId (165), variantId (165), level, quality, enabled, count, enableGlobal1/2, skillMinion*, skillPart`. **No `qualityId`**, which fits a save by PoB ≥ v2.64.0 (it also uses `imbuedSupport`, new in v2.64.0).
- The 8 gems without `gemId`/`variantId` are exactly the generated-group gems (EnemyExplode, ApproachingFlames, AspectOfTheSpider).
- `gemId` ≠ name-derived path in places (`Purity of Elements` → `SkillGemPurity`, `Malevolence` → `SkillGemDamageOverTimeAura`), so don't build gemIds from names.
- Variants in this build: one Vaal gem (`Vaal Righteous Fire`, own gemId). No transfigured, awakened or alt-quality gems, so those rules are checked against source only.
- `{n}` resolution simulated with PoB's regex and map rules gives the table in section 5.
