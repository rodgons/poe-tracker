# Tauri platform notes: passive key listener, corner window, 3-OS builds

*Research for issue #4 · November 2026 · Tauri 2.12.x (latest `tauri` 2.12.7, 18 Nov 2026)*

Answers three questions for the Corner panel (see GLOSSARY.md): how to see the player press **P** / **Esc** without taking the key from the game, how to show a small always-on-top window that never steals focus, and how to build unsigned installers for macOS, Windows and Linux on GitHub Actions.

Labels: **[source]** = read in the primary source linked. **[inferred]** = my reasoning from sources, not stated by them. **[untested]** = needs a check on real hardware before relying on it.

---

## 1. Short answer

| Question | Answer |
|---|---|
| Can we use `tauri-plugin-global-shortcut` for P / Esc? | **No.** It registers OS hot keys (Win32 `RegisterHotKey`, Carbon `RegisterEventHotKey`, X11 `XGrabKey`). Those route the key to our app instead of the game, so the passive tree would stop opening. It also has no Wayland backend. |
| What should we use instead? | A **listen-only** hook. Each OS has one that observes keys and passes them on: macOS `CGEventTap` with the `listenOnly` option, Windows `WH_KEYBOARD_LL` with `CallNextHookEx`, X11 `XRecord`. The `rdev` crate's `listen()` wraps all three. Use the git `main` of rdev pinned to a commit, not the crates.io 0.5.3 release from 2023, and on macOS call `set_is_main_thread(false)` before `listen`, or it crashes under Tauri. |
| Permissions? | macOS: **Input Monitoring** (Accessibility on some setups; see 2.4). Windows: none. Linux X11: none. |
| Corner window? | `alwaysOnTop`, `focus: false`, `focusable: false`, `decorations: false`, `transparent`, `skipTaskbar`, `visibleOnAllWorkspaces`, plus click-through (`set_ignore_cursor_events(true)`). On macOS that is not enough over a full-screen Space: convert the window to a non-activating `NSPanel` (crate `tauri-nspanel`) with `fullScreenAuxiliary` and a higher window level, and use the Accessory activation policy. `macOSPrivateApi` no longer matters: it does nothing in 2.12.1+. |
| Over windowed-fullscreen PoE? | Windows: fine over borderless windowed. macOS: fine over a normal window. A native full-screen Space needs the panel settings above. Linux X11: the spec ranks a **focused fullscreen** window *above* "always on top" windows, so a WM that follows it hides the panel. Use PoE "Windowed" (borderless, maximised) on Linux, or test the WM. |
| Builds? | `tauri-apps/tauri-action@v1` matrix: `macos-latest` ×2 (aarch64, x86_64) or one `universal-apple-darwin`, `windows-latest` (NSIS `.exe` + `.msi`), `ubuntu-22.04` (AppImage + deb). No secrets beyond `GITHUB_TOKEN`. Set `bundle.macOS.signingIdentity: "-"` (ad-hoc). |
| Opening unsigned builds | macOS 15+: try to open it, then **System Settings › Privacy & Security › Open Anyway** (Control-click Open no longer works). If it says "damaged": `xattr -dr com.apple.quarantine "/Applications/POE Tracker.app"`. Windows: SmartScreen › **More info › Run anyway**. Smart App Control on Windows 11 can block unsigned apps outright. Linux: `chmod +x *.AppImage`. |

---

## 2. Passive key listener

### 2.1 Why the global-shortcut plugin is wrong here

`tauri-plugin-global-shortcut` 2.4.1 depends on the `global-hotkey` crate 0.8 [source: [plugin Cargo.toml](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/global-shortcut/Cargo.toml)]. `global-hotkey` registers keys like this:

