// Zone layouts: the known arrangements of the campaign zones that have them, as small line
// drawings with markers (bundled by scripts/extract_layouts.py from Exile-UI's Act-Decoder).
// Each zone is a pick tree: the player picks the start picture that matches their minimap,
// down to a full layout. The Guide panel draws Directions on it for the current Guide step.
use crate::guide::Step;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

struct Bundle {
    zones: HashMap<String, Value>,
}

fn bundle() -> &'static Bundle {
    static B: OnceLock<Bundle> = OnceLock::new();
    B.get_or_init(|| {
        let mut v: Value = serde_json::from_str(include_str!("../data/layouts.json")).expect("bundled layouts");
        let zones = serde_json::from_value(v["zones"].take()).expect("bundled layout zones");
        Bundle { zones }
    })
}

/// The player's pick in each zone, kept only for the instance it was made in.
#[derive(Default)]
pub struct Picks(HashMap<String, (u64, String)>);

impl Picks {
    /// The Character entered `area`: a different seed is a new instance, so the old pick goes.
    pub fn entered(&mut self, area: &str, seed: u64) {
        if self.0.get(area).is_some_and(|(s, _)| *s != seed) {
            self.0.remove(area);
        }
    }

    pub fn set(&mut self, area: &str, seed: u64, pick: Option<String>) {
        match pick {
            Some(p) => self.0.insert(area.to_string(), (seed, p)),
            None => self.0.remove(area),
        };
    }

    pub fn get(&self, area: &str, seed: u64) -> Option<&str> {
        self.0.get(area).filter(|(s, _)| *s == seed).map(|(_, p)| p.as_str())
    }
}

/// What the Guide panel needs to pick a Zone layout and draw Directions for the current step.
#[derive(Serialize, Debug)]
pub struct LayoutView {
    pub area: String,
    /// the zone's pick tree: nodes with `id`, `lines`, `clues`, and `children` or `marks`
    pub nodes: &'static Value,
    /// the pick so far ("2", "2_1"), None before the first
    pub pick: Option<String>,
    /// the names of the places the step sends the player to, as its kill, quest item and arena fragments write them
    pub objectives: Vec<String>,
    /// the area the step ends by entering, and its name
    pub exit: Option<String>,
    pub exit_name: Option<String>,
    /// the step gets the waypoint, so the Directions pass it
    pub get_waypoint: bool,
    /// the Character came by waypoint, so the Directions start there
    pub by_waypoint: bool,
}

/// The layout view for `area` and the step the Character is on there, if the zone has Zone layouts.
pub fn view(area: &str, area_name: &str, step: &Step, pick: Option<&str>) -> Option<LayoutView> {
    let nodes = bundle().zones.get(area)?;
    let parts = || step.lines.iter().flat_map(|l| &l.parts);
    let mut objectives: Vec<String> = Vec::new();
    for p in parts().filter(|p| matches!(p.kind.as_str(), "kill" | "quest_text" | "arena")) {
        if !objectives.contains(&p.text) {
            objectives.push(p.text.clone());
        }
    }
    Some(LayoutView {
        area: area.to_string(),
        nodes,
        pick: pick.map(str::to_string),
        objectives,
        exit: step.to.clone(),
        exit_name: step.to_name.clone(),
        get_waypoint: parts().any(|p| p.kind == "waypoint_get"),
        by_waypoint: parts().any(|p| p.kind == "waypoint" && p.text == area_name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guide;

    #[test]
    fn a_pick_lasts_for_its_instance_only() {
        let mut picks = Picks::default();
        picks.set("1_1_3", 7, Some("2_1".into()));
        picks.entered("1_1_3", 7);
        assert_eq!(picks.get("1_1_3", 7), Some("2_1"));
        // a town trip and back to the same instance keeps it
        picks.entered("1_1_town", 1);
        assert_eq!(picks.get("1_1_3", 7), Some("2_1"));
        picks.entered("1_1_3", 8);
        assert_eq!(picks.get("1_1_3", 8), None);
        assert_eq!(picks.get("1_1_3", 7), None);
    }

    #[test]
    fn every_bundled_zone_is_a_pick_tree_of_marked_layouts() {
        fn walk(n: &Value, zone: &str) {
            assert!(n["id"].is_string() && n["lines"].is_array(), "{zone}: {n}");
            match n.get("children") {
                Some(kids) => kids.as_array().unwrap().iter().for_each(|k| walk(k, zone)),
                None => assert!(n["marks"].is_array(), "{zone} {}: a layout without marks", n["id"]),
            }
        }
        assert!(!bundle().zones.is_empty());
        for (zone, nodes) in &bundle().zones {
            assert!(guide::route().areas.contains_key(zone), "{zone} is not a route area");
            nodes.as_array().unwrap().iter().for_each(|n| walk(n, zone));
        }
    }

    #[test]
    fn the_mud_flats_step_asks_for_the_glyphs_and_heads_for_the_submerged_passage() {
        let route = guide::route();
        let steps = guide::steps(route, &guide::Options::default());
        let step = steps.iter().find(|s| s.to.as_deref() == Some("1_1_4_1")).expect("a step entering The Submerged Passage");
        let v = view("1_1_3", "The Mud Flats", step, None).expect("The Mud Flats has Zone layouts");
        assert_eq!(v.objectives, ["Glyph"]);
        assert_eq!(v.exit.as_deref(), Some("1_1_4_1"));
        assert!(!v.by_waypoint);
    }
}
