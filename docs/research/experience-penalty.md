# PoE1 Experience Penalty by Area Level

*Path of Exile 1 · game-mechanics research for the Guide panel · November 2026 · resolves #54*

**Question.** How does the experience a Character earns depend on the gap between Character level and area (monster) level? We need the safe range, the curve beyond it, and the level 95+ multiplier, so the Guide panel can show which area levels give full experience and how far the current area is too low or too high.

---

## 1. Short answer

- **Safe range:** `SafeZone = 3 + floor(CharacterLevel / 16)`. Areas whose level is within `CharacterLevel ± SafeZone` give full experience (multiplier 1). The window is the same width above and below.
- **Beyond it:** `d = max(|CharacterLevel − AreaLevel| − SafeZone, 0)` and
  `multiplier = ((L + 5) / (L + 5 + d^2.5))^1.5`, never below **0.01** (1%).
- **Areas above level 70** count as a *lower* level for this calculation (an effective level; area 84 counts as 77.7). So a level 86+ Character can never get full experience anywhere.
- **Level 95–99** get extra multipliers on top of that, applied even inside the safe range.

Sources: GGG's original statement (Malice, 2011), Chris Wilson's 2.3.0 and 3.1.0 posts, and the poewiki.net *Experience* page. Details and caveats follow.

## 2. The formula

### 2.1 Safe zone

