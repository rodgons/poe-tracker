// Listen-only key hook: observes the player's key binds (by default the panel hotkey
// Alt/Option+P, the zoom keys Alt/Option + = - 0, panning Alt/Option + arrows and Esc),
// never consumes or simulates keys.
use rdev::{listen, EventType, Key};
use serde::{Deserialize, Serialize};
use std::sync::{mpsc::Sender, LazyLock, RwLock};

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

/// Swaps the binds the hook listens for; takes effect on the next key press.
pub fn set(binds: Binds) {
    *BINDS.write().unwrap() = binds;
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
                let hit = binds.all().into_iter().find(|(b, _)| {
                    same_key(&b.code, &code) && (b.alt, b.ctrl, b.shift, b.meta) == (alt, ctrl, shift, meta)
                });
                if let Some((_, msg)) = hit {
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
    fn defaults_pass_and_shared_keys_fail() {
        assert!(Binds::default().check().is_ok());
        let mut b = Binds::default();
        b.hide = b.toggle.clone();
        assert!(b.check().is_err());
        b.hide.code.clear();
        assert!(b.check().is_err());
    }
}
