// Skeleton: a corner panel toggled by the panel hotkey (Alt/Option+P) while
// Path of Exile is frontmost, via a listen-only key hook.
// Nothing here ever sends input to the game.
use serde::Serialize;
use std::{sync::mpsc, time::Duration};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow,
};

mod foreground;
mod keys;
#[cfg(target_os = "macos")]
mod macos;

pub enum Msg {
    Hotkey,
    Esc,
    Frontmost(String),
}

#[derive(Clone, Serialize)]
struct Status {
    key: &'static str,
    presses: u32,
    visible: bool,
    permission: &'static str,
    frontmost: String,
    in_poe: bool,
}

fn permission() -> &'static str {
    #[cfg(target_os = "macos")]
    return macos::listen_permission();
    #[cfg(not(target_os = "macos"))]
    return "not needed";
}

fn place_in_corner(win: &WebviewWindow) {
    let Ok(Some(monitor)) = win.current_monitor() else { return };
    let Ok(size) = win.outer_size() else { return };
    let area = monitor.work_area();
    let margin = 16;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + margin;
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn show(win: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::order_front(win);
    #[cfg(not(target_os = "macos"))]
    let _ = win.show();
}

fn hide(win: &WebviewWindow) {
    #[cfg(target_os = "macos")]
    macos::order_out(win);
    #[cfg(not(target_os = "macos"))]
    let _ = win.hide();
}

fn run_toggle_loop(app: AppHandle, rx: mpsc::Receiver<Msg>) {
    let mut presses = 0;
    // shown at launch so the player can see the app is running
    let mut visible = true;
    let mut frontmost = foreground::frontmost_app();
    let mut key = "none yet";
    for msg in rx {
        let in_poe = foreground::is_poe(&frontmost);
        match msg {
            Msg::Hotkey if in_poe => {
                presses += 1;
                key = "⌥/Alt+P";
                visible = !visible;
            }
            Msg::Esc if in_poe && visible => {
                key = "Esc";
                visible = false;
            }
            Msg::Frontmost(name) => {
                if name == frontmost {
                    continue;
                }
                // leaving the game hides the panel; coming back doesn't reopen it
                if in_poe && !foreground::is_poe(&name) {
                    visible = false;
                }
                frontmost = name;
            }
            _ => continue,
        }
        let status = Status {
            key,
            presses,
            visible,
            permission: permission(),
            in_poe: foreground::is_poe(&frontmost),
            frontmost: frontmost.clone(),
        };
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            let Some(win) = handle.get_webview_window("corner") else { return };
            if status.visible { show(&win) } else { hide(&win) }
            let _ = win.emit("status", status);
        });
    }
}

fn poll_frontmost(tx: mpsc::Sender<Msg>) {
    loop {
        if tx.send(Msg::Frontmost(foreground::frontmost_app())).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[tauri::command]
fn initial_status() -> Status {
    let frontmost = foreground::frontmost_app();
    Status {
        key: "none yet",
        presses: 0,
        visible: true,
        permission: permission(),
        in_poe: foreground::is_poe(&frontmost),
        frontmost,
    }
}

pub fn run() {
    // On a Wayland session, run under XWayland so the window can sit on top
    // and the X11 key hook sees keys going to the game (an X11 client).
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![initial_status])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                macos::request_listen_permission();
            }

            let win = app.get_webview_window("corner").expect("corner window");
            place_in_corner(&win);
            // display-only panel: clicks fall through to the game, so it never takes focus
            let _ = win.set_ignore_cursor_events(true);
            #[cfg(target_os = "macos")]
            macos::float_over_fullscreen(&win);

            let quit = MenuItem::with_id(app, "quit", "Quit POE Tracker", true, None::<&str>)?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("POE Tracker")
                .menu(&Menu::with_items(app, &[&quit])?)
                .on_menu_event(|app, e| {
                    if e.id() == "quit" {
                        app.exit(0)
                    }
                })
                .build(app)?;

            let (tx, rx) = mpsc::channel();
            keys::spawn(tx.clone());
            std::thread::spawn(move || poll_frontmost(tx));
            let handle = app.handle().clone();
            std::thread::spawn(move || run_toggle_loop(handle, rx));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running POE Tracker");
}
