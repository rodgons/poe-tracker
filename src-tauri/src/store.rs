// What the app remembers between runs: one JSON file in the OS app config folder.
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
        }
    }
}

pub fn path(config_dir: PathBuf) -> PathBuf {
    config_dir.join("settings.json")
}

pub fn load(path: &PathBuf) -> Saved {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(path: &PathBuf, saved: &Saved) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(saved).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}
