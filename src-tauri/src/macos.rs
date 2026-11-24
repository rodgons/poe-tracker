// macOS window and permission glue, done with raw AppKit messages.
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject},
};
use tauri::WebviewWindow;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightListenEventAccess() -> bool;
    fn CGRequestListenEventAccess() -> bool;
}

extern "C" {
    fn object_setClass(obj: *mut AnyObject, cls: *const AnyClass) -> *const AnyClass;
}

// NSWindowStyleMaskNonactivatingPanel
const NONACTIVATING_PANEL: usize = 1 << 7;

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

/// Turns the panel into a non-activating NSPanel, so scrolling or dragging on it
/// never takes focus from the game (the same trick as tauri-nspanel). Call before
/// float_over_fullscreen, which sets the level.
pub fn make_nonactivating(win: &WebviewWindow) {
    let Some(w) = ns_window(win) else { return };
    unsafe {
        object_setClass(w, objc2::class!(NSPanel));
        let mask: usize = msg_send![w, styleMask];
        let _: () = msg_send![w, setStyleMask: mask | NONACTIVATING_PANEL];
        let _: () = msg_send![w, setBecomesKeyOnlyIfNeeded: true];
        // panels hide when their app deactivates by default; ours is rarely active
        let _: () = msg_send![w, setHidesOnDeactivate: false];
    }
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

/// Brings the app forward so the Main window takes keyboard focus. The app runs
/// as an accessory (no Dock icon), which otherwise never becomes active.
pub fn activate() {
    unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, activateIgnoringOtherApps: true];
    }
}

/// Shows the Dock icon while the Main window is open (a regular app), and hides
/// it again after (an accessory app).
pub fn set_dock_icon(shown: bool) {
    unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyRegular = 0, Accessory = 1
        let policy: isize = if shown { 0 } else { 1 };
        let _: bool = msg_send![app, setActivationPolicy: policy];
    }
}
