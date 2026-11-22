// Listen-only key hook: observes the panel hotkey (Alt/Option+P), the zoom keys
// (Alt/Option + = - 0), panning (Alt/Option + arrows) and Esc, never consumes or simulates keys.
use rdev::{listen, EventType, Key};
use std::sync::mpsc::Sender;

use crate::Msg;

pub fn spawn(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        // Tauri owns the main thread; without this rdev crashes on macOS on the first key.
        #[cfg(target_os = "macos")]
        rdev::set_is_main_thread(false);

        let mut alt = false;
        // Auto-repeat sends many presses while a key is held; count one per press.
        let mut held: Option<Key> = None;
        let result = listen(move |event| match event.event_type {
            EventType::KeyPress(Key::Alt | Key::AltGr) => alt = true,
            EventType::KeyRelease(Key::Alt | Key::AltGr) => alt = false,
            EventType::KeyPress(
                key @ (Key::KeyP
                | Key::Escape
                | Key::Equal
                | Key::KpPlus
                | Key::Minus
                | Key::KpMinus
                | Key::Num0
                | Key::LeftArrow
                | Key::RightArrow
                | Key::UpArrow
                | Key::DownArrow),
            ) => {
                if held == Some(key) {
                    return;
                }
                held = Some(key);
                // the callback runs inside the OS hook: hand off and return at once
                match key {
                    Key::KeyP if alt => {
                        let _ = tx.send(Msg::Hotkey);
                    }
                    Key::Escape => {
                        let _ = tx.send(Msg::Esc);
                    }
                    Key::Equal | Key::KpPlus if alt => {
                        let _ = tx.send(Msg::Zoom(1));
                    }
                    Key::Minus | Key::KpMinus if alt => {
                        let _ = tx.send(Msg::Zoom(-1));
                    }
                    Key::Num0 if alt => {
                        let _ = tx.send(Msg::Zoom(0));
                    }
                    Key::LeftArrow if alt => {
                        let _ = tx.send(Msg::Pan(-1, 0));
                    }
                    Key::RightArrow if alt => {
                        let _ = tx.send(Msg::Pan(1, 0));
                    }
                    Key::UpArrow if alt => {
                        let _ = tx.send(Msg::Pan(0, -1));
                    }
                    Key::DownArrow if alt => {
                        let _ = tx.send(Msg::Pan(0, 1));
                    }
                    _ => {}
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
