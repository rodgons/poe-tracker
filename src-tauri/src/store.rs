// What the app remembers between runs: one JSON file in the OS app config folder.
use crate::guide;
use crate::keys::Binds;
use crate::pob::Build;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// One Character's place in the Leveling guide, and its route switches.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GuideProgress {
    /// the current Guide step; the step count when the campaign is done
    pub step: usize,
    pub options: guide::Options,
    /// the Character's level, from the log's level-up lines and the items import
    pub level: Option<u32>,
}

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
    /// the same for the Gem panel; None sits beside the gem level-up icons
    pub gem_spot: Option<(f64, f64)>,
    /// the keys the hook listens for
    pub binds: Binds,
    /// the Skill set the player picked in the Gems section; None follows the Tree spec
    pub skill_set: Option<usize>,
    /// where the player dragged the Guide panel; None is bottom-centre, above the skill bar
    pub guide_spot: Option<(f64, f64)>,
    /// each Character's Leveling guide progress, by lowercased name ("" before one is named)
    pub guide: HashMap<String, GuideProgress>,
    /// the Item set the player picked in the Items section; None follows the Tree spec
    pub item_set: Option<usize>,
    /// the Character's league, from the last items import; Trade links search it
    pub league: String,
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
            gem_spot: None,
            binds: Binds::default(),
            skill_set: None,
            guide_spot: None,
            guide: HashMap::new(),
            item_set: None,
            league: String::new(),
        }
    }
}

impl Saved {
    /// The Character's Leveling guide progress, made when it has none yet.
    pub fn guide_mut(&mut self) -> &mut GuideProgress {
        let key = self.character.to_lowercase();
        self.guide.entry(key).or_default()
    }

    pub fn guide_progress(&self) -> GuideProgress {
        self.guide.get(&self.character.to_lowercase()).cloned().unwrap_or_default()
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
