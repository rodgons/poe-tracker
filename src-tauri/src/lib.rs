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
    AppHandle, Emitter, LogicalPosition, Manager, PhysicalPosition, State, WebviewWindow,
};

mod clientlog;
mod foreground;
mod gamewin;
mod gems;
mod keys;
#[cfg(target_os = "macos")]
mod macos;
mod placement;
mod pob;
mod poeapi;
mod store;
mod tree;

struct App {
    path: PathBuf,
    saved: store::Saved,
    /// a build loaded in the Build section but not saved yet, with what was pasted
    pending: Option<(String, pob::Build)>,
    /// the Client.txt being followed, if one was found
    watching: Option<PathBuf>,
    /// the character the last level-up line named
    playing: Option<String>,
    /// what the last Re-sync did, shown until the next tick
    synced: Option<String>,
    /// the game window's frame while PoE is frontmost
    frame: Option<gamewin::Frame>,
    /// where the Corner panel was (logical) when the player started dragging it
    drag_from: Option<(f64, f64)>,
    /// nodes clicked for refund in the game. The log says nothing on Apply or Cancel,
    /// so they stay Allocated until an import on the next area change settles them.
    refunding: Vec<u32>,
    /// that import failed (or there's no account): wait for a Re-sync instead
    refund_unchecked: bool,
    /// a downloaded update's version, waiting for a restart
    update_ready: Option<String>,
    /// the Character's socketed gems from the last items import
    gems: Gems,
}

/// The last items import from pathofexile.com, and when it can run again.
#[derive(Default)]
struct Gems {
    owned: Option<Vec<poeapi::OwnedGem>>,
    /// seconds since 1970 when `owned` was imported
    at: Option<u64>,
    /// why the last import failed
    error: Option<String>,
    busy: bool,
    tried: Option<std::time::Instant>,
}

/// Imports at most this often, unless the player asked (Save, Re-sync): get-items allows
/// 90 requests per 30 minutes per IP.
const GEMS_EVERY: Duration = Duration::from_secs(60);

impl App {
    /// Ticking pauses while the log shows a different character playing.
    fn paused_for(&self) -> Option<&str> {
        let playing = self.playing.as_deref()?;
        let chosen = self.saved.character.as_str();
        (!chosen.is_empty() && !playing.eq_ignore_ascii_case(chosen)).then_some(playing)
    }

    /// A picked file that's gone (game moved, drive unplugged) counts as not found,
    /// so the log follower waits for it instead of spinning.
    fn log_path(&self) -> Option<PathBuf> {
        self.saved.log_path.clone().or_else(clientlog::find).filter(|p| p.is_file())
    }
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
    account: String,
    character: String,
    spec: usize,
    taken: usize,
    /// how many Next points the panel shows
    points: usize,
    specs: Vec<SpecChoice>,
    tree_version: String,
    /// the Client.txt being followed; None when none was found
    log_path: Option<String>,
}

pub enum Msg {
    Hotkey,
    Esc,
    /// +1 in, -1 out, 0 back to the whole spec
    Zoom(i8),
    /// move the zoomed Mini-tree view: (x, y) steps
    Pan(i8, i8),
    /// the tray's Show/Hide panel item
    TogglePanel,
    /// the Gem panel hotkey
    GemHotkey,
    /// the tray's Show/Hide gem panel item
    ToggleGemPanel,
    /// the frontmost app, and its window frame when it's the game
    Frontmost(String, Option<gamewin::Frame>),
    /// the player saved new key binds
    Rebind(keys::Binds),
}

/// Where key presses go: the toggle loop.
struct KeyTx(Mutex<mpsc::Sender<Msg>>);

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

/// Imports the Character's gems in the background. `asked` (Save, Re-sync) skips the
/// throttle; area changes and opening the Gems section don't.
fn import_gems(handle: &AppHandle, asked: bool) {
    let job = {
        let st = handle.state::<AppState>();
        let mut st = st.lock().unwrap();
        let s = &st.saved;
        let ready = s.build.is_some() && !s.account.is_empty() && !s.character.is_empty();
        let recent = st.gems.tried.is_some_and(|t| t.elapsed() < GEMS_EVERY);
        if !ready || st.gems.busy || (recent && !asked) {
            None
        } else {
            let job = (s.account.clone(), s.character.clone());
            st.gems.busy = true;
            st.gems.tried = Some(std::time::Instant::now());
            Some(job)
        }
    };
    let Some((account, character)) = job else { return };
    let _ = handle.emit("gems", ());
    let app = handle.clone();
    std::thread::spawn(move || {
        let got = poeapi::items(&account, &character);
        let st = app.state::<AppState>();
        let mut st = st.lock().unwrap();
        st.gems.busy = false;
        // the Character changed while importing: this answer is someone else's
        if st.saved.character != character || st.saved.account != account {
            return;
        }
        match got {
            Ok(owned) => {
                st.gems.owned = Some(owned);
                st.gems.at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs());
                st.gems.error = None;
            }
            Err(e) => st.gems.error = Some(e),
        }
        drop(st);
        let _ = app.emit("gems", ());
    });
}

