// The game's Client.txt: where it lives, and the lines that matter for the tree.
// Read-only; the app only ever reads new lines the game appends.
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum Event {
    Allocated { id: String, name: String },
    Unallocated { id: String, name: String },
    /// a level-up line: tells which character is playing
    LevelUp { character: String, level: u32 },
}

/// Turns one log line into an event, if it's one the app uses.
pub fn parse(line: &str) -> Option<Event> {
    // "2026/11/10 12:06:02 3336199872 e63da844 [INFO Client 54611] message"
    let msg = &line[line.find("] ")? + 2..];
    let msg = msg.trim_end();
    for (prefix, allocated) in [
        ("Successfully allocated passive skill id: ", true),
        ("Successfully unallocated passive skill id: ", false),
    ] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            let (id, name) = rest.split_once(", name: ")?;
            let (id, name) = (id.trim().to_string(), name.trim().to_string());
            return Some(if allocated { Event::Allocated { id, name } } else { Event::Unallocated { id, name } });
        }
    }
    // ": YourChar (Inquisitor) is now level 80"
    let rest = msg.strip_prefix(": ")?;
    let (who, level) = rest.rsplit_once(" is now level ")?;
    let level = level.parse().ok()?;
    let character = who.rsplit_once(" (").map_or(who, |(name, _)| name).trim().to_string();
    (!character.is_empty() && !character.contains(' ')).then_some(Event::LevelUp { character, level })
}

/// Where the game usually writes Client.txt, most likely first.
pub fn candidates() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    let mut out = Vec::new();
    if let Some(home) = &home {
        if cfg!(target_os = "macos") {
            out.push(home.join("Library/Application Support/Path of Exile/Cache/Logs/Client.txt"));
            out.push(home.join("Library/Application Support/Steam/steamapps/common/Path of Exile/logs/Client.txt"));
        }
        if cfg!(target_os = "linux") {
            out.push(home.join(".local/share/Steam/steamapps/common/Path of Exile/logs/Client.txt"));
            out.push(home.join(".steam/steam/steamapps/common/Path of Exile/logs/Client.txt"));
        }
    }
    if cfg!(target_os = "windows") {
        out.push(PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Path of Exile\logs\Client.txt"));
        out.push(PathBuf::from(r"C:\Program Files (x86)\Grinding Gear Games\Path of Exile\logs\Client.txt"));
    }
    out
}

pub fn find() -> Option<PathBuf> {
    candidates().into_iter().find(|p| p.is_file())
}

/// Follows a log file, handing each new line to `on_line`. Starts at the current
/// end (the Seed covers what came before) and starts over if the file shrinks.
/// Returns when `keep_going` says the watched file changed.
pub fn follow(path: &Path, mut on_line: impl FnMut(&str), keep_going: impl Fn() -> bool) {
    let mut pos = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let mut partial = String::new();
    while keep_going() {
        std::thread::sleep(Duration::from_millis(500));
        let Ok(len) = std::fs::metadata(path).map(|m| m.len()) else { continue };
        if len < pos {
            pos = 0;
            partial.clear();
        }
        if len == pos {
            continue;
        }
        let Ok(mut file) = File::open(path) else { continue };
        if file.seek(SeekFrom::Start(pos)).is_err() {
            continue;
        }
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        while let Ok(n) = reader.read_until(b'\n', &mut buf) {
            if n == 0 {
                break;
            }
            pos += n as u64;
            partial.push_str(&String::from_utf8_lossy(&buf));
            buf.clear();
            // a line the game is still writing waits for its newline
            if partial.ends_with('\n') {
                on_line(partial.trim_end_matches(['\r', '\n']));
                partial.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "2026/11/10 12:06:02 3336199872 e63da844 [INFO Client 54611] ";

    #[test]
    fn reads_allocations() {
        assert_eq!(
            parse(&format!("{HEAD}Successfully allocated passive skill id: life_regeneration1087, name: Life Regeneration")),
            Some(Event::Allocated { id: "life_regeneration1087".into(), name: "Life Regeneration".into() })
        );
        assert_eq!(
            parse(&format!("{HEAD}Successfully unallocated passive skill id: melee_damage42, name: Melee Damage\r")),
            Some(Event::Unallocated { id: "melee_damage42".into(), name: "Melee Damage".into() })
        );
    }

    #[test]
    fn reads_level_ups() {
        assert_eq!(
            parse(&format!("{HEAD}: Some_Char (Inquisitor) is now level 80")),
            Some(Event::LevelUp { character: "Some_Char".into(), level: 80 })
        );
    }

    #[test]
    fn ignores_other_lines() {
        assert_eq!(parse(&format!("{HEAD}: You have entered The Coast.")), None);
        assert_eq!(parse(&format!("{HEAD}@From Trader: is now level 5")), None);
        assert_eq!(parse("***** LOG FILE OPENING *****"), None);
    }

    #[test]
    fn follows_appended_lines() {
        let dir = std::env::temp_dir().join(format!("poe-tracker-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Client.txt");
        std::fs::write(&path, "old line\n").unwrap();
        let writer = {
            let path = path.clone();
            std::thread::spawn(move || {
                use std::io::Write;
                std::thread::sleep(Duration::from_millis(200));
                let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
                f.write_all(b"first\nsec").unwrap();
                f.flush().unwrap();
                std::thread::sleep(Duration::from_millis(700));
                f.write_all(b"ond\n").unwrap();
            })
        };
        let lines = std::sync::Mutex::new(Vec::new());
        let start = std::time::Instant::now();
        follow(&path, |l| lines.lock().unwrap().push(l.to_string()), || start.elapsed() < Duration::from_millis(2200));
        writer.join().unwrap();
        assert_eq!(*lines.lock().unwrap(), vec!["first", "second"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
