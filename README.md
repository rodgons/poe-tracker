# POE Tracker

A small companion app for Path of Exile 1 that shows which passive points to take next. Paste a Path of Building build, pick a Tree spec, and press ⌥/Alt+P in game: a Corner panel inside the game window shows the whole spec as a Mini-tree with the next 5 points numbered. Points tick off by themselves as the game writes them to its `Client.txt` log.

Status: unsigned test builds (v0.x) for macOS, Windows and Linux. macOS is the platform being tested; Windows and Linux build in CI but are untested. The words used here are defined in [`GLOSSARY.md`](GLOSSARY.md).

## What it does

- **Set up build…** (menu bar / tray icon): paste a PoB code or a pobb.in / pastebin link, pick the Tree spec, and choose the Character. The points it already has are imported from its public pathofexile.com profile, or set by hand as "the first N points".
- **Corner panel**: toggled with ⌥/Alt+P while Path of Exile is in front, and only ever shown inside the game window. Scroll to zoom the Mini-tree, drag to move it, and drag the title to move the panel. It takes the mouse but never the keyboard.
- **Open build…**: a normal window with the whole Tree spec large, every point in order (taken ✓, next numbered, then the rest), the Refunds in red, hover details, and the choice of Tree spec.
- **Re-sync character…**: imports the Character's points again from pathofexile.com.

## Install a test build

Download from [Releases](https://github.com/rodgons/poe-tracker/releases). The builds are unsigned:

- **macOS**: open the `.dmg`, drag the app to Applications, then right-click it → Open the first time. If macOS says it's damaged, run `xattr -dr com.apple.quarantine "/Applications/POE Tracker.app"`. Allow it under System Settings → Privacy & Security → **Input Monitoring** so the hotkey works, then reopen it.
- **Windows**: run the `-setup.exe` and choose "More info → Run anyway" if SmartScreen warns.
- **Linux**: make the `.AppImage` executable and run it (X11 or XWayland).

## First run

1. Open **Set up build…** from the menu bar / tray icon (it also opens by itself on first launch).
2. Paste a PoB export code or a pobb.in / pastebin link and press **Load build**.
3. Pick the **Tree spec** to follow and type your **Character** name. To import the points it already has, enter your pathofexile.com account (`Name#1234`, Characters tab public) and use **Find characters**; otherwise choose **Set by hand**.
4. Optionally change **Next points** (how many the Corner panel numbers, 1–15, default 5) and check the **Game log** path; pick `Client.txt` yourself if it wasn't found.
   Under **Key binds**, click an action and press new keys to rebind it (the panel hotkey, hide, zoom and moving the view). They take effect at once; **Reset to defaults** puts them back.
5. **Save**, switch to Path of Exile and press the panel hotkey, **⌥P** (macOS) or **Alt+P** (Windows/Linux) unless you changed it, to show or hide the Corner panel. Esc or the tray icon hides it too.

When a Tree spec is complete the panel names the Build's next spec; switch to it in **Open build…**.

## Ground rules

- Read the log, the clipboard and our own window only. Never read game memory, inject code or edit game files.
- Never send input to the game. The key hook only listens.
- Only the player's own pathofexile.com profile is fetched, and only when they ask (setup or Re-sync).

## Develop

Tauri v2: Rust in [`src-tauri/`](src-tauri), plain HTML/JS pages in [`ui/`](ui) (no bundler).

```sh
npm ci
npx tauri dev                                    # run the app
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Pushing a `task/**` branch builds all three platforms in CI and publishes a prerelease named after the version in `src-tauri/tauri.conf.json`, so bump the version for each new test build. Release builds are signed for Tauri's updater (the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` repo secrets) and CI uploads a `latest.json` update manifest with each release. The updater plugin is built in but off: the app never checks for or installs updates. The bundled passive tree comes from [`scripts/extract_tree.py`](scripts/extract_tree.py).

## Docs

- [`docs/adr/`](docs/adr): decisions, such as Tauri over Electron.
- [`docs/research/overlay-field-guide.md`](docs/research/overlay-field-guide.md): the original research on GGG's policy, overlay windows on each OS and the log format.