/// What the Gems section (and later the Gem panel) shows.
#[derive(Serialize)]
struct GemsView {
    skill_sets: Vec<String>,
    skill_set: Option<usize>,
    chosen: Option<gems::Chosen>,
    spec: String,
    character: String,
    /// None until an import has worked
    comparison: Option<gems::Comparison>,
    /// the Skill set alone, when there's nothing to compare against
    groups: Vec<gems::GroupRow>,
    as_of: Option<u64>,
    busy: bool,
    /// why gems can't be compared (no account, private profile…)
    notice: Option<String>,
}

fn gems_view_of(app: &App) -> Option<GemsView> {
    let s = &app.saved;
    let build = s.build.as_ref()?;
    let spec = build.specs.get(s.spec)?;
    let chosen = gems::choose(build, s.spec, s.skill_set);
    let set = chosen.and_then(|(i, _)| build.skill_sets.get(i));
    let comparison = match (set, &app.gems.owned) {
        (Some(set), Some(owned)) => Some(gems::compare(set, owned)),
        _ => None,
    };
    let groups = match (set, &comparison) {
        (Some(set), None) => gems::compare(set, &[]).groups,
        _ => Vec::new(),
    };
    let notice = if build.skill_sets.is_empty() {
        Some("This build has no gems. Paste it again in Build if it was saved before v0.6.".into())
    } else if s.account.is_empty() {
        Some("Add your pathofexile.com account in Build to compare your gems.".into())
    } else {
        app.gems.error.as_ref().map(|e| {
            if e.contains("private") {
                "Make your Characters tab public on pathofexile.com (My Account → Privacy) to compare your gems.".to_string()
            } else {
                format!("Couldn't import your gems: {e}")
            }
        })
    };
    Some(GemsView {
        skill_sets: build.skill_sets.iter().map(|t| pob::clean_title(&t.title)).collect(),
        skill_set: chosen.map(|(i, _)| i),
        chosen: chosen.map(|(_, c)| c),
        spec: spec.title.clone(),
        character: s.character.clone(),
        comparison,
        groups,
        as_of: app.gems.at,
        busy: app.gems.busy,
        notice,
    })
}

#[tauri::command]
fn gems_view(state: State<'_, AppState>) -> Option<GemsView> {
    gems_view_of(&state.lock().unwrap())
}

/// The Gems section opened: import again if the last import is over a minute old.
#[tauri::command]
fn refresh_gems(app: AppHandle) {
    import_gems(&app, false);
}

/// The player's Skill set pick; None follows the Tree spec again.
#[tauri::command]
fn set_skill_set(skill_set: Option<usize>, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut st = state.lock().unwrap();
    st.saved.skill_set = skill_set;
    store::save(&st.path, &st.saved)?;
    drop(st);
    let _ = app.emit("gems", ());
    Ok(())
}

/// Builds saved before v0.6 kept only their trees: read the pasted build again for its
/// Skill sets (and the raw spec titles that pair with them).
fn upgrade_build(handle: AppHandle) {
    let input = {
        let st = handle.state::<AppState>();
        let st = st.lock().unwrap();
        match &st.saved.build {
            Some(b) if b.skill_sets.is_empty() && !st.saved.input.is_empty() => st.saved.input.clone(),
            _ => return,
        }
    };
    std::thread::spawn(move || {
        let Ok(fresh) = pob::code_from_input(&input).and_then(|c| pob::decode(&c)) else { return };
        let st = handle.state::<AppState>();
        let mut st = st.lock().unwrap();
        if st.saved.input != input {
            return;
        }
        let Some(build) = st.saved.build.as_mut() else { return };
        if build.specs.len() == fresh.specs.len() {
            for (old, new) in build.specs.iter_mut().zip(&fresh.specs) {
                old.raw_title = new.raw_title.clone();
            }
        }
        build.skill_sets = fresh.skill_sets;
        build.active_skill_set = fresh.active_skill_set;
        let _ = store::save(&st.path, &st.saved);
        drop(st);
        let _ = handle.emit("gems", ());
    });
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

/// The game frame in logical pixels, for the panel window's scale.
fn logical_frame(win: &WebviewWindow, frame: gamewin::Frame) -> gamewin::Frame {
    let scale = if gamewin::LOGICAL { 1.0 } else { win.scale_factor().unwrap_or(1.0) };
    gamewin::Frame { x: frame.x / scale, y: frame.y / scale, w: frame.w / scale, h: frame.h / scale }
}

fn logical_size(win: &WebviewWindow) -> (f64, f64) {
    let scale = win.scale_factor().unwrap_or(1.0);
    win.outer_size().map_or((436.0, 520.0), |s| (s.width as f64 / scale, s.height as f64 / scale))
}

/// The Gem panel's window; the Corner panel's is "corner".
const GEM_PANEL: &str = "gempanel";

/// The panel window a page names: "gems" is the Gem panel, anything else the Corner panel.
fn panel_label(which: Option<&str>) -> &'static str {
    if which == Some("gems") {
        GEM_PANEL
    } else {
        "corner"
    }
}

