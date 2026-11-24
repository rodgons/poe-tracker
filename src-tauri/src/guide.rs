// The Leveling guide: exile-leveling's campaign route (bundled by scripts/extract_route.py),
// cut into Guide steps that each end with the area to enter next. Entering that area, or a
// later step's area, moves the guide on; nothing moves it back except the player.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

/// One piece of a route line: plain text, or a fragment like ["enter", "1_1_2"].
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
enum RawPart {
    Text(String),
    Fragment(Vec<String>),
}

#[derive(Deserialize)]
struct RawLine {
    act: u8,
    /// the innermost #ifdef (true) or #ifndef (false) around the line
    when: Option<(String, bool)>,
    parts: Vec<RawPart>,
    subs: Vec<Vec<RawPart>>,
}

#[derive(Deserialize)]
pub struct Area {
    pub name: String,
}

#[derive(Deserialize)]
struct Quest {
    name: String,
}

#[derive(Deserialize)]
pub struct Route {
    /// the exile-leveling commit the route came from
    pub source: String,
    lines: Vec<RawLine>,
    pub areas: HashMap<String, Area>,
    quests: HashMap<String, Quest>,
}

pub fn route() -> &'static Route {
    static ROUTE: OnceLock<Route> = OnceLock::new();
    ROUTE.get_or_init(|| serde_json::from_str(include_str!("../data/route.json")).expect("bundled route"))
}

/// Which Act 2 bandit the player helps, or none (kill them all).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Bandit {
    #[default]
    Kill,
    Oak,
    Kraityn,
    Alira,
}

/// The route's switches, kept per Character.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// a fresh character in a new league: trials, extra waypoints and side content
    pub league_start: bool,
    pub bandit: Bandit,
}

impl Default for Options {
    fn default() -> Self {
        Options { league_start: true, bandit: Bandit::Kill }
    }
}

impl Options {
    fn defines(&self, symbol: &str) -> bool {
        match symbol {
            "LEAGUE_START" => self.league_start,
            // exile-leveling's library option is on unless a build turns it off; we keep it on
            "LIBRARY" => true,
            "BANDIT_KILL" => self.bandit == Bandit::Kill,
            "BANDIT_OAK" => self.bandit == Bandit::Oak,
            "BANDIT_KRAITYN" => self.bandit == Bandit::Kraityn,
            "BANDIT_ALIRA" => self.bandit == Bandit::Alira,
            _ => false,
        }
    }
}

