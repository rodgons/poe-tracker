// What the app remembers between runs: one JSON file in the OS app config folder.
use crate::keys::Binds;
use crate::pob::Build;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    /// what the player pasted: a build code or a link
    pub input: String,
    pub build: Option<Build>,
    pub spec: usize,
    pub character: String,
    /// pathofexile.com account (Name#1234) for the character import; empty sets points by hand
    pub account: String,
    pub allocated: Vec<u32>,
    /// how many Next points the panel shows
    pub points: usize,
    /// Client.txt the player picked; None looks in the usual install folders
    pub log_path: Option<PathBuf>,
    /// where the player dragged the Corner panel inside the game window, (0,0)–(1,1); None is top-right
    pub panel_spot: Option<(f64, f64)>,
    /// the keys the hook listens for
    pub binds: Binds,
}

impl Default for Saved {
    fn default() -> Self {
        Saved {
            input: String::new(),
            build: None,
            spec: 0,
            character: String::new(),
            account: String::new(),
            allocated: Vec::new(),
            points: 5,
            log_path: None,
            panel_spot: None,
            binds: Binds::default(),
        }
    }
}

pub fn path(config_dir: PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(path: &PathBuf) -> Saved {
    let Ok(text) = std::fs::read_to_string(path) else { return Saved::default() };
    serde_json::from_str(&text).unwrap_or_else(|_| {
        // set aside what couldn't be read, so the next save doesn't wipe it
        let _ = std::fs::rename(path, path.with_extension("json.bad"));
        Saved::default()
    })
}

pub fn save(path: &PathBuf, saved: &Saved) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(saved).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_an_unreadable_file_aside() {
        let dir = std::env::temp_dir().join(format!("poe-tracker-store-{}", std::process::id()));
        let file = path(dir.clone());
        let mut saved = Saved::default();
        saved.character = "shiefire".into();
        save(&file, &saved).unwrap();
        assert_eq!(load(&file).character, "shiefire");
        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(load(&file).character, "");
        assert_eq!(std::fs::read_to_string(file.with_extension("json.bad")).unwrap(), "{ not json");
        let _ = std::fs::remove_dir_all(dir);
    }
}