/// Puts a panel inside the game window at the player's spot (the Corner panel top-right,
/// the Gem panel beside the gem level-up icons by default), or in the screen corner when
/// the game window can't be found.
fn position_panel(app: &AppHandle, win: &WebviewWindow) {
    let gems = win.label() == GEM_PANEL;
    let (frame, spot, dragging) = {
        let st = app.state::<AppState>();
        let st = st.lock().unwrap();
        (st.frame, if gems { st.saved.gem_spot } else { st.saved.panel_spot }, st.drag_from.is_some())
    };
    if dragging {
        return;
    }
    let Some(frame) = frame else { return place_in_corner(win) };
    let (frame, size) = (logical_frame(win, frame), logical_size(win));
    let (x, y) = match spot {
        None if gems => placement::beside_gem_icons(frame, size),
        _ => placement::place(frame, size, spot),
    };
    let _ = win.set_position(LogicalPosition::new(x, y));
}

/// Clicking the panel mustn't take focus from the game.
#[cfg(target_os = "windows")]
fn no_activate(win: &WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE};
    let Ok(hwnd) = win.hwnd() else { return };
    unsafe {
        let hwnd = hwnd.0 as _;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | WS_EX_NOACTIVATE as isize);
    }
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
    // never shown outside the game, so not at launch either: the tray icon says it's running
    let mut visible = false;
    let mut gems_visible = false;
    let mut frontmost = foreground::frontmost_app();
    let mut key = "none yet";
    for msg in rx {
        let in_poe = foreground::is_poe(&frontmost);
        match msg {
            // showing needs the game in front; hiding works from anywhere
            Msg::Hotkey if in_poe || visible => {
                presses += 1;
                key = "panel hotkey";
                visible = !visible;
            }
            Msg::Esc if visible || gems_visible => {
                key = "Esc";
                visible = false;
                gems_visible = false;
            }
            Msg::TogglePanel if in_poe || visible => visible = !visible,
            Msg::GemHotkey | Msg::ToggleGemPanel if in_poe || gems_visible => {
                key = "gem panel hotkey";
                gems_visible = !gems_visible;
                // opening it brings the gems up to date, at most once a minute
                if gems_visible {
                    import_gems(&app, false);
                }
            }
            Msg::Zoom(step) if visible => {
                let _ = app.emit_to("corner", "zoom", step);
                continue;
            }
            Msg::Pan(dx, dy) if visible => {
                let _ = app.emit_to("corner", "pan", (dx, dy));
                continue;
            }
            Msg::Frontmost(name, frame) => {
                let moved = {
                    let st = app.state::<AppState>();
                    let mut st = st.lock().unwrap();
                    let moved = st.frame != frame;
                    st.frame = frame;
                    moved
                };
                if name == frontmost && !(moved && (visible || gems_visible)) {
                    continue;
                }
                // leaving the game hides the panels; coming back doesn't reopen them
                if in_poe && !foreground::is_poe(&name) {
                    visible = false;
                    gems_visible = false;
                }
                frontmost = name;
            }
            Msg::Rebind(binds) => keys::rebind(&app, binds),
            _ => continue,
        }
        // take the binds from the game only while it's in front (zoom and pan only with the panel shown)
        let want = if foreground::is_poe(&frontmost) { keys::TOGGLE | if visible { keys::VIEW } else { 0 } } else { 0 };
        keys::grab(&app, want);
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
            if let Some(win) = handle.get_webview_window(GEM_PANEL) {
                if gems_visible {
                    position_panel(&handle, &win);
                    show(&win)
                } else {
                    hide(&win)
                }
            }
            let Some(win) = handle.get_webview_window("corner") else { return };
            if status.visible {
                position_panel(&handle, &win);
                show(&win)
            } else {
                hide(&win)
            }
            let _ = win.emit("status", status);
        });
    }
}

