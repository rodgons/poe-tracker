# Campaign layout variance (PoE1, Acts 1–10)

*Research for [#74](https://github.com/rodgons/poe-tracker/issues/74), map [#72](https://github.com/rodgons/poe-tracker/issues/72) · November 2026*

**Question:** how many layouts does each zone the Leveling guide's route enters have, and how does a player tell them apart?

## Short answer

The route (`src-tauri/data/route.json`) enters **135 non-town campaign zones**. Aspirants' Plaza (`Labyrinth_Airlock`) is left out because it belongs to the Lab, and the Twilight Strand (`1_1_1`) is left out because the route starts there without entering it.

| Class | Zones | What it means for the Guide panel |
|---|---|---|
| **Fixed** | 11 | One picture. Show it straight away. |
| **Known variants** | 35 | A small set (2–9, median about 4) told apart by a clue at or near the entrance. The player clicks the one that matches. 2 of these (Lunaris Temple Level 2, Acts 3 and 8) are random for the first half and use variants only in the second half. |
| **Random** | 87 | The overall shape repeats, but details change too much to pin one layout. Act-Decoder shows only "inconclusive examples" for these. Best handled with text directions plus at most one rough sketch. |
| **No data** | 2 | The Den (`1_2_2a`) and The Imperial Fields (`2_8_10`) have no Act-Decoder pictures. |

- **Mirroring / rotation:** mostly **no** in PoE1. Exile-UI's Act-Decoder turns on its rotate and flip buttons for every PoE2 zone but for only one PoE1 zone, The Tidal Island (`1_1_2a`). Its pictures for Lunaris Temple Level 2 (`1_3_14_2`, `2_8_7_2`) also say "part 2: mirrored/rota. variations exist". So mirror/rotate controls are not needed in general. If those zones are covered at all, they can be stored as extra variants instead.
- **Area seed:** it identifies an **instance**, not a layout. The same seed means the same generated instance, so it is the right signal to reset a player's pick. It does not say *which* layout was generated: no tool decodes campaign layouts from it, and doing so would mean reimplementing GGG's level generator.

## Sources

1. **Exile-UI Act-Decoder** (MIT licence), commit [`535086b`](https://github.com/Lailloken/Exile-UI/tree/535086b120c7f6b8c669903a0660c1a1c341f1e6) (2026-11-18):
   - `img/GUI/act-decoder/file-list.json` lists 467 PoE1 layout pictures plus one legend (`explanation.jpg`). Each picture is named `<area id> <path>.jpg`.
   - `modules/act-decoder.ahk` holds the selection logic and the rotate/flip whitelist.
   - `data/english/help tooltips.json` explains the meaning of picture border colours.
2. **Game data via RePoE** ([`world_areas.json`](https://repoe-fork.github.io/world_areas.json), last modified 2026-11-23). It is extracted from the game files, and each area's `topologies` lists its terrain graph files (`.tgr` outdoor tile graphs, `.dgr` dungeon graphs). The field layout comes from [poe-tool-dev/dat-schema `_Core.gql`](https://github.com/poe-tool-dev/dat-schema) (`type Topologies { Id, DGRFile, …unnamed }`).
3. **exile-leveling route notes**: the `subs` already bundled in `route.json` (source commit `d6951db` of HeartofPhos/exile-leveling).
4. **This repo**: `src-tauri/src/clientlog.rs` tests and `docs/research/overlay-field-guide.md` (Client log line format).
5. poewiki.net pages could not be fetched: the site's Anubis bot wall blocks automated reads. No claim here depends on them.

## How Act-Decoder encodes a zone

Read from `act-decoder.ahk` (`Actdecoder_ZoneLayouts`, `Actdecoder_ImageSelect`) and the help tooltips:

- **`<id> 1`, `<id> 2`, …** are the first pictures shown on entering the zone.
- **`<id> N_M` (and `N_M_K`)** are children of picture `N`. When children exist, the first pictures are *identifiers*: small cut-outs of what the minimap shows around the entrance, with "mini-map clues" (wall, cliff, an arrow for the path direction, a dotted edge line, "S/W is blocked off") drawn in green. The player clicks the matching identifier, or presses its number key, and the children are loaded. The children are the full layouts with entrance, waypoint, exit and objectives marked. Some trees go two levels deep (Submerged Passage). Many have a last identifier labelled "leftover layout(s)" as a catch-all.
- **`x`, `x_x`** are legend or instruction cards, e.g. "rotate/align the img, then select it" for Tidal Island, and "requires full HP; player marker shows where to stand to see map icons" for Crematorium.
- **Orange-bordered pictures** are, in the help text's words, "inconclusive examples of possible layouts (just to give a rough idea)". **Unbordered or white pictures** are exact. A script sampled the border colour of every picture to sort the zones (orange: 175 pictures; white or none: 292).
- The legend (`explanation.jpg`) marks: point of interest, optional point, suggested order, alternate location, shared location, boss, optional boss, mini-map clues.

Each zone was classified as follows:

- **variants**: the zone has an identifier tree. The count is the number of leaf pictures, i.e. distinct full layouts, including any catch-all.
- **fixed**: one exact picture.
- **random**: only orange example pictures.
- One override: The Library (`1_3_17_1`) has two exact pictures, but they are "step 1: find this room / step 2: find this room along edges/corners (3 slight variations)", a search rule rather than two layouts. It is counted as random.

### Rotation and mirroring

In `Actdecoder_ZoneLayouts`, the rotate and flip buttons are drawn only when this holds:

```
(vars.poe_version || rota_whitelist[vars.log.areaID]) ... && !(rota_block[...])
rota_whitelist := {"1_1_2a": 1}
```

`vars.poe_version` is empty for PoE1 and `" 2"` for PoE2. So PoE2 zones can be rotated and mirrored by default, apart from a blocklist of `g…` PoE2 ids. In PoE1, only The Tidal Island gets the controls. The help text for those controls says "when unsure how to match indicators, always try mirroring before rotating", a PoE2-oriented hint. The one other PoE1 sign of mirroring is the Lunaris Temple Level 2 instruction card ("part 2: mirrored/rota. variations exist") and its picture, which is drawn inside a circle with 90° tick marks. Community PoE2 guides (e.g. Mobalytics) describe rotation and mirroring as the *normal* PoE2 behaviour. That fits Act-Decoder's PoE1/PoE2 split.

Game data has no named flip or rotate field: the `Topologies` row's extra fields are unnamed in dat-schema. The answer above therefore rests on Act-Decoder, which comes from someone who has hand-matched every zone.

### Game graphs vs visible variants

The "Game graphs" column counts each area's non-league topology files from game data. It is a structural count and **not** the number of layouts a player can tell apart:

- Dungeon graphs (`.dgr`) produce many visible layouts from a single graph. Upper Prison (`1_1_7_2`) has 1 graph but 9 Act-Decoder layouts.
- Several graphs can look the same for routing. Ashen Fields (`2_7_7`) has 5 graphs and Feeding Trough (`2_10_6`) has 4, but both are a single "fixed" picture.
- Leagues can add graphs. The Coast lists a duplicate `…/Leagues/Deepwater/…` set next to its 9 base graphs. That is a reason to re-check layouts each league.

## The area seed

- Line format: `Generating level N area "<id>" with seed <s>`. Our parser test fixtures in `clientlog.rs` show campaign zones with large seeds (`1_1_2` … `seed 2103837163`) and a hideout with `seed 1`. Fixed, non-generated areas appear to use a constant seed.
- Exile-UI uses the seed only to recognise a **revisited instance**: its Lab tracker matches `room.name = areaname && room.seed = areaseed` to detect backtracking (`modules/client log.ahk`). Act-Decoder never reads the seed. Its picks last until the player presses Q or reset.
- Nothing found maps a seed to a layout. The generator that turns seed + topology into tiles lives in the game client.
- **For #72:** keep the planned rule. The pick holds per area until a new seed appears for that area id. Re-entering the same instance (same seed, e.g. via waypoint or portal) keeps the pick, and a fresh instance clears it. The seed cannot pre-select the variant.

## Implications for the Guide panel (#72)

- **Worth drawing first:** the 35 variant zones plus 11 fixed zones (46 zones, 175 full layouts in Act-Decoder). The 87 random zones fall back to text directions, which the route's `subs` already give for most of them. At most one rough sketch per random zone, marked as approximate.
- **No mirror or rotate controls.** Tidal Island can be shipped as a few extra rotated variants, or left as text. The same goes for Lunaris Temple Level 2's second half.
- **Picker UI:** Act-Decoder's flow is a good model. First show small *identifier* schematics of what the minimap shows at the entrance; one click picks the full layout. Allow up to 2 levels, and a "none of these" catch-all.
- **Licence:** Exile-UI is MIT-licensed (code and the bundled pictures in the repo), so reuse with attribution is allowed. But the pictures are traced minimap shapes, and PRODUCT.md rules out GGG art in panels. Our own schematics, drawn using these as reference, are the safer path, and the owner should decide this.
- **Upkeep:** both Act-Decoder (its `layouts` branch has a version file) and game data change across patches. Act-Decoder's own warning is that "accuracy can vary from patch to patch".

## Per-zone table

Columns:

- **Class**: from Act-Decoder, as described above.
- **Variants**: the number of distinct full layouts (leaf pictures, including catch-alls).
- **Game graphs**: non-league topology files in game data.
- **Clue / rule**: the Act-Decoder identifier clue, read from its pictures, then the exile-leveling route note for that zone ("route:"), where one exists.

| Area id | Act | Zone | Class | Variants | Game graphs | Clue near the entrance / rough rule |
|---|---|---|---|---|---|---|
| `1_1_2` | 1 | The Coast | variants | 8 | 9 | Shape of the shoreline / cliff wall right at the entrance (where the wall runs, which way the shore bends) |
| `1_1_3` | 1 | The Mud Flats | variants | 4 | 7 | Which way the first trail leaves the entrance; route: Little streams connect the 3 Rhoa nests / Usually form a triangle, but will rarely form a line |
| `1_1_4_1` | 1 | The Submerged Passage | variants | 6 | 6 | Direction from entrance to the nearby waypoint (diagonal one way or the other), then the outer edge shape; 3-level pick |
| `1_1_2a` | 1 | The Tidal Island | variants | 3 | 15 | Shoreline shape at the entrance; **rotated** in PoE1, so the picture is turned to match before picking; route: Go [270°] / If you find a ledge, turn around and go [90°] / Recommended Level: 4 |
| `1_1_4_0` | 1 | The Flooded Depths | random |  | 4 | route: Go right of the bridge / Search opposite the entrance / Look for the large empty room |
| `1_1_5` | 1 | The Ledge | random |  | 16 |  |
| `1_1_6` | 1 | The Climb | variants | 5 | 6 | Which way the path leaves the entrance (two families: up-right vs up-left) |
| `1_1_7_1` | 1 | The Lower Prison | variants | 7 | 6 | Corridor shape from entrance to waypoint, and where the trial/map icon shows; route: Usually [45°] |
| `1_1_7_2` | 1 | The Upper Prison | variants | 9 | 1 | Outline of the first room block and which edge it hugs; last picture is a catch-all; route: Look for map icon, access with nearby switch / High chance for Quicksilver Flask / Vendor Quicksilver Flask + Orb of Augmentation + Normal Boots / Vendor Quicksilver Flask + Orb of Augmentation + Movement Speed Boots / Recommended Level: 8 |
| `1_1_9` | 1 | The Ship Graveyard | variants | 5 | 18 | Shoreline shape near the entrance; route: Go down the ledge next to the road |
| `1_1_11_1` | 1 | The Cavern of Wrath | random |  | 4 |  |
| `1_1_11_2` | 1 | The Cavern of Anger | random |  | 4 | route: Follow the water / Recommended Level: 12 |
| `1_2_1` | 2 | The Southern Forest | random |  | 3 |  |
| `1_2_2` | 2 | The Old Fields | random |  | 6 | route: Go [45°] |
| `1_2_3` | 2 | The Crossroads | fixed | 1 | 1 | route: Follow the road |
| `1_2_2a` | 2 | The Den | no data |  | 8 |  |
| `1_2_6_1` | 2 | The Chamber of Sins Level 1 | fixed | 1 | 3 | route: Go [315°] |
| `1_2_6_2` | 2 | The Chamber of Sins Level 2 | variants | 4 | 4 | Which way the corridor runs from the entrance (zig-zag vs diagonal along an edge); route: Go in same direction as waypoint / Look for the long hallway / Recommended Level: 14-15 |
| `1_2_15` | 2 | The Fellshrine Ruins | random |  | 3 | route: Go [135°] |
| `1_2_5_1` | 2 | The Crypt Level 1 | variants | 4 | 4 | Waypoint beside the entrance; which way the long hall runs; route: Follow the road |
| `1_2_5_2` | 2 | The Crypt Level 2 | variants | 4 | 8 | Shape of the first room |
| `1_2_7` | 2 | The Riverways | random |  | 11 | route: Go [225°] / Follow the road |
| `1_2_9` | 2 | The Western Forest | random |  | 4 | route: Follow the road / Search opposite direction of the encampment entrance / Go [180°] look for the torch touching the road / Follow the trail in the direction of the torch / Follow the road [225°] |
| `1_2_10` | 2 | The Weaver's Chambers | variants | 4 | 4 | Is the west path open, or are west + south + east blocked; route: Search the side of road opposite waypoint / Try go [270°], if it's blocked go [45°] / Recommended Level: 16 |
| `1_2_4` | 2 | The Broken Bridge | random |  | 6 | route: Go [45°] / Follow the road |
| `1_2_12` | 2 | The Wetlands | variants | 4 | 9 | Edge / water line next to the entrance; route: Look for 2 pillars near waypoint, follow the trail |
| `1_2_11` | 2 | The Vaal Ruins | random |  | 10 | route: Search opposite direction of the encampment entrance |
| `1_2_8` | 2 | The Northern Forest | variants | 3 | 12 | Which way the path leaves the entrance (west vs north-east); route: S shape or L shape leads to exit |
| `1_2_14_2` | 2 | The Caverns | random |  | 4 |  |
| `1_2_14_3` | 2 | The Ancient Pyramid | random |  | 1 | route: First floor exit will be in one of the 3 corners / Remaining floors will have the exit diagonally across from the entrance / Recommended Level: 20-22 |
| `1_3_1` | 3 | The City of Sarn | random |  | 1 |  |
| `1_3_2` | 3 | The Slums | variants | 6 | 6 | Shape of the road junction near the entrance; route: Go [0°] |
| `1_3_3_1` | 3 | The Crematorium | variants | 9 | 9 | Map icons (trial, quest item) visible from a spot by the entrance waypoint; route: Follow Stairs |
| `1_3_10_1` | 3 | The Sewers | variants | 3 | 3 | Start is fixed; the corridor and quest-item room differ |
| `1_3_5` | 3 | The Marketplace | random |  | 15 |  |
| `1_3_6` | 3 | The Catacombs | variants | 7 | 8 | Shape of the first room, then which edge the wall follows; route: Look around nearby |
| `1_3_7` | 3 | The Battlefront | random |  | 6 | route: Search [0°] / Go [315°] / Go [225°] |
| `1_3_8_1` | 3 | The Solaris Temple Level 1 | variants | 6 | 6 | Shape of the entrance room and which edge the layout follows; route: Go [45°] |
| `1_3_8_2` | 3 | The Solaris Temple Level 2 | variants | 7 | 6 | Shape of the entrance room |
| `1_3_9` | 3 | The Docks | variants | 3 | 3 | Which way the wall runs from the entrance; route: Go [315°] |
| `1_3_13` | 3 | The Ebony Barracks | random |  | 1 | route: Go [315°] |
| `1_3_14_1` | 3 | The Lunaris Temple Level 1 | random |  | 4 |  |
| `1_3_14_2` | 3 | The Lunaris Temple Level 2 | random + variants | 2 | 12 | Part 1 very random (entrance and exit diagonally opposite, follow 3 stairs up); part 2 has **mirrored/rotated** versions; route: Follow stairs that are going up / At the fork in the road with wagons, go the route with 1 wagon, not 2 |
| `1_3_15` | 3 | The Imperial Gardens | random |  | 7 | route: Go [45°] / Follow the road / Go [0°] |
| `1_3_17_1` | 3 | The Library | random |  | 3 | Act-Decoder: find a known room, then a second room along the edges (3 slight variations); route: Go [315°] / Put currency needed to buy skill gems in inventory |
| `1_3_17_2` | 3 | The Archives | random |  | 1 | route: Look for a candle in a narrow corridor |
| `1_3_18_1` | 3 | The Sceptre of God | random |  | 1 |  |
| `1_3_18_2` | 3 | The Upper Sceptre of God | random |  | 1 | route: Stairs spawn in one of the corners / Look for barricades |
| `1_4_1` | 4 | The Aqueduct | random |  | 1 |  |
| `1_4_2` | 4 | The Dried Lake | random |  | 8 |  |
| `1_4_3_1` | 4 | The Mines Level 1 | random |  | 6 |  |
| `1_4_3_2` | 4 | The Mines Level 2 | variants | 6 | 6 | Which edge the layout runs along (V shape vs one diagonal) |
| `1_4_3_3` | 4 | The Crystal Veins | variants | 6 | 6 | Corridor directions from the entrance |
| `1_4_5_1` | 4 | Daresso's Dream | random |  | 5 |  |
| `1_4_5_2` | 4 | The Grand Arena | random |  | 3 | route: Go [225°] follow the exits keeping [270°] |
| `1_4_4_1` | 4 | Kaom's Dream | random |  | 4 |  |
| `1_4_4_3` | 4 | Kaom's Stronghold | variants | 2 | 4 | Cliff shape next to the entrance; route: Go [45°] / Follow Bridges |
| `1_4_6_1` | 4 | The Belly of the Beast Level 1 | random |  | 6 |  |
| `1_4_6_2` | 4 | The Belly of the Beast Level 2 | random |  | 5 |  |
| `1_4_6_3` | 4 | The Harvest | random |  | 3 |  |
| `1_4_7` | 4 | The Ascent | random |  | 2 |  |
| `1_5_1` | 5 | The Slave Pens | random |  | 4 | route: Go [180°] until you find the hanged Karui / Loop around clockwise to go [270°] |
| `1_5_2` | 5 | The Control Blocks | random |  | 4 | route: Go [270°] as much as possible / Go [45°] from Miasmeter |
| `1_5_3` | 5 | Oriath Square | random |  | 1 |  |
| `1_5_4` | 5 | The Templar Courts | random |  | 4 | route: Go [45°] |
| `1_5_5` | 5 | The Chamber of Innocence | random |  | 4 | route: Loop around clockwise / Farm Level: 40-42 / Go [315°] counter-clockwise spiral |
| `1_5_4b` | 5 | The Torched Courts | random |  | 4 |  |
| `1_5_3b` | 5 | The Ruined Square | random |  | 1 | route: Loop around counter-clockwise / Go [315°] |
| `1_5_6` | 5 | The Ossuary | variants | 2 | 4 | Which way the first corridor runs |
| `1_5_7` | 5 | The Reliquary | random |  | 4 | route: Go [225°] find the plaza / Go [180°] |
| `1_5_8` | 5 | The Cathedral Rooftop | random |  | 1 | route: Go [225°] |
| `2_6_1` | 6 | The Twilight Strand | random |  | 4 |  |
| `2_6_2` | 6 | The Coast | variants | 9 | 9 | Same idea as Act 1 Coast: shoreline / cliff wall at the entrance |
| `2_6_4` | 6 | The Mud Flats | variants | 3 | 7 | Which way the first trail leaves the entrance; route: [45°] corner of area, normally by a house |
| `2_6_5` | 6 | The Karui Fortress | random |  | 2 | route: Go [45°] / Find and kill the pack of monster on the island [225°] for a Tattoo |
| `2_6_6` | 6 | The Ridge | variants | 2 | 4 | Where the waypoint sits relative to the entrance; route: Go [0°] |
| `2_6_7_1` | 6 | The Lower Prison | variants | 5 | 5 | Which icons (exit, trial, book) are already on the minimap at the entrance |
| `2_6_7_2` | 6 | Shavronne's Tower | random |  | 3 |  |
| `2_6_8` | 6 | Prisoner's Gate | random |  | 16 | route: Go down the ledge next to the road |
| `2_6_9` | 6 | The Western Forest | random |  | 4 | route: Follow the road / Look for the torch touching the road / Follow the trail in the direction of the torch |
| `2_6_10` | 6 | The Riverways | random |  | 11 | route: Follow the road |
| `2_6_11` | 6 | The Wetlands | random |  | 8 | route: Look for 2 pillars near waypoint, follow the trail / Go [315°] |
| `2_6_12` | 6 | The Southern Forest | random |  | 3 | route: Follow the road until it ends / Go [135°] |
| `2_6_13` | 6 | The Cavern of Anger | random |  | 4 |  |
| `2_6_14` | 6 | The Beacon | random |  | 4 | route: Follow the shore |
| `2_6_15` | 6 | The Brine King's Reef | variants | 3 | 4 | Is the south-west blocked off or open |
| `2_7_1` | 7 | The Broken Bridge | random |  | 6 | route: Follow the road, look for the broken waypoint |
| `2_7_2` | 7 | The Crossroads | fixed | 1 | 1 | route: Follow the road |
| `2_7_3` | 7 | The Fellshrine Ruins | random |  | 3 | route: Go [135°] |
| `2_7_4` | 7 | The Crypt | random |  | 4 | route: Follow the road |
| `2_7_5_1` | 7 | The Chamber of Sins Level 1 | fixed | 1 | 3 | route: Go [315°] |
| `2_7_5_map` | 7 | Maligaro's Sanctum | random |  | 4 | route: Follow edges of the zone and cross bridges in the corners / Typically bridges spawn in the opposite diagonal corners |
| `2_7_5_2` | 7 | The Chamber of Sins Level 2 | variants | 4 | 4 | Which way the corridor runs from the entrance; route: Go in same direction as waypoint / Find the switch hidden behind Trial of Ascendancy Plaque to access Hidden Gem: Quality |
| `2_7_6` | 7 | The Den | random |  | 8 |  |
| `2_7_7` | 7 | The Ashen Fields | fixed | 1 | 5 | route: Go [225°] |
| `2_7_8` | 7 | The Northern Forest | random |  | 12 |  |
| `2_7_10` | 7 | The Causeway | random |  | 4 |  |
| `2_7_11` | 7 | The Vaal City | variants | 4 | 4 | Cliff vs wall right by the entrance |
| `2_7_9` | 7 | The Dread Thicket | random |  | 2 |  |
| `2_7_12_1` | 7 | The Temple of Decay Level 1 | random |  | 4 | route: In the room with a central pit, look [225°] or [45°] for a side area containing a Corrupted Unique |
| `2_7_12_2` | 7 | The Temple of Decay Level 2 | random |  | 4 |  |
| `2_8_1` | 8 | The Sarn Ramparts | random |  | 4 |  |
| `2_8_2_1` | 8 | The Toxic Conduits | random |  | 4 | route: Go [270°] |
| `2_8_2_2` | 8 | Doedre's Cesspool | random |  | 4 | route: Black splashes of paint on ground points to the right direction to go |
| `2_8_8` | 8 | The Quay | random |  | 2 | route: Go [45°] / Follow [270°] wall / Go [135°] |
| `2_8_9` | 8 | The Grain Gate | variants | 3 | 3 | Where the waypoint sits relative to the entrance corridor; route: Follow the dead guards by doorways / Look for a book icon |
| `2_8_10` | 8 | The Imperial Fields | no data |  | 4 | route: Follow the dead guards by doorways |
| `2_8_12_1` | 8 | The Solaris Temple Level 1 | random |  | 4 | route: Follow the road until waypoint / Go [315°] |
| `2_8_12_2` | 8 | The Solaris Temple Level 2 | random |  | 4 |  |
| `2_8_11` | 8 | The Solaris Concourse | random |  | 6 |  |
| `2_8_13` | 8 | The Harbour Bridge | fixed | 1 | 2 | route: Go [225°] / Go [135°] |
| `2_8_6` | 8 | The Lunaris Concourse | random |  | 1 | route: Go [315°] |
| `2_8_7_1_` | 8 | The Lunaris Temple Level 1 | random |  | 4 |  |
| `2_8_7_2` | 8 | The Lunaris Temple Level 2 | random + variants | 2 | 12 | Same as Act 3 Lunaris Temple Level 2 |
| `2_9_1` | 9 | The Blood Aqueduct | random |  | 1 | route: Farm Level: 58-62 |
| `2_8_5` | 8 | The Bath House | random |  | 4 | route: Go [180°] / Search [270°] side |
| `2_8_4` | 8 | The High Gardens | random |  | 4 | route: Search [270°] side |
| `2_9_2` | 9 | The Descent | random |  | 4 |  |
| `2_9_3` | 9 | The Vastiri Desert | random |  | 4 | route: Go [90°] |
| `2_9_5` | 9 | The Foothills | fixed | 1 | 4 | route: Go [315°] / Go [45°] until you find the cliff |
| `2_9_6` | 9 | The Boiling Lake | random |  | 4 | route: Go [45°] look for petrified soldiers |
| `2_9_4` | 9 | The Oasis | random |  | 4 | route: Go [45°] |
| `2_9_7` | 9 | The Tunnel | random |  | 4 | route: Go [315°] / Look for the hidden switch halfway to Trial of Ascendancy Plaque in the corner opposite the entrance to access Hidden Gem: Transfigured |
| `2_9_8` | 9 | The Quarry | fixed | 1 | 3 | route: Go [315°] / [45°] or [225°] |
| `2_9_9` | 9 | The Refinery | random |  | 3 | route: [315°] or [45°] / Look for a sewer grate leading to Refinery Tunnels, it is attached to the cart tracks / Go [45°] follow cart tracks |
| `2_9_10_1` | 9 | The Belly of the Beast | random |  | 4 |  |
| `2_9_10_2` | 9 | The Rotting Core | random |  | 1 |  |
| `2_10_1` | 10 | The Cathedral Rooftop | fixed | 1 | 1 | route: Go [0°] |
| `2_10_2` | 10 | The Ravaged Square | fixed | 1 | 1 | route: Go [45°] |
| `2_10_7` | 10 | The Control Blocks | random |  | 4 | route: Go [180°] |
| `2_10_9` | 10 | The Ossuary | random |  | 6 | route: Interact with the switch just before the zig-zagging walls, go to the opposite side to access Hidden Gem: Facetor's Lens |
| `2_10_3` | 10 | The Torched Courts | random |  | 4 | route: Go [135°] |
| `2_10_4` | 10 | The Desecrated Chambers | random |  | 4 | route: Loop around clockwise / Go [315°] counter-clockwise spiral |
| `2_10_5` | 10 | The Canals | random |  | 4 |  |
| `2_10_6` | 10 | The Feeding Trough | fixed | 1 | 4 |  |
