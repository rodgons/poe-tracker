// The Leveling guide: exile-leveling's campaign route (bundled by scripts/extract_route.py),
// cut into Guide steps that each end with the area to enter next. Entering that area, or a
// later step's area, moves the guide on; nothing moves it back except the player.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    /// the monster level, which sets the experience penalty
    pub level: u32,
    pub town: bool,
}

/// What one way of finishing a quest offers: a gem to take as the reward (by the
/// classes that may take it) and gems the NPC sells from then on.
#[derive(Deserialize)]
struct Offer {
    npc: String,
    take: HashMap<String, Vec<String>>,
    buy: HashMap<String, Buy>,
}

#[derive(Deserialize)]
struct Buy {
    classes: Vec<String>,
    npc: String,
}

#[derive(Deserialize)]
struct Quest {
    name: String,
    /// in the game's order, which decides the gem a hand-in gives
    offers: Vec<(String, Offer)>,
}

#[derive(Deserialize)]
struct GemInfo {
    name: String,
}

#[derive(Deserialize)]
pub struct Route {
    /// the exile-leveling commit the route came from
    pub source: String,
    lines: Vec<RawLine>,
    pub areas: HashMap<String, Area>,
    quests: HashMap<String, Quest>,
    gems: HashMap<String, GemInfo>,
    /// Path of Building gem ids that differ from the game's
    remap: HashMap<String, String>,
    /// Vaal and Awakened gems → the plain gem a quest gives
    vaal: HashMap<String, String>,
    awakened: HashMap<String, String>,
    /// each class's starting gem and first-chest gem
    characters: HashMap<String, (String, String)>,
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
    /// the Build's gems to take or buy at this line's quest hand-ins
    pub gems: Vec<GemPick>,
    /// the quests handed in on this line, with the reward offers they count
    #[serde(skip)]
    quests: Vec<(String, Vec<String>)>,
}

/// A gem of the Build to collect at a quest hand-in.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GemPick {
    /// the name to search for in the game ("Added Fire Damage Support")
    pub name: String,
    /// bought from the NPC rather than taken as the quest reward
    pub buy: bool,
    pub npc: String,
    /// the Character already has it in a worn item
    pub socketed: bool,
}

/// What the gem lines need to know about the Build and the Character.
pub struct GemPlan {
    /// the Character's base class ("Witch")
    pub class: String,
    /// the Build's gems as quest-reward ids, in the Build's order, each once
    pub wanted: Vec<String>,
    /// lowercased names of the gems in the Character's worn items
    pub socketed: HashSet<String>,
}