fn poll_frontmost(tx: mpsc::Sender<Msg>) {
    loop {
        let app = foreground::frontmost_app();
        let frame = if foreground::is_poe(&app) { gamewin::frontmost_frame() } else { None };
        if tx.send(Msg::Frontmost(app, frame)).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// the most Next points the Settings section lets the panel number
const MAX_POINTS: usize = 15;

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

fn panel_view_of(app: &App) -> Option<tree::PanelView> {
    let saved = &app.saved;
    let build = saved.build.as_ref()?;
    let spec = build.specs.get(saved.spec)?;
    let mut view = match tree::Plan::new(tree::tree(), spec) {
        Ok(plan) => {
            let allocated: HashSet<u32> = saved.allocated.iter().copied().collect();
            plan.view(tree::tree(), &allocated, saved.points)
        }
        Err(e) => tree::PanelView::unreadable(&spec.title, e),
    };
    view.character = saved.character.clone();
    view.next_spec = build.specs.get(saved.spec + 1).map(|s| s.title.clone());
    if let Some(note) = &app.synced {
        view.notices.insert(0, note.clone());
    }
    if let Some(other) = app.paused_for() {
        view.stopped = Some(format!(
            "Ticking paused: the game log shows {other} playing, not {}. Re-sync character to resume.",
            saved.character
        ));
        view.stopped_fix = Some("resync");
    } else if app.watching.is_none() {
        view.stopped = Some("Client.txt not found, so points won't tick. Pick it in Settings.".into());
        view.stopped_fix = Some("log");
    }
    if !app.refunding.is_empty() {
        let t = tree::tree();
        let names: Vec<&str> = app.refunding.iter().filter_map(|n| t.nodes.get(n)).map(|n| n.name.as_str()).collect();
        let when = if app.refund_unchecked || app.saved.account.is_empty() {
            "Re-sync character once applied"
        } else {
            "checked when you change area"
        };
        view.pending_note = Some(format!("{} ({when})", names.join(", ")));
        view.pending = app.refunding.clone();
    }
    Some(view)
}

/// Applies one Client.txt line to the Allocated nodes, then saves and redraws.
fn on_log_line(handle: &AppHandle, line: &str) {
    let Some(event) = clientlog::parse(line) else { return };
    let state = handle.state::<AppState>();
    let mut guard = state.lock().unwrap();
    let app = &mut *guard;
    let t = tree::tree();
    let changed = match event {
        clientlog::Event::LevelUp { character, .. } => {
            let changed = app.playing.as_deref() != Some(character.as_str());
            app.playing = Some(character);
            changed
        }
        _ if app.paused_for().is_some() => false,
        clientlog::Event::Allocated { id, name } => {
            let Some(build) = app.saved.build.as_ref() else { return };
            let Some(Ok(plan)) = build.specs.get(app.saved.spec).map(|s| tree::Plan::new(t, s)) else { return };
            let have: HashSet<u32> = app.saved.allocated.iter().copied().collect();
            match t.node_for_log(&id, &name, plan.order(t, &have)) {
                // nodes outside the main tree (ascendancy, cluster jewels) are left out
                Some(hash) if t.nodes.contains_key(&hash) && !have.contains(&hash) => {
                    app.saved.allocated.push(hash);
                    true
                }
                // taking back a node that was waiting to be refunded: it stays
                Some(hash) if app.refunding.contains(&hash) => {
                    app.refunding.retain(|n| *n != hash);
                    true
                }
                _ => false,
            }
        }
        // logged on the click, before Apply or Cancel: keep the node until an import says
        clientlog::Event::Unallocated { id, name } => {
            let have = app.saved.allocated.clone();
            match t.node_for_log(&id, &name, have) {
                Some(hash) if app.saved.allocated.contains(&hash) && !app.refunding.contains(&hash) => {
                    app.refunding.push(hash);
                    true
                }
                _ => false,
            }
        }
        clientlog::Event::Entered => {
            // the log handler holds the state: import once it lets go
            let h = handle.clone();
            std::thread::spawn(move || import_gems(&h, false));
            let s = &app.saved;
            let spec = s.build.as_ref().and_then(|b| b.specs.get(s.spec));
            if let Some(spec) = spec.filter(|_| !app.refunding.is_empty() && !app.refund_unchecked && !s.account.is_empty()) {
                settle_refunds(handle, s.account.clone(), s.character.clone(), spec.clone());
            }
            false
        }
    };
    if changed {
        app.synced = None;
        let _ = store::save(&app.path, &app.saved);
        let _ = handle.emit("plan", panel_view_of(app));
    }
}

/// Imports the Character from pathofexile.com to learn which pending refunds were
/// applied. When it can't, the refunds wait for a Re-sync.
fn settle_refunds(handle: &AppHandle, account: String, character: String, spec: pob::Spec) {
    let app = handle.clone();
    std::thread::spawn(move || {
        let imported = import_allocated(&account, &character, &spec);
        let st = app.state::<AppState>();
        let mut st = st.lock().unwrap();
        match imported {
            Ok(allocated) => {
                st.saved.allocated = allocated;
                st.refunding.clear();
                let _ = store::save(&st.path, &st.saved);
            }
            Err(_) => st.refund_unchecked = true,
        }
        let _ = app.emit("plan", panel_view_of(&st));
    });
}

/// Follows Client.txt for the app's lifetime, switching files when the player picks another.
fn follow_log(handle: AppHandle) {
    loop {
        let path = handle.state::<AppState>().lock().unwrap().log_path();
        {
            let state = handle.state::<AppState>();
            let mut app = state.lock().unwrap();
            if app.watching != path {
                app.watching = path.clone();
                let _ = handle.emit("plan", panel_view_of(&app));
            }
        }
        let Some(path) = path else {
            std::thread::sleep(Duration::from_secs(3));
            continue;
        };
        let watched = path.clone();
        clientlog::follow(
            &path,
            |line| on_log_line(&handle, line),
            || {
                let app = handle.state::<AppState>();
                let app = app.lock().unwrap();
                // switch when the player picks another file, or, when they haven't picked
                // one, when the game starts writing a different usual log
                let still = match &app.saved.log_path {
                    Some(p) => *p == watched,
                    None => clientlog::find().is_none_or(|p| p == watched),
                };
                still && watched.is_file()
            },
        );
    }
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
        account: saved.account.clone(),
        character: saved.character.clone(),
        spec: saved.spec,
        taken: panel_view_of(&app).map_or(0, |v| v.done),
        points: saved.points,
        specs: saved.build.as_ref().map(spec_choices).unwrap_or_default(),
        tree_version: tree::tree().version.clone(),
        log_path: app.watching.clone().or_else(|| app.log_path()).map(|p| p.display().to_string()),
    }
}

#[tauri::command]
fn key_binds(state: State<'_, AppState>) -> keys::Binds {
    state.lock().unwrap().saved.binds.clone()
}

/// Saves new key binds and listens for them at once; None puts back the defaults.
#[tauri::command]
fn set_binds(binds: Option<keys::Binds>, app: AppHandle, state: State<'_, AppState>) -> Result<keys::Binds, String> {
    let binds = binds.unwrap_or_default();
    binds.check()?;
    let mut guard = state.lock().unwrap();
    guard.saved.binds = binds.clone();
    store::save(&guard.path, &guard.saved)?;
    let _ = app.state::<KeyTx>().0.lock().unwrap().send(Msg::Rebind(binds.clone()));
    let _ = app.emit("binds", &binds);
    Ok(binds)
}

/// Lets the player pick Client.txt when it isn't in a usual install folder.
#[tauri::command]
async fn choose_log(app: AppHandle, state: State<'_, AppState>) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().set_title("Find Path of Exile's Client.txt").blocking_pick_file();
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    let mut guard = state.lock().unwrap();
    guard.saved.log_path = Some(path.clone());
    store::save(&guard.path, &guard.saved)?;
    Ok(Some(path.display().to_string()))
}

const CLASSES: [&str; 7] = ["Scion", "Marauder", "Ranger", "Witch", "Duelist", "Templar", "Shadow"];

/// The Character's allocated nodes, imported from pathofexile.com.
fn import_allocated(account: &str, character: &str, spec: &pob::Spec) -> Result<Vec<u32>, String> {
    let t = tree::tree();
    let plan = tree::Plan::new(t, spec)?;
    let passives = poeapi::passives(account, character)?;
    if passives.class_id != spec.class_id {
        let class = |id: u32| CLASSES.get(id as usize).copied().unwrap_or("another class");
        return Err(format!(
            "{character} is a {}, but this tree spec is for a {}.",
            class(passives.class_id),
            class(spec.class_id)
        ));
    }
    let mut allocated: Vec<u32> = plan.from_import(t, &passives.hashes).into_iter().collect();
    allocated.sort_unstable();
    Ok(allocated)
}

/// Lists the account's characters for the Build section.
#[tauri::command]
async fn find_characters(account: String) -> Result<Vec<poeapi::Character>, String> {
    tauri::async_runtime::spawn_blocking(move || poeapi::characters(&account)).await.map_err(|e| e.to_string())?
}

/// Saves the setup. With `taken` the Seed is the first points in order; without,
/// it's imported from pathofexile.com.
#[tauri::command]
async fn save_setup(
    spec: usize,
    account: String,
    character: String,
    taken: Option<usize>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (account, character) = (account.trim().to_string(), character.trim().to_string());
    let (chosen, kept) = {
        let st = state.lock().unwrap();
        let build = st.pending.as_ref().map(|(_, b)| b).or(st.saved.build.as_ref()).ok_or("Load a build first.")?;
        // re-saving the same spec with the taken count as it stands (say, to fix the
        // character name) keeps the exact points, not the first ones in order
        let unchanged = st.pending.is_none()
            && spec == st.saved.spec
            && taken.is_some()
            && taken == panel_view_of(&st).map(|v| v.done);
        (build.specs.get(spec).ok_or("Pick a tree spec.")?.clone(), unchanged.then(|| st.saved.allocated.clone()))
    };
    let allocated = match (kept, taken) {
        (Some(kept), _) => kept,
        (None, Some(taken)) => {
            let plan = tree::Plan::new(tree::tree(), &chosen)?;
            let mut allocated: Vec<u32> = plan.seed(tree::tree(), taken).into_iter().collect();
            allocated.sort_unstable();
            allocated
        }
        (None, None) => {
            let (a, c) = (account.clone(), character.clone());
            tauri::async_runtime::spawn_blocking(move || import_allocated(&a, &c, &chosen))
                .await
                .map_err(|e| e.to_string())??
        }
    };
    let mut guard = state.lock().unwrap();
    let st = &mut *guard;
    let new_build = st.pending.is_some();
    if let Some((input, build)) = st.pending.take() {
        st.saved.input = input;
        st.saved.build = Some(build);
    }
    // a new Build or Tree spec follows its paired Skill set again
    if new_build || spec != st.saved.spec {
        st.saved.skill_set = None;
    }
    // another Character's gems aren't this one's
    if character != st.saved.character || account != st.saved.account {
        st.gems = Gems::default();
    }
    st.saved.spec = spec;
    st.saved.account = account;
    st.saved.character = character;
    st.saved.allocated = allocated;
    st.refunding.clear();
    st.refund_unchecked = false;
    // saving (or re-syncing) says the Character is the one playing
    st.playing = None;
    st.synced = None;
    store::save(&st.path, &st.saved)?;
    let _ = app.emit("plan", panel_view_of(st));
    drop(guard);
    let _ = app.emit("gems", ());
    import_gems(&app, true);
    Ok(())
}

/// How many Next points the panel numbers: saved as soon as the player changes it.
#[tauri::command]
fn set_points(points: usize, app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let mut guard = state.lock().unwrap();
    let st = &mut *guard;
    st.saved.points = points.clamp(1, MAX_POINTS);
    store::save(&st.path, &st.saved)?;
    let _ = app.emit("plan", panel_view_of(st));
    Ok(st.saved.points)
}

/// The Main window's Re-sync character button.
#[tauri::command]
fn resync_character(app: AppHandle) {
    resync(&app);
}

#[derive(Serialize)]
struct AppInfo {
    version: String,
    /// a downloaded update, waiting for a restart
    update: Option<String>,
}

#[tauri::command]
fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    AppInfo { version: app.package_info().version.to_string(), update: state.lock().unwrap().update_ready.clone() }
}

