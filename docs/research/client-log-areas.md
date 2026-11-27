# What the Client log records when entering an area or levelling

Research for issue #53. Covers PoE1 `Client.txt` only. Builds on
[overlay-field-guide.md](overlay-field-guide.md) §5, which lists the event types; this note goes
deeper on area changes.

## Answer in brief

- Every area load writes **`Generating level N area "<id>" with seed S`** (a `[DEBUG Client …]` line),
  then two `[SCENE] Set Source [...]` lines, then **`: You have entered <Name>.`** about 1–2 s later.
  This happens for towns, hideouts and re-entered instances too, not only for new zones.
- The **area id** in the Generating line tells apart every zone that shares a name. 33 display names
  repeat in the campaign data, for example The Coast `1_1_2` / `2_6_2` and Lioneye's Watch
  `1_1_town` / `2_6_town`. "You have entered" alone cannot tell them apart.
- **Area level** is the `N` in the Generating line, on every load. For campaign zones it matches the
  game's WorldAreas level. For hideouts it is the hideout's own level (20 or 60 in the sample below),
  which has nothing to do with the character.
- The Generating line is the one to rely on across languages. "You have entered", the Set Source
  name and the level-up line are all translated in non-English clients.

## 1. The lines, in order

A real PoE1 log posted on the official forum (Nov 2025, hideout hopping). The lines in between are
Tile hash, Doodad hash, height map and similar engine lines.
Source: [pathofexile.com forum thread 3876981](https://www.pathofexile.com/forum/view-thread/3876981).

```
2025/11/11 17:31:37 10811171 1186a8e1 [DEBUG Client 23728] Generating level 60 area "HideoutBattleground" with seed 1
2025/11/11 17:31:37 10811406 e0345484 [INFO Client 23728] [SCENE] Set Source [(null)]
2025/11/11 17:31:37 10811406 e0345484 [INFO Client 23728] [SCENE] Set Source [Battle-scarred Hideout]
2025/11/11 17:31:38 10812406 cff945bb [INFO Client 23728] : You have entered Battle-scarred Hideout.
```

| # | Line | Level tag | Content |
|---|---|---|---|
| 1 | `Generating level N area "<id>" with seed S` | `DEBUG` | area level, internal area id, instance seed |
| 2 | `[SCENE] Set Source [(null)]` | `INFO` | reset, no information |
| 3 | `[SCENE] Set Source [<Name>]` | `INFO` | display name, translated |
| 4 | `: You have entered <Name>.` | `INFO` | system chat line, display name, translated |

The owner's own log also has `[LOADING SCREEN] (<Name>) Duration = … seconds` near the end of each
load ([overlay-field-guide.md](overlay-field-guide.md) §5). If the load hangs, line 4 never appears
(the last block of the forum post). So line 1 means "loading started" and line 4 means "player is in".

**Counts show this happens on every area change.** The owner's log (163k lines, July–Nov 2026) has
4,239 Generating lines, 4,237 "You have entered" lines and 4,225 loading-screen lines
([overlay-field-guide.md](overlay-field-guide.md) §5). The counts are almost one-to-one, and that log
includes many hideout, town and map entries.

### Towns, hideouts and re-entered instances

- **Hideouts:** yes. The id is `Hideout<Kind>`, for example `HideoutTemplarLab`, `HideoutOasis` or
  `HideoutRitualLeague`, always with `seed 1` in the sample above. Exile-UI detects hideouts with the
  regex `^hideout` on the area id
  ([`map tracker.ahk` `Maptracker_Towncheck`](https://github.com/Lailloken/Exile-UI/blob/main/modules/map%20tracker.ahk)).
  The field guide's `HideoutSpace` is just one of these.
- **Towns:** yes, with ids like `1_1_town` … `2_10_town` and `2_11_endgame_town`. Exile-UI only
  learns the area id from the Generating line, and it tests `InStr(vars.log.areaID, "_town")` in many
  places, for example to show the leveling overlay in town
  ([`client log.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/client%20log.ahk),
  [`leveling tracker.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/leveling%20tracker.ahk)).
  That only works if towns write the line.
- **Re-entering an existing instance:** yes, and it carries the **same seed** as before. Exile-UI's
  labyrinth helper decides the player is backtracking when the room name *and* seed match a room
  already visited (`room.name = areaname && room.seed = vars.log.areaseed`). Its hideout notification
  also compares id and seed to spot a new instance (`client log.ahk`, `Log_Parse` and surrounding
  code). So a new seed means a new instance, and the same id with the same seed means going back to
  one already visited. Hideouts and probably towns always use seed 1, so for those the seed proves
  nothing.

## 2. Area ids: telling same-named zones apart

The campaign follows `part_act_zone`: part 1 is Acts 1–5, part 2 is Acts 6–10. Towns are
`<part>_<act>_town`. Letter suffixes mark side areas (`1_1_2a` The Tidal Island). Numeric suffixes
mark floors (`1_1_7_1` / `1_1_7_2` Lower and Upper Prison). Source: exile-leveling's `areas.json`,
which its seeding script builds from the game's `WorldAreas.datc64`
([`seeding/src/data/index.ts`](https://github.com/HeartofPhos/exile-leveling/blob/main/seeding/src/data/index.ts),
[`common/data/json/areas.json`](https://github.com/HeartofPhos/exile-leveling/blob/main/common/data/json/areas.json)).
Exile-UI ships the same ids with its own levels in
[`data/english/[leveltracker] areas.json`](https://github.com/Lailloken/Exile-UI/tree/main/data/english).

That data has 163 areas, and 33 display names are used more than once. Each pair has its own id:

| Name | Acts 1–5 id (level) | Acts 6–10 id (level) |
|---|---|---|
| The Twilight Strand | `1_1_1` (1) | `2_6_1` (45) |
| The Coast | `1_1_2` (2) | `2_6_2` (45) |
| The Tidal Island | `1_1_2a` (3) | `2_6_3` (45) |
| The Mud Flats | `1_1_3` (4) | `2_6_4` (46) |
| Lioneye's Watch (town) | `1_1_town` | `2_6_town` |
| The Sarn Encampment (town) | `1_3_town` | `2_8_town` |
| Highgate (town) | `1_4_town` | `2_9_town` |
| The Broken Bridge | `1_2_4` (16) | `2_7_1` (50) |
| The Cathedral Rooftop | `1_5_8` (45) | `2_10_1` (64) |
| The Lunaris Temple Level 1 | `1_3_14_1` (29) | `2_8_7_1_` (59) |

The other repeats include Prisoner's Gate, The Lower Prison, the Cavern of Anger, the forests, the
Riverways, the Wetlands, the Solaris and Lunaris levels, the Chamber of Sins, the Ossuary, the
Reliquary, the Control Blocks, the Torched Courts, the Den, the Crossroads, Fellshrine, the Dread
Thicket, Sarn Arena (`ffa1_1` / `ffa3_1`) and the four Aspirant's Trial labs.

Pitfalls:

- **Act 6 ids don't line up with Act 1 ids.** The Coast is `2_6_2`, but the Tidal Island in Act 6 is
  `2_6_3` and the Mud Flats `2_6_4`. Map ids through a table and never compute them.
- **Ids can be odd.** `2_8_7_1_` has a trailing underscore in the game data. Exile-UI patches two
  PoE2 ids with the same problem (`c_g2_9_2_`, `c_g3_16_`). Match ids exactly as the game writes them.
- **The level also separates the parts.** Part 2 zones are level 45 or higher. Use this only as a
  sanity check; the id is enough.
- Other ids seen outside the campaign: `MapWorlds<Name>` (maps), `Hideout<Kind>`, `HeistHub`,
  `Menagerie`, `SanctumFoyer…`, `Labyrinth_Airlock`, `labyrinth_…`, and the PoE2 ids (`g1_1`, `c_g…`),
  which this app can ignore. Exile-UI's list of "town-like" ids is
  `^hideout|heisthub|menagerie|sanctumfoyer|abyss_hub` plus `_town`, `KalguuranSettlersLeague` and
  `incursionhub` (`map tracker.ahk`, `clone-frames.ahk`).

## 3. Area level

- The `N` in `Generating level N area` is the area (monster) level. Exile-leveling's game-data level
  for `1_1_8` (Prisoner's Gate) is 10. The owner's log has
  `Generating level 10 area "1_1_8"` ([overlay-field-guide.md](overlay-field-guide.md) §5).
- It appears on every load, so it is always available. Exile-UI reads it as `arealevel` and gets the
  map tier as `level − 67` in PoE1 (`client log.ahk`, `Log_Parse`).
- For towns and hideouts the number is real but not useful. Hideouts showed 20 or 60 in one session.
  WorldAreas gives towns a fixed level, for example 13 for `1_1_town` and 50 for `2_6_town`. Don't
  read any of these as "character progress".
- **The character's own level** is a different line: `: <Char> (<Class>) is now level N`. It appears
  only at level-up, so after a restart the app doesn't know the level until the next level-up (or a
  `/whois`, which Exile-UI also parses as `log_whois`).

## 4. How open-source tools parse it

| Tool | What it reads | How |
|---|---|---|
| **Exile-UI** (Lailloken, AHK) | Area id, seed, level, tier, act from `Generating level `. Display name from `[SCENE] Set Source [...]`, skipping `(null)` and `Act N` | Hard-coded English substrings for the Generating line. The level-up, death and `/whois` strings come from a per-language file ([`client log.ahk`](https://github.com/Lailloken/Exile-UI/blob/main/modules/client%20log.ahk), [`data/english/client.txt`](https://github.com/Lailloken/Exile-UI/blob/main/data/english/client.txt)). It **never uses "You have entered"** for detection; `log_enter` appears only in that file's comments. On start it reads back to the last 3 Generating lines to restore the current area |
| **exile-leveling** (HeartofPhos, web) | Area id only | Its companion [exile-log-api](https://github.com/HeartofPhos/exile-log-api) (Rust) tails Client.txt, forwards only lines that match `Generating level \d+ area` over a WebSocket on `localhost:6754`, and the web app moves to the next route step when `/Generating level \d+ area "(.*?)"/` gives the next expected id ([`web/src/state/route.ts`](https://github.com/HeartofPhos/exile-leveling/blob/main/web/src/state/route.ts)) |
| **Awakened PoE Trade** (SnosMe) | Chat only (whispers, trade, party) | Tails Client.txt ([`GameLogWatcher.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/main/src/host-files/GameLogWatcher.ts)) and matches translated chat regexes. It does not track areas ([`client-log.ts`](https://github.com/SnosMe/awakened-poe-trade/blob/master/renderer/src/web/client-log/client-log.ts)) |

Both area trackers key on the Generating line's id. Neither uses the "You have entered" name.

## 5. Localisation

- **"You have entered" is translated, and the word order changes.** German writes
  `Ihr habt 'Hohenpforte' betreten.` with the name in quotes in the middle. That is why Exile-UI's
  language file supports placeholders (`log_enter = "Ihr habt # betreten"`)
  ([`data/english/client.txt`](https://github.com/Lailloken/Exile-UI/blob/main/data/english/client.txt)).
  The zone name is translated too.
- **The level-up line is translated.** Exile-UI keeps `log_level = "is now level"` and
  `system_parenthesis = "(#)"` per language because Japanese uses different brackets. Other languages
  likely use other sentence-ending punctuation too (`system_fullstop`).
- **The Generating line looks language-independent.** It is a `DEBUG` engine line holding the
  internal id. Exile-UI matches it with a hard-coded English `"Generating level "` even though it
  moved every player-visible string into a per-language file. That strongly suggests the line is the
  same in every client. This is inferred from the code; no non-English log was available to check.
- `[SCENE] Set Source [...]` holds the translated display name (Exile-UI filters out `Act N` using
  its translated `log_act` string).

## 6. What this means for poe-tracker

`clientlog.rs` today turns `: You have entered …` into `Event::Entered`, which settles Pending refunds
(see GLOSSARY). Two problems follow from the findings above:

1. On non-English clients `You have entered` never matches, so Pending refunds would never settle at
   area change. The `LevelUp` parser (`is now level`) has the same English-only weakness.
2. Any area-aware feature, such as campaign or act tracking, needs the id, and only the Generating
   line has it.

Suggested direction (a design choice, not decided here): parse
`Generating level (\d+) area "([^"]+)" with seed (\d+)` into something like
`AreaGenerated { id, level, seed }`. Use it as the area-change signal, or keep "You have entered" only
as an English confirmation that loading finished. Treat `Hideout*`, `*_town` and the other hub ids
as places that don't move campaign progress. `clientlog.rs` already strips the header up to `"] "`,
so the `[DEBUG Client …]` tag needs no special handling.

## Open points (not verified)

- Whether towns also always use `seed 1` (likely, but no sample was seen).
- The exact text of the Generating and level-up lines in a real non-English client.