GGG (Malice, official forum mechanics thread, Nov 2011): *"There is a safe level range where no penalty is applied, which is equal to three, plus one for every sixteen complete player levels."* Any extra difference is the *Effective Difference*, and it counts **in either direction**: being too high or too low is penalised the same way. ([forum 11707](https://www.pathofexile.com/forum/view-thread/11707/page/1#p216023))

```
SafeZone = 3 + floor(L / 16)        // L = Character level
```

| Character level | 1–15 | 16–31 | 32–47 | 48–63 | 64–79 | 80–95 | 96–100 |
|---|---|---|---|---|---|---|---|
| SafeZone | 3 | 4 | 5 | 6 | 7 | 8 | 9 |

([poewiki Experience](https://www.poewiki.net/wiki/Experience#Experience_penalties), matches GGG)

### 2.2 Penalty curve

```
d          = max(|L − A_eff| − SafeZone, 0)          // A_eff = effective area level, §2.3
multiplier = max( ((L + 5) / (L + 5 + d^2.5))^1.5 , 0.01 )      // L ≤ 94
```

GGG gives the core term verbatim as `((PlayerLevel +5)^1.5) / ((PlayerLevel+5+(EffectiveDifference^2.5))^1.5)` (Malice, same post). The 1% floor comes from poewiki. The multiplier scales the raw experience of each monster killed.

Strictly, the formula compares Character level with **monster** level. Almost every monster in an area is at the area level, which is why tools use area level. The game logs area level in `Client.txt` (`Generating level N area "…"`), so the app can read it.

### 2.3 Effective level of areas above 70 (patch 2.3.0)

Chris Wilson (2.3.0, June 2016): *"We reduced the effective level of endgame areas for the purposes of the experience equation"*. He posted a chart ending at *"Level 84: 80.2 -> 77.7"* and pointed to the wiki for the equation. ([forum 1693152](https://www.pathofexile.com/forum/view-thread/1693152)) This changes only the experience calculation. Monster stats and loot stay the same.

| Area level | ≤70 | 71 | 72 | 73 | 74 | 75 | 76 | 77 | 78 | 79 | 80 | 81 | 82 | 83 | 84 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Effective level | same | 70.94 | 71.82 | 72.64 | 73.40 | 74.10 | 74.74 | 75.32 | 75.84 | 76.30 | 76.70 | 77.04 | 77.32 | 77.54 | 77.70 |

Fitted closed form (poewiki, also used by Exile-UI; it reproduces the table to 2 dp):

```
A_eff = A                                   if A ≤ 70
A_eff = −0.03·A² + 5.17·A − 144.9           if A > 70
```

Caveat: poewiki's version of this table has a "map tier" column from the 2016 Atlas (T3 = 70 … T15 = 82). It is out of date. Use area level only.

### 2.4 Level 95+ multipliers

poewiki's formula for L ≥ 95 multiplies the result by two more factors:

```
multiplier = max( base × 1/(1 + 0.1·(L − 94)) × 1/P(L) , 0.01 )
P(95..99) = 1.065, 1.115, 1.187, 1.2825, 1.4
```

- **P(L)** comes from Chris Wilson, *Path of Nerfs* (3.1.0, Dec 2017): *"small additional experience penalties at levels 95-99"*: 95→96 *"6.5% more experience needed"*, 96→97 11.5%, 97→98 18.7%, 98→99 28.25%, 99→100 40%. ([forum 2035872](https://www.pathofexile.com/forum/view-thread/2035872))
- **The `1/(1 + 0.1·(L−94))` factor** comes from poewiki only. I found no GGG post for it. In 2.3.0 Chris only said *"We'll probably apply experience penalties directly to the upper levels themselves"*. Treat it as community-derived. Exile-UI uses it too.
- **The tools disagree on how to read "X% more experience needed."** poewiki divides by `1 + X` (1/1.065…). Exile-UI divides by `1 − X` (its table is 1.0695, 1.1299, 1.2300, 1.3937, 1.6667 = 1/0.935 … 1/0.6). The literal wording supports poewiki's reading. Also, poewiki's own worked tables for 95–99 leave out P(L) (they were written before 3.1).
- None of this changes the **safe range**, only the percentage shown at 95+. At 95+ the multiplier stays below 1 even inside the safe range.

## 3. Worked examples

Computed with the formulas above, and checked against poewiki's own examples.

**Level 24 Character** (SafeZone 4 → full experience in areas 20–28):

| Area level | Gap past safe range | d | Multiplier |
|---|---|---|---|
| 20–28 | 0 | 0 | 100% |
| 19 or 29 | 1 too low / too high | 1 | (29/30)^1.5 = **95.0%** |
| 17 | 3 too low | 3 | (29/44.59)^1.5 = **52.5%** |
| 15 | 5 too low | 5 | (29/84.90)^1.5 = **20.0%** |

(poewiki gives the same 95% / 52.5% / 20%.)

**Level 40 Character in a level 52 area:** SafeZone 5, |40−52| = 12, d = 7, 7^2.5 = 129.6 → (45/174.6)^1.5 = **13.1%**. Seven levels too high.

**Endgame:** the full-experience band, converted back to real area levels using §2.3:

| Character level | SafeZone | Effective band | Real area levels with 100% |
|---|---|---|---|
| 70 | 7 | 63–77 | 63–80 |
| 75 | 7 | 68–82 | 68–84 (all maps) |
| 80 | 8 | 72–88 | 73–84 |
| 84 | 8 | 76–92 | 79–84 |
| 85 | 8 | 77–93 | 81–84 |
| 86+ | 8+ | ≥78 | **none** (max effective level is 77.7) |

Level 90 in a level 77 area (effective 75.32): d = 14.68 − 8 = 6.68 → **30.4%**, which matches poewiki's 0.30355.

## 4. How other tools show it

**Exile-UI** (Lailloken, AutoHotkey, `modules/leveling tracker.ahk`, `Leveltracker_Experience`, commit `4b34ada`, Nov 2026)
- Implements the full formula: safe zone, `d^2.5`, `^1.5`, the 1% floor, the quadratic effective level above 70, and both 95+ factors (with the `1 − X` table above).
- Shows a compact readout on the leveling panel:
  - **Inside the safe range:** `100%` plus `min | area | max`, e.g. `100% 20 | 24 | 28`.
  - **Outside it:** the percentage to one decimal plus the **signed number of levels past the edge of the safe range**, e.g. `52.5% (−3)` (area too low) or `95.0% (+1)` (too high).
- Treats town and hideout as 100%. Returns `???` for PoE2.

**exile-leveling** (HeartofPhos, web, `web/src/components/FragmentStep/Fragment/index.tsx`)
- Does not show a percentage. Next to each area name in the route it prints the lowest Character level that still gets full experience there, e.g. `The Ledge 9+`, computed as `max(1, A − (3 + floor(A/16)))`.
- This approximates by taking floor(·/16) of the *area* level instead of the Character level. It is one level too low for areas 16–19, 32–36, 48–53, 64–70 and 80–84 (e.g. area 50 shows `44+`, but a level 44 has SafeZone 5 and tops out at 49, so the correct value is 45).
- It shows no upper bound and does not apply the effective-level adjustment above 70.

## 5. Recommendation for the Guide panel

- Read Character level from the app's existing level tracking and area level from the `Generating level N area` log line.
- Compute `SafeZone = 3 + floor(L/16)`. Show the full-experience band as `L − SafeZone … L + SafeZone`, clamped at 1, with the upper end converted back to real area level when above 70 (§3 table).
- Status:
  - **inside** → "Full XP".
  - **below** → "N levels too low", with N = `(L − SafeZone) − A_eff`.
  - **above** → "N levels too high", with N = `A_eff − (L + SafeZone)`.
  - Optionally add the percentage from §2.2, which matches Exile-UI's signed offset.
- Treat towns and hideouts as neutral.
- For L ≥ 95, mark the percentage as approximate: two of the factors are not confirmed by GGG, and the tools read the 3.1 penalty differently (§2.4).

## Sources

- Malice (GGG), *Mechanics thread*, Nov 2011. Safe zone and penalty formula, verbatim. https://www.pathofexile.com/forum/view-thread/11707/page/1#p216023
- Chris Wilson (GGG), *2.3.0 Changes to the High-level Experience Equation*, Jun 2016. Effective levels above 70. https://www.pathofexile.com/forum/view-thread/1693152
- Chris Wilson (GGG), *Path of Nerfs*, Dec 2017. Level 95–99 penalties. https://www.pathofexile.com/forum/view-thread/2035872
- poewiki.net, *Experience*, "Experience penalties" section (read via the Wayback snapshot of 2026-06-15, because the live site blocks automated fetches). https://www.poewiki.net/wiki/Experience
- Exile-UI source. https://github.com/Lailloken/Exile-UI/blob/main/modules/leveling%20tracker.ahk
- exile-leveling source. https://github.com/HeartofPhos/exile-leveling/blob/main/web/src/components/FragmentStep/Fragment/index.tsx
