// macOS window and permission glue, done with raw AppKit messages.
use objc2::{msg_send, runtime::AnyObject};
use tauri::WebviewWindow;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

// NSWindowCollectionBehavior bits
const CAN_JOIN_ALL_SPACES: usize = 1 << 0;
const STATIONARY: usize = 1 << 4;
const FULL_SCREEN_AUXILIARY: usize = 1 << 8;
// NSScreenSaverWindowLevel
const SCREEN_SAVER_LEVEL: isize = 1000;

pub fn request_listen_permission() {
    // Shows the Input Monitoring prompt the first time; harmless afterwards.
    unsafe {
        if !CGPreflightListenEventAccess() {
            CGRequestListenEventAccess();
        }
    }
}

pub fn listen_permission() -> &'static str {
    if unsafe { CGPreflightListenEventAccess() } {
        "Input Monitoring granted"
    } else {
        "Input Monitoring NOT granted"
    }
}

fn ns_window(win: &WebviewWindow) -> Option<*mut AnyObject> {
    win.ns_window().ok().map(|w| w as *mut AnyObject)
}

/// Lets the panel appear over a game in its own full-screen Space.
pub fn float_over_fullscreen(win: &WebviewWindow) {
    let Some(w) = ns_window(win) else { return };
    unsafe {
        let _: () = msg_send![w, setLevel: SCREEN_SAVER_LEVEL];
        let _: () = msg_send![
            w,
            setCollectionBehavior: CAN_JOIN_ALL_SPACES | STATIONARY | FULL_SCREEN_AUXILIARY
        ];
    }
}

/// Shows the panel without activating the app, so the game keeps focus.
pub fn order_front(win: &WebviewWindow) {
    let Some(w) = ns_window(win) else { return };
    unsafe {
        let _: () = msg_send![w, orderFrontRegardless];
    }
}

pub fn order_out(win: &WebviewWindow) {
    let Some(w) = ns_window(win) else { return };
    unsafe {
        let nil: *mut AnyObject = std::ptr::null_mut();
        let _: () = msg_send![w, orderOut: nil];
    }
}

/// Brings the app forward so the Setup window takes keyboard focus. The app runs
/// as an accessory (no Dock icon), which otherwise never becomes active.
pub fn activate() {
    unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}
