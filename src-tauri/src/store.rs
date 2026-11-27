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
    /// every Loadout. The fields above (`input` to `item_set`, and `league`) are the active
    /// one's, kept up to date as the player goes; its entry here is only a snapshot, refreshed
    /// by `stash` before switching or listing.
    pub loadouts: Vec<Loadout>,
    pub active: usize,
    /// each Character's own state, by lowercased name, as it was when its Loadout was last left;
    /// shared by all of that Character's Loadouts
    pub characters: HashMap<String, CharacterState>,
    /// the app version whose changes the player last saw (the What's new pop-up), empty before
    pub seen_version: String,
}

/// A named pairing of a Character and a Build; one Character may have several.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Loadout {
    /// what the player named it; empty shows a name made from the Character and the Build
    pub name: String,
    pub input: String,
    pub build: Option<Build>,
    pub spec: usize,
    pub skill_set: Option<usize>,
    pub item_set: Option<usize>,
    pub character: String,
    pub account: String,
    /// seconds since 1970 when the player last switched to it, so a Character's latest wins
    pub used: u64,
}

/// What belongs to the Character in the game, not to a Build.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterState {
    pub allocated: Vec<u32>,
    pub league: String,
}

/// A Loadout as the Build section and the tray list it.
#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct LoadoutEntry {
    pub name: String,
    pub character: String,
    /// the followed Tree spec's title, empty with no Build
    pub spec: String,
    pub active: bool,
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
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
            loadouts: Vec::new(),
            active: 0,
            characters: HashMap::new(),
            seen_version: String::new(),
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

    /// Settings from before Loadouts become the first one.
    pub fn ensure_loadout(&mut self) {
        if self.loadouts.is_empty() {
            self.loadouts.push(Loadout { used: now(), ..Loadout::default() });
            self.active = 0;
            self.stash();
        }
        self.active = self.active.min(self.loadouts.len() - 1);
        // every Loadout has a name: ones from before get the one they were shown with
        self.stash();
        for l in &mut self.loadouts {
            if l.name.trim().is_empty() {
                l.name = loadout_name(l);
            }
        }
    }

    /// The Characters the app already tracks points for, lowercased.
    pub fn known_characters(&self) -> Vec<String> {
        let mut known: Vec<String> = self.characters.keys().cloned().collect();
        if !self.character.is_empty() && !known.contains(&self.character.to_lowercase()) {
            known.push(self.character.to_lowercase());
        }
        known.sort();
        known
    }

    /// The points the app tracks for `character`, if it tracks any.
    pub fn tracked(&self, character: &str) -> Option<CharacterState> {
        if !self.character.is_empty() && self.character.eq_ignore_ascii_case(character) {
            return Some(CharacterState { allocated: self.allocated.clone(), league: self.league.clone() });
        }
        self.characters.get(&character.to_lowercase()).cloned()
    }

    /// Writes the active Loadout's fields, and its Character's state, back to their snapshots.
    pub fn stash(&mut self) {
        let Some(l) = self.loadouts.get_mut(self.active) else { return };
        l.input = self.input.clone();
        l.build = self.build.clone();
        l.spec = self.spec;
        l.skill_set = self.skill_set;
        l.item_set = self.item_set;
        l.character = self.character.clone();
        l.account = self.account.clone();
        if !self.character.is_empty() {
            let c = CharacterState { allocated: self.allocated.clone(), league: self.league.clone() };
            self.characters.insert(self.character.to_lowercase(), c);
        }
    }

    /// Makes Loadout `i` the active one; its Character's state comes along.
    pub fn switch_to(&mut self, i: usize) -> Result<(), String> {
        if i >= self.loadouts.len() {
            return Err("No such loadout.".into());
        }
        self.stash();
        let l = &mut self.loadouts[i];
        l.used = now();
        let l = l.clone();
        let c = self.characters.get(&l.character.to_lowercase()).cloned().unwrap_or_default();
        self.input = l.input;
        self.build = l.build;
        self.spec = l.spec;
        self.skill_set = l.skill_set;
        self.item_set = l.item_set;
        self.character = l.character;
        self.account = l.account;
        self.allocated = c.allocated;
        self.league = c.league;
        self.active = i;
        Ok(())
    }

    /// A new Loadout, switched to: a copy of the active one, or the same Character with no Build yet.
    pub fn new_loadout(&mut self, copy: bool) {
        self.stash();
        let mut l = if copy {
            self.loadouts[self.active].clone()
        } else {
            let cur = &self.loadouts[self.active];
            Loadout { character: cur.character.clone(), account: cur.account.clone(), ..Loadout::default() }
        };
        if copy && !l.name.is_empty() {
            l.name = format!("{} (copy)", l.name);
        }
        self.loadouts.push(l);
        let _ = self.switch_to(self.loadouts.len() - 1);
    }

    pub fn rename_loadout(&mut self, i: usize, name: &str) -> Result<(), String> {
        let l = self.loadouts.get_mut(i).ok_or("No such loadout.")?;
        l.name = name.trim().to_string();
        Ok(())
    }

    /// Deletes Loadout `i`, never the last one; deleting the active one switches to the latest used.
    pub fn delete_loadout(&mut self, i: usize) -> Result<(), String> {
        if i >= self.loadouts.len() {
            return Err("No such loadout.".into());
        }
        if self.loadouts.len() == 1 {
            return Err("This is your only loadout.".into());
        }
        if i == self.active {
            let next = (0..self.loadouts.len()).filter(|&j| j != i).max_by_key(|&j| self.loadouts[j].used).unwrap();
            self.switch_to(next)?;
        }
        self.stash();
        self.loadouts.remove(i);
        if self.active > i {
            self.active -= 1;
        }
        Ok(())
    }

    /// The Character's most recently used Loadout other than the active one.
    pub fn latest_for(&self, character: &str) -> Option<usize> {
        (0..self.loadouts.len())
            .filter(|&i| i != self.active && self.loadouts[i].character.eq_ignore_ascii_case(character))
            .max_by_key(|&i| self.loadouts[i].used)
    }

    pub fn loadout_list(&mut self) -> Vec<LoadoutEntry> {
        self.stash();
        let active = self.active;
        self.loadouts
            .iter()
            .enumerate()
            .map(|(i, l)| LoadoutEntry {
                name: loadout_name(l),
                character: l.character.clone(),
                spec: l.build.as_ref().and_then(|b| b.specs.get(l.spec)).map(|s| s.title.clone()).unwrap_or_default(),
                active: i == active,
            })
            .collect()
    }
}

