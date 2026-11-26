// The Gems comparison: which Skill set goes with the Tree spec, and how the
// Character's socketed gems line up with it (had, missing, unlinked, remove, optional).
use crate::pob::{clean_title, Build, Gem, SkillSet};
use crate::poeapi::OwnedGem;
use serde::Serialize;

/// How the shown Skill set was chosen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Chosen {
    /// paired with the Tree spec by PoB's own rule
    Paired,
    /// nothing paired: the set the Build was saved on
    Active,
    /// the player's pick in the Gems section
    Picked,
}

/// The first `{…}` group in a title, split on commas: "Lvl 1-30 {4,5}" → ["4", "5"].
fn brace_ids(title: &str) -> Vec<String> {
    let Some(open) = title.find('{') else { return Vec::new() };
    let Some(len) = title[open..].find('}') else { return Vec::new() };
    title[open + 1..open + len].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// PoB's pairing: shared brace ids, else an identical raw title, else a Build's only Skill set.
pub fn paired(build: &Build, spec: usize) -> Option<usize> {
    let titles: Vec<&str> = build.skill_sets.iter().map(|s| s.title.as_str()).collect();
    pair(build, spec, &titles)
}

/// PoB's loadout pairing (`Build.lua` `SyncLoadouts`), the same for Skill sets and Item sets:
/// the first set sharing a brace id with the Tree spec's title, else one with an identical raw
/// title, else the only set there is.
pub fn pair(build: &Build, spec: usize, titles: &[&str]) -> Option<usize> {
    let spec = build.specs.get(spec)?;
    let title = if spec.raw_title.is_empty() { &spec.title } else { &spec.raw_title };
    let ids = brace_ids(title);
    if !ids.is_empty() {
        if let Some(i) = titles.iter().position(|t| brace_ids(t).iter().any(|id| ids.contains(id))) {
            return Some(i);
        }
    }
    if let Some(i) = titles.iter().position(|t| *t == title.as_str()) {
        return Some(i);
    }
    (titles.len() == 1).then_some(0)
}

/// The Skill set to compare: the player's pick, else the paired one, else the active one.
pub fn choose(build: &Build, spec: usize, picked: Option<usize>) -> Option<(usize, Chosen)> {
    if build.skill_sets.is_empty() {
        return None;
    }
    if let Some(p) = picked.filter(|p| *p < build.skill_sets.len()) {
        return Some((p, Chosen::Picked));
    }
    match paired(build, spec) {
        Some(i) => Some((i, Chosen::Paired)),
        None => Some((build.active_skill_set.min(build.skill_sets.len() - 1), Chosen::Active)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Had,
    Missing,
    /// socketed, but not linked to its group's main skill
    Unlinked,
    /// the author's optional pick, whether or not it's socketed
    Optional,
}

#[derive(Debug, Serialize)]
pub struct GemRow {
    pub name: String,
    pub support: bool,
    pub status: Status,
    /// an optional pick that's socketed
    pub socketed: bool,
    pub want_level: u32,
    pub want_quality: u32,
    pub have_level: Option<u32>,
    pub have_quality: Option<u32>,
    /// the gem that covers it when its name differs (Vaal or Awakened)
    pub have_name: Option<String>,
    /// the socketed gem's own picture, when it's socketed
    pub have_icon: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupRow {
    /// the main skill's name, else the label
    pub title: String,
    /// a label with no gems: the author's heading or note
    pub heading: bool,
    pub slot: String,
    pub gems: Vec<GemRow>,
}

#[derive(Debug, Serialize)]
pub struct Extra {
    pub name: String,
    pub icon: String,
    pub level: u32,
    pub quality: u32,
    pub slot: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Counts {
    pub missing: usize,
    pub unlinked: usize,
    pub remove: usize,
}

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub groups: Vec<GroupRow>,
    pub remove: Vec<Extra>,
    pub counts: Counts,
}

/// Lowercase, " Support" dropped: the name both sides are matched on.
fn key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    lower.strip_suffix(" support").map(str::to_string).unwrap_or(lower)
}

/// 0 when `have` is the very gem `want` names, 1 when it covers it (Vaal X or Awakened X
/// for plain X), None otherwise. Transfigured gems are their own gems.
fn covers(have: &OwnedGem, want: &str) -> Option<u8> {
    let name = key(&have.name);
    if name == want {
        return Some(0);
    }
    let vaal = have.vaal_base.as_deref().map(key).or_else(|| name.strip_prefix("vaal ").map(str::to_string));
    if vaal.as_deref() == Some(want) || name.strip_prefix("awakened ") == Some(want) {
        return Some(1);
    }
    None
}

/// The groups that hold gems to socket: switched on, not granted by an item or the tree.
fn wanted(set: &SkillSet) -> impl Iterator<Item = (usize, &crate::pob::Group)> {
    set.groups.iter().enumerate().filter(|(_, g)| g.enabled && !g.granted && !g.gems.is_empty())
}

/// A group's main skill: its first switched-on active gem.
fn anchor(gems: &[Gem]) -> Option<usize> {
    gems.iter().position(|g| g.enabled && !g.support).or_else(|| gems.iter().position(|g| g.enabled))
}

pub fn compare(set: &SkillSet, owned: &[OwnedGem]) -> Comparison {
    // which owned gem each wanted gem got, by (group, gem)
    let mut taken = vec![false; owned.len()];
    let mut got: std::collections::HashMap<(usize, usize), usize> = Default::default();
    // `linked`: only a gem linked to `prefer` (the group's main skill) will do
    let pick = |want: &Gem, prefer: Option<u32>, linked: bool, taken: &mut Vec<bool>| -> Option<usize> {
        let k = key(&want.name);
        let mut best: Option<(u8, bool, usize)> = None;
        for (i, have) in owned.iter().enumerate() {
            if taken[i] {
                continue;
            }
            let Some(rank) = covers(have, &k) else { continue };
            // a gem linked to the group's main skill first, then exact names
            let off_link = prefer.is_some_and(|l| l != have.link);
            if linked && (off_link || prefer.is_none()) {
                continue;
            }
            if best.is_none_or(|(r, o, _)| (rank, off_link) < (r, o)) {
                best = Some((rank, off_link, i));
            }
        }
        let i = best?.2;
        taken[i] = true;
        Some(i)
    };
    // main skills first, so their supports can prefer gems linked to them; then the
    // other required gems; optional picks last, so they never take a required gem's copy
    let groups: Vec<_> = wanted(set).collect();
    for (gi, g) in &groups {
        if let Some(a) = anchor(&g.gems) {
            if let Some(i) = pick(&g.gems[a], None, false, &mut taken) {
                got.insert((*gi, a), i);
            }
        }
    }
    // per kind: linked copies across every group first, so one group's stray pick
    // never takes the copy linked in another; then whatever's left
    for (pass_optional, linked) in [(false, true), (false, false), (true, true), (true, false)] {
        for (gi, g) in &groups {
            let link = anchor(&g.gems).and_then(|a| got.get(&(*gi, a))).map(|i| owned[*i].link);
            for (n, gem) in g.gems.iter().enumerate() {
                if gem.enabled == pass_optional || got.contains_key(&(*gi, n)) {
                    continue;
                }
                if let Some(i) = pick(gem, link, linked, &mut taken) {
                    got.insert((*gi, n), i);
                }
            }
        }
    }

    let mut counts = Counts::default();
    let mut rows = Vec::new();
    for (gi, g) in set.groups.iter().enumerate() {
        if !g.enabled || g.granted {
            continue;
        }
        let label = clean_title(&g.label);
        if g.gems.is_empty() {
            if !label.is_empty() {
                rows.push(GroupRow { title: label, heading: true, slot: String::new(), gems: Vec::new() });
            }
            continue;
        }
        let a = anchor(&g.gems);
        let link = a.and_then(|a| got.get(&(gi, a))).map(|i| owned[*i].link);
        let gems = g
            .gems
            .iter()
            .enumerate()
            .map(|(n, gem)| {
                let have = got.get(&(gi, n)).map(|i| &owned[*i]);
                let status = match have {
                    _ if !gem.enabled => Status::Optional,
                    None => Status::Missing,
                    Some(h) if Some(n) != a && link.is_some_and(|l| l != h.link) => Status::Unlinked,
                    Some(_) => Status::Had,
                };
                match status {
                    Status::Missing => counts.missing += 1,
                    Status::Unlinked => counts.unlinked += 1,
                    _ => {}
                }
                GemRow {
                    name: gem.name.clone(),
                    support: gem.support,
                    status,
                    socketed: have.is_some(),
                    want_level: gem.level,
                    want_quality: gem.quality,
                    have_level: have.map(|h| h.level),
                    have_quality: have.map(|h| h.quality),
                    have_name: have.filter(|h| key(&h.name) != key(&gem.name)).map(|h| h.name.clone()),
                    have_icon: have.map(|h| h.icon.clone()).filter(|i| !i.is_empty()),
                }
            })
            .collect();
        let title = a.map(|a| g.gems[a].name.clone()).filter(|_| label.is_empty()).unwrap_or(label);
        rows.push(GroupRow { title, heading: false, slot: g.slot.clone(), gems });
    }
    // a heading with no gem group after it (the author's trailing notes) adds nothing
    while rows.last().is_some_and(|r| r.heading) {
        rows.pop();
    }
    let remove: Vec<Extra> = owned
        .iter()
        .zip(&taken)
        .filter(|(_, t)| !**t)
        .map(|(h, _)| Extra { name: h.name.clone(), icon: h.icon.clone(), level: h.level, quality: h.quality, slot: h.slot.clone() })
        .collect();
    counts.remove = remove.len();
    Comparison { groups: rows, remove, counts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pob::{decode, Group};

    fn have(name: &str, link: u32) -> OwnedGem {
        OwnedGem {
            name: name.into(),
            vaal_base: None,
            support: name.ends_with(" Support"),
            level: 20,
            quality: 0,
            slot: "BodyArmour".into(),
            link,
            icon: String::new(),
        }
    }
    fn gem(name: &str, support: bool) -> Gem {
        Gem { name: name.into(), support, level: 20, quality: 20, enabled: true, id: String::new() }
    }
    fn set(groups: Vec<Vec<Gem>>) -> SkillSet {
        SkillSet {
            title: "t".into(),
            groups: groups.into_iter().map(|gems| Group { enabled: true, gems, ..Default::default() }).collect(),
        }
    }
    fn statuses(c: &Comparison) -> Vec<Vec<Status>> {
        c.groups.iter().map(|g| g.gems.iter().map(|r| r.status).collect()).collect()
    }

    #[test]
    fn pairs_pohx_specs_with_skill_sets() {
        let b = decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        // "Lvl 01-30 {1}" ↔ "Level 1-30 {1}"
        let i = paired(&b, 1).unwrap();
        assert_eq!(clean_title(&b.skill_sets[i].title), "Level 1-30 {1}");
        // the notes spec pairs with nothing and falls back to the active set
        assert_eq!(paired(&b, 0), None);
        assert_eq!(choose(&b, 0, None), Some((b.active_skill_set, Chosen::Active)));
        assert_eq!(choose(&b, 1, Some(2)), Some((2, Chosen::Picked)));
        // every brace-numbered spec in Pohx except {7} finds a set
        let unpaired: Vec<_> = (1..b.specs.len()).filter(|s| paired(&b, *s).is_none()).map(|s| b.specs[s].title.clone()).collect();
        assert!(unpaired.len() <= 1, "{unpaired:?}");
    }

    #[test]
    fn brace_ids_take_the_first_group() {
        assert_eq!(brace_ids("^4Level {4,5} later {9}"), vec!["4", "5"]);
        assert!(brace_ids("Final").is_empty());
    }

    #[test]
    fn marks_had_missing_and_remove() {
        let s = set(vec![vec![gem("Righteous Fire", false), gem("Burning Damage", true), gem("Elemental Focus", true)]]);
        let c = compare(&s, &[have("Righteous Fire", 1), have("Burning Damage Support", 1), have("Clarity", 2)]);
        assert_eq!(statuses(&c), vec![vec![Status::Had, Status::Had, Status::Missing]]);
        assert_eq!(c.remove.len(), 1);
        assert_eq!(c.remove[0].name, "Clarity");
        assert_eq!((c.counts.missing, c.counts.remove), (1, 1));
    }

    #[test]
    fn vaal_and_awakened_cover_the_plain_gem_only() {
        let s = set(vec![vec![gem("Righteous Fire", false), gem("Added Fire Damage", true)]]);
        let mut vaal = have("Vaal Righteous Fire", 1);
        vaal.vaal_base = Some("Righteous Fire".into());
        let c = compare(&s, &[vaal, have("Awakened Added Fire Damage Support", 1)]);
        assert_eq!(statuses(&c), vec![vec![Status::Had, Status::Had]]);
        assert_eq!(c.groups[0].gems[0].have_name.as_deref(), Some("Vaal Righteous Fire"));
        // the reverse doesn't hold
        let s = set(vec![vec![gem("Vaal Righteous Fire", false)]]);
        let c = compare(&s, &[have("Righteous Fire", 1)]);
        assert_eq!(statuses(&c), vec![vec![Status::Missing]]);
        assert_eq!(c.remove.len(), 1);
    }

    #[test]
    fn transfigured_gems_are_their_own_gems() {
        let s = set(vec![vec![gem("Righteous Fire of Arcane Devotion", false)]]);
        let c = compare(&s, &[have("Righteous Fire", 1)]);
        assert_eq!(statuses(&c), vec![vec![Status::Missing]]);
    }

    #[test]
    fn supports_away_from_the_main_skill_are_unlinked() {
        let s = set(vec![vec![gem("Fireball", false), gem("Spell Echo", true), gem("Faster Casting", true)]]);
        let c = compare(&s, &[have("Fireball", 1), have("Spell Echo Support", 1), have("Faster Casting Support", 7)]);
        assert_eq!(statuses(&c), vec![vec![Status::Had, Status::Had, Status::Unlinked]]);
        assert_eq!(c.counts.unlinked, 1);
    }

    #[test]
    fn copies_count_and_linked_copies_win() {
        // two groups each want Lifetap; one copy is socketed, linked to the second group
        let s = set(vec![
            vec![gem("Herald of Ash", false), gem("Lifetap", true)],
            vec![gem("Fireball", false), gem("Lifetap", true)],
        ]);
        let c = compare(&s, &[have("Herald of Ash", 1), have("Fireball", 2), have("Lifetap Support", 2)]);
        assert_eq!(statuses(&c), vec![vec![Status::Had, Status::Missing], vec![Status::Had, Status::Had]]);
        // an extra copy beyond what the set wants is a Remove
        let s = set(vec![vec![gem("Clarity", false)]]);
        let c = compare(&s, &[have("Clarity", 1), have("Clarity", 2)]);
        assert_eq!(c.remove.len(), 1);
    }

    #[test]
    fn optional_picks_are_never_missing_or_remove() {
        let mut opt = gem("Inspiration", true);
        opt.enabled = false;
        let s = set(vec![vec![gem("Cleave", false), opt.clone()], vec![gem("Ground Slam", false), opt]]);
        let c = compare(&s, &[have("Cleave", 1), have("Inspiration Support", 1)]);
        assert_eq!(statuses(&c)[0], vec![Status::Had, Status::Optional]);
        assert!(c.groups[0].gems[1].socketed);
        assert!(!c.groups[1].gems[1].socketed);
        assert_eq!((c.counts.missing, c.counts.remove), (1, 0));
    }

    #[test]
    fn skips_disabled_and_granted_groups_and_keeps_headings() {
        let mut s = set(vec![vec![], vec![gem("Cleave", false)], vec![gem("Curse", false)], vec![gem("Old", false)], vec![]]);
        s.groups[0].label = "^6<< Damage >>".into();
        s.groups[2].granted = true;
        s.groups[3].enabled = false;
        s.groups[4].label = "trailing note".into();
        let c = compare(&s, &[]);
        let titles: Vec<_> = c.groups.iter().map(|g| (g.title.as_str(), g.heading)).collect();
        assert_eq!(titles, vec![("<< Damage >>", true), ("Cleave", false)]);
    }

    #[test]
    fn compares_a_pohx_skill_set() {
        let b = decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        let s = &b.skill_sets[paired(&b, 1).unwrap()];
        let c = compare(s, &[have("Ground Slam", 1), have("Chance to Bleed Support", 1), have("Frostblink", 3)]);
        let slam = c.groups.iter().find(|g| g.title == "Ground Slam").unwrap();
        assert_eq!(slam.gems[0].status, Status::Had);
        assert_eq!(slam.gems[1].status, Status::Had);
        assert!(c.counts.missing > 0);
    }
}
