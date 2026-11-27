# exile-leveling route data: can we bundle it, and in what shape?

*Path of Exile 1 · third-party data research · November 2026 · resolves #52*

All file references are to [HeartofPhos/exile-leveling](https://github.com/HeartofPhos/exile-leveling) at commit
[`d6951db`](https://github.com/HeartofPhos/exile-leveling/tree/d6951db0ca9463e42074e1e05e9ffde3d42d1604)
(main, last pushed 2026-11-12). Links below are relative to that tree; `EL/` is short for
`https://github.com/HeartofPhos/exile-leveling/blob/d6951db0ca9463e42074e1e05e9ffde3d42d1604/`.

---

## 1. Short answer

| Question | Answer |
|---|---|
| Can a GPL-3.0 app bundle it? | **Yes.** The whole repo is MIT (`EL/LICENSE`, "Copyright (c) 2025 HeartofPhos"). MIT is GPL-compatible. |
| Credit required? | **Yes, the MIT notice.** Ship the copyright line and permission text with any copy of the route files / JSON (e.g. a `THIRD_PARTY_LICENSES` entry and an About/credits line). Courtesy credit to exile-leveling in the Leveling guide UI is good form. |
| Caveats | (a) The JSON game data is extracted from GGG's game files, and quest **vendor** rewards are pulled from poewiki.net, whose content is CC BY-NC-SA 3.0. These are facts (which gem is sold after which quest), so low risk, but credit poewiki too. (b) The author does not take route PRs (`EL/README.md`), so we follow upstream, we do not contribute route changes back. |
| Shape | A **plain-text DSL** (`common/data/routes/act-1.txt` ... `act-10.txt`), one line per Guide step, with `{kind|arg|...}` inline fragments, `#sub` sub-steps, `#section` per act and C-style `#ifdef`/`#ifndef`/`#endif` conditionals. Zones, quests and gems are referenced by **GGG internal ids**, resolved against generated JSON in `common/data/json/`. |
| Build trimming | Gem rewards are **not in the route text**. Each `{quest|id}` step is expanded at runtime against `quests.json` and the gems found in the PoB Build's Skill sets. |
| Regeneration | Manual, per league: `exile-export` (Docker datamining tool) dumps `.datc64.json` tables from the game's GGPK, then `npm run seed data -w seeding` writes the JSON. CI only regenerates the passive tree. The route `.txt` files are hand-written. |

## 2. Licence

- `EL/LICENSE`: MIT, copyright 2025 HeartofPhos. GitHub reports `spdx_id: MIT` for the repo.
- MIT allows copying, modifying and redistributing under another licence (including GPL-3.0) provided "the above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software". The route `.txt` and JSON files are part of "the Software", so the notice travels with them.
- The upstream data tool, [HeartofPhos/exile-export](https://github.com/HeartofPhos/exile-export), has **no licence** (GitHub `license: null`). We do not need to ship it; we only need its output if we regenerate data ourselves.
- poewiki.net site licence (MediaWiki `siteinfo` API): `https://creativecommons.org/licenses/by-nc-sa/3.0/`. Only the vendor-reward table comes from it (`EL/seeding/src/seeding/quests.ts`, comment "QuestVendorRewards.dat no longer exists, use wiki data").

## 3. Route file format

Loaded by `EL/web/src/state/route-files.ts` (act-1 through act-10, concatenated), split into sections by
`getRouteFiles` and parsed by `parseRoute` in `EL/common/src/route-processing/index.ts`.

Sample (`EL/common/data/routes/act-1.txt`):

```
#section Act 1
Find and kill {kill|Hillock}
➞ {enter|1_1_town} #Lioneye's Watch
Hand in {quest|a1q1} #Enemy at the Gate
➞ {enter|1_1_2} #The Coast
#ifdef LEAGUE_START
    Get {waypoint_get}
#endif
Find 3x{quest_text|Glyph}
    #sub Little streams connect the 3 Rhoa nests
```

### Line kinds (regexes in `ROUTE_PATTERNS`)

| Syntax | Meaning |
|---|---|
| `#section <name>` | Starts a section (one per act in the stock route). |
| plain line | A **fragment step** (one Guide step): free text mixed with `{...}` fragments. |
| `#sub <text>` | A sub-step (hint) attached to the previous step. Indented one level deeper. |
| `#ifdef X` / `#ifndef X` / `#endif` | Conditional block, nestable. Indentation of 4 spaces per depth is enforced (warn only). |
| trailing `# text` | Human-readable comment, usually the zone/quest name next to an id. Ignored by the parser's fragments. |

### Fragment kinds (`EL/common/src/route-processing/fragment/index.ts`)

`{kind|arg1|arg2...}`. Counts are occurrences in the stock route:

| Fragment | Args | Notes |
|---|---|---|
| `enter` (58) | area id | Validated against `areas.json`; warns if not connected to the current area. Updates the tracked current area. |
| `waypoint` (39) | area id (optional) | Waypoint travel; `{waypoint}` alone just renders the icon. |
| `waypoint_get` (5) | none | Pick up the waypoint in the current area. |
| `logout` (17) | none | Return to last town (tracked state). |
| `portal` (3) | `set` / `use` | Town portal. |
| `quest` (23) | quest id, then optional reward-offer ids | Hand in. With no offer ids, all of the quest's reward offers apply. |
| `quest_text` (37) | text | Quest items/objects ("Glyph", "Allflame"). Display only. |
| `kill` (34), `arena` (10), `area` (1), `generic` (12) | name or id | Display (`area` takes an id). |
| `trial` (4), `ascend`, `crafting` (7) | | Lab trials, ascend, crafting-recipe unlock. Parser warns if any area with recipes is missing a `crafting` step. |
| `dir` (27) | degrees | Compass direction arrow (`{dir|270}`). |
| `reward_quest`, `reward_vendor` | item, cost | Exist in the grammar (town-only) but are **not used** in the stock route; gem rewards are injected from data instead. |

### Zones

Always by GGG `WorldAreas` id (`1_1_2` = The Coast, `1_2_town`, `1_1_4_1` ...), with the English name as a
trailing comment. `areas.json` (163 areas) gives, per id: `name`, `map_name`, `act`, `level`,
`has_waypoint`, `is_town_area`, `parent_town_area_id`, `connection_ids`, `crafting_recipes`.
Example: `{"id":"1_1_2","name":"The Coast","act":1,"level":2,"has_waypoint":true,"is_town_area":false,"parent_town_area_id":"1_1_town","connection_ids":["1_1_town","1_1_3","1_1_2a"],"crafting_recipes":[]}`.

This matters for us: the client log prints zone **names** ("You have entered The Coast."), so auto-ticking
an `enter` step needs the id-to-name map from `areas.json`. Names are not unique across acts
(Part 1 / Part 2 repeats), so match on name **plus** expected act/sequence, not name alone.

The parser also produces `route.edges`: the sequence of area ids the route visits, with each step's
`edgeIndex` pointing into it, i.e. a ready-made "which step am I on, given my zone" index.

### Quests, gem rewards and vendor buys

`{quest|a1q2|a1q2b}` names quest `a1q2` (The Caged Brute) and only its reward offer `a1q2b`
(the support-gem offer). `quests.json` (94 quests) per quest:

```json
"a1q2": { "id": "a1q2", "name": "The Caged Brute", "act": "1",
  "reward_offers": {
    "a1q2b": { "quest_npc": "Nessa",
      "quest":  { "Metadata/Items/Gems/SupportGemAddedLightningDamage": { "classes": ["Witch","Templar","Shadow","Scion"] }, ... },
      "vendor": { "<gem metadata id>": { "classes": [...], "npc": "Nessa" }, ... } } } }
```

- Gems are keyed by GGG **BaseItemTypes metadata id** (the same id PoB writes as `gemId`).
- `quest` = free pick on hand-in, `vendor` = buyable from that NPC afterwards, each with class restrictions (empty = all classes).
- Vendor price is not stored; the UI derives the currency icon from the gem's `required_level` in `gems.json` (`EL/web/src/components/GemCost/index.tsx`: <8 wisdom, <16 transmutation, <28 alteration, <38 chance, else alchemy).

### Optionals, league start and bandits

There is **no "optional" marker**. Optional content is expressed with preprocessor symbols, set from the
user's Build options in `EL/web/src/state/route.ts`:

| Symbol | Set when | Used for |
|---|---|---|
| `LEAGUE_START` | "league start" option (default on when no Build) | Extra side content for a fresh character: trials, The Tidal Island / Medicine Chest, The Den, Chemist's Strongbox, extra waypoints. `#ifndef LEAGUE_START` gives the shorter alternative. 10 `#ifdef`, 2 `#ifndef`. |
| `BANDIT_KILL` / `BANDIT_OAK` / `BANDIT_KRAITYN` / `BANDIT_ALIRA` | bandit choice (`None` → `BANDIT_KILL`) | Four alternative Act 2 blocks (`act-2.txt` lines 63-160). |
| `LIBRARY` | "library" option | Defined by the app but not referenced by any stock route file at this commit. |

One more trim exists: the `gemsOnly` setting (`EL/web/src/state/config.ts`) drops every step that yields no gem step.

## 4. How it trims gem quests to a PoB build

1. `processPob` (`EL/web/src/state/pob/process.ts`) decodes the PoB code and walks every `<SkillSet>` (or the root if none), collecting each enabled `<Skill>`'s `<Gem gemId=...>` into `requiredGems` (deduped, `count: 1`, note = Skill set title / skill label). `MapGemId` normalises ids: a PoB quirk table (`GEM_ID_REMAP`), then Vaal → base gem (`vaal-gem-lookup.json`) and Awakened → base gem (`awakened-gem-lookup.json`).
2. `findCharacterGems` (`EL/common/src/route-processing/gems.ts`) marks gems the class already gets (`characters.json`: `start_gem_id`, `chest_gem_id`).
3. `routeSelector` (`EL/web/src/state/route.ts`) walks the parsed route; for each `quest` fragment, `buildGemSteps`:
   - **Quest pick**: for each listed reward offer, the *first* still-unclaimed required gem (in PoB order) that the offer gives to this class becomes one `gem_step` with `rewardType: "quest"`. At most one per offer.
   - **Vendor buys**: every remaining required gem the offer sells to this class (and not already bought) becomes a `gem_step` with `rewardType: "vendor"`.
   - Greedy and in route order: a gem is taken at the earliest quest that offers it.
4. Steps with gem rewards also get a copy-to-clipboard `"GemA|GemB"` vendor search string appended.

So the route text stays build-agnostic; the build only adds gem steps after quest hand-ins.

## 5. How it is regenerated each patch

- **Game data** (`gems.json`, `quests.json`, `areas.json`, `characters.json`, Vaal/Awakened lookups): `EL/README.md` "Seeding": use exile-export to produce the `.datc64.json` files listed in `EL/seeding/src/data/index.ts`, then `npm run seed data -w seeding` (`EL/seeding/src/index.ts`).
  - GGG tables read: `BaseItemTypes`, `Characters`, `GemEffects`, `GrantedEffects`, `GrantedEffectsPerLevel`, `MapPins`, `NPCs`, `NPCTalk`, `Quest`, `QuestRewardOffers`, `QuestRewards`, `RecipeUnlockDisplay`, `SkillGems`, `WorldAreas`.
  - Areas: breadth-first walk of `WorldAreas.Connections_WorldAreasKeys` from The Twilight Strand (`1_1_1`) and the lab areas (`EL/seeding/src/seeding/areas.ts`).
  - Quest rewards: `QuestRewards` + `QuestRewardOffers`, NPC from `NPCTalk`; vendor rewards from the poewiki cargo table `vendor_rewards` joined to `items` (`EL/seeding/src/wiki.ts`, `https://www.poewiki.net/w/api.php`). Hand-written fix-ups: `a1q2`/`a1q4` vendor offers split by item, `a2q9` skipped ("Wiki is outdated").
- **Route text**: hand-edited by the author when speedrun strategy or campaign changes (recent commits: "update for 3.27" 2025-11-14, "Allflame route update" 2026-07-24). Not generated.
- **Passive tree**: `npm run seed tree` runs in CI on every push (`.github/workflows/build-and-deploy.yml`); latest data commit "passive tree json 3.30" (2026-11-12). Game data JSON was last reseeded for 3.28 gems (2026-05-05), so it can lag a league.

## 6. What this means for poe-tracker

- **Bundle the source, not a rendering.** Vendor `common/data/routes/*.txt` plus `areas.json`, `quests.json`, `gems.json`, `characters.json` and the two gem lookups, with the MIT notice. Total size is small.
- **Port the parser, not the web app.** `common/src/route-processing/` is about 1,200 lines of dependency-free TypeScript (including types and an editor language definition) (regex line patterns + fragment evaluators), straightforward to run in our frontend or port to Rust. Our Guide step = one fragment step with its sub-steps.
- **Expose the same switches** as Build options: league start on/off and bandit choice.
- **Reuse our Skill set import** for gem trimming: PoB `gemId` already matches `quests.json` keys, so `buildGemSteps` logic applies directly to the gems we already parse.
- **Auto-tick on zone entry** via `enter`/`waypoint` area ids → `areas.json` name → Client.txt "You have entered" line, disambiguated by position in `route.edges`.
- **Updates**: pull upstream on each league; we inherit both their route edits and their reseeded JSON without running exile-export ourselves. If upstream lags, the seeding script is the fallback, but needs exile-export (unlicensed) or another GGPK extractor.
