// The player's key binds (by default the panel hotkey Alt/Option+P, the zoom keys
// Alt/Option + = - 0, panning Alt/Option + arrows and Esc). A listen-only hook observes
// them; while the game is in front the toggle (and, with the panel shown, zoom and pan)
// are also registered as OS hot keys, so the game never sees them. The game ignores
// modifiers, so without that ⌥P would also open its passive tree. Nothing is ever
// simulated or sent to the game, and Esc always reaches it.
use rdev::{listen, EventType, Key};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicU16, Ordering},
    mpsc::Sender,
    LazyLock, RwLock,
};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::Msg;

/// One key with its modifiers. `code` is the physical key as the web's
/// `KeyboardEvent.code` names it ("KeyP", "Digit0", "ArrowLeft"), so the Setup
/// window can record it and the hook can match it.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Bind {
    pub code: String,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub meta: bool,
}

impl Bind {
    fn alt(code: &str) -> Bind {
        Bind { code: code.into(), alt: true, ctrl: false, shift: false, meta: false }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Binds {
    /// the Panel hotkey
    pub toggle: Bind,
    pub hide: Bind,
    pub zoom_in: Bind,
    pub zoom_out: Bind,
    /// zoom back out to the whole spec
    pub zoom_whole: Bind,
    pub pan_left: Bind,
    pub pan_right: Bind,
    pub pan_up: Bind,
    pub pan_down: Bind,
}

impl Default for Binds {
    fn default() -> Self {
        Binds {
            toggle: Bind::alt("KeyP"),
            hide: Bind { alt: false, ..Bind::alt("Escape") },
            zoom_in: Bind::alt("Equal"),
            zoom_out: Bind::alt("Minus"),
            zoom_whole: Bind::alt("Digit0"),
            pan_left: Bind::alt("ArrowLeft"),
            pan_right: Bind::alt("ArrowRight"),
            pan_up: Bind::alt("ArrowUp"),
            pan_down: Bind::alt("ArrowDown"),
        }
    }
}

impl Binds {
    fn all(&self) -> [(&Bind, Msg); 9] {
        [
            (&self.toggle, Msg::Hotkey),
            (&self.hide, Msg::Esc),
            (&self.zoom_in, Msg::Zoom(1)),
            (&self.zoom_out, Msg::Zoom(-1)),
            (&self.zoom_whole, Msg::Zoom(0)),
            (&self.pan_left, Msg::Pan(-1, 0)),
            (&self.pan_right, Msg::Pan(1, 0)),
            (&self.pan_up, Msg::Pan(0, -1)),
            (&self.pan_down, Msg::Pan(0, 1)),
        ]
    }

    /// Refuses empty binds and two actions on the same keys.
    pub fn check(&self) -> Result<(), String> {
        let all = self.all();
        for (i, (a, _)) in all.iter().enumerate() {
            if a.code.is_empty() {
                return Err("Every action needs a key.".into());
            }
            if all[..i].iter().any(|(b, _)| b == a) {
                return Err("Two actions can't share the same keys.".into());
            }
        }
        Ok(())
    }
}

static BINDS: LazyLock<RwLock<Binds>> = LazyLock::new(|| RwLock::new(Binds::default()));

/// Binds the OS is delivering as hot keys, bit i for `Binds::all()[i]`. The hook
/// leaves those alone so they don't fire twice.
static GRABBED: AtomicU16 = AtomicU16::new(0);

/// Bits of `Binds::all()`: the toggle, then zoom and pan; hide (Esc) is never taken from the game.
pub const TOGGLE: u16 = 1;
pub const VIEW: u16 = 0b1_1111_1100;

/// Swaps the binds the hook listens for; takes effect on the next key press.
pub fn set(binds: Binds) {
    *BINDS.write().unwrap() = binds;
}

fn shortcut(b: &Bind) -> Option<Shortcut> {
    let code: Code = b.code.parse().ok()?;
    let mut mods = Modifiers::empty();
    mods.set(Modifiers::ALT, b.alt);
    mods.set(Modifiers::CONTROL, b.ctrl);
    mods.set(Modifiers::SHIFT, b.shift);
    mods.set(Modifiers::META, b.meta);
    Some(Shortcut::new(Some(mods), code))
}

/// The message for a hot key the OS delivered, when it's one of the grabbed binds.
pub fn grabbed_message(pressed: &Shortcut) -> Option<Msg> {
    let grabbed = GRABBED.load(Ordering::Relaxed);
    let binds = BINDS.read().unwrap();
    let hit = binds
        .all()
        .into_iter()
        .enumerate()
        .find(|(i, (b, _))| grabbed & 1 << i != 0 && shortcut(b).as_ref() == Some(pressed))
        .map(|(_, (_, msg))| msg);
    hit
}

/// Registers the wanted binds (bits of `Binds::all()`) as OS hot keys and drops the
/// others. A bind the OS refuses (another app holds it, Wayland) stays with the hook.
/// Blocks on the main thread, so never call it from there.
pub fn grab(app: &AppHandle, want: u16) {
    let had = GRABBED.load(Ordering::Relaxed);
    if want == had {
        return;
    }
    let os = app.global_shortcut();
    let binds = BINDS.read().unwrap().clone();
    let mut now = had;
    for (i, (b, _)) in binds.all().iter().enumerate() {
        let bit = 1 << i;
        let Some(s) = shortcut(b) else { continue };
        if had & bit != 0 && want & bit == 0 {
            let _ = os.unregister(s);
            now &= !bit;
        } else if had & bit == 0 && want & bit != 0 && os.register(s).is_ok() {
            now |= bit;
        }
    }
    GRABBED.store(now, Ordering::Relaxed);
}

/// Drops every hot key and swaps in new binds; the next `grab` takes them.
pub fn rebind(app: &AppHandle, binds: Binds) {
    let _ = app.global_shortcut().unregister_all();
    GRABBED.store(0, Ordering::Relaxed);
    set(binds);
}

/// The web `KeyboardEvent.code` of a key; None for modifiers and unknown keys.
fn web_code(key: Key) -> Option<String> {
    let code = match key {
        Key::Alt | Key::AltGr | Key::ControlLeft | Key::ControlRight | Key::ShiftLeft | Key::ShiftRight => return None,
        Key::MetaLeft | Key::MetaRight | Key::Function | Key::Unknown(_) => return None,
        Key::Num0 => "Digit0",
        Key::Num1 => "Digit1",
        Key::Num2 => "Digit2",
        Key::Num3 => "Digit3",
        Key::Num4 => "Digit4",
        Key::Num5 => "Digit5",
        Key::Num6 => "Digit6",
        Key::Num7 => "Digit7",
        Key::Num8 => "Digit8",
        Key::Num9 => "Digit9",
        Key::Kp0 => "Numpad0",
        Key::Kp1 => "Numpad1",
        Key::Kp2 => "Numpad2",
        Key::Kp3 => "Numpad3",
        Key::Kp4 => "Numpad4",
        Key::Kp5 => "Numpad5",
        Key::Kp6 => "Numpad6",
        Key::Kp7 => "Numpad7",
        Key::Kp8 => "Numpad8",
        Key::Kp9 => "Numpad9",
        Key::KpPlus => "NumpadAdd",
        Key::KpMinus => "NumpadSubtract",
        Key::KpMultiply => "NumpadMultiply",
        Key::KpDivide => "NumpadDivide",
        Key::KpReturn => "NumpadEnter",
        Key::KpDelete => "NumpadDecimal",
        Key::LeftArrow => "ArrowLeft",
        Key::RightArrow => "ArrowRight",
        Key::UpArrow => "ArrowUp",
        Key::DownArrow => "ArrowDown",
        Key::LeftBracket => "BracketLeft",
        Key::RightBracket => "BracketRight",
        Key::BackQuote => "Backquote",
        Key::SemiColon => "Semicolon",
        Key::BackSlash => "Backslash",
        Key::Dot => "Period",
        Key::Return => "Enter",
        // the rest share their name: KeyA, F1, Escape, Tab, Space, Minus, Equal, Comma…
        other => return Some(format!("{other:?}")),
    };
    Some(code.into())
}

/// Whether a pressed key counts as the bound one; the keypad + and − also zoom.
fn same_key(bound: &str, pressed: &str) -> bool {
    bound == pressed
        || matches!((bound, pressed), ("Equal", "NumpadAdd") | ("Minus", "NumpadSubtract"))
}

pub fn spawn(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        // Tauri owns the main thread; without this rdev crashes on macOS on the first key.
        #[cfg(target_os = "macos")]
        rdev::set_is_main_thread(false);

        let (mut alt, mut ctrl, mut shift, mut meta) = (false, false, false, false);
        // Auto-repeat sends many presses while a key is held; count one per press.
        let mut held: Option<Key> = None;
        let result = listen(move |event| match event.event_type {
            EventType::KeyPress(Key::Alt | Key::AltGr) => alt = true,
            EventType::KeyRelease(Key::Alt | Key::AltGr) => alt = false,
            EventType::KeyPress(Key::ControlLeft | Key::ControlRight) => ctrl = true,
            EventType::KeyRelease(Key::ControlLeft | Key::ControlRight) => ctrl = false,
            EventType::KeyPress(Key::ShiftLeft | Key::ShiftRight) => shift = true,
            EventType::KeyRelease(Key::ShiftLeft | Key::ShiftRight) => shift = false,
            EventType::KeyPress(Key::MetaLeft | Key::MetaRight) => meta = true,
            EventType::KeyRelease(Key::MetaLeft | Key::MetaRight) => meta = false,
            EventType::KeyPress(key) => {
                if held == Some(key) {
                    return;
                }
                held = Some(key);
                let Some(code) = web_code(key) else { return };
                // the callback runs inside the OS hook: hand off and return at once
                let binds = BINDS.read().unwrap();
                let grabbed = GRABBED.load(Ordering::Relaxed);
                let hit = binds.all().into_iter().enumerate().find(|(i, (b, _))| {
                    grabbed & 1 << i == 0
                        && same_key(&b.code, &code)
                        && (b.alt, b.ctrl, b.shift, b.meta) == (alt, ctrl, shift, meta)
                });
                if let Some((_, (_, msg))) = hit {
                    let _ = tx.send(msg);
                }
            }
            EventType::KeyRelease(key) if held == Some(key) => held = None,
            _ => {}
        });
        if let Err(e) = result {
            eprintln!("key listener stopped: {e:?}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_codes_match_the_setup_window() {
        assert_eq!(web_code(Key::KeyP).as_deref(), Some("KeyP"));
        assert_eq!(web_code(Key::Num0).as_deref(), Some("Digit0"));
        assert_eq!(web_code(Key::LeftArrow).as_deref(), Some("ArrowLeft"));
        assert_eq!(web_code(Key::Equal).as_deref(), Some("Equal"));
        assert_eq!(web_code(Key::Escape).as_deref(), Some("Escape"));
        assert_eq!(web_code(Key::F5).as_deref(), Some("F5"));
        assert_eq!(web_code(Key::Alt), None);
    }

    #[test]
    fn binds_become_os_hot_keys() {
        let b = Binds::default();
        assert_eq!(shortcut(&b.toggle), Some(Shortcut::new(Some(Modifiers::ALT), Code::KeyP)));
        assert_eq!(shortcut(&b.pan_left), Some(Shortcut::new(Some(Modifiers::ALT), Code::ArrowLeft)));
        assert_eq!(shortcut(&b.zoom_whole), Some(Shortcut::new(Some(Modifiers::ALT), Code::Digit0)));
    }

    #[test]
    fn defaults_pass_and_shared_keys_fail() {
        assert!(Binds::default().check().is_ok());
        let mut b = Binds::default();
        b.hide = b.toggle.clone();
        assert!(b.check().is_err());
        b.hide.code.clear();
        assert!(b.check().is_err());
    }
}
