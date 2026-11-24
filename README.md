# POE Tracker

A small companion app for Path of Exile 1 that shows which passive points to take next. Paste a Path of Building build, pick a Tree spec, and press ⌥/Alt+P in game: a Corner panel inside the game window shows the whole spec as a Mini-tree with the next 5 points numbered. Points tick off by themselves as the game writes them to its `Client.txt` log.

Status: unsigned test builds (v0.x) for macOS, Windows and Linux. macOS is the platform being tested; Windows and Linux build in CI but are untested. The words used here are defined in [`GLOSSARY.md`](GLOSSARY.md).

## What it does

- **Main window** (menu bar / tray icon): one ordinary window with a sidebar of sections. The tray's **Open tree…**, **Open gems…**, **Set up build…** and **Settings…** open it at the matching section; **Show/Hide panel** and **Show/Hide gem panel** toggle the in-game panels.
  - **Tree**: the whole Tree spec large, every point in order (taken ✓, next numbered, then the rest), the Refunds in red, hover details, and the choice of Tree spec.
  - **Gems**: the Build's Skill set for the current Tree spec, each gem with its picture, beside the gems socketed in your Character's gear (from your public pathofexile.com profile), each marked ✓ had, + missing, ⛓ unlinked (not linked to its group's main skill) or ○ optional, with the gems to − remove listed apart. Pick another Skill set there if the one paired with your Tree spec isn't the one you want.
  - **Build**: paste a PoB code or a pobb.in / pastebin link, pick the Tree spec, and choose the Character. The points it already has are imported from its public pathofexile.com profile, or set by hand as "the first N points". Changes wait for **Save**.
  - **Settings**: Next points, the game log and key binds. Changes save straight away.
  - Problems that stop the app working (Input Monitoring off, ticking paused, `Client.txt` not found) show as a banner across every section, with a button to fix them.
- **Corner panel**: toggled with ⌥/Alt+P while Path of Exile is in front, and only ever shown inside the game window. Scroll to zoom the Mini-tree, drag to move it, and drag the title to move the panel. It takes the mouse but never the keyboard.
- **Gem panel**: toggled with ⌥/Alt+G while Path of Exile is in front, and only ever shown inside the game window, by default just left of the gem level-up icons on the right. It lists only what needs doing to match the Skill set: + missing and ⛓ unlinked gems by socket group, and − gems to remove, or "All gems match". Drag the title to move it; it remembers the spot. Its marks dim when your gems were last imported more than 10 minutes ago. Like the Corner panel it takes the mouse, never the keyboard, and Esc hides both.
- **Re-sync character** (tray, or the button at the bottom of the Main window's sidebar, in every section): imports the Character's points and gems again from pathofexile.com.
- **Gems stay current by themselves**: they're imported at Save and Re-sync, on area changes (at most once a minute) and when you open the Gems section, so a gem you just socketed shows within a minute or so.
- **Refunds in game**: the game logs a refund when you click the node, not when you Apply, so the panel marks it as a *Pending refund* and keeps the point. On your next area change the app checks your pathofexile.com profile once to see whether you applied it (or waits for Re-sync if the profile is private).

## Install a test build

Download from [Releases](https://github.com/rodgons/poe-tracker/releases). The builds are unsigned:

- **macOS**: open the `.dmg`, drag the app to Applications, then right-click it → Open the first time. If macOS says it's damaged, run `xattr -dr com.apple.quarantine "/Applications/POE Tracker.app"`. Allow it under System Settings → Privacy & Security → **Input Monitoring** so the hotkey works, then reopen it.
- **Windows**: run the `-setup.exe` and choose "More info → Run anyway" if SmartScreen warns.
- **Linux**: make the `.AppImage` executable and run it (X11 or XWayland).

## First run

1. Open **Set up build…** from the menu bar / tray icon (the Main window also opens by itself on first launch, at the Build section).
2. Paste a PoB export code or a pobb.in / pastebin link and press **Load build**.
3. Pick the **Tree spec** to follow and type your **Character** name. To import the points it already has, enter your pathofexile.com account (`Name#1234`, Characters tab public) and use **Find characters**; otherwise choose **Set by hand**. Press **Save**.
4. Optionally, in **Settings**, change **Next points** (how many the Corner panel numbers, 1–15, default 5) and check the **Game log** path; pick `Client.txt` yourself if it wasn't found.
   Under **Key binds**, click an action and press new keys to rebind it (the panel hotkey, the Gem panel hotkey, hide, zoom and moving the view). They take effect at once; **Reset to defaults** puts them back.
5. Switch to Path of Exile and press the panel hotkey, **⌥P** (macOS) or **Alt+P** (Windows/Linux) unless you changed it, to show or hide the Corner panel. Esc or the tray icon hides it too.

When a Tree spec is complete the panel names the Build's next spec; switch to it in the Tree section (**Open tree…**).

## Ground rules

- Read the log, the clipboard and our own window only. Never read game memory, inject code or edit game files.
- Never send input to the game. While it's in front, the app takes its own key binds (the two panel hotkeys, and zoom/pan while the Corner panel is shown) so the game doesn't also act on them; every other key, Esc included, reaches the game untouched.
- Only the player's own pathofexile.com profile is fetched: its passives at setup, on Re-sync and once on the area change after a refund click; its gear (for the Gems section) at setup, on Re-sync, on area changes at most once a minute and when the Gems section opens. The Gems section also loads each gem's picture from GGG's image server (web.poecdn.com). The only other request is the update check (below).
- **Updates**: from v0.5.4 the app checks this repo's latest release once at launch. If there's a newer one it installs it and restarts right away, or, if the Corner panel is showing, adds **Restart to update** to the tray menu and the Main window's sidebar instead. Updates are signed, and the app refuses any that doesn't match its built-in key.

## Develop

Tauri v2: Rust in [`src-tauri/`](src-tauri), plain HTML/JS pages in [`ui/`](ui) (no bundler).

```sh
pnpm install
pnpm tauri dev                                   # run the app
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Pushing a `task/**` branch builds all three platforms in CI and publishes a prerelease named after the version in `src-tauri/tauri.conf.json`, so bump the version for each new test build. Merging to `main` publishes that version as a full release, which is what installed apps update to: the updater reads `releases/latest/download/latest.json`, and GitHub's "latest" skips prereleases. Builds are signed for Tauri's updater (the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repo secrets), and CI uploads the `latest.json` update manifest with each release. A version that was already published as a prerelease must have that prerelease and its tag deleted before merging, or the release stays a prerelease. The bundled passive tree comes from [`scripts/extract_tree.py`](scripts/extract_tree.py), and the gem picture table (`ui/gem-icons.js`) from [`scripts/extract_gem_icons.py`](scripts/extract_gem_icons.py); regenerate both each patch.

## Docs

- [`docs/adr/`](docs/adr): decisions, such as Tauri over Electron.
- [`docs/research/overlay-field-guide.md`](docs/research/overlay-field-guide.md): the original research on GGG's policy, overlay windows on each OS and the log format.

## License

[GPL-3.0](LICENSE). You may use, change and share this app, but anything you distribute that is built from it must also be released under GPL-3.0 with its source.

Path of Exile is a trademark of Grinding Gear Games. This is a fan-made tool, not affiliated with or endorsed by Grinding Gear Games.