#[tauri::command]
fn restart_to_update(app: AppHandle) {
    app.restart();
}

/// Opens System Settings at Input Monitoring, where the player allows the key hook.
#[tauri::command]
fn open_input_monitoring() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Re-sync character: imports again when there's an account; without one, or
/// when the import fails, opens the Main window's Build section instead.
fn resync(app: &AppHandle) {
    let job = {
        let st = app.state::<AppState>();
        let st = st.lock().unwrap();
        let s = &st.saved;
        match s.build.as_ref().and_then(|b| b.specs.get(s.spec)) {
            Some(spec) if !s.account.is_empty() && !s.character.is_empty() => {
                Some((s.account.clone(), s.character.clone(), spec.clone()))
            }
            _ => None,
        }
    };
    if job.is_some() {
        import_gems(app, true);
    }
    let Some((account, character, spec)) = job else {
        let has_build = app.state::<AppState>().lock().unwrap().saved.build.is_some();
        if has_build {
            let _ = app.emit_to(
                "main",
                "resync_error",
                "Re-sync needs your pathofexile.com account: type it below, find your Character and Save.",
            );
        }
        return open_main(app, Some("build"));
    };
    let app = app.clone();
    std::thread::spawn(move || match import_allocated(&account, &character, &spec) {
        Ok(allocated) => {
            let st = app.state::<AppState>();
            let mut st = st.lock().unwrap();
            st.saved.allocated = allocated;
            st.playing = None;
            st.refunding.clear();
            st.refund_unchecked = false;
            st.synced = Some(format!("Re-synced {character} from pathofexile.com"));
            let _ = store::save(&st.path, &st.saved);
            let _ = app.emit("plan", panel_view_of(&st));
            let _ = app.emit_to("main", "resynced", format!("Re-synced {character}"));
        }
        Err(e) => {
            let _ = app.emit_to("main", "resync_error", format!("Re-sync failed: {e}"));
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || open_main(&handle, Some("build")));
        }
    });
}