| OS | API in `global-hotkey` source | Effect |
|---|---|---|
| Windows | `RegisterHotKey(self.hwnd, id, mods, vk)` [source: [windows/mod.rs](https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/windows/mod.rs)] | Windows posts `WM_HOTKEY` to *our* window when the key matches [source: [RegisterHotKey docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey)]. The key press goes to the registering app, so the game would not get it. |
| macOS | Carbon `RegisterEventHotKey` with `kEventHotKeyExclusive`. A CGEventTap is used only for media keys, and it returns null (swallows) on a match [source: [macos/mod.rs](https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/macos/mod.rs)] | Carbon hot keys go to the registering app. [inferred; Carbon docs are archived] |
| Linux | x11rb `grab_key(..., GrabMode::ASYNC, ...)` on the root window = `XGrabKey` [source: [x11/mod.rs](https://github.com/tauri-apps/global-hotkey/blob/dev/src/platform_impl/x11/mod.rs)] | A passive grab sends the key to the grabbing client, not the focused window. No Wayland: the crate errors with "Other window systems on Linux are not supported". |

Nothing in the plugin or crate offers a pass-through option. A bare `P` hot key would also fire while the player types `p` in chat. **Verdict: do not use it for P / Esc.** A user-chosen show/hide hot key with a modifier would be fine.

### 2.2 Options compared

| Option | How it works | macOS | Windows | Linux | Notes |
|---|---|---|---|---|---|
| **rdev `listen()`** (recommended) | Event hook, push | `CGEvent::tap_create(..., CGEventTapOptions::ListenOnly, ...)` on the current thread's run loop [source: [macos/listen.rs](https://github.com/Narsil/rdev/blob/main/src/macos/listen.rs)] | `SetWindowsHookExA(WH_KEYBOARD_LL, ...)`, and the hook proc always ends in `CallNextHookEx` [source: [windows/common.rs](https://github.com/Narsil/rdev/blob/main/src/windows/common.rs), [listen.rs](https://github.com/Narsil/rdev/blob/main/src/windows/listen.rs)] | `x11` feature uses `XRecord` for all clients [source: [linux/x11/listen.rs](https://github.com/Narsil/rdev/blob/main/src/linux/x11/listen.rs)]. README: does not work on Wayland. | Never consumes keys. Has a `simulate` API too, which we must never call. |
| device_query | **Polls** key state on a timer (`DeviceEventsHandler::new(Duration)`) | README: needs Accessibility, or only modifier keys show up | none | X11 only | Repo **archived 31 Mar 2026**, moved to Codeberg; 4.0.1 (Jul 2025) [source: [README](https://github.com/ostrosco/device_query), [crates.io](https://crates.io/crates/device_query)]. Polling can miss a short tap. Fallback only. |
| Hand-rolled per OS | Same three APIs as rdev | `CGEventTapCreate` + `listenOnly`, or `NSEvent.addGlobalMonitorForEvents` (needs Accessibility for keys [source: [Apple](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:))]) | `WH_KEYBOARD_LL` or Raw Input (`RIDEV_INPUTSINK`) | XRecord or XInput2 raw events | About 100 to 200 lines per OS. Worth it only if rdev's unreleased `main` becomes a problem. |

### 2.3 rdev pitfalls (from its issue tracker)

- **Released crate is stale.** crates.io has 0.5.3, published 26 Jun 2023 [source: [crates.io API](https://crates.io/api/v1/crates/rdev)]. `main` is version 0.6.0 (edition 2024), with commits up to May 2026 [source: repo Cargo.toml and commit log]. The macOS fix below is only on `main`. **Pin a git rev**; the maintainer asks for this too: "pin `rdev` to a specific revision ... not on `main`" ([#165](https://github.com/Narsil/rdev/issues/165)).
- **macOS crash under Tauri when a key is pressed.** Tauri owns the main thread, so `listen` runs on a spawned thread. rdev's key-name lookup then calls `TISGetInputSourceProperty` off the main thread and the process dies ([#146](https://github.com/Narsil/rdev/issues/146), [#165](https://github.com/Narsil/rdev/issues/165), also [#100](https://github.com/Narsil/rdev/issues/100) with bevy and [#129](https://github.com/Narsil/rdev/issues/129) with Electron). The fix, PR #147, adds `rdev::set_is_main_thread(false)`: "If you're calling `listen` from a different thread from the main thread ... setting `is_main_thread` to `false` ... lets it know that it has to take measures to avoid calling a certain Mac function directly in that thread" [source: #165 comments]. #165 reports it works when run through `cargo run` but crashes under `tauri dev`.
- **Silent failure without permission.** README: without Accessibility, "macOS will silently ignore rdev's `listen` callback", with no error. The process must also not fork before `listen`. Check permission yourself (2.4) and show the user a status.
- **Keep the callback tiny.** On Windows the callback runs inside the LL hook. Microsoft: if the hook takes longer than `LowLevelHooksTimeout` (at most 1000 ms on Windows 10 1709 and later), "on Windows 7 and later, the hook is silently removed ... There is no way for the application to know" [source: [LowLevelKeyboardProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc)]. Send the event down a channel and return.
- **Match on `event_type` (`Key::KeyP`, `Key::Escape`), not `event.name`.** `EventType` is the physical QWERTY position, and the README warns that `name` may be `None` or junk. PoE binds by position by default, so physical P is right on AZERTY too. [inferred; the user may rebind the key, so make it a setting]

Sketch:

```rust
// src-tauri/src/keys.rs — listen-only, never simulate.
use rdev::{listen, EventType, Key};
use tauri::{AppHandle, Emitter};

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        #[cfg(target_os = "macos")]
        rdev::set_is_main_thread(false); // rdev main only; avoids the TIS crash
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for key in rx { let _ = app.emit("tree-key", key); } // toggle logic lives elsewhere
        });
        let _ = listen(move |e| match e.event_type {
            EventType::KeyPress(Key::KeyP) => { let _ = tx.send("p"); }
            EventType::KeyPress(Key::Escape) => { let _ = tx.send("esc"); }
            _ => {}
        });
    });
}
```

```toml
# Cargo.toml
rdev = { git = "https://github.com/Narsil/rdev", rev = "<pin a commit>" , default-features = false, features = ["x11"] }
```

(Check the feature names on the pinned commit. `src/linux/mod.rs` fails to compile unless either `x11` or `wayland` is enabled.)

**Game-logic pitfalls** [inferred]: (a) only react while PoE is the foreground app. Use macOS `NSWorkspace.frontmostApplication` (no permission), Windows `GetForegroundWindow`, X11 `_NET_ACTIVE_WINDOW`. (b) `p` typed in chat must not toggle. Chat opens on Enter, so track Enter/Esc as a "chat open" flag, or show the panel only after a `Successfully allocated passive` line. (c) Key auto-repeat sends many KeyPress events, so ignore repeats until KeyRelease.

### 2.4 Permissions per OS

| OS | What the user grants | Detail |
|---|---|---|
| macOS | **Input Monitoring** (System Settings › Privacy & Security › Input Monitoring [source: [Apple support](https://support.apple.com/guide/mac-help/control-access-to-input-monitoring-on-mac-mchl4cedafb6/mac)]) | A `listenOnly` tap is "a passive listener" [source: [Apple](https://developer.apple.com/documentation/coregraphics/cgeventtapoptions/listenonly)]. Apple DTS says listen-only taps need the Input Monitoring privilege, not Accessibility, through `CGPreflightListenEventAccess` / `CGRequestListenEventAccess` (macOS 10.15+) [source: [Apple docs](https://developer.apple.com/documentation/coregraphics/cgrequestlisteneventaccess()), [dev forum 122492](https://developer.apple.com/forums/thread/122492)]. `CGEventTapCreate` returns NULL when the tap may not watch the requested events [source: [Apple](https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:))]. But rdev's README asks for Accessibility, and some forum reports say the tap only worked after Accessibility was granted ([735204](https://developer.apple.com/forums/thread/735204)). **Plan:** call `CGRequestListenEventAccess()` on first run. If no events arrive, point the user to Accessibility as a fallback. The app usually needs a restart after the grant [untested]. |
| macOS caveat | Re-grant after updates | [inferred, untested] TCC grants are tied to the code signature. Ad-hoc builds change signature every build, so users may have to remove and re-add the app after each update. Say so in release notes. |
| Windows | Nothing | LL hooks need no permission. [untested] If PoE runs as Administrator, a non-elevated hook may not see its keys (UIPI), the same rule as the overlay rule in the field guide. |
| Linux X11 / XWayland | Nothing | XRecord needs the RECORD extension, which is on by default in Xorg and Xwayland [untested for every distro]. Under XWayland, XRecord sees only input going to X11 clients. PoE under Proton/Wine is an X11 client, so P reaches us while the game has focus [inferred]. On pure Wayland with a native Wayland game, no listener works without the `input` group / evdev, which we avoid. |

---

## 3. Corner window

### 3.1 Config

All fields below exist in Tauri 2.12 `WindowConfig` [source: [config reference](https://v2.tauri.app/reference/config/), [config.rs](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-utils/src/config.rs)]:

```jsonc
// tauri.conf.json (excerpt)
"app": {
  "windows": [{
    "label": "corner",
    "url": "index.html",
    "width": 360, "height": 300,
    "visible": false,            // shown on P
    "focus": false,              // "Whether the window will be initially focused"
    "focusable": false,          // Win: WS_EX_NOACTIVATE; mac: canBecomeKey/Main = false
    "alwaysOnTop": true,
    "visibleOnAllWorkspaces": true, // not supported on Windows
    "decorations": false,
    "transparent": true,
    "shadow": false,
    "resizable": false,
    "skipTaskbar": true          // Windows and Linux
  }]
}
```

What the flags do in `tao` (Tauri's window layer):

- `focusable: false` on Windows adds `WS_EX_NOACTIVATE`. Topmost uses `HWND_TOPMOST` with `SWP_NOACTIVATE` [source: [tao windows/window_state.rs](https://github.com/tauri-apps/tao/blob/dev/src/platform_impl/windows/window_state.rs)].
- On macOS, `focusable` drives `canBecomeKeyWindow` / `canBecomeMainWindow` [source: [tao macos/window.rs](https://github.com/tauri-apps/tao/blob/dev/src/platform_impl/macos/window.rs)]. `show()` calls `makeKeyAndOrderFront`, which does not make a non-key-able window key. `set_focus()` calls `activateIgnoringOtherApps(true)`, so **never call `set_focus` on this window** [source: [tao macos/window_delegate.rs](https://github.com/tauri-apps/tao/blob/dev/src/platform_impl/macos/window_delegate.rs)].
- On macOS, `alwaysOnTop` sets `NSFloatingWindowLevel` only, and `visibleOnAllWorkspaces` adds `CanJoinAllSpaces` only, **without `FullScreenAuxiliary`** [source: window_delegate.rs `set_always_on_top`, `set_visible_on_all_workspaces`]. Apple: `fullScreenAuxiliary` = "The window displays on the same space as the full screen window" [source: [Apple](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct/fullscreenauxiliary)]. So stock Tauri will **not** show over a native full-screen Space [inferred].
- **Known bug:** [tauri#14102](https://github.com/tauri-apps/tauri/issues/14102) (open): on macOS, clicking a `focusable: false` window still steals focus. Avoid it two ways: make the panel click-through with `window.set_ignore_cursor_events(true)`, which suits a display-only panel, and use a non-activating panel (3.2).
- **Transparency:** `macOSPrivateApi` is documented as "No-op in Tauri 2.12.1+ because the APIs are always enabled now" [source: config.rs]. On older Tauri it had to be `true` for `transparent` to work on macOS (and it blocks App Store submission, which does not apply to us). Linux needs a running compositor or the window draws black (field guide §3).
- **Placement:** compute the corner from `window.current_monitor()?.work_area()` and call `set_position`, or use `tauri-plugin-positioner` (`Position::TopRight`, etc.). Pick the monitor the game is on, not the primary [inferred].

### 3.2 macOS: non-activating panel over full-screen Spaces

Use [`tauri-nspanel`](https://github.com/ahkohd/tauri-nspanel) (Tauri v2; `tauri-nspanel = "2.1"` or git branch `v2.1`; maintained; 426 stars). It converts a Tauri window into an `NSPanel`. Apple: `nonactivatingPanel` = a panel "that does not activate the owning app" [source: [Apple](https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/nonactivatingpanel)]. Set:

- style mask `nonactivatingPanel`, `can_become_key_window = false`
- collection behaviour `canJoinAllSpaces | fullScreenAuxiliary | stationary`
- level above floating, e.g. status or screen-saver level. Electron's overlay recipe uses `'screen-saver'` (field guide §3) [untested in Tauri]
- `app.set_activation_policy(tauri::ActivationPolicy::Accessory)`: no Dock icon. Commonly needed so an app's window can join another app's full-screen Space [inferred, untested]

If we skip the crate, the same can be done with `window.ns_window()` and `objc2` calls in about 20 lines.

### 3.3 Behaviour over the game

| OS / mode | Expected | Basis |
|---|---|---|
| Windows, PoE "Windowed Fullscreen" (borderless) | Topmost window shows above it | Field guide §3; topmost + no-activate styles in tao |
| Windows, exclusive Fullscreen | May be covered | Field guide §3 |
| macOS, PoE windowed / windowed-fullscreen (not a native Space) | Floating level is enough | tao level = floating |
| macOS, native full-screen Space (green button) | Needs §3.2 | Apple `fullScreenAuxiliary`; tao lacks it |
| Linux X11 | WM-dependent. EWMH order from bottom: desktop, below, normal, dock/**above**, **focused fullscreen** [source: [EWMH §9.10](https://specifications.freedesktop.org/wm/latest/ar01s09.html)]. A focused `_NET_WM_STATE_FULLSCREEN` game outranks our `_NET_WM_STATE_ABOVE` | Ask Linux users to run PoE in borderless "Windowed" maximised, not fullscreen state [untested]. |
| Linux Wayland session | Run our app under XWayland (`GDK_BACKEND=x11`) so it can position itself and stay on top. Native Wayland won't let a client place itself over other apps (field guide §3) | [inferred, untested] |

---

## 4. Unsigned builds on GitHub Actions

`tauri-apps/tauri-action` latest is **v1.0.0** (29 Jun 2026). The README example matrix, adapted [source: [tauri-action README](https://github.com/tauri-apps/tauri-action)]:

```yaml
name: build
on:
  push: { tags: ['v*'] }
  workflow_dispatch:
jobs:
  build:
    permissions: { contents: write }
    strategy:
      fail-fast: false
      matrix:
        include:
          - { platform: macos-latest,  args: '--target universal-apple-darwin' } # or two rows: aarch64-apple-darwin / x86_64-apple-darwin
          - { platform: ubuntu-22.04,  args: '' }   # oldest supported Ubuntu = widest glibc reach for the AppImage
          - { platform: windows-latest, args: '' }
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: lts/* }
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.platform == 'macos-latest' && 'aarch64-apple-darwin,x86_64-apple-darwin' || '' }}
      - if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils libxtst-dev
      - run: npm ci
      - uses: tauri-apps/tauri-action@v1
        env: { GITHUB_TOKEN: '${{ secrets.GITHUB_TOKEN }}' }
        with:
          tagName: v__VERSION__
          releaseName: 'POE Tracker v__VERSION__'
          releaseDraft: true
          args: ${{ matrix.args }}
          # for CI-only builds without a release: drop tagName/releaseName and set
          # uploadWorkflowArtifacts: true
```

Notes:

- The Linux apt line is the README's, plus `libxtst-dev`, because rdev's X11 backend links libXtst/XRecord [inferred; check the build log].
- `universal-apple-darwin` needs both Apple Rust targets installed. Tauri's own v2 App Store guide builds with `--target universal-apple-darwin` [source: [v2 App Store guide](https://v2.tauri.app/distribute/app-store/)]. The tauri-action README instead shows two separate macOS rows, which gives two smaller .dmg files. Either works. One universal .dmg is simpler for users.
- Bundles come from `bundle.targets` (default `"all"`: `deb, rpm, appimage, nsis, msi, app, dmg`) [source: config reference]. For our three asks: `dmg`, `nsis` (setup `.exe`), `msi`, `appimage`.
- No signing secrets are needed. Set `"bundle": { "macOS": { "signingIdentity": "-" } }` for **ad-hoc signing**: Tauri says this "is useful on ARM (Apple Silicon) devices, where code-signing is required for all apps from the Internet", but "does not prevent MacOS from requiring users to whitelist the installation" [source: [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)].
- Do not set `createUpdaterArtifacts` unless we add the updater plugin. `uploadUpdaterSignatures` is harmless.

### 4.1 What a user does to open an unsigned build

**macOS 15 Sequoia and later**: Apple: "users will no longer be able to Control-click to override Gatekeeper when opening software that isn't signed correctly or notarized. They'll need to visit System Settings > Privacy & Security" [source: [Apple developer news, 6 Aug 2024](https://developer.apple.com/news/?id=saqachfa)].
1. Open the .dmg and drag the app to Applications. Double-click it; macOS blocks it.
2. System Settings › Privacy & Security › scroll down › **Open Anyway** › confirm **Open** [source: [Apple support 102445](https://support.apple.com/en-us/102445)].
3. If macOS says the app "is damaged and can't be opened" (common for downloaded unsigned arm64 apps), run once in Terminal:
   `xattr -dr com.apple.quarantine "/Applications/POE Tracker.app"` (or `xattr -cr`, which strips all extended attributes) [inferred; widely used, removes the quarantine flag Gatekeeper checks].
4. Grant Input Monitoring when asked (2.4), then quit and reopen.
On macOS 14 and earlier, Control-click › Open › Open still works.

**Windows**
1. Run the `_x64-setup.exe` (NSIS) or `.msi`. SmartScreen shows "Windows protected your PC": click **More info › Run anyway**. Tauri: signing "is not required to execute your application on Windows" if users accept the SmartScreen warning [source: [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/)].
2. On Windows 11 with **Smart App Control** on, unsigned apps are blocked even when not downloaded from a browser, and users "can't allow a single app, only turn Smart App Control off entirely" [source: same page]. Document this. The real fix is signing later.
3. The WebView2 runtime ships with Windows 10/11, and the NSIS installer bootstraps it if missing (Tauri default) [untested].

**Linux**
`chmod +x POE*.AppImage && ./POE*.AppImage`. Some distros need `libfuse2` for AppImages [inferred]. The `.deb` is an alternative on Debian/Ubuntu.

---

## 5. Open items to test on hardware

1. macOS: does a listen-only rdev tap work with only Input Monitoring, or does it need Accessibility? Does the grant survive an ad-hoc-signed update?
2. macOS: does the `tauri-nspanel` panel show over PoE in a native full-screen Space without activating our app? Which level is needed?
3. Linux: is the panel visible above PoE (Proton) in borderless windowed mode on GNOME (Mutter/XWayland) and KDE?
4. Windows: does the LL hook see keys when PoE runs elevated?

## 6. Sources

- Tauri config reference: https://v2.tauri.app/reference/config/ and `crates/tauri-utils/src/config.rs` (tauri `dev`, Nov 2026)
- tao window code: `src/platform_impl/macos/window.rs`, `window_delegate.rs`, `windows/window_state.rs` (https://github.com/tauri-apps/tao)
- tauri#14102 `focusable: false` broken on macOS: https://github.com/tauri-apps/tauri/issues/14102
- global-hotkey source: https://github.com/tauri-apps/global-hotkey (platform_impl)
- plugins-workspace global-shortcut Cargo.toml: https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/global-shortcut
- rdev README, source and issues #100, #129, #146, #165: https://github.com/Narsil/rdev
- device_query README: https://github.com/ostrosco/device_query
- Microsoft: RegisterHotKey, LowLevelKeyboardProc (learn.microsoft.com)
- Apple: CGEventTapOptions.listenOnly, CGEvent.tapCreate, CGRequestListenEventAccess, NSEvent.addGlobalMonitorForEvents, NSWindow.CollectionBehavior.fullScreenAuxiliary, NSWindow.StyleMask.nonactivatingPanel; Input Monitoring support page; support 102445; developer news "Updates to runtime protection in macOS Sequoia"
- Apple developer forums threads 122492, 735204 (Input Monitoring vs Accessibility for event taps)
- EWMH spec §9.10 Stacking order: https://specifications.freedesktop.org/wm/latest/ar01s09.html
- tauri-nspanel: https://github.com/ahkohd/tauri-nspanel
- tauri-action README (v1.0.0): https://github.com/tauri-apps/tauri-action
- Tauri distribution: https://v2.tauri.app/distribute/sign/macos/, https://v2.tauri.app/distribute/sign/windows/, https://v2.tauri.app/distribute/app-store/
