# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(A Tauri v2 desktop app for macOS, Windows and Linux. Its interface is plain HTML/JS pages in `ui/`, so the design language is web, not native.)

## Users

Path of Exile 1 players levelling a character from a Path of Building build: league starters, people levelling alts, and newer players copying a guide's PoB. They are mid-game, often in a fight or a map, and want to know the next passive points without alt-tabbing to PoB or a second screen.

## Product Purpose

It shows which passive points to take next. The player pastes a PoB build, picks a Tree spec and names their Character once. From then on the Corner panel shows the spec's Mini-tree with the next points numbered, and the points tick off by themselves as the game writes allocations to `Client.txt`. It succeeds when the player follows the build all the way through without ever looking it up or touching the app mid-session.

## Positioning

Three things together, which a second-screen PoB or a generic overlay can't honestly claim:

1. **It tracks itself.** Allocations are read from the game's own Client log, so taken points tick off with no input from the player.
2. **It lives inside the game.** The Corner panel shows only inside the Path of Exile window, toggled by the Panel hotkey.
3. **It stays clearly within the game's rules.** It reads the log, the clipboard and its own window only. It never reads game memory, injects code, edits game files or sends input to the game. Its only network requests are the player's own pathofexile.com profile and a launch-time update check against its GitHub releases.

## Operating Context

- **Setup window** (opened on first launch and from the tray / menu bar): paste a PoB code or a pobb.in or pastebin link, pick the Tree spec and Character, and set the Seed (imported from a public profile, or "the first N points"). Also holds Next points (1–15), the Game log path and Key binds.
- **Corner panel**: a small, translucent in-game panel, top-right by default. The player zooms and moves its Mini-tree with the mouse. It takes the mouse but never the keyboard.
- **Build window**: a normal window with the whole Tree spec large, every point in order, Refunds, hover details and the choice of Tree spec.
- Refunds show as *Refund pending* until the next area change or a Re-sync settles them.
- Terms are defined in `GLOSSARY.md` and must be used as written there: Build, Tree spec, Character, Seed, Next points, Refund, Pending refund, Mini-tree, Corner panel, Setup window, Build window, Panel hotkey, Client log.

## Capabilities and Constraints

- Stack: Tauri v2, Rust backend in `src-tauri/`, and plain HTML/JS pages in `ui/` with no bundler and no framework.
- The Corner panel must never take keyboard focus. While the game is in front, the app takes only its own key binds; every other key, Esc included, reaches the game.
- The panel is never shown outside the game window.
- Status: unsigned v0.x builds, published as public GitHub releases. macOS is the platform being tested; Windows and Linux build in CI but are untested.
- Auto-update is on from v0.5.4: it checks once at launch, installs signed updates, and never restarts the app while the Corner panel is showing.
- Undecided: distribution and any business model beyond free test builds.

## Brand Commitments

- Colours follow Path of Exile's own palette (user's direction, 2026-11-23), so the app reads as part of the game's world. This covers colour only: never use GGG's art, textures, logos or other assets.

## Evidence on Hand

- `README.md`, `GLOSSARY.md`, `docs/adr/`, and `docs/research/overlay-field-guide.md` (GGG policy, overlay windows on each OS, the log format).
- No users, testimonials, download counts or endorsements are on hand. Never invent them, and never imply GGG approves the app.

## Product Principles

1. **The game comes first.** Every in-game surface stays small, takes only the mouse, and gives way to the game.
2. **Zero upkeep after setup.** Anything the app can learn from the log or the profile, it learns itself instead of asking.
3. **Visibly rule-abiding.** Never do or suggest anything that would look like automation or memory reading.
4. **Speak the player's language.** Use the glossary terms and PoE/PoB vocabulary, not app jargon.

## Accessibility & Inclusion

- Color-blind safety: allocated points, Next points, a spec's later points, Refunds and Pending refunds must be distinguishable without relying on hue alone (use shape, numbering, outline, pattern or labels as well as color).
- Beyond the colour commitment under Brand Commitments, the visual direction is deliberately left open; it is not pinned to the current look.
