# Partial click-through for the Tree overlay

*Research for issue #45 (map #42), November 2026. Tauri 2.12.7, tao 0.37.3 and wry 0.57.3, the versions in `src-tauri/Cargo.lock`.*

**Question.** Can a Tauri v2 window the size of the game window be transparent, never take keyboard focus, and let clicks through everywhere except a small grip? And can it switch to taking the whole mouse (Align mode) and back? macOS first.

**Labels.** **Confirmed** means read in a primary source: Tauri, tao or wry source in the cargo registry, Apple, Microsoft, GTK, X.org or Wayland docs, or the Tauri issue tracker. **Inference** means my reasoning from those sources, not yet checked on a Mac. Items under *Owner checks* need a Mac.

---

## 1. Short answer

| Question | Answer |
|---|---|
| Can one window be click-through in part? | **Not natively in Tauri.** `set_ignore_cursor_events` covers the whole window. Transparent pixels still catch clicks, and Tauri has said it won't add alpha-based hit testing. |
| Is a click-through grip feasible anyway? | **Yes.** There are two ways: (A) watch the cursor and switch the whole window's ignore flag as the cursor enters or leaves the grip, or (B) make the grip its own small window. |
| Recommended | **(B) A separate grip window.** The overlay window ignores the mouse all the time, except in Align mode. The grip is a second small non-activating panel built like the Gem panel. The OS window server routes the mouse, so there is no polling, no race at the grip's edge and no hit-test code. |
| Align mode | `overlay.set_ignore_cursor_events(false)` on Align and `(true)` on Done, called from Rust. The overlay is already a non-activating panel (the same trick as the Corner panel), so dragging and scrolling never take focus, and Esc still goes to the game. |
| Native full-screen Spaces | **Yes**, with the existing `macos::float_over_fullscreen` (`FULL_SCREEN_AUXILIARY`, `CAN_JOIN_ALL_SPACES`, screen-saver level), as with the Corner panel. |
| GPU cost | **Small when idle, but not zero.** A static transparent webview draws no new frames. The window server still has to blend it over every game frame, and any overlapping layer can move a Metal game from *direct* to *composited* presentation, which adds latency. The Corner panel probably does this already. The owner can check with the Metal HUD. |
| Windows / Linux | The same design works. Tauri's switch uses `WS_EX_TRANSPARENT \| WS_EX_LAYERED` on Windows and an X11/GDK input shape on Linux. Approach B flips it only on Align and Done, which avoids a known Windows 10 black-flash bug that is triggered by frequent flipping. |

---

## 2. What Tauri gives us

### `set_ignore_cursor_events` covers the whole window (confirmed)

- Tauri `Window::set_ignore_cursor_events(ignore)` passes straight through to tao (`tauri-2.12.7/src/window/mod.rs:2357`).
- tao's doc comment: "If `true`, the events are passed through the window such that any other window behind it receives them" (`tao-0.37.3/src/window.rs:1323-1331`). It takes no region and no `forward` option.
- Per-platform implementation:
  - **macOS:** `self.window().setIgnoresMouseEvents(ignore)` (`tao/src/platform_impl/macos/window_delegate.rs:904-906`).
  - **Windows:** sets `WindowFlags::IGNORE_CURSOR_EVENT`, which adds `WS_EX_TRANSPARENT | WS_EX_LAYERED` (`tao/src/platform_impl/windows/window_state.rs:282-283`, `window.rs:536`).
  - **Linux:** `ignore = true` sets the GDK input shape to a **1×1 rectangle at (0, 0)**. `false` clears it (`tao/src/platform_impl/linux/event_loop.rs:455-462`). The overlay's top-left pixel therefore still catches clicks on Linux, which is harmless but real.
- Calling it from Rust needs no capability. Calling it from JS would need `core:window:allow-set-ignore-cursor-events` added to `src-tauri/capabilities/default.json`, which today holds only `core:default`.

### Transparent pixels don't let clicks through (confirmed via the tracker)

