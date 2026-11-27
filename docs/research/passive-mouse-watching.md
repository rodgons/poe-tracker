# Watching the mouse over the game without blocking it

*Research for issue #44 (map #42, v0.7 Tree overlay) · November 2026*

**Question.** Can poe-tracker see left-button drags and scroll-wheel steps over the game window, without blocking or changing them, so a click-through Tree overlay can follow the game's pan and zoom? Nothing may be sent to the game, and the app must not read memory or the screen.

## Short answer

**Yes, and the app already does it.** The listen-only hook that `keys.rs` installs through `rdev` already receives every mouse move, button and wheel event on macOS, Windows and X11. Today `keys.rs` drops them in its `_ => {}` arm. Following drags and scrolls means handling events the app already gets. It needs no new crate, no new permission and no new OS hook.

| | macOS | Windows | Linux |
|---|---|---|---|
| Mechanism already running | `CGEventTap`, **listen-only**, all event types | `WH_MOUSE_LL` + `WH_KEYBOARD_LL` | XRecord (X11; Wayland sessions are forced onto XWayland) |
| Can it block or change input? | No, by API contract | Only if the callback returns nonzero. `rdev` always calls `CallNextHookEx`. | No, XRecord only records |
| Permission | Input Monitoring, which the app already requests | None | None |
| Main caveat | Smooth trackpad scrolling rounds to whole steps | Can't see input to an **elevated** game; hook removed if the callback is slow | Native Wayland games can't be observed |

**Recommendation:** extend the existing `keys.rs` callback rather than adding a second listener. Gate it hard: Tree overlay open, PoE frontmost, the press started inside the game frame and not over our own panel or grip. In the callback, do only O(1) work. Coalesce drag moves in Rust and push at most one position update per display frame to the overlay webview. Keep "fully zoomed out + Anchor re-align" as the fallback, because the hook shows what the *mouse* did, not what the *game* did with it (see Open risks).

Labels used below: **Confirmed** = read in a primary source or in the code. **Inference** = reasoned from confirmed facts but not tested.

---

## 1. What the app already has (confirmed, from the code)

- `src-tauri/Cargo.toml` pins `rdev` from git (`Narsil/rdev` rev `b7e201bf`, feature `x11`) as a "listen-only global key hook". `keys.rs::spawn` runs `rdev::listen` on its own thread. On macOS it calls `rdev::set_is_main_thread(false)` first.
- **macOS, rdev `src/macos/listen.rs`:** `CGEvent::tap_create(HIDEventTap, HeadInsertEventTap, CGEventTapOptions::ListenOnly, kCGEventMaskForAllEvents, …)`. The callback returns the event pointer unchanged. The tap mask is **all events**, so mouse events already reach the callback.
- **macOS, rdev `src/macos/common.rs::convert`:** `LeftMouseDown/Up` → `ButtonPress/Release(Left)`. `MouseMoved`, `LeftMouseDragged` and `RightMouseDragged` → `MouseMove{x,y}` (global location). `ScrollWheel` → `Wheel{delta_x, delta_y}` from the **integer** fields `ScrollWheelEventDeltaAxis1/2`. `OtherMouse*` (middle button) is not mapped.
- **Windows, rdev `src/windows/listen.rs` and `common.rs`:** `listen` installs **both** `SetWindowsHookExA(WH_KEYBOARD_LL, …)` and `SetWindowsHookExA(WH_MOUSE_LL, …)`, then blocks in `GetMessageA`. The callback always returns `CallNextHookEx(...)`, so it never swallows input. Wheel delta = `(HIWORD(mouseData) as i16) / WHEEL_DELTA`, using **integer division**.
- **Linux, rdev `src/linux/x11/listen.rs` and `common.rs`:** an XRecord context over all clients, covering device events up to `MotionNotify`. Buttons 4 and 5 map to `Wheel`, and `MotionNotify` maps to `MouseMove`. `lib.rs::run` sets `GDK_BACKEND=x11` on Wayland sessions so the app and its hook run under XWayland.
- **Permission:** `macos.rs` calls `CGPreflightListenEventAccess` / `CGRequestListenEventAccess` at startup (`request_listen_permission`). The Main window reports "Input Monitoring granted / NOT granted" and offers `open_input_monitoring`, which opens `Privacy_ListenEvent`. The app does **not** request Accessibility, and it needs none for this.
- The hook callback hands off to an `mpsc::Sender<Msg>`. `run_toggle_loop` forwards to webviews with `app.emit_to("corner", "zoom" | "pan", …)`. `foreground.rs` polls the frontmost app every 500 ms. `gamewin.rs` gives the game frame (logical points on macOS, physical pixels elsewhere).

