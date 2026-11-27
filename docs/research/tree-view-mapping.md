# How the game's passive tree view maps to our tree data

*Path of Exile 1 · research for the Tree overlay (map #42, ticket #43) · November 2026*

**Short answer.** With the in-game tree **fully zoomed out**, a tree-data unit is `H / 10000` screen pixels, where `H` is the game's client-area height in physical pixels (0.108 px/unit at 1080p). Positions are that uniform scale plus a translation: no rotation, no flip, y pointing down. That is how Exile-UI's working "Skill-Tree Overlay (PoB schematics)" draws GGG's data, and its docs say it only works fully zoomed out, that the in-game tree resets its position on every relog, and that dragging both trees together keeps them aligned. Nothing first-party documents the game's zoom steps, zoom anchor or pan clamp. Those need the in-game checks at the end.

Legend: **Confirmed** = read in source code or first-party docs. **Inferred** = follows from a source but isn't stated by it. **Unknown** = no source found; only an in-game measurement settles it.

## Sources

| Source | What it gave | Where |
|---|---|---|
| Exile-UI source, `modules/leveling tracker.ahk`, `Leveltracker_PobSkilltree()` | The overlay's scale formula, node-position maths, placement and drag code | [535086b](https://github.com/Lailloken/Exile-UI/blob/535086b120c7f6b8c669903a0660c1a1c341f1e6/modules/leveling%20tracker.ahk) lines 2715–3157 |
| Exile-UI `modules/hotkeys.ahk` | Overlay hotkeys: R-drag, MButton snap, Q/E/W/1–4, Alt hide | same commit, lines 487–498 |
| Exile-UI `data/english/help tooltips.json` (`leveltrackerschematics.how-to`) | "l-click: drag in-game tree, r-click: drag overlay (both can be dragged simultaneously); m-click: snap white/yellow circle to cursor" | same commit, lines 1696–1715 |
| Exile-UI wiki, *Act‐Tracker* page, "Skill-Tree Overlay (PoB schematics)" | Author's usage rules: fully zoomed out, relog resets position, MButton re-anchor | [wiki](https://github.com/Lailloken/Exile-UI/wiki/Act%E2%80%90Tracker#skill-tree-overlay-pob-schematics) |
| Exile-UI wiki, *Home* | Supported client heights: 720/900/1080/1200/1440/1600/1800/2160p, windowed or windowed fullscreen | [wiki](https://github.com/Lailloken/Exile-UI/wiki) |
| Path of Building `src/Classes/PassiveTreeView.lua`, `PassiveTree.lua` | PoB's own (not the game's) view model, for comparison | [0d3691d](https://github.com/PathOfBuildingCommunity/PathOfBuilding/blob/0d3691db2f99f415a249d2ea3dc362d8af2b68db/src/Classes/PassiveTreeView.lua) |
| GGG `grindinggear/skilltree-export`, tag `3.30.0`, `data.json` + README | Bounds, `imageZoomLevels`, orbit constants, orbit angle table | [3.30.0](https://github.com/grindinggear/skilltree-export/tree/3.30.0) |
| This repo: `scripts/extract_tree.py`, `src-tauri/src/tree.rs`, `src-tauri/data/tree.json` | Our coordinates are GGG's, unchanged except rounding | — |

## 1. Our data's coordinate system

- **Confirmed.** `extract_tree.py` computes each node as `x = group.x + r·sin(a)`, `y = group.y − r·cos(a)` from GGG's `groups`, `orbitRadii`, `skillsPerOrbit` and the 16/40-slot angle tables, and rounds to integers. This is the formula GGG's README gives for orbits (the 3.17.0 angle table) and the one Exile-UI uses (`x = r·cos(a−90°)`, `y = r·sin(a−90°)`, the same thing). So `tree.json` x/y are **GGG tree units**, y down, origin at the Scion start.
- **Confirmed.** Rounding moves a node by at most 0.5 unit, which is 0.05 px at 1080p fully zoomed out. It doesn't matter.
- **Confirmed** (computed from `tree.json` 3.30.0). Main-tree nodes span x −9772…9782 and y −9124…9362 (19 554 × 18 486 units). GGG's `min_x/max_x/min_y/max_y` (−14159, 12430, −10689, 10900) are wider because they include ascendancy art. Use our own node extents, not GGG's bounds, for anything about the main tree.
- **Confirmed.** The class start circles (the overlay's natural Anchor) in `tree.json`: Scion (0) 3,7 · Marauder (1) −2824,1642 · Ranger (2) 2844,1642 · Witch (3) −4,−3180 · Duelist (4) 3,3188 · Templar (5) −2837,−1629 · Shadow (6) 2844,−1628. Exile-UI anchors on exactly this point (the start node's group centre).

## 2. Uniform scale + offset?

- **Confirmed (Exile-UI's model).** Exile-UI draws every node and connection as `(tree.x − xOffset)·scale`, `(tree.y − yOffset)·scale` into one bitmap and positions that bitmap with a single window x/y. Only scale and translation are applied: no rotation, no flip, no per-region terms.
- **Inferred.** The game uses the same affine mapping. A widely used overlay that relies on it, with only a manual translation, would visibly drift at the tree's edges otherwise. No GGG document says so.
- **To check in game:** item 1 of the checklist (two horizontal and two vertical start-to-start distances).

## 3. Scale at full zoom-out

- **Confirmed (Exile-UI code).** `scale := "0." (StrLen(vars.client.h) < 4 ? "0" : "") vars.client.h`, which for 3- or 4-digit heights is exactly **`scale = H / 10000` pixels per tree unit**. `H` is the client-area height in physical pixels (the GUI is created `-DPIScale`). Width doesn't appear in it.
- **Confirmed (Exile-UI wiki).** "For the time being, the overlay has to be used while the in-game tree is fully zoomed out." So this scale is the **fully-zoomed-out** scale.

| Client height | px per unit | Main tree on screen (19 554 × 18 486 units) |
|---|---|---|
| 720 | 0.072 | 1408 × 1331 px |
| 1080 | 0.108 | 2112 × 1997 px |
| 1440 | 0.144 | 2816 × 2662 px |
| 2160 | 0.216 | 4224 × 3993 px |

  At every 16:9 size the whole main tree is about 1.1 window widths by 1.85 window heights, so even fully zoomed out the player sees only part of it.
- **Inferred.** The game scales the tree view with window height only (the formula has no width term, and Exile-UI supports several aspect ratios: 1200p, 1600p). How exact `H/10000` is (exact, or Exile-UI's rounding of something like 0.1078·H/1000) isn't stated. Treat it as the starting value and keep the Align-mode fine-tune.
- **Unknown.** The game's UI-scale setting. Exile-UI has no reference to any UI-scale option (searched its whole source). If PoE 1 offers one, whether it changes the tree's scale needs measuring.
- **Unknown, Mac-specific.** On a Retina Mac, `H` must be the game's **rendered** height in pixels, and our overlay draws in **points**: `points per unit = H_px / 10000 / backingScaleFactor`. Whether the macOS client renders at full Retina resolution or at the point size needs checking (checklist item 7).
- **Not the game's scale.** GGG's `imageZoomLevels` `[0.1246, 0.2109, 0.2972, 0.3835, 0.5]` are the website planner's sprite resolutions. Sprite sizes scale exactly with them (background sprite 31→53→76→98→128 px), and PoB uses them only to choose which sprite sheet to load (`data[0.3835]`). They aren't the in-game zoom steps. 0.1246 is close to the in-game zoom-out scale at ~1246 px tall only by coincidence.

## 4. Zoom steps and anchor

- **Unknown for the game.** No source gives the number of in-game zoom steps, the factor per step, or whether the wheel zooms around the cursor. Exile-UI doesn't follow zoom at all, which is why it needs full zoom-out.
- **For comparison only (PoB, not the game).** PoB's view has 13 levels (`zoomLevel` 0–12), `zoom = 1.2^level`, the wheel steps ±1 (±3 with Shift), and it zooms **around the cursor**: `zoomX = relX + (zoomX − relX)·factor`. Its base scale is `min(viewport w, h) / (1.1 · min(treeWidth, treeHeight))`. Don't assume the game matches.
- **Design consequence (inferred).** Full zoom-out is the one zoom state the player can always get back to (scroll until it stops), so it's the right reference for the fallback "keep the tree fully zoomed out and re-align". Following zoom needs the step count, factor and anchor from checklist item 4.

## 5. Panning

- **Confirmed (Exile-UI docs).** "After aligning, you can hold left- and right-click to drag both trees at the same time": left-drag pans the game's tree while Exile-UI moves its overlay window by exactly the mouse delta (`Leveltracker_PobSkilltreeMove`, a 15 ms timer setting `x = mouse − grab offset`).
- **Inferred.** The game's left-drag pan is **1:1 with the mouse** (at least fully zoomed out), or that trick would drift. Whether there's a dead zone before a drag starts (PoB uses 5 px) and whether it stays 1:1 at other zooms is unknown.
- **Unknown.** Clamping at the edges. Exile-UI says nothing. PoB clamps its own pan to ±(2/3)·zoom·viewport, which says nothing about the game. Clamping matters: if the game stops panning while the mouse keeps moving, an overlay that follows mouse deltas drifts. Checklist item 5.

## 6. Reset behaviour and starting view

- **Confirmed (Exile-UI wiki).** "The in-game skill-tree resets its position on every relog, so the overlay has to be re-aligned every time." Exile-UI's fix is the same as our planned Anchor: move the cursor to the starting circle and middle-click, which sets `overlay.x = mouse.x − classOrigin.x` (and the same for y).
- **Inferred.** Within a session the game keeps the tree's pan between closing and reopening the panel: Exile-UI keeps its overlay position across close/reopen and only asks for re-alignment after a relog. Area changes are unconfirmed either way. Exile-UI doesn't react to them, which weakly suggests the pan survives them. Whether **zoom** also survives a relog or a reopen isn't documented.
- **Inferred.** The view starts centred on the class start. On first open Exile-UI places the class origin at the **client-area centre** (`xPos := monitor.x + client.xc − classOrigin.x`), which only makes sense if that's where the game puts it. The zoom level on first open after login is unknown.

## 7. What this means for the alignment model

1. `screen = anchor_screen + (node − classStart) · s`, with `s = H_px / 10000` (converted to points on macOS) at full zoom-out. One scalar plus one 2-D offset. No rotation.
2. The Anchor click fixes the offset (as Exile-UI does). Saving the offset per window size is only valid until a relog. Saving the **scale** per window size is valid indefinitely.
3. Following pans by mouse deltas is plausible (1:1 inferred) but breaks at any edge clamp. Following zoom needs the measured step table and anchor rule.
4. Keep a manual scale fine-tune: the `H/10000` constant comes from a third-party tool, not GGG.

## In-game checklist for the owner (Mac)

Take screenshots with ⌘⇧4 (they're at Retina pixel size) and measure in Preview. Use windowed mode, game tree open with P, **scrolled fully out** unless a step says otherwise. Expected values use `s = H/10000` with `H` the game's rendered height in pixels.

1. **Scale and uniformity.** Measure Marauder→Ranger start circles (both at y = 1642, 5668 units apart). Expect a horizontal line, `5668·s` px long (612 px at H = 1080, 816 at 1440). Then Witch→Duelist (6368 units, nearly vertical): expect `6368·s` (688 / 917). The same ratio on both axes means a uniform scale with no rotation.
2. **Height only.** Change the window height and repeat item 1: the length should scale with height. Change only the width: the length shouldn't change.
3. **UI scale.** If the game's options have a UI-scale setting, change it and repeat item 1.
4. **Zoom.** Count wheel notches from fully out to fully in. At each notch, measure Marauder→Ranger (or any fixed pair) to get the factor per step. Put the cursor on a node near a corner and scroll one notch: if the node stays under the cursor, zoom is around the cursor; if it moves toward the centre, zoom is around the screen centre.
5. **Pan.** Left-drag the tree 300 px measured on screen: does a node move exactly 300 px? Is there a small dead zone before it moves? Drag far past the tree's edge: does it stop (clamp), and where?
6. **Resets.** Note the view (pan + zoom). Close and reopen the tree, change area, then log out and back in, checking after each whether pan and zoom are kept. After login, is the class start at the window centre, and at which zoom?
7. **Retina.** Compare the game window's height in points (from the window frame or the app's own window query) with the screenshot's pixel height and the game's resolution setting, to learn whether `H` is points × 2 or something else.
