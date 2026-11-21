// Which app is frontmost, as a readable "name (id)" string. Read-only OS queries.

pub fn is_poe(app: &str) -> bool {
    let a = app.to_lowercase().replace(' ', "");
    // Proton/Wine windows carry the Steam app id in WM_CLASS
    a.contains("pathofexile") || a.contains("steam_app_238960")
}

#[cfg(target_os = "macos")]
pub fn frontmost_app() -> String {
    use objc2::{class, msg_send, rc::autoreleasepool, runtime::AnyObject};
    use std::ffi::{c_char, CStr};

    unsafe fn text(s: *mut AnyObject) -> String {
        if s.is_null() {
            return String::new();
        }
        let p: *const c_char = msg_send![s, UTF8String];
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }

    autoreleasepool(|_| unsafe {
        let ws: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: *mut AnyObject = msg_send![ws, frontmostApplication];
        if app.is_null() {
            return String::new();
        }
        let name: *mut AnyObject = msg_send![app, localizedName];
        let id: *mut AnyObject = msg_send![app, bundleIdentifier];
        format!("{} ({})", text(name), text(id))
    })
}

#[cfg(target_os = "windows")]
pub fn frontmost_app() -> String {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
    };
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        if pid == 0 {
            return String::new();
        }
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(h);
        if ok == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().unwrap_or_default().to_string()
    }
}

#[cfg(target_os = "linux")]
pub fn frontmost_app() -> String {
    use std::ffi::CStr;
    use x11::xlib;
    unsafe {
        let d = xlib::XOpenDisplay(std::ptr::null());
        if d.is_null() {
            return String::new();
        }
        let root = xlib::XDefaultRootWindow(d);
        let atom = xlib::XInternAtom(d, c"_NET_ACTIVE_WINDOW".as_ptr(), 0);
        let (mut kind, mut format, mut items, mut after) = (0, 0, 0, 0);
        let mut prop: *mut u8 = std::ptr::null_mut();
        let mut out = String::new();
        let got = xlib::XGetWindowProperty(
            d, root, atom, 0, 1, 0, xlib::XA_WINDOW,
            &mut kind, &mut format, &mut items, &mut after, &mut prop,
        );
        if got == 0 && !prop.is_null() {
            let win = if items == 1 { *(prop as *const xlib::Window) } else { 0 };
            xlib::XFree(prop as _);
            let mut hint: xlib::XClassHint = std::mem::zeroed();
            if win != 0 && xlib::XGetClassHint(d, win, &mut hint) != 0 {
                for p in [hint.res_name, hint.res_class] {
                    if !p.is_null() {
                        out.push_str(&CStr::from_ptr(p).to_string_lossy());
                        out.push(' ');
                        xlib::XFree(p as _);
                    }
                }
            }
        }
        xlib::XCloseDisplay(d);
        out.trim().to_string()
    }
}
