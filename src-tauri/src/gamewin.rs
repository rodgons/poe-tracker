// Where the frontmost app's window is on screen, so the Corner panel can sit
// inside the game window. Read-only OS queries; only called while PoE is frontmost.

/// A window frame. Logical points on macOS, physical pixels elsewhere (see LOGICAL).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub const LOGICAL: bool = cfg!(target_os = "macos");

#[cfg(target_os = "macos")]
pub fn frontmost_frame() -> Option<Frame> {
    use objc2::{class, msg_send, runtime::AnyObject};
    use std::ffi::c_void;

    type CFTypeRef = *const c_void;
    #[repr(C)]
    #[derive(Default)]
    struct CGRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
        fn CGRectMakeWithDictionaryRepresentation(dict: CFTypeRef, rect: *mut CGRect) -> bool;
        static kCGWindowOwnerPID: CFTypeRef;
        static kCGWindowLayer: CFTypeRef;
        static kCGWindowBounds: CFTypeRef;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(array: CFTypeRef) -> isize;
        fn CFArrayGetValueAtIndex(array: CFTypeRef, i: isize) -> CFTypeRef;
        fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
        fn CFNumberGetValue(number: CFTypeRef, kind: isize, out: *mut c_void) -> bool;
        fn CFRelease(cf: CFTypeRef);
    }
    const ON_SCREEN_ONLY: u32 = 1 << 0;
    const EXCLUDE_DESKTOP: u32 = 1 << 4;
    const SINT32: isize = 3;

    unsafe {
        let ws: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: *mut AnyObject = msg_send![ws, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let pid: i32 = msg_send![app, processIdentifier];
        let number = |dict: CFTypeRef, key: CFTypeRef| {
            let v = CFDictionaryGetValue(dict, key);
            let mut out = 0i32;
            (!v.is_null() && CFNumberGetValue(v, SINT32, &mut out as *mut i32 as *mut c_void)).then_some(out)
        };
        let list = CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP, 0);
        if list.is_null() {
            return None;
        }
        let mut found = None;
        // front to back: the game's first normal-layer window is the one in front
        for i in 0..CFArrayGetCount(list) {
            let w = CFArrayGetValueAtIndex(list, i);
            if number(w, kCGWindowOwnerPID) != Some(pid) || number(w, kCGWindowLayer) != Some(0) {
                continue;
            }
            let mut r = CGRect::default();
            let bounds = CFDictionaryGetValue(w, kCGWindowBounds);
            if !bounds.is_null() && CGRectMakeWithDictionaryRepresentation(bounds, &mut r) && r.w > 100.0 {
                found = Some(Frame { x: r.x, y: r.y, w: r.w, h: r.h });
                break;
            }
        }
        CFRelease(list);
        found
    }
}

#[cfg(target_os = "windows")]
pub fn frontmost_frame() -> Option<Frame> {
    use windows_sys::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::ClientToScreen,
        UI::WindowsAndMessaging::{GetClientRect, GetForegroundWindow},
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut r: RECT = std::mem::zeroed();
        let mut origin = POINT { x: 0, y: 0 };
        if GetClientRect(hwnd, &mut r) == 0 || ClientToScreen(hwnd, &mut origin) == 0 {
            return None;
        }
        let (w, h) = ((r.right - r.left) as f64, (r.bottom - r.top) as f64);
        (w > 100.0).then_some(Frame { x: origin.x as f64, y: origin.y as f64, w, h })
    }
}

#[cfg(target_os = "linux")]
pub fn frontmost_frame() -> Option<Frame> {
    use x11::xlib;
    unsafe {
        let d = xlib::XOpenDisplay(std::ptr::null());
        if d.is_null() {
            return None;
        }
        let root = xlib::XDefaultRootWindow(d);
        let atom = xlib::XInternAtom(d, c"_NET_ACTIVE_WINDOW".as_ptr(), 0);
        let (mut kind, mut format, mut items, mut after) = (0, 0, 0, 0);
        let mut prop: *mut u8 = std::ptr::null_mut();
        let mut found = None;
        let got = xlib::XGetWindowProperty(
            d, root, atom, 0, 1, 0, xlib::XA_WINDOW,
            &mut kind, &mut format, &mut items, &mut after, &mut prop,
        );
        if got == 0 && !prop.is_null() {
            let win = if items == 1 { *(prop as *const xlib::Window) } else { 0 };
            xlib::XFree(prop as _);
            let mut attrs: xlib::XWindowAttributes = std::mem::zeroed();
            let (mut x, mut y, mut child) = (0, 0, 0);
            if win != 0
                && xlib::XGetWindowAttributes(d, win, &mut attrs) != 0
                && xlib::XTranslateCoordinates(d, win, root, 0, 0, &mut x, &mut y, &mut child) != 0
                && attrs.width > 100
            {
                found = Some(Frame { x: x as f64, y: y as f64, w: attrs.width as f64, h: attrs.height as f64 });
            }
        }
        xlib::XCloseDisplay(d);
        found
    }
}
