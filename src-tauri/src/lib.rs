// A corner panel showing the next passive points of a Path of Building tree spec,
// toggled by the panel hotkey (Alt/Option+P) while Path of Exile is frontmost,
// via a listen-only key hook. Nothing here ever sends input to the game.
use serde::Serialize;
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{mpsc, Mutex},
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow,
};

mod foreground;
mod keys;
#[cfg(target_os = "macos")]
mod macos;
mod pob;
mod store;
mod tree;

struct App {
    path: PathBuf,
    saved: store::Saved,
    /// a build loaded in the Setup window but not saved yet, with what was pasted
    pending: Option<(String, pob::Build)>,
}

type AppState = Mutex<App>;

#[derive(Serialize)]
struct SpecChoice {
    title: String,
    points: usize,
}

#[derive(Serialize)]
struct Loaded {
    specs: Vec<SpecChoice>,
    /// the spec the build was saved on in Path of Building
    active: usize,
}

#[derive(Serialize)]
struct Setup {
    input: String,
    character: String,
    spec: usize,
    taken: usize,
    specs: Vec<SpecChoice>,
    tree_version: String,
}

pub enum Msg {
    Hotkey,
    Esc,
    /// +1 in, -1 out, 0 back to the whole spec
    Zoom(i8),
    /// the tray's Show/Hide panel item
    TogglePanel,
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
            // showing needs the game in front; hiding works from anywhere
            Msg::Hotkey if in_poe || visible => {
                presses += 1;
                key = "⌥/Alt+P";
                visible = !visible;
            }
            Msg::Esc if visible => {
                key = "Esc";
                visible = false;
            }
            Msg::TogglePanel => visible = !visible,
            Msg::Zoom(step) if visible => {
                let _ = app.emit_to("corner", "zoom", step);
                continue;
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

fn spec_choices(build: &pob::Build) -> Vec<SpecChoice> {
    build
        .specs
        .iter()
        .map(|s| SpecChoice {
            title: s.title.clone(),
            points: tree::Plan::new(tree::tree(), s).map_or(0, |p| p.nodes.len()),
        })
        .collect()
}

fn panel_view_of(saved: &store::Saved) -> Option<tree::PanelView> {
    let spec = saved.build.as_ref()?.specs.get(saved.spec)?;
    let plan = tree::Plan::new(tree::tree(), spec).ok()?;
    let allocated: HashSet<u32> = saved.allocated.iter().copied().collect();
    Some(plan.view(tree::tree(), &allocated, saved.points))
}

#[tauri::command]
async fn load_build(input: String, state: State<'_, AppState>) -> Result<Loaded, String> {
    let fetched = input.clone();
    let build = tauri::async_runtime::spawn_blocking(move || pob::decode(&pob::code_from_input(&fetched)?))
        .await
        .map_err(|e| e.to_string())??;
    let loaded = Loaded { specs: spec_choices(&build), active: build.active };
    state.lock().unwrap().pending = Some((input, build));
    Ok(loaded)
}

/// The spec's points in the order the panel will suggest them.
#[tauri::command]
fn spec_order(spec: usize, state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let app = state.lock().unwrap();
    let build = app.pending.as_ref().map(|(_, b)| b).or(app.saved.build.as_ref()).ok_or("No build loaded.")?;
    let plan = tree::Plan::new(tree::tree(), build.specs.get(spec).ok_or("No such tree spec.")?)?;
    let t = tree::tree();
    Ok(plan.order(t, &HashSet::new()).iter().map(|id| t.nodes[id].name.clone()).collect())
}

#[tauri::command]
fn current_setup(state: State<'_, AppState>) -> Setup {
    let app = state.lock().unwrap();
    let saved = &app.saved;
    Setup {
        input: saved.input.clone(),
        character: saved.character.clone(),
        spec: saved.spec,
        // the class start is always allocated and isn't a point
        taken: saved.allocated.len().saturating_sub(1),
        specs: saved.build.as_ref().map(spec_choices).unwrap_or_default(),
        tree_version: tree::tree().version.clone(),
    }
}

#[tauri::command]
fn save_setup(
    spec: usize,
    character: String,
    taken: usize,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.lock().unwrap();
    let st = &mut *guard;
    if let Some((input, build)) = st.pending.take() {
        st.saved.input = input;
        st.saved.build = Some(build);
    }
    let build = st.saved.build.as_ref().ok_or("Load a build first.")?;
    let plan = tree::Plan::new(tree::tree(), build.specs.get(spec).ok_or("Pick a tree spec.")?)?;
    let mut allocated: Vec<u32> = plan.seed(tree::tree(), taken).into_iter().collect();
    allocated.sort_unstable();
    st.saved.spec = spec;
    st.saved.character = character.trim().to_string();
    st.saved.allocated = allocated;
    store::save(&st.path, &st.saved)?;
    let _ = app.emit_to("corner", "plan", panel_view_of(&st.saved));
    if let Some(win) = app.get_webview_window("setup") {
        let _ = win.hide();
    }
    Ok(())
}

#[tauri::command]
fn panel_view(state: State<'_, AppState>) -> Option<tree::PanelView> {
    panel_view_of(&state.lock().unwrap().saved)
}

fn open_setup(app: &AppHandle) {
    let Some(win) = app.get_webview_window("setup") else { return };
    #[cfg(target_os = "macos")]
    macos::activate();
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
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
        .invoke_handler(tauri::generate_handler![
            initial_status,
            load_build,
            spec_order,
            current_setup,
            save_setup,
            panel_view
        ])
        .setup(|app| {
            let path = store::path(app.path().app_config_dir()?);
            let saved = store::load(&path);
            let first_run = saved.build.is_none();
            app.manage(Mutex::new(App { path, saved, pending: None }));

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

            // closing the Setup window only hides it, so the tray can reopen it
            if let Some(setup) = app.get_webview_window("setup") {
                let handle = setup.clone();
                setup.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = handle.hide();
                    }
                });
            }

            let (tx, rx) = mpsc::channel();
            let tray_tx = tx.clone();
            let panel_item = MenuItem::with_id(app, "panel", "Show/Hide panel", true, None::<&str>)?;
            let setup_item = MenuItem::with_id(app, "setup", "Set up build…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit POE Tracker", true, None::<&str>)?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("POE Tracker")
                .menu(&Menu::with_items(app, &[&panel_item, &setup_item, &quit])?)
                .on_menu_event(move |app, e| match e.id().as_ref() {
                    "panel" => {
                        let _ = tray_tx.send(Msg::TogglePanel);
                    }
                    "setup" => open_setup(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            if first_run {
                open_setup(app.handle());
            }

            keys::spawn(tx.clone());
            std::thread::spawn(move || poll_frontmost(tx));
            let handle = app.handle().clone();
            std::thread::spawn(move || run_toggle_loop(handle, rx));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running POE Tracker");
}