## 2. macOS

**Confirmed:**
- A listen-only tap is "a passive listener" ([`CGEventTapOptions.listenOnly`](https://developer.apple.com/documentation/coregraphics/cgeventtapoptions/listenonly)). For a passive listener, the callback "may return the event that is passed in, or NULL. In either case, the event stream is not affected" ([`CGEventTapCallBack`](https://developer.apple.com/documentation/coregraphics/cgeventtapcallback)). So the tap cannot block or change what the game gets.
- Permission: "a listen-only event [tap] requires authorization for input monitoring, a modifying event [tap] requires authorization for accessibility features." On first run, `CGEventTapCreate` fails while macOS points the user to the privacy pane ([WWDC19 session 701, *Advances in macOS Security*](https://developer.apple.com/videos/play/wwdc2019/701)). That is the Input Monitoring grant the app already asks for. `CGPreflightListenEventAccess` exists from macOS 10.15 ([docs](https://developer.apple.com/documentation/coregraphics/cgpreflightlisteneventaccess())).
- "If the event tap is not permitted to monitor one or more of the events specified in the eventsOfInterest parameter, then the appropriate bits in the mask are cleared" ([`CGEvent.tapCreate`](https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:))).
- The system can disable a tap that is too slow: there is a `kCGEventTapDisabledByTimeout` event type ([docs](https://developer.apple.com/documentation/coregraphics/cgeventtype/tapdisabledbytimeout)). rdev's callback does not handle it, so it does not re-enable the tap.
- Alternative API: `NSEvent.addGlobalMonitorForEvents` delivers events "asynchronously… you can only observe the event; you cannot modify or otherwise prevent" them. Only key events need Accessibility. It is "not called for events that are sent to your own application" ([docs](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:))).

**Inference:**
- Mouse events seem not to be gated by Input Monitoring: the WWDC talk mentions only keyboard events, and the mask-clearing rule above suggests only key bits are dropped. NSEvent's global monitor would then be a permission-free option for the mouse alone. This doesn't matter for poe-tracker, because the app already holds Input Monitoring for its key binds. Not tested.
- The rdev README says `listen` needs "access to the Accessibility API". That is older guidance. The WWDC statement and the app's own working setup (Input Monitoring only) contradict it for listen-only taps. `docs/research/overlay-field-guide.md` §3 also says global hotkeys need Accessibility. For this app that line is outdated: hot keys go through `tauri-plugin-global-shortcut` and the hook uses Input Monitoring.
- Trackpad and Magic Mouse scrolling sends many small continuous deltas. rdev reads only the integer line fields, so small gestures may round to 0 or to uneven steps. How PoE turns those gestures into zoom steps is unknown. Following zoom is most reliable with a notched wheel. For trackpads, a small custom tap or a patch to rdev to read the point-delta and continuous-scroll fields would be needed.
- If the callback ever blocks, macOS can turn the tap off, and key binds would silently stop working too. Any new work in the callback must stay O(1) with no locks held for long.

## 3. Windows

**Confirmed** ([LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc), [SetWindowsHookEx](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexa), [MSLLHOOKSTRUCT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msllhookstruct)):
- `WH_MOUSE_LL` is global-only. It "is not injected into another process": the system switches back to the installing thread, which "must have a message loop". This is not DLL injection into the game.
- A hook "may return a nonzero value to prevent the system from passing the message". Calling `CallNextHookEx` passes it on. rdev always does the latter.
- Timeouts: past `LowLevelHooksTimeout` (capped at 1000 ms since Windows 10 1709), "on Windows 7 and later, the hook is silently removed… There is no way for the application to know". Microsoft advises running the hook "on a dedicated thread that passes the work off to a worker thread and then immediately returns". keys.rs already follows that pattern.
- `MSLLHOOKSTRUCT.pt` is in "per-monitor-aware screen coordinates". One wheel notch is `WHEEL_DELTA` = 120. Finer-resolution wheels send smaller values, which should be accumulated ([WM_MOUSEWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel)).
- Raw input: `RIDEV_INPUTSINK` "enables the caller to receive the input even when the caller is not in the foreground" (needs an `hwndTarget`). Raw input carries relative device motion, not the cursor position ([RAWINPUTDEVICE](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice)).
- **Administrator caveat.** Under UIPI, a lower-integrity process cannot hook into or send messages to a higher-integrity one. Only a `UIAccess` process "has read input for all integrity levels using low-level hooks, raw input…" ([Windows Integrity Mechanism Design](https://learn.microsoft.com/en-us/previous-versions/dotnet/articles/bb625963(v=msdn.10))).

**Inference:**
- If PoE runs as Administrator and poe-tracker does not, the hook will most likely not see mouse (or key) input while the game has focus. The fix is the same one the field guide already gives for the overlay window: run both at the same level. This affects the existing key binds equally. It is not new.
- rdev's `delta / WHEEL_DELTA` truncates high-resolution wheel deltas below 120 to 0. To follow zoom on such mice, use a fork or a small custom hook that accumulates raw deltas.
- Raw input brings no benefit here. It needs a hidden window. Its relative motion includes OS cursor acceleration only implicitly, while the game reacts to the cursor position, which the LL hook already gives. Keep the LL hook rdev already installs.

## 4. Linux (brief)

- **X11 (confirmed from the code):** XRecord sees pointer motion, buttons and wheel (buttons 4/5) for all X clients, without root. PoE under Proton/Wine is an X client, and so is the app (forced to XWayland).
- **Wayland (confirmed, rdev README):** "The listen function uses X11 APIs, and so will not work in Wayland". rdev's alternative Wayland/evdev backends need root or the `input` group. **Inference:** under XWayland, XRecord sees only input that goes to X clients, which covers a Proton game. A native-Wayland game, or input to native-Wayland windows, is invisible. That matches the map's "builds in CI, untested" stance.

## 5. Other crates

- **device_query** (README): has both on-demand polling (`DeviceState::get_mouse`) and callbacks. Linux support is X11 only. On newer macOS it needs Accessibility to see keys. Its README does not mention scroll. Its macOS key monitoring needs a *different* permission (Accessibility) than the app uses now, and polling at frame rate risks missing short clicks. No reason to switch.
- **rdev**: already in the tree, already listen-only on all three OSes, and already gets the mouse. Keep it. Two known gaps: integer wheel deltas (macOS trackpad, Windows hi-res wheels) and no tap-timeout re-enable on macOS. Both can be fixed with a small patch to the git-pinned fork if testing shows they matter.

## 6. Latency and CPU of forwarding drags to a webview

**Confirmed:**
- Tauri v2: "The event system is not designed for low latency or high throughput situations". Payloads "are always JSON strings", and emitting "directly evaluates JavaScript code". For "ordered, high-throughput data delivery, consider using Channels" ([Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/)).
- The hook callback runs inside the OS input path on both macOS and Windows (§2, §3). Slow work there delays everyone's input and can get the hook disabled.

**Inference (not measured):**
- Mice report at 125–1000 Hz, and some gaming mice at up to 8000 Hz. Emitting one event per move would mean hundreds to thousands of JS evaluations per second for no visible gain, because the overlay can only repaint once per display frame.
- Recommended pipeline:
  1. **Callback (hook thread):** if the drag is being followed, store the latest `(x, y)` in atomics. Send only button press/release and wheel steps through the existing `mpsc`. No allocation, no JSON.
  2. **Worker (existing toggle-loop thread or a new one):** while a followed drag is active, every ~16 ms (or at the monitor's refresh rate), read the latest position. If it changed, send one small message (`dx, dy` since the drag started) to the overlay. Wheel steps go out as they come.
  3. **Overlay JS:** apply it as a CSS `transform` inside `requestAnimationFrame`. No layout and no SVG rebuild per frame.
- Cost: at ≤ 60–120 tiny messages per second, either `emit_to` or a Tauri `Channel` is fine. The app already emits zoom/pan per key press. A `Channel` is the documented choice if this ever needs more. CPU should be negligible next to the webview's own compositing of a full-window transparent layer. That layer is the real cost and should be profiled in the v0.7 spike.
- Expected lag: hook delivery and the hand-off take well under 1 ms, then the coalescing tick waits ≤ 1 frame, then the webview paints and composites in about 1 frame. So the overlay trails the game by roughly 1–2 frames during a drag, and lands exactly on release. Confirm on the owner's Mac with a timestamped log during the build spike.

## 7. Policy

**Confirmed** ([pathofexile.com/developer/docs](https://www.pathofexile.com/developer/docs), third-party policy):
- Separate executable apps are "permitted" though "not encouraged".
- The macro rules cover "automation of key-strokes (or other inputs) that affect the game". Each must be manual, one action per invocation, never triggered by timers, file changes or screen reading.
- The overlay rules cover "information sourced from the screen, the clipboard, or the game's logs": the user must be aware, and screen or clipboard reads happen once per manual key press.
- "Executable apps that interact with the game or game files" break ToU 7b, 7c and 7i.

**Inference (our reading, not a GGG ruling):**
- Observing the OS input stream sends nothing to the game, so the macro rules don't apply. It is not screen, clipboard or log data, so the overlay rules' once-per-press limit doesn't apply either. It does not touch the game process, memory or files (the LL hook and CGEventTap run in our own process, not inside the game).
- It sits in the same category as the key-bind hook the app already ships. `PRODUCT.md`'s "never reads game memory, injects code, edits game files or sends input to the game" still holds.
- Keep the user-awareness spirit: say in Settings or the overlay help that the app watches mouse drags and scrolls over the game only while the Tree overlay is open, to move the overlay with the tree.
- Lines not to cross: never react to the observed mouse by sending input, and never use mouse data together with screen reading.

## Open risks for the map (not answered by this ticket)

1. **Input ≠ game state.** The hook sees what the mouse did, not what PoE did with it. Unknowns that only an owner check can settle:
   - how many screen pixels the tree pans per pixel of drag (likely 1:1, unverified);
   - whether pan stops at the tree's edges;
   - how much one wheel step zooms, around which point (the cursor or the centre), and the min/max zoom;
   - the drag threshold that tells a node click from a pan;
   - whether the game's own tree view resets when it opens.

   Every clamp the overlay doesn't model makes it drift. The planned fallback (fully zoomed out + Anchor re-align) stays necessary.
2. Drags that start outside the tree view, or over PoE UI inside the tree screen (search box, buttons), or over our Corner panel or grip, must be ignored. Gate on "press began inside the game frame and not over our own windows". Our windows are known rects.
3. Trackpad scrolling (macOS) and high-res wheels (Windows) need delta accumulation that rdev doesn't do (§2, §3).
4. Elevated game on Windows (§3), and native-Wayland games (§4), can't be followed. This matches the existing key-bind limits.

## Sources

- Code: `src-tauri/src/keys.rs`, `macos.rs`, `foreground.rs`, `lib.rs`, `gamewin.rs`, `Cargo.toml`. rdev at rev `b7e201bf`: `src/macos/{listen,common}.rs`, `src/windows/{listen,common}.rs`, `src/linux/x11/{listen,common}.rs`, `Cargo.toml`.
- Apple: [CGEventTapOptions.listenOnly](https://developer.apple.com/documentation/coregraphics/cgeventtapoptions/listenonly), [CGEventTapCallBack](https://developer.apple.com/documentation/coregraphics/cgeventtapcallback), [CGEvent.tapCreate](https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:)), [CGPreflightListenEventAccess](https://developer.apple.com/documentation/coregraphics/cgpreflightlisteneventaccess()), [tapDisabledByTimeout](https://developer.apple.com/documentation/coregraphics/cgeventtype/tapdisabledbytimeout), [NSEvent.addGlobalMonitorForEvents](https://developer.apple.com/documentation/appkit/nsevent/addglobalmonitorforevents(matching:handler:)), [WWDC19 701](https://developer.apple.com/videos/play/wwdc2019/701).
- Microsoft: [LowLevelMouseProc](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc), [SetWindowsHookExA](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowshookexa), [MSLLHOOKSTRUCT](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msllhookstruct), [WM_MOUSEWHEEL](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel), [RAWINPUTDEVICE](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-rawinputdevice), [Windows Integrity Mechanism Design (UIPI)](https://learn.microsoft.com/en-us/previous-versions/dotnet/articles/bb625963(v=msdn.10)).
- Crates: [rdev README](https://github.com/Narsil/rdev), [device_query README](https://github.com/ostrosco/device_query).
- Tauri: [Calling the Frontend from Rust](https://v2.tauri.app/develop/calling-frontend/).
- GGG: [Path of Exile developer docs, third-party policy](https://www.pathofexile.com/developer/docs). Repo: `docs/research/overlay-field-guide.md` §2–3, `PRODUCT.md`.