/// Shrinks the corner window to the panel, so the empty space under it doesn't
/// catch clicks meant for the game.
#[tauri::command]
fn fit_panel(height: f64, which: Option<String>, app: AppHandle) {
    let Some(win) = app.get_webview_window(panel_label(which.as_deref())) else { return };
    let Ok(scale) = win.scale_factor() else { return };
    let Ok(size) = win.outer_size() else { return };
    let width = size.width as f64 / scale;
    let _ = win.set_size(tauri::LogicalSize::new(width, height.clamp(80.0, 900.0)));
    // a panel near the bottom of the game window has to stay inside it
    position_panel(&app, &win);
}

/// The player dragging the Corner panel by its title: `dx`, `dy` are logical pixels
/// since the drag started. Dropping it remembers the spot within the game window.
#[tauri::command]
fn drag_panel(
    phase: String,
    dx: f64,
    dy: f64,
    which: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let gems = which.as_deref() == Some("gems");
    let Some(win) = app.get_webview_window(panel_label(which.as_deref())) else { return Ok(()) };
    // window queries first, so the app state is never locked while waiting on the window
    let scale = win.scale_factor().map_err(|e| e.to_string())?;
    let pos = win.outer_position().map_err(|e| e.to_string())?;
    let size = logical_size(&win);
    let mut st = state.lock().unwrap();
    if phase == "start" {
        st.drag_from = Some((pos.x as f64 / scale, pos.y as f64 / scale));
        return Ok(());
    }
    let Some(from) = st.drag_from else { return Ok(()) };
    let mut at = (from.0 + dx, from.1 + dy);
    let frame = st.frame.map(|f| {
        let s = if gamewin::LOGICAL { 1.0 } else { scale };
        gamewin::Frame { x: f.x / s, y: f.y / s, w: f.w / s, h: f.h / s }
    });
    if let Some(frame) = frame {
        at = placement::place(frame, size, Some(placement::spot_of(frame, size, at)));
    }
    if phase == "end" {
        st.drag_from = None;
        if let Some(frame) = frame {
            let spot = Some(placement::spot_of(frame, size, at));
            if gems {
                st.saved.gem_spot = spot;
            } else {
                st.saved.panel_spot = spot;
            }
            store::save(&st.path, &st.saved)?;
        }
    }
    drop(st);
    let _ = win.set_position(LogicalPosition::new(at.0, at.1));
    Ok(())
}