- tauri#2090 (closed 2021), maintainer amrbashir: "Detecting whether an area within a window is transparent or not is a near impossible task, even electron gave up on the idea". Only a whole-window switch was added, in Tauri 1.2.
- tauri#13070 (closed as a duplicate): a large transparent Tauri window "still captures user input, making a large area unclickable". FabianLars: "This won't be implemented … the next closest alternative to that is to switch the ignore mode on demand."
- tauri#6164 (open): the request for Electron's `forward` option, which keeps mouse-move events flowing while the window is ignored. It is unimplemented. The workaround posted there is approach A: a backend mouse hook emits the cursor position and the page switches `setIgnoreCursorEvents` on a hit box.
- tauri#10564 (open): making the switch per-webview rather than per-window. It is unimplemented.

### Cursor position (confirmed)

- `AppHandle::cursor_position()` and `Window::cursor_position()` return the global desktop position (`tauri-2.12.7/src/app.rs:928`, `window/mod.rs:1845`). On macOS this is `NSEvent::mouseLocation()`, flipped with the **main display's** height and scaled with the **primary monitor's** scale (`tao/src/platform_impl/macos/util/mod.rs:101-106`). That is wrong-ish on mixed-DPI multi-display setups (inference from the code). On Windows it is `GetCursorPos`.
- It is poll-only, with no move events. The app already runs `rdev::listen` with `kCGEventMaskForAllEvents` on a listen-only tap (`rdev …/src/macos/listen.rs:52-53`). That tap already delivers mouse-move events, so approach A wouldn't need polling.

### Transparency on macOS (confirmed)

- wry turns off `drawsBackground` through a private KVC key (`wry-0.57.3/src/wkwebview/mod.rs:416, 1058-1064`). `macOSPrivateApi` is "No-op in Tauri 2.12.1+ because the APIs are always enabled now" (`tauri-utils-2.10.4/src/config.rs:3381-3385`). The current config, with `transparent: true` and no private-API flag, is fine.

---

## 3. What the OS gives us

### macOS