/// The Loadout's own name, or "Character · Build" when the player hasn't named it.
pub fn loadout_name(l: &Loadout) -> String {
    if !l.name.is_empty() {
        return l.name.clone();
    }
    let build = l.build.as_ref().and_then(|b| b.specs.get(l.spec)).map(|s| s.title.clone());
    match (l.character.as_str(), build) {
        ("", None) => "New loadout".into(),
        ("", Some(b)) => b,
        (c, None) => format!("{c} · no build yet"),
        (c, Some(b)) => format!("{c} · {b}"),
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

    fn named(c: &str, allocated: &[u32]) -> Saved {
        let mut s = Saved::default();
        s.character = c.into();
        s.input = "build-a".into();
        s.allocated = allocated.to_vec();
        s.league = "Settlers".into();
        s.ensure_loadout();
        s
    }

    #[test]
    fn old_settings_become_the_first_loadout() {
        let mut s = named("Shiefire", &[1, 2]);
        assert_eq!(s.loadouts.len(), 1);
        assert_eq!(s.loadouts[0].input, "build-a");
        assert_eq!(s.loadout_list(), [LoadoutEntry { name: "Shiefire · no build yet".into(), character: "Shiefire".into(), spec: String::new(), active: true }]);
        assert_eq!(s.loadouts[0].name, "Shiefire · no build yet");
    }

    #[test]
    fn loadouts_of_one_character_share_its_tree_and_league() {
        let mut s = named("Shiefire", &[1, 2]);
        s.new_loadout(false);
        assert_eq!((s.character.as_str(), s.input.as_str()), ("Shiefire", ""));
        assert_eq!(s.allocated, [1, 2]);
        assert_eq!(s.league, "Settlers");
        // ticking on the second one shows on the first
        s.allocated.push(3);
        s.input = "build-b".into();
        s.switch_to(0).unwrap();
        assert_eq!(s.allocated, [1, 2, 3]);
        assert_eq!(s.input, "build-a");
        s.switch_to(1).unwrap();
        assert_eq!(s.input, "build-b");
    }

    #[test]
    fn tracked_points_come_from_the_active_character_or_the_stash() {
        let mut s = named("Shiefire", &[1, 2]);
        s.new_loadout(false);
        s.character = "Other".into();
        s.allocated = vec![9];
        assert_eq!(s.tracked("shiefire").unwrap().allocated, [1, 2]);
        assert_eq!(s.tracked("OTHER").unwrap().allocated, [9]);
        assert!(s.tracked("nobody").is_none());
        assert_eq!(s.known_characters(), ["other", "shiefire"]);
    }

    #[test]
    fn another_character_brings_its_own_tree() {
        let mut s = named("Shiefire", &[1, 2]);
        s.new_loadout(false);
        s.character = "Other".into();
        s.allocated = vec![9];
        s.league.clear();
        s.switch_to(0).unwrap();
        assert_eq!((s.character.as_str(), s.allocated.as_slice()), ("Shiefire", &[1, 2][..]));
        s.switch_to(1).unwrap();
        assert_eq!((s.character.as_str(), s.allocated.as_slice()), ("Other", &[9][..]));
    }

    #[test]
    fn the_latest_used_loadout_of_a_character_is_found_and_deleting_keeps_one() {
        let mut s = named("Shiefire", &[1]);
        s.new_loadout(true);
        s.new_loadout(false);
        s.character = "Other".into();
        s.loadouts[0].used = 10;
        s.loadouts[1].used = 20;
        assert_eq!(s.latest_for("shiefire"), Some(1));
        assert_eq!(s.latest_for("nobody"), None);
        s.delete_loadout(2).unwrap();
        assert_eq!((s.active, s.character.as_str()), (1, "Shiefire"));
        s.delete_loadout(0).unwrap();
        assert_eq!(s.active, 0);
        assert!(s.delete_loadout(0).is_err());
    }
}
