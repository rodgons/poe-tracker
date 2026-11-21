# PoE Overlay Field Guide

*Path of Exile 1 · third-party tool research · November 2026*

**Building an interactive overlay with an automatic checklist.** How overlay apps sit on top of the game, which game events an outside program can legitimately see, and how to turn those events into a checklist that ticks itself. Event formats were checked against a real `Client.txt` on the owner's Mac.

Source: https://claude.ai/artifact/Vt4D1sThYrhCtfjZPwLYhg (original page saved next to this file as `overlay-field-guide.html`, which also has the diagrams).

---

## 1. Short answer

| Answer | Claim | Detail |
|---|---|---|
| **Yes** | An interactive overlay is very doable | It is a normal transparent, always-on-top desktop window. It never touches the game process. Awakened PoE Trade and Exile-UI already work this way. |
| **Yes, with limits** | You can track events for an auto-checklist | The game writes a live log file. Zone changes, level-ups, deaths, quest skill points, passive allocations and NPC dialogue all appear in it. |
| **Not available** | No kills, loot or quest-state feed | No event for monster kills, item pickups or a quest log. Infer progress from zones, rewards and dialogue, with a manual tick as fallback. |

Target result: a transparent overlay window covering the game that ignores the mouse, with one checklist panel (e.g. "Act 1 checklist 3/7") that is the only part capturing clicks. Ticks come from the log; some steps need a manual tick.

## 2. What GGG allows

GGG publishes a third-party policy in its developer docs. It does not approve individual tools, and staff decline to confirm whether a given tool is safe.

| Status | What | Why |
|---|---|---|
| Permitted | A separate program that draws its own window over the game | Executable apps that run independently from the game are "permitted" though not encouraged. |
| Permitted | Reading the game's log file | OK "as long as the user is aware of what you are doing with that data." |
| Permitted | Reading the clipboard after the player presses Ctrl+C on an item | Built-in game feature; how every price checker works. |
| Restricted | Sending keystrokes or chat commands to the game | Only when the player triggers it by hand, one game action per press. Timers, file-change triggers and screen-reading triggers may not fire inputs. |
| Grey area | Screen pixels / OCR to update your own UI | Not named in the policy. Exile-UI uses it only to detect open panels. Never use it to trigger input. |
| Forbidden | Reading game memory, injecting code, hooking the renderer, editing game files, packet sniffing | Breaks Terms of Use 7b, 7c, 7i. "Will result in immediate account termination." |

> **Implication for an auto-checklist:** reading the log and updating our own window sends nothing to the game, so the macro rules don't apply (our reading of the policy, not a GGG ruling). Never let a log event automatically fire a keystroke or chat command — file-change reactions are a banned trigger.

## 3. How an overlay works

No game API. The overlay is a second window owned by the app, placed over the game window, transparent, always on top. Click-through is a per-window switch: mouse capture is on only while the pointer is over the panel.

### The four things every overlay must get right

1. **Display mode.** Game must run in Windowed Fullscreen or Windowed. Exclusive Fullscreen can cover always-on-top windows.
2. **Click-through.** Ignore the mouse by default; re-enable over interactive elements. Electron: `setIgnoreMouseEvents(true, { forward: true })` — `forward` keeps hover events flowing so you can detect the pointer entering the panel.
3. **Focus.** Clicking the panel must not steal keyboard focus from the game. Use a non-activating window (`focusable: false`, or a panel-type window on macOS). Take focus only when the player must type, then hand it back.
4. **Following the game.** Show only while the game is foreground; match its position/size; hide on alt-tab. `electron-overlay-window` does this by finding the game window by title.

### Platform notes

| Platform | Overlay window | Things that bite |
|---|---|---|
| Windows | Layered, topmost window with transparent + no-activate extended styles. Best supported. | If the game runs as Administrator, the overlay must too. |
| macOS | Floating panel above normal windows, allowed on all Spaces incl. full-screen. Electron: `setAlwaysOnTop(true, 'screen-saver')` + `setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })`. | Native full-screen puts the game in its own Space. Tracking another app's window and global hotkeys need Accessibility permission. Unsigned apps trigger Gatekeeper. |
| Linux | X11 works with a compositor. Wayland restricts positioning over other apps. | Needs compositing for transparency, or the overlay may render black. |