impl Route {
    /// The quest-reward id for a Path of Building gem: PoB's id put right, Vaal and
    /// Awakened gems as the gem a quest gives. Builds saved before gem ids were kept
    /// go by name.
    pub fn reward_id(&self, pob_id: &str, name: &str) -> Option<String> {
        if pob_id.is_empty() {
            let support = format!("{name} Support");
            return self.gems.iter().find(|(_, g)| g.name == name || g.name == support).map(|(id, _)| id.clone());
        }
        let mut id = self.remap.get(pob_id).map_or(pob_id, String::as_str);
        id = self.vaal.get(id).map_or(id, String::as_str);
        id = self.awakened.get(id).map_or(id, String::as_str);
        Some(id.to_string())
    }
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
        let quests = raw
            .parts
            .iter()
            .filter_map(|p| match p {
                RawPart::Fragment(f) if f[0] == "quest" && f.len() > 1 => Some((f[1].clone(), f[2..].to_vec())),
                _ => None,
            })
            .collect();
        lines.push(Line {
            parts: raw.parts.iter().map(|p| part(route, p)).collect(),
            subs: raw.subs.iter().map(|s| s.iter().map(|p| part(route, p)).collect()).collect(),
            gems: Vec::new(),
            quests,
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

/// Fills in each line's gems to take or buy, as exile-leveling does: walking the route
/// in order, each hand-in gives the first gem of the Build its class may take that it
/// isn't getting already, and its NPCs sell every gem of the Build not taken or bought
/// before. The class's starting gems are never picked as rewards.
pub fn pick_gems(route: &Route, steps: &mut [Step], plan: &GemPlan) {
    let allowed = |classes: &[String]| classes.is_empty() || classes.contains(&plan.class);
    let mut taken: HashSet<&str> = HashSet::new();
    if let Some((start, chest)) = route.characters.get(&plan.class) {
        taken.extend([start.as_str(), chest.as_str()]);
    }
    let mut bought: HashSet<&str> = HashSet::new();
    let pick = |id: &str, buy: bool, npc: &str| {
        let name = route.gems.get(id).map_or_else(|| id.to_string(), |g| g.name.clone());
        let socketed = plan.socketed.contains(&name.to_lowercase());
        GemPick { name, buy, npc: npc.to_string(), socketed }
    };
    for line in steps.iter_mut().flat_map(|s| s.lines.iter_mut()) {
        for (quest_id, chosen) in &line.quests {
            let Some(quest) = route.quests.get(quest_id) else { continue };
            let offers: Vec<&Offer> =
                quest.offers.iter().filter(|(id, _)| chosen.is_empty() || chosen.contains(id)).map(|(_, o)| o).collect();
            for offer in &offers {
                let reward = plan.wanted.iter().find(|id| {
                    !taken.contains(id.as_str()) && offer.take.get(id.as_str()).is_some_and(|c| allowed(c))
                });
                if let Some(id) = reward {
                    taken.insert(id);
                    line.gems.push(pick(id, false, &offer.npc));
                }
            }
            for offer in &offers {
                for id in &plan.wanted {
                    if taken.contains(id.as_str()) || bought.contains(id.as_str()) {
                        continue;
                    }
                    if let Some(buy) = offer.buy.get(id).filter(|b| allowed(&b.classes)) {
                        bought.insert(id);
                        line.gems.push(pick(id, true, &buy.npc));
                    }
                }
            }
        }
    }
}

/// The area levels around the Character's `level` that give full experience.
pub fn full_exp(level: u32) -> (u32, u32) {
    let margin = 3 + level / 16;
    (level.saturating_sub(margin).max(1), level + margin)
}

impl Route {
    /// Towns and hideouts: places the player goes back to between zones.
    pub fn is_town(&self, area: &str) -> bool {
        self.areas.get(area).is_some_and(|a| a.town) || area.starts_with("Hideout")
    }
}

/// Where the guide goes when the Character enters `area` (a `town` or not) while on step
/// `current`. The current step's target moves it to the next step; a later step's target
/// skips ahead past it, counting the steps between as done. A town, or a zone some earlier
/// step already entered, only counts as the current step's target: the route sends the
/// player back to town to hand in quests and back through zones it has passed, and those
/// trips mustn't skip to the step that enters them again later. Anywhere else leaves it
/// where it is. `steps.len()` means the campaign is done.
pub fn advance(steps: &[Step], current: usize, area: &str, town: bool) -> usize {
    let targets = |s: &Step| s.to.as_deref() == Some(area);
    if steps.get(current).is_some_and(targets) {
        return current + 1;
    }
    if town || steps[..current.min(steps.len())].iter().any(targets) {
        return current;
    }
    steps.iter().enumerate().skip(current).find(|(_, s)| targets(s)).map_or(current, |(i, _)| i + 1)
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

    fn plan(class: &str, names: &[&str]) -> GemPlan {
        let r = route();
        GemPlan {
            class: class.into(),
            wanted: names.iter().map(|n| r.reward_id("", n).unwrap_or_else(|| panic!("{n}"))).collect(),
            socketed: HashSet::new(),
        }
    }

    fn picks(class: &str, names: &[&str]) -> Vec<(u8, String, bool, String)> {
        let r = route();
        let mut s = steps(r, &Options::default());
        pick_gems(r, &mut s, &plan(class, names));
        s.iter().flat_map(|s| s.lines.iter().flat_map(move |l| l.gems.iter().map(move |g| (s.act, g.name.clone(), g.buy, g.npc.clone())))).collect()
    }

    #[test]
    fn the_first_hand_in_gives_the_builds_gem() {
        let got = picks("Witch", &["Freezing Pulse", "Frostblink"]);
        assert_eq!(got[0], (1, "Freezing Pulse".into(), false, "Tarkleigh".into()));
        // each gem once: taken or bought, never both
        let names: Vec<_> = got.iter().map(|g| g.1.as_str()).collect();
        assert_eq!(names.iter().filter(|n| **n == "Freezing Pulse").count(), 1);
        assert_eq!(names.iter().filter(|n| **n == "Frostblink").count(), 1);
    }

    #[test]
    fn a_gem_the_class_cant_take_is_bought() {
        // Marauders can't take Freezing Pulse from Enemy at the Gate
        let got = picks("Marauder", &["Freezing Pulse"]);
        assert_eq!(got.len(), 1);
        assert!(got[0].2, "bought: {got:?}");
    }

    #[test]
    fn the_starting_gem_is_never_picked_as_a_reward() {
        let got = picks("Witch", &["Fireball"]);
        assert!(got.iter().all(|g| g.2), "Fireball only bought, if at all: {got:?}");
    }

    #[test]
    fn socketed_gems_are_marked() {
        let r = route();
        let mut s = steps(r, &Options::default());
        let mut p = plan("Witch", &["Freezing Pulse"]);
        p.socketed.insert("freezing pulse".into());
        pick_gems(r, &mut s, &p);
        assert!(s.iter().flat_map(|s| &s.lines).flat_map(|l| &l.gems).all(|g| g.socketed));
    }

    #[test]
    fn pob_ids_are_put_right() {
        let r = route();
        assert_eq!(r.reward_id("Metadata/Items/Gems/SkillGemVaalFireball", "Vaal Fireball").as_deref(), Some("Metadata/Items/Gems/SkillGemFireball"));
        assert_eq!(r.reward_id("Metadata/Items/Gems/Smite", "Smite").as_deref(), Some("Metadata/Items/Gems/SkillGemSmite"));
        assert_eq!(r.reward_id("", "Added Fire Damage").as_deref(), Some("Metadata/Items/Gems/SupportGemAddedFireDamage"));
    }

    #[test]
    fn full_experience_widens_with_level() {
        assert_eq!(full_exp(1), (1, 4));
        assert_eq!(full_exp(10), (7, 13));
        assert_eq!(full_exp(32), (27, 37));
        assert_eq!(full_exp(68), (61, 75));
    }

    fn mini() -> Vec<Step> {
        ["a", "b", "town", "c", "b", "d"]
            .iter()
            .map(|to| Step { act: 1, lines: Vec::new(), to: Some(to.to_string()), to_name: None })
            .collect()
    }

    #[test]
    fn entering_the_target_moves_to_the_next_step() {
        assert_eq!(advance(&mini(), 0, "a", false), 1);
        assert_eq!(advance(&mini(), 1, "b", false), 2);
        assert_eq!(advance(&mini(), 2, "town", true), 3);
    }

    #[test]
    fn a_later_area_skips_ahead_and_nothing_moves_back() {
        // skipping c's step: entering d jumps past it
        assert_eq!(advance(&mini(), 1, "d", false), 6);
        // back in an earlier zone: stays put
        assert_eq!(advance(&mini(), 3, "a", false), 3);
        // b is entered twice: going back to it before c stays put, the second time counts on its own step
        assert_eq!(advance(&mini(), 3, "b", false), 3);
        assert_eq!(advance(&mini(), 4, "b", false), 5);
        // a trip to town never skips to the step that enters it later
        assert_eq!(advance(&mini(), 1, "town", true), 1);
        // an area the route never enters (the hideout)
        assert_eq!(advance(&mini(), 2, "HideoutFelled", true), 2);
        // the campaign done
        assert_eq!(advance(&mini(), 6, "d", false), 6);
    }

    #[test]
    fn a_logout_in_act_10_stays_on_its_step() {
        // the route logs out to Oriath Docks to hand in quests long before the step that enters it
        let r = route();
        let s = steps(r, &Options::default());
        let docks = s.iter().position(|s| s.to.as_deref() == Some("2_10_town")).unwrap();
        let earlier = (0..docks).rev().find(|&i| s[i].act == 10 && s[i].lines.iter().any(|l| l.parts.iter().any(|p| p.kind == "logout"))).unwrap();
        assert_eq!(advance(&s, earlier, "2_10_town", r.is_town("2_10_town")), earlier);
        assert!(r.is_town("HideoutFelled") && !r.is_town("2_10_1"));
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
        assert_eq!(advance(&mini(), 0, "c", false), 4);
    }
}

#[cfg(test)]
mod build_tests {
    use super::*;

    #[test]
    fn the_test_build_gets_its_gems() {
        let r = route();
        let b = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        let mut wanted = Vec::new();
        for g in b.skill_sets.iter().flat_map(|s| &s.groups).filter(|g| g.enabled && !g.granted).flat_map(|g| &g.gems).filter(|g| g.enabled) {
            if let Some(id) = r.reward_id(&g.id, &g.name) {
                if !wanted.contains(&id) {
                    wanted.push(id);
                }
            }
        }
        let mut s = steps(r, &Options::default());
        pick_gems(r, &mut s, &GemPlan { class: "Marauder".into(), wanted, socketed: HashSet::new() });
        let got: Vec<String> = s.iter().flat_map(|s| s.lines.iter().flat_map(move |l| l.gems.iter().map(move |g| format!("A{} {} {} ({})", s.act, if g.buy { "$" } else { "★" }, g.name, g.npc)))).collect();
        assert!(got[0].starts_with("A1 ★"), "{got:?}");
        assert!(got.iter().any(|g| g.contains("Righteous Fire")), "{got:?}");
    }
}

#[cfg(test)]
mod walk_tests {
    use super::*;

    #[test]
    fn walking_the_route_with_town_trips_moves_one_step_at_a_time() {
        let r = route();
        for options in [Options::default(), Options { league_start: false, bandit: Bandit::Alira }] {
            let s = steps(r, &options);
            let towns: Vec<&str> = r.areas.iter().filter(|(_, a)| a.town).map(|(id, _)| id.as_str()).collect();
            let mut at = 0;
            while at < s.len() - 1 {
                // a trip to any town, and back into a zone already passed, changes nothing
                for t in towns.iter().filter(|t| s[at].to.as_deref() != Some(**t)) {
                    assert_eq!(advance(&s, at, t, true), at, "town {t} on step {at}");
                }
                if at > 0 {
                    let back = s[at - 1].to.as_deref().unwrap();
                    if s[at].to.as_deref() != Some(back) {
                        assert_eq!(advance(&s, at, back, r.is_town(back)), at, "back to {back} on step {at}");
                    }
                }
                let to = s[at].to.as_deref().unwrap();
                assert_eq!(advance(&s, at, to, r.is_town(to)), at + 1, "step {at}");
                at += 1;
            }
        }
    }
}