/// One piece of a line as the panels show it. `kind` is the fragment kind ("enter",
/// "kill", "quest", "dir"…) or "text"; `text` is what to print (an area or quest name
/// for ids, the degrees for "dir").
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Part {
    pub kind: String,
    pub text: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Line {
    pub parts: Vec<Part>,
    /// hints under the line
    pub subs: Vec<Vec<Part>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Step {
    pub act: u8,
    pub lines: Vec<Line>,
    /// the area id whose entry completes the step; None for the route's last lines
    pub to: Option<String>,
    pub to_name: Option<String>,
}

fn part(route: &Route, raw: &RawPart) -> Part {
    let area = |id: &str| route.areas.get(id).map_or_else(|| id.to_string(), |a| a.name.clone());
    let (kind, text) = match raw {
        RawPart::Text(t) => ("text".to_string(), t.clone()),
        RawPart::Fragment(f) => {
            let kind = f[0].clone();
            let arg = f.get(1).map(String::as_str);
            let text = match (kind.as_str(), arg) {
                ("enter" | "area", Some(id)) | ("waypoint", Some(id)) => area(id),
                ("waypoint" | "waypoint_get", _) => "Waypoint".into(),
                ("quest", Some(id)) => route.quests.get(id).map_or_else(|| id.to_string(), |q| q.name.clone()),
                ("logout", _) => "Logout".into(),
                ("portal", Some("set")) => "Set a portal".into(),
                ("portal", _) => "Take the portal".into(),
                ("trial", _) => "Trial of Ascendancy".into(),
                ("ascend", Some(lab)) => {
                    let lab = match lab {
                        "normal" => "the Labyrinth",
                        "cruel" => "the Cruel Labyrinth",
                        "merciless" => "the Merciless Labyrinth",
                        other => other,
                    };
                    format!("Ascend in {lab}")
                }
                ("crafting", _) => "Crafting recipe".into(),
                (_, Some(arg)) => arg.to_string(),
                (_, None) => kind.clone(),
            };
            (kind, text)
        }
    };
    Part { kind, text }
}

/// The route for these switches, in Guide steps. A step runs up to and including the
/// line that enters the next area.
pub fn steps(route: &Route, options: &Options) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut lines = Vec::new();
    let mut act = 1;
    for raw in &route.lines {
        if let Some((symbol, wanted)) = &raw.when {
            if options.defines(symbol) != *wanted {
                continue;
            }
        }
        act = raw.act;
        let enters = raw.parts.iter().rev().find_map(|p| match p {
            RawPart::Fragment(f) if f[0] == "enter" => f.get(1).cloned(),
            _ => None,
        });
        lines.push(Line {
            parts: raw.parts.iter().map(|p| part(route, p)).collect(),
            subs: raw.subs.iter().map(|s| s.iter().map(|p| part(route, p)).collect()).collect(),
        });
        if let Some(to) = enters {
            let to_name = route.areas.get(&to).map(|a| a.name.clone());
            steps.push(Step { act, lines: std::mem::take(&mut lines), to: Some(to), to_name });
        }
    }
    if !lines.is_empty() {
        steps.push(Step { act, lines, to: None, to_name: None });
    }
    steps
}

/// Where the guide goes when the Character enters `area` while on step `current`: past
/// the nearest step from `current` on whose target it is, counting the steps between as
/// done. Any other area (a town, the hideout, an earlier zone) leaves it where it is.
/// `steps.len()` means the campaign is done.
pub fn advance(steps: &[Step], current: usize, area: &str) -> usize {
    steps
        .iter()
        .enumerate()
        .skip(current)
        .find(|(_, s)| s.to.as_deref() == Some(area))
        .map_or(current, |(i, _)| i + 1)
}

/// The step in `after` matching step `current` of `before`, when the switches change and
/// steps come or go: the same target area (the same occurrence of it, for areas entered
/// twice), or the next step's that still exists.
pub fn same_place(before: &[Step], current: usize, after: &[Step]) -> usize {
    for (i, step) in before.iter().enumerate().skip(current) {
        let Some(to) = step.to.as_deref() else { continue };
        let nth = before[..i].iter().filter(|s| s.to.as_deref() == Some(to)).count();
        if let Some((j, _)) = after.iter().enumerate().filter(|(_, s)| s.to.as_deref() == Some(to)).nth(nth) {
            return j;
        }
    }
    // the last lines (no target) or the campaign done
    after.len() - (before.len() - current).min(after.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(steps: &[Step]) -> Vec<Option<&str>> {
        steps.iter().map(|s| s.to.as_deref()).collect()
    }

    #[test]
    fn every_step_ends_in_a_known_area() {
        let r = route();
        let steps = steps(r, &Options::default());
        assert!(steps.len() > 100, "the whole campaign: {}", steps.len());
        assert_eq!(steps[0].to.as_deref(), Some("1_1_town"));
        assert_eq!(steps[0].to_name.as_deref(), Some("Lioneye's Watch"));
        assert_eq!(steps[0].act, 1);
        for s in &steps[..steps.len() - 1] {
            let to = s.to.as_deref().unwrap();
            assert!(r.areas.contains_key(to), "{to}");
        }
        assert_eq!(steps.last().unwrap().act, 10);
        assert!(r.source.contains("exile-leveling"));
    }

    #[test]
    fn a_step_shows_names_not_ids() {
        let steps = steps(route(), &Options::default());
        let first = &steps[0].lines;
        assert_eq!(first[0].parts, vec![
            Part { kind: "text".into(), text: "Find and kill ".into() },
            Part { kind: "kill".into(), text: "Hillock".into() },
        ]);
        assert_eq!(first.last().unwrap().parts[1], Part { kind: "enter".into(), text: "Lioneye's Watch".into() });
        let quest = &steps[1].lines[0].parts[1];
        assert_eq!((quest.kind.as_str(), quest.text.as_str()), ("quest", "Enemy at the Gate"));
    }

    #[test]
    fn league_start_adds_side_content() {
        let r = route();
        let full = steps(r, &Options::default());
        let short = steps(r, &Options { league_start: false, ..Options::default() });
        assert!(ids(&full).contains(&Some("1_1_2a")), "The Tidal Island at league start");
        assert!(!ids(&short).contains(&Some("1_1_2a")));
        assert!(short.len() < full.len());
    }

    #[test]
    fn the_bandit_choice_picks_one_act_2_route() {
        let r = route();
        let text = |o: Options| -> String {
            steps(r, &o).iter().filter(|s| s.act == 2).flat_map(|s| &s.lines).flat_map(|l| &l.parts).map(|p| p.text.clone()).collect()
        };
        let kill = text(Options::default());
        let oak = text(Options { bandit: Bandit::Oak, ..Options::default() });
        assert_ne!(kill, oak);
        assert!(kill.contains("Oak") && kill.contains("Kraityn") && kill.contains("Alira"));
    }

    fn mini() -> Vec<Step> {
        ["a", "b", "town", "c", "b", "d"]
            .iter()
            .map(|to| Step { act: 1, lines: Vec::new(), to: Some(to.to_string()), to_name: None })
            .collect()
    }

    #[test]
    fn entering_the_target_moves_to_the_next_step() {
        assert_eq!(advance(&mini(), 0, "a"), 1);
        assert_eq!(advance(&mini(), 1, "b"), 2);
    }

    #[test]
    fn a_later_area_skips_ahead_and_nothing_moves_back() {
        // skipping c's step: entering d jumps past it
        assert_eq!(advance(&mini(), 1, "d"), 6);
        // back in an earlier zone: stays put
        assert_eq!(advance(&mini(), 3, "a"), 3);
        // b is entered twice; from step 3 on only the later b counts
        assert_eq!(advance(&mini(), 3, "b"), 5);
        // an area the route never enters (the hideout)
        assert_eq!(advance(&mini(), 2, "HideoutFelled"), 2);
        // the campaign done
        assert_eq!(advance(&mini(), 6, "d"), 6);
    }

    #[test]
    fn changing_the_switches_keeps_the_place() {
        let r = route();
        let full = steps(r, &Options::default());
        let short = steps(r, &Options { league_start: false, ..Options::default() });
        // on the step heading to The Ledge, with or without league-start side content
        let ledge = |s: &[Step]| s.iter().position(|s| s.to.as_deref() == Some("1_1_5")).unwrap();
        assert_eq!(same_place(&full, ledge(&full), &short), ledge(&short));
        assert_eq!(same_place(&short, ledge(&short), &full), ledge(&full));
        // the step to The Tidal Island exists only at league start: the next one that's left
        let tidal = full.iter().position(|s| s.to.as_deref() == Some("1_1_2a")).unwrap();
        let landed = same_place(&full, tidal, &short);
        assert!(short[landed].act == 1 && landed <= ledge(&short));
        // done stays done
        assert_eq!(same_place(&full, full.len(), &short), short.len());
    }

    #[test]
    fn a_new_character_catches_up_on_its_first_campaign_area() {
        // no progress yet: the first area entered lands right after its first step
        assert_eq!(advance(&mini(), 0, "c"), 4);
    }
}