> **macOS behaviour is version-sensitive.** The window-level/Spaces settings come from community write-ups and have shifted between Electron and macOS releases. Test on the machine first. Working references: Awakened PoE Trade ships a universal macOS build; `electron-overlay-window` has a macOS backend in source though its README lists only Windows and Linux.

## 4. Picking a stack

| Option | Good for | Trade-off | Proven by |
|---|---|---|---|
| **Electron + TypeScript** | Fastest route to a polished interactive UI on Win/macOS/Linux. Ready-made overlay library. | ~150–300 MB memory, large download. | Awakened PoE Trade, PoE Overlay |
| Tauri (Rust + web UI) | Same web UI, much lighter. | Write game-window tracking per platform; click-through needs more native work. | Newer community overlays |
| AutoHotkey | Tiny scripts, quick prototypes. | Windows only; UI drawn by hand with GDI+. | Exile-UI |
| Native (Swift / C# WPF) | Lowest overhead, full control. | One codebase per platform. | Sidekick (C#) |
| Overwolf | Handles rendering and distribution. | Windows only, ad-supported, store rules. | PoE Overlay II |

**Recommendation: Electron with TypeScript.** The owner is on a Mac with Steam, and Electron is the only option with an open-source PoE overlay already running on macOS to learn from. The parser and checklist engine are plain TypeScript and can move to Tauri later unchanged.

## 5. Game events you can read

Four sources. Only the log is a live push stream.

```
Client.txt → Tailer (reads new bytes) → Parser (line → event) → Rule engine (event → step) → Checklist (saved state) → Overlay UI
                                         on demand into the rule engine/checklist: Clipboard (Ctrl+C item text), Web API (character snapshot), Player (click or hotkey)
```

### Source 1: the Client.txt log

| Install | Path |
|---|---|
| macOS (found on owner's Mac) | `~/Library/Application Support/Path of Exile/Cache/Logs/Client.txt` |
| Windows, Steam | `C:\Program Files (x86)\Steam\steamapps\common\Path of Exile\logs\Client.txt` |
| Windows, standalone | `C:\Program Files (x86)\Grinding Gear Games\Path of Exile\logs\Client.txt` |

Line anatomy:

```
2026/11/10 12:06:02  3336199872  e63da844  [INFO Client 54611]  : You have entered City Square.
└ local date/time    └ ms counter └ source id └ level + process id └ message (leading colon = system chat line)
```

Events found in the owner's log (163,140 lines, 22 Jul – 10 Nov 2026; counts = regex matches from the starter parser):

| Event | Message as written | Matches | Checklist use |
|---|---|---:|---|
| Area generated | `Generating level 10 area "1_1_8" with seed 895163853` | 4,239 | Best zone signal: internal area id + area level ~1s before load-in. |
| Area entered | `: You have entered The Coast.` | 4,237 | Display name; ambiguous alone (names repeat across acts). |
| Level up | `: YourChar (Inquisitor) is now level 80` | 396 | Level goals; reveals character name and class/ascendancy. |
| Death | `: YourChar has been slain.` | 1,280 | Death counter, hardcore warnings. |
| Skill point reward | `: You have received a Passive Skill Point.` | 114 | Confirms a point-granting quest hand-in (doesn't say which). |
| Other point rewards | `: You have received 2 Passive Respec Points.` / `: You have received an Atlas Skill Point.` | 655 | Respec rewards, atlas progress. |
| Quest point list | `: 22 Passive Skill Points from quests:` / `: (1 from The Marooned Mariner)` | 335 | Printed on `/passives`. Names every rewarded quest → full resync. |
| Passive allocated | `Successfully allocated passive skill id: life_regeneration1087, name: Life Regeneration` | 2,505 | Tick planned tree nodes. `unallocated` variant exists. |
| Act boss milestone | `: You have been permanently weakened by Kitava's cruel affliction. …` | 10 | Marker for finishing act 5 (cruel) and act 10 (merciless). |
| NPC dialogue | `Einhar, Beastmaster: Exile! You are a welcome omen.` / `Izaro: Your destination is more dangerous …` | many | Encounter cues (league masters, Izaro, Maven, Exarch). Match against known speaker names. |
| Trade result | `: Trade accepted.` / `: Trade cancelled.` | 249 | Trade steps. |
| Monsters remaining | `: 2 monsters remain.` | 31 | Printed on `/remaining`. |
| Loading time | `[LOADING SCREEN] (City Square) Duration = 1.96797 seconds` | 4,225 | Zone playable moment; timers. |
| Whisper / player joined | `@From <player>: …` / `: <player> has joined the area.` | few | Trade and party helpers. |
| Client started | `***** LOG FILE (id …) OPENING *****` | n/a | New game session; no bracketed header. |

### Area ids are the key to campaign tracking

The id in `Generating level … area "…"` is unique per zone. Campaign ids follow `part_act_zone`: part 1 = acts 1–5, part 2 = acts 6–10. 152 distinct campaign ids were found in the log. Act 1:

| Id | Zone | Id | Zone |
|---|---|---|---|
| `1_1_1` | The Twilight Strand | `1_1_6` | The Climb |
| `1_1_town` | Lioneye's Watch | `1_1_7_1` | The Lower Prison |
| `1_1_2` | The Coast | `1_1_7_2` | The Upper Prison |
| `1_1_2a` | The Tidal Island | `1_1_8` | Prisoner's Gate |
| `1_1_3` | The Mud Flats | `1_1_9` | The Ship Graveyard |
| `1_1_4_1` | The Submerged Passage | `1_1_9a` | The Ship Graveyard Cave |
| `1_1_4_0` | The Flooded Depths | `1_1_11_1` | The Cavern of Wrath |
| `1_1_5` | The Ledge | `1_1_11_2` | The Cavern of Anger |

Other ids: `2_9_1` (act 9), `2_11_endgame_town` (Karui Shores), `HideoutSpace` (hideout), `MapWorlds…` for maps (e.g. `MapWorldsCitySquare`). The MIT `exile-leveling` project has full area and quest data extracted from game files.

### Source 2: clipboard

Ctrl+C over an item copies its text; Ctrl+Alt+C copies an extended version. Watch and parse the clipboard for item checklist steps ("pick up a 4-link") that the player confirms by hovering and pressing a key.

### Source 3: official web API

GGG OAuth API: characters with equipment and passives (`account:characters`), stash tabs (`account:stashes`), atlas passives (`account:league_accounts`). Desktop app = public client: Authorization Code + PKCE, local redirect like `http://127.0.0.1:8080/callback`, 10-hour access tokens, rate limits shared across public clients.

> **Registration is closed right now** — docs say GGG is "unable to process new applications." Plan v1 without the API. It's a snapshot source with no quest state or push events anyway.

### Source 4: looking at the screen

Pixel checks/OCR can detect open inventory or tooltips. Fragile across resolution/UI scale, costs CPU, policy grey area, needs macOS Screen Recording permission. Leave out until the log proves insufficient.

### What you cannot detect

- **Monster and boss kills** — not logged. Use next zone, a reward line, or boss dialogue.
- **Item pickups/drops** — not logged. Clipboard on demand only.
- **Quest state** — no quest log. Skill point lines and `/passives` cover point-granting quests; others need zone inference or manual tick.
- **Waypoints, gems socketed, flasks used, current life** — not logged; reading memory is forbidden.
- **Which character is logged in** — not announced at login. Learn from first level-up/death line, or ask the player.

## 6. Auto-checklist design

Each step = text + completion condition. The engine holds a pointer to the current step and compares each event with the conditions of the next few open steps.

```json
[
  { "id": "a1-town",   "text": "Reach Lioneye's Watch",   "when": { "type": "area",  "id": "1_1_town" } },
  { "id": "a1-coast",  "text": "Enter The Coast",         "when": { "type": "area",  "id": "1_1_2" } },
  { "id": "a1-lvl2",   "text": "Reach level 2",           "when": { "type": "level", "min": 2 } },
  { "id": "a1-island", "text": "Enter The Tidal Island",  "when": { "type": "area",  "id": "1_1_2a" } },
  { "id": "a1-hail",   "text": "Kill Hailrake",           "when": { "type": "manual" } },
  { "id": "a1-point",  "text": "Hand in to Bestel for the skill point",
                       "when": { "type": "points", "kind": "Passive Skill" } },
  { "id": "a1-mud",    "text": "Enter The Mud Flats",     "when": { "type": "area",  "id": "1_1_3" } }
]
```

Reaching a later zone closes skipped steps before it, marked as **inferred** so the player can reopen them.

### Rules that make it feel reliable

- **Look ahead, not just at the next step.** If an event matches a step a few places ahead, complete it and mark skipped ones "inferred" (different colour).
- **Every step can be ticked/unticked by hand** — click or global hotkey. Detection is an assist, never the only way forward.
- **Key on area id, not name.** Use the `Generating level` id; treat the following `You have entered` as confirmation.
- **Ignore noise zones.** Hideouts, revisited towns and map ids don't move the campaign pointer.
- **Offer a resync.** On `/passives`, read the quest names that follow and tick all matching quest steps. If bound to a hotkey, the player must press it (one press, one chat command).
- **Catch up on start.** On launch, scan back through the log tail to the last `LOG FILE … OPENING` and replay those events.
- **Save per character.** Key progress by character name (from level-up/death lines), manual picker as fallback.
- **Tell the user what you read.** First-run notice satisfies the policy's awareness condition.

## 7. Starter code (TypeScript / Electron)

### 7.1 Overlay window

```ts
// main.ts
import { app, BrowserWindow, ipcMain, screen } from 'electron'

app.whenReady().then(() => {
  const { bounds } = screen.getPrimaryDisplay()
  const win = new BrowserWindow({
    ...bounds,
    transparent: true, frame: false, hasShadow: false,
    resizable: false, skipTaskbar: true,
    focusable: false,                 // clicks never steal focus from the game
    webPreferences: { preload: __dirname + '/preload.js' },
  })
  win.setAlwaysOnTop(true, 'screen-saver')
  win.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })  // macOS Spaces
  win.setIgnoreMouseEvents(true, { forward: true })                 // click-through by default

  // renderer tells us when the pointer is over the panel
  ipcMain.on('overlay:hover', (_e, over: boolean) =>
    win.setIgnoreMouseEvents(!over, { forward: true }))

  win.loadFile('index.html')
})
```

```ts
// renderer: flip click-through as the pointer enters or leaves the panel
const panel = document.querySelector('#panel')!
panel.addEventListener('mouseenter', () => window.overlay.hover(true))
panel.addEventListener('mouseleave', () => window.overlay.hover(false))
```

For production, replace fixed primary-display bounds with `electron-overlay-window`, which attaches to the game window by title and shows/hides with game focus.

### 7.2 Log tailer

```ts
// tail.ts: emit each new line appended to Client.txt
import fs from 'node:fs'

export function tail(path: string, onLine: (line: string) => void) {
  let pos = fs.statSync(path).size        // start at the end: the file is large
  let rest = ''
  setInterval(() => {
    const size = fs.statSync(path).size
    if (size < pos) pos = 0                // file was truncated or replaced
    if (size === pos) return
    const buf = Buffer.alloc(size - pos)
    const fd = fs.openSync(path, 'r')
    fs.readSync(fd, buf, 0, buf.length, pos); fs.closeSync(fd)
    pos = size
    const lines = (rest + buf.toString('utf8')).split(/\r?\n/)
    rest = lines.pop() ?? ''               // keep a partial last line for next time
    lines.forEach(l => l && onLine(l))
  }, 250)
}
```

Polling size every 250 ms is more dependable than fs watch events, which can be delayed or merged while the game holds the file open. The owner's log is 18 MB — never re-read from the start.

### 7.3 Parser

```ts
// parse.ts: turn one log line into a typed event, or null
const LINE = /^(\d{4}\/\d\d\/\d\d \d\d:\d\d:\d\d) \d+ [0-9a-f]+ \[(INFO|DEBUG|WARN|CRIT) Client \d+\] (.*)$/

const RULES = {
  areaGen:  /^Generating level (\d+) area "([^"]+)" with seed (\d+)/,
  entered:  /^: You have entered (.+)\.$/,
  level:    /^: (.+) \((\w+)\) is now level (\d+)$/,
  slain:    /^: (.+) has been slain\.$/,
  points:   /^: You have received (?:an?|(\d+)) (.+?) Points?\.$/,
  passive:  /^Successfully (allocated|unallocated) passive skill id: (\S+), name: (.+)$/,
  questPt:  /^: \((\d+) from (.+)\)$/,
  kitava:   /^: You have been permanently weakened by Kitava's (cruel|merciless) affliction/,
  trade:    /^: Trade (accepted|cancelled)\.$/,
  whisper:  /^@(From|To) (?:<.+?> )?(.+?): (.*)$/,
  loading:  /^\[LOADING SCREEN\] \((.+)\) Duration = ([\d.]+) seconds/,
}
// NPC dialogue has the form "Speaker: text". Match against known names only,
// because engine lines such as "Tile hash: 123" have the same shape.
const NPCS = new Set(['Izaro', 'Einhar, Beastmaster', 'Jun, Veiled Master', 'The Maven'])

export function parse(line: string) {
  const m = LINE.exec(line)
  if (!m) return line.includes('LOG FILE') ? { type: 'clientStart' } : null
  const [, time, , body] = m
  for (const [type, re] of Object.entries(RULES)) {
    const x = re.exec(body)
    if (x) return { type, time, args: x.slice(1) }
  }
  const i = body.indexOf(': ')
  if (i > 0 && NPCS.has(body.slice(0, i)))
    return { type: 'npc', time, args: [body.slice(0, i), body.slice(i + 2)] }
  return null
}
```

> **Language matters.** Patterns match the English client; system messages are localised. `Generating level … area` and `Successfully allocated passive` are engine output and probably stay English, but only an English log was checked.

## 8. Build order

1. **Log reader in a terminal** — tailer + parser printing events while playing. No UI. Proves the core idea in an evening.
2. **Bare overlay window** — transparent Electron window with one box above the game in Windowed Fullscreen. Sort out click-through and focus on the Mac before anything depends on it.
3. **Checklist engine** — step JSON, rule matching with look-ahead, manual tick, saved state. Unit-test by replaying slices of the existing log.
4. **Wire them together** — events from main process to renderer over IPC. Act 1 checklist ticking itself is the first real milestone.
5. **Game-window tracking and hotkeys** — attach to game window, hide on alt-tab, show/hide hotkey, "tick current step" hotkey.
6. **Content and extras** — full campaign data from `exile-leveling`, `/passives` resync, passive-tree checklist, clipboard item checks, checklist editor.

## 9. Tools to study

| Project | Stack | Why read it |
|---|---|---|
| [Awakened PoE Trade](https://github.com/SnosMe/awakened-poe-trade) | Electron, Vue, TS | Reference overlay: window handling, focus logic, clipboard item parsing. Builds for Win/Linux/macOS. |
| [electron-overlay-window](https://github.com/SnosMe/electron-overlay-window) | Native Node addon | Attaches an Electron window to a target window by title and keeps it in sync. |
| [Exile-UI](https://github.com/Lailloken/Exile-UI) | AutoHotkey | Closest to this idea: Act-Tracker is an automated campaign guide driven by Client.txt. |
| [exile-leveling](https://github.com/HeartofPhos/exile-leveling) | TS web app, MIT | Campaign route + area, quest, gem data from game files. |
| [poe-log-monitor](https://github.com/klayveR/poe-log-monitor) | Node library | Older Client.txt event list: area, level, death, whisper, trade, NPC, AFK. |

## 10. Sources

- [Path of Exile developer docs](https://www.pathofexile.com/developer/docs) — third-party policy, rate limits, registration status
- [Developer docs, Authorization](https://www.pathofexile.com/developer/docs/authorization) — client types, PKCE, token lifetimes, scopes
- [Awakened PoE Trade download page](https://snosme.github.io/awakened-poe-trade/download) — platforms, display-mode requirement
- [electron-overlay-window README and source](https://github.com/SnosMe/electron-overlay-window)
- [Exile-UI README](https://github.com/Lailloken/Exile-UI)
- [exile-leveling README](https://github.com/HeartofPhos/exile-leveling)
- [poe-log-monitor README](https://github.com/klayveR/poe-log-monitor)
- [GGG forum replies on third-party tools](https://pathofexile.com/forum/view-thread/3637217)
- Owner's local `Client.txt`, 163,140 lines, read 10 Nov 2026

*Not affiliated with or endorsed by GGG. Policies change — re-read the developer docs before publishing a tool. Clipboard shortcuts and Windows window-style notes come from general knowledge and were not re-verified.*