#[tauri::command]
fn panel_view(state: State<'_, AppState>) -> Option<tree::PanelView> {
    panel_view_of(&state.lock().unwrap())
}

/// The Tree section: everything about the current Tree spec, away from the game.
#[derive(Serialize)]
struct BuildView {
    view: tree::PanelView,
    points: Vec<tree::ListedPoint>,
    specs: Vec<SpecChoice>,
    spec: usize,
    character: String,
}

#[tauri::command]
fn build_view(state: State<'_, AppState>) -> Option<BuildView> {
    let app = state.lock().unwrap();
    let saved = &app.saved;
    let build = saved.build.as_ref()?;
    let allocated: HashSet<u32> = saved.allocated.iter().copied().collect();
    // an unreadable spec still opens, so the player can pick another one
    let points = tree::Plan::new(tree::tree(), build.specs.get(saved.spec)?)
        .map(|plan| plan.points(tree::tree(), &allocated))
        .unwrap_or_default();
    Some(BuildView {
        view: panel_view_of(&app)?,
        points,
        specs: spec_choices(build),
        spec: saved.spec,
        character: saved.character.clone(),
    })
}

/// Switches the current Tree spec; the Allocated nodes carry over.
#[tauri::command]
fn set_spec(spec: usize, app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut st = state.lock().unwrap();
    let title = st.saved.build.as_ref().and_then(|b| b.specs.get(spec)).ok_or("No such tree spec.")?.title.clone();
    st.saved.spec = spec;
    st.saved.skill_set = None;
    // a hand Seed only counted the old spec's points; an import holds them all
    st.synced = st
        .saved
        .account
        .is_empty()
        .then(|| format!("Switched to {title}: Re-sync character so your taken points count"));
    store::save(&st.path, &st.saved)?;
    let _ = app.emit("plan", panel_view_of(&st));
    let _ = app.emit("gems", ());
    Ok(())
}

/// Shows the Main window, at `section` ("tree", "gems", "build" or "settings") when given;
/// otherwise the page picks (Build until a build is saved, then the last section used).
fn open_main(app: &AppHandle, section: Option<&str>) {
    let Some(win) = app.get_webview_window("main") else { return };
    #[cfg(target_os = "macos")]
    {
        macos::set_dock_icon(true);
        macos::activate();
    }
    if let Some(section) = section {
        let _ = win.emit("section", section);
    }
    let _ = win.show();
    let _ = win.unminimize();
    let _ = win.set_focus();
}