| Fact | Source |
|---|---|
| `ignoresMouseEvents`: "whether the window is transparent to mouse events." Set per window. | Apple, [NSWindow.ignoresMouseEvents](https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents) |
| `nonactivatingPanel`: "a panel or a subclass of NSPanel that does not activate the owning app." | Apple, [NSWindow.StyleMask.nonactivatingPanel](https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/nonactivatingpanel) |
| `becomesKeyOnlyIfNeeded`: a non-activating panel "becomes key only if the hit view returns true from needsPanelToBecomeKey". `macos::make_nonactivating` already sets it. | Apple, [NSPanel.becomesKeyOnlyIfNeeded](https://developer.apple.com/documentation/appkit/nspanel/becomeskeyonlyifneeded) |
| `fullScreenAuxiliary`: "The window displays on the same space as the full screen window." `canJoinAllSpaces`: "The window can appear in all spaces." | Apple, [fullScreenAuxiliary](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct/fullscreenauxiliary), [canJoinAllSpaces](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct/canjoinallspaces) |
| `NSView.hitTest(_:)` "is used primarily by an NSWindow object to determine which view should receive a mouse-down event". It picks a view *inside* the window. | Apple, [NSView.hitTest(_:)](https://developer.apple.com/documentation/appkit/nsview/hittest(_:)) |
| Global event monitors only observe events; "you cannot modify or otherwise prevent the event from being delivered". | Apple, [NSEvent.addGlobalMonitorForEvents](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:)) |

**Inference: a `hitTest` override in `macos.rs` won't work.** By the time `hitTest` runs, the window server has already given the click to our window. Returning nil drops the click inside our app and does not pass it to the game. On macOS, the only switch that sends a click to the window underneath is `ignoresMouseEvents`, a window-level flag, which is what tao calls. That leaves A (flip it) or B (more windows).

### Windows (confirmed, Microsoft Learn)

- "Hit testing of a layered window is based on the shape and transparency of the window … areas … whose alpha value is zero will let the mouse messages through. However, if the layered window has the **WS_EX_TRANSPARENT** extended window style, the shape of the layered window will be ignored and the mouse events will be passed to other windows underneath" ([Window Features: Layered Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)).
- `WS_EX_NOACTIVATE`: "does not become the foreground window when the user clicks it" ([Extended Window Styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles)). `lib.rs::no_activate` already sets it, and tao sets it for `focusable: false` (`window_state.rs:292-294`).
- Returning `HTTRANSPARENT` from `WM_NCHITTEST` only passes the message to "underlying windows **in the same thread**" ([WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)). It can't send a click to the game.
- Alpha-zero pass-through would in theory give true per-pixel click-through, but WebView2 draws through DirectComposition rather than `UpdateLayeredWindow`, so it does not apply (inference). tauri#2090 is consistent with that.
- **Known bug, tauri#15947 (open, Aug 2026):** on Windows 10, a large transparent always-on-top window that flips `set_ignore_cursor_events` often sometimes shows black transparent areas. The reporter's reading of tao is that each flip of `WS_EX_LAYERED` makes DWM rebuild the window's composition. That counts against approach A on Windows.

### Linux (confirmed)

- GDK `input_shape_combine_region`: "Mouse events which happen while the pointer position corresponds to an unset bit in the mask will be passed on [to] the window below … On the X11 platform, this requires version 1.1 of the shape extension" ([GDK3 docs](https://docs.gtk.org/gdk3/method.Window.input_shape_combine_region.html)). X Shape spec: "The input region is the subset of the bounding region that can 'contain' the pointer" ([X11 Shape extension](https://xorg.freedesktop.org/archive/current/doc/xextproto/shape.html)).
- Wayland `wl_surface.set_input_region`: "Input events happening outside of this region will try the next surface in the server surface stack" ([wayland.xml](https://gitlab.freedesktop.org/wayland/wayland/-/raw/main/protocol/wayland.xml)).
- **Linux is the only platform with true per-region click-through.** Setting the input shape to just the grip rectangle would give a single-window grip with no flipping, but it needs gtk calls of our own, because tao only offers all or a 1×1 shape. Positioning over another app's window on Wayland is restricted anyway (`overlay-field-guide.md` §3).

---

## 4. The candidate approaches

### A. One window: watch the cursor and flip the ignore flag

The overlay ignores the mouse. On every mouse move (rdev already sees them), Rust checks whether the cursor is inside the grip rectangle and calls `set_ignore_cursor_events(!inside)`.

- **For:** one webview, and the grip can be drawn anywhere in the overlay page.
- **Against:**
  - **Race at the edge.** The flip goes hook thread → main thread → `setIgnoresMouseEvents`. A fast move-and-click onto the grip can reach the game first. Inflating the hit box hides most of this, but not all (inference).
  - The flip must be held while a button is down, so a drag that leaves the grip doesn't drop half way. That means tracking button state from rdev.
  - Coordinate conversion between rdev's global points, the game frame and the page is extra code. tao's `cursor_position` has the mixed-DPI quirk above.
  - On Windows, frequent `WS_EX_LAYERED` flips run into tauri#15947.
  - The grip only works while the global hook works: if Input Monitoring is missing, the grip is dead.

### B. Two windows: a full overlay plus a small grip window (recommended)

- **Overlay window:** game-window sized, transparent, a non-activating panel, `set_ignore_cursor_events(true)` all the time, and drawing only.
- **Grip window:** a small non-activating panel like the Gem panel (`focusable: false`, `acceptFirstMouse: true`, transparent, `make_nonactivating` + `float_over_fullscreen`), holding the grip and the Align/Done button.
- **Align:** the grip calls a command, Rust runs `overlay.set_ignore_cursor_events(false)`, and the overlay page handles drag-to-move and scroll-to-scale with pointer and wheel events. It transforms the drawing; the window doesn't move. Done (on the grip) sets it back to `true`.

- **For:**
  - The window server does the hit test: no hook, no race, no coordinate maths for routing.
  - The flag flips twice per Align session, never per mouse move.
  - The grip keeps working without Input Monitoring.
  - Almost everything is reuse:
    - window config copied from `gempanel` in `tauri.conf.json`;
    - `macos::make_nonactivating`, `float_over_fullscreen`, `order_front` and `order_out`;
    - `no_activate` on Windows;
    - `gamewin::frontmost_frame` and `logical_frame` to size the overlay;
    - `placement::place` and `spot_of` for the grip's spot inside the game window;
    - the show and hide logic in `run_toggle_loop`.
- **Against:**
  - One more webview process. tauri#13070's reporter measured about 20–30 MB per webview, on an unstated OS.
  - The two windows must be kept in front-to-back order, grip above overlay (see the gotchas).
  - Align state crosses two pages through a Tauri event.

### C. Ruled out

- **NSView `hitTest` override:** it picks a view inside our own window and can't pass the click to the game (§3).
- **Per-pixel alpha hit testing:** Tauri won't do it (tauri#2090, #13070), and WebView2 doesn't take the layered-window alpha path.
- **`WM_NCHITTEST` / `HTTRANSPARENT`:** only reaches windows in the same thread.

---

## 5. Recommendation: B, separate grip window

1. **New windows** in `tauri.conf.json`: `treeoverlay` and `treegrip`, both copies of `gempanel` (`focus: false`, `focusable: false`, `transparent`, `alwaysOnTop`, `visibleOnAllWorkspaces`, `decorations: false`, `shadow: false`, `skipTaskbar`, `visible: false`). Add both labels to `capabilities/default.json`. Add `treeoverlay` to the window-state denylist next to `corner`.
2. **Setup** (as for `corner` and `gempanel` in `lib.rs`): `make_nonactivating` then `float_over_fullscreen` on both windows on macOS, and `no_activate` on Windows. Then `treeoverlay.set_ignore_cursor_events(true)` once.
3. **Showing:** size the overlay to `logical_frame(frontmost_frame)` with `set_position` + `set_size`, place the grip with `placement`, then `order_front(overlay)` followed by `order_front(grip)`. On macOS, giving the grip window level 1001 (one above `SCREEN_SAVER_LEVEL`) makes the stacking certain rather than dependent on call order (inference; a one-line `setLevel:`). Re-place both windows on every `Msg::Frontmost` frame change, as `position_panel` does now. Hide the Corner panel while the overlay is open, as charted.
4. **Align:** the grip's Align button calls a command. Rust calls `set_ignore_cursor_events(false)` on the overlay and emits `align: true` to the overlay page, which shows a frame or tint so the player knows the game is blocked. Done calls `set_ignore_cursor_events(true)`, emits `align: false` and saves the alignment. Area change and the hotkey also end Align before closing.
5. **Keyboard:** neither window ever becomes key (non-activating, `becomesKeyOnlyIfNeeded`, no text fields), so Esc and every other key reach the game, as with the Corner panel today.
6. **Following the game's pan and zoom** (later ticket): read-only rdev mouse events, as the map allows. That is separate from routing clicks, which B leaves to the OS.

**Gotchas to carry into the build ticket:**

- **Stacking:** the grip must stay above the overlay, or the overlay, while in Align, will take clicks meant for the grip's Done button.
- **Turning the flag on again:** set ignore back to `true` *before* hiding, and on every way out of Align (Done, hotkey, area change, leaving the game). A stuck `false` blocks the whole game.
- **Linux:** the overlay's 1×1 hot pixel at (0, 0) (§2).
- **Frame sizes:** frames from `gamewin` are logical points on macOS and physical pixels elsewhere. Reuse `logical_frame`.
- **Overlay page performance:** keep it static. Draw the tree once (canvas or SVG) and do Align moves and scales with CSS `transform`, so WebKit composites instead of repainting (inference: standard WebKit layer behaviour).

---

## 6. GPU and compositing cost over the game

- **Confirmed:** "When a Metal drawable is direct-to-display, the hardware composites it directly to the display at a very low performance cost". It needs "full-screen mode, displaying an opaque CAMetalLayer layer and RGB content, and … a Mac with Apple silicon" ([Managing your game window for Metal in macOS](https://developer.apple.com/documentation/metal/managing-your-game-window-for-metal-in-macos)).
- **Confirmed:** "When you are composited, there may be some additional buffering and display latency due to system UI elements or your own additional layers overlapping the screen" (Apple tech talk on the Metal Performance HUD, [110339](https://developer.apple.com/videos/play/tech-talks/110339)). The HUD's top line shows "whether the present mode is direct or composited", and `MTL_HUD_ENABLED=1` turns it on for any Metal app ([Monitoring your Metal app's graphics performance](https://developer.apple.com/documentation/xcode/monitoring-your-metal-apps-graphics-performance)).
- **Inferences:**
  - Any visible window over a full-screen game probably moves it to *composited*. The Corner panel and Gem panel probably already do this, so the overlay changes *how much* is blended, not which path the game takes. The cost is a little latency, not frame rate: blending one static full-screen layer is a cheap GPU pass on Apple silicon.
  - A WKWebView whose content doesn't change sends no new frames. The overlay's own GPU work is near zero between log events and Align gestures.
  - The extra webview's memory (tens of MB) is a bigger cost than its GPU use.
  - Unknown: whether PoE on macOS is direct-to-display at all, as its layer may not qualify. If it is already composited, the overlay adds almost nothing.
- **Windows:** a topmost layered window over a borderless-windowed game is how every PoE overlay works (`overlay-field-guide.md`). Exclusive fullscreen can cover it. tauri#15947 is the known rendering bug, and approach B keeps away from its trigger.

---

## 7. Owner checks (Mac)

A throwaway build with the two windows (no tree drawing needed, just a tinted overlay and a grip) answers these:

1. With PoE in **native full-screen** and in **windowed**, the overlay and grip both show over the game, sized to the game window.
2. With the overlay up, clicking, right-clicking and scrolling anywhere off the grip reaches the game, and the game keeps the keyboard (Esc closes a game panel).
3. Clicking the grip works on the first click, without the game losing focus (`acceptFirstMouse`).
4. In Align, dragging and scrolling on the overlay move it, the game gets no clicks, and Esc still reaches the game. Done gives the clicks back at once.
5. With Steam launch option `MTL_HUD_ENABLED=1 %command%`, the HUD's present mode is the same with only the Corner panel up and with the overlay up, and FPS does not change noticeably.
6. Activity Monitor: the app's extra memory with the overlay and grip open.

---

## Sources

**Local source** (cargo registry, versions as in `Cargo.lock`):
- `tauri-2.12.7/src/window/mod.rs`, `src/app.rs`
- `tao-0.37.3/src/window.rs`, `platform_impl/{macos,windows,linux}`
- `wry-0.57.3/src/wkwebview/mod.rs`
- `tauri-utils-2.10.4/src/config.rs`
- rdev at commit `b7e201b`, `src/macos/listen.rs`

**Repo:** `src-tauri/src/{lib.rs,macos.rs,placement.rs,gamewin.rs}`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`.

**Tauri issues:**
- [#2090](https://github.com/tauri-apps/tauri/issues/2090) ignore mouse on transparent areas
- [#6164](https://github.com/tauri-apps/tauri/issues/6164) `forward` option
- [#10564](https://github.com/tauri-apps/tauri/issues/10564) per-webview ignore
- [#13070](https://github.com/tauri-apps/tauri/issues/13070) transparent click-through
- [#15947](https://github.com/tauri-apps/tauri/issues/15947) Windows 10 black flashes when toggling

**Apple:**
- NSWindow [ignoresMouseEvents](https://developer.apple.com/documentation/appkit/nswindow/ignoresmouseevents)
- [nonactivatingPanel](https://developer.apple.com/documentation/appkit/nswindow/stylemask-swift.struct/nonactivatingpanel)
- NSPanel [becomesKeyOnlyIfNeeded](https://developer.apple.com/documentation/appkit/nspanel/becomeskeyonlyifneeded)
- [fullScreenAuxiliary](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct/fullscreenauxiliary)
- [canJoinAllSpaces](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct/canjoinallspaces)
- NSView [hitTest(_:)](https://developer.apple.com/documentation/appkit/nsview/hittest(_:))
- [NSEvent global monitors](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:))
- [Managing your game window for Metal in macOS](https://developer.apple.com/documentation/metal/managing-your-game-window-for-metal-in-macos)
- [Metal HUD monitoring](https://developer.apple.com/documentation/xcode/monitoring-your-metal-apps-graphics-performance)
- [Tech talk 110339](https://developer.apple.com/videos/play/tech-talks/110339)

**Microsoft:**
- [Extended Window Styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles)
- [Window Features: Layered Windows](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-features)
- [WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)

**Linux:**
- [GDK3 input_shape_combine_region](https://docs.gtk.org/gdk3/method.Window.input_shape_combine_region.html)
- [X11 Shape extension](https://xorg.freedesktop.org/archive/current/doc/xextproto/shape.html)
- [Wayland wl_surface.set_input_region](https://gitlab.freedesktop.org/wayland/wayland/-/raw/main/protocol/wayland.xml)