/// Auto-update, once at launch: if GitHub has a newer release, download and install it, then
/// restart into it straight away unless the Corner panel is showing. Then the tray menu offers
/// the restart instead, so an update never takes the panel away mid-game. (On Windows the
/// installer closes the app itself.) Any failure just leaves the current version running.
async fn update(app: AppHandle, menu: Menu<tauri::Wry>) {
    use tauri_plugin_updater::UpdaterExt;
    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(e) => return eprintln!("updater unavailable: {e}"),
    };
    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => return,
        Err(e) => return eprintln!("update check failed: {e}"),
    };
    if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
        return eprintln!("update to {} failed: {e}", update.version);
    }
    let panel_shown = ["corner", GEM_PANEL]
        .iter()
        .any(|l| app.get_webview_window(l).and_then(|w| w.is_visible().ok()).unwrap_or(false));
    if !panel_shown {
        app.restart();
    }
    if let Some(st) = app.try_state::<AppState>() {
        st.lock().unwrap().update_ready = Some(update.version.clone());
    }
    let _ = app.emit_to("main", "update_ready", &update.version);
    let label = format!("Restart to update to v{}", update.version);
    match MenuItem::with_id(&app, "update", label, true, None::<&str>) {
        Ok(item) => {
            let _ = menu.insert(&item, 0);
        }
        Err(e) => eprintln!("update menu item: {e}"),
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
        .plugin(tauri_plugin_dialog::init())
        // the Main window's size and position; never whether it's showing, and never the panel
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_denylist(&["corner"])
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::all() - tauri_plugin_window_state::StateFlags::VISIBLE,
                )
                .build(),
        )
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        return;
                    }
                    if let (Some(msg), Some(tx)) = (keys::grabbed_message(shortcut), app.try_state::<KeyTx>()) {
                        let _ = tx.0.lock().unwrap().send(msg);
                    }
                })
                .build(),
        )
        // checked from Rust at launch (see `update`); no capability lets a page call it
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            initial_status,
            load_build,
            spec_order,
            current_setup,
            key_binds,
            set_binds,
            save_setup,
            panel_view,
            choose_log,
            find_characters,
            fit_panel,
            drag_panel,
            build_view,
            set_spec,
            set_points,
            resync_character,
            gems_view,
            refresh_gems,
            set_skill_set,
            app_info,
            restart_to_update,
            open_input_monitoring
        ])
        .setup(|app| {
            let path = store::path(app.path().app_config_dir()?);
            let saved = store::load(&path);
            keys::set(saved.binds.clone());
            let first_run = saved.build.is_none();
            app.manage(Mutex::new(App { path, saved, pending: None, watching: None, playing: None, synced: None, frame: None, drag_from: None, refunding: Vec::new(), refund_unchecked: false, update_ready: None, gems: Gems::default() }));
            upgrade_build(app.handle().clone());

            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                macos::request_listen_permission();
            }

            let win = app.get_webview_window("corner").expect("corner window");
            place_in_corner(&win);
            // the panel takes the mouse (scroll to zoom, drag to move) but never focus,
            // so the game keeps the keyboard
            #[cfg(target_os = "macos")]
            {
                macos::make_nonactivating(&win);
                macos::float_over_fullscreen(&win);
            }
            #[cfg(target_os = "windows")]
            no_activate(&win);
            // the Gem panel follows the same rules: mouse only, never the keyboard
            if let Some(gem_win) = app.get_webview_window(GEM_PANEL) {
                place_in_corner(&gem_win);
                #[cfg(target_os = "macos")]
                {
                    macos::make_nonactivating(&gem_win);
                    macos::float_over_fullscreen(&gem_win);
                }
                #[cfg(target_os = "windows")]
                no_activate(&gem_win);
            }

            // closing the Main window only hides it, so the tray can reopen it
            if let Some(main) = app.get_webview_window("main") {
                let handle = main.clone();
                main.on_window_event(move |e| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                        api.prevent_close();
                        let _ = handle.hide();
                        #[cfg(target_os = "macos")]
                        macos::set_dock_icon(false);
                    }
                });
            }

            let (tx, rx) = mpsc::channel();
            let tray_tx = tx.clone();
            app.manage(KeyTx(Mutex::new(tx.clone())));
            let panel_item = MenuItem::with_id(app, "panel", "Show/Hide panel", true, None::<&str>)?;
            let gem_item = MenuItem::with_id(app, "gempanel", "Show/Hide gem panel", true, None::<&str>)?;
            let gems_item = MenuItem::with_id(app, "gems", "Open gems…", true, None::<&str>)?;
            let tree_item = MenuItem::with_id(app, "tree", "Open tree…", true, None::<&str>)?;
            let build_item = MenuItem::with_id(app, "build", "Set up build…", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
            let resync_item = MenuItem::with_id(app, "resync", "Re-sync character…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit POE Tracker", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&panel_item, &gem_item, &tree_item, &gems_item, &build_item, &settings_item, &resync_item, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("POE Tracker")
                .menu(&menu)
                .on_menu_event(move |app, e| match e.id().as_ref() {
                    "panel" => {
                        let _ = tray_tx.send(Msg::TogglePanel);
                    }
                    "gempanel" => {
                        let _ = tray_tx.send(Msg::ToggleGemPanel);
                    }
                    "gems" => open_main(app, Some("gems")),
                    "tree" => open_main(app, Some("tree")),
                    "build" => open_main(app, Some("build")),
                    "settings" => open_main(app, Some("settings")),
                    "resync" => resync(app),
                    "update" => app.restart(),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            // the panel no longer shows at launch, so a missing permission is told in the Main window
            if first_run || permission().contains("NOT") {
                open_main(app.handle(), None);
            }

            keys::spawn(tx.clone());
            std::thread::spawn(move || poll_frontmost(tx));
            let handle = app.handle().clone();
            std::thread::spawn(move || run_toggle_loop(handle, rx));
            let handle = app.handle().clone();
            std::thread::spawn(move || follow_log(handle));
            tauri::async_runtime::spawn(update(app.handle().clone(), menu));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running POE Tracker");
}
