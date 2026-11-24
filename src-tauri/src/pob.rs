// Path of Building builds: fetch a pasted code or link, decode it, read its tree specs.
use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub title: String,
    /// the title as PoB wrote it, colour codes and all, for pairing with a Skill set
    #[serde(default)]
    pub raw_title: String,
    pub tree_version: String,
    pub class_id: u32,
    pub nodes: Vec<u32>,
    /// mastery node → chosen effect
    pub masteries: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Build {
    pub specs: Vec<Spec>,
    /// index into `specs` of the spec the build was saved on
    pub active: usize,
    /// the Build's Skill sets; empty in builds saved before v0.6
    #[serde(default)]
    pub skill_sets: Vec<SkillSet>,
    /// index into `skill_sets` of the set the build was saved on
    #[serde(default)]
    pub active_skill_set: usize,
}

/// One of the Build's named sets of gems, for one stage of the Build.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillSet {
    /// the title as PoB wrote it (colour codes kept, for pairing)
    pub title: String,
    pub groups: Vec<Group>,
}

/// A socket group: gems meant to be linked together, or a label-only heading.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub label: String,
    pub slot: String,
    pub enabled: bool,
    /// made by PoB from an item, the tree or config (`source`), not gems to socket
    pub granted: bool,
    pub gems: Vec<Gem>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gem {
    /// the gem's name as PoB shows it, without " Support"
    pub name: String,
    pub support: bool,
    pub level: u32,
    pub quality: u32,
    /// off in PoB: the author's optional pick
    pub enabled: bool,
    /// PoB's gem id ("Metadata/Items/Gems/SkillGemCleave"); empty in builds saved before v0.8.1
    #[serde(default)]
    pub id: String,
}

/// Turns what the player pasted into a raw build code, fetching links.
pub fn code_from_input(input: &str) -> Result<String, String> {
    let input = input.trim();
    let raw_url = if let Some(id) = link_id(input, "pobb.in/") {
        Some(format!("https://pobb.in/pob/{id}"))
    } else {
        link_id(input, "pastebin.com/").map(|id| format!("https://pastebin.com/raw/{id}"))
    };
    match raw_url {
        Some(url) => ureq::get(&url)
            .timeout(crate::poeapi::TIMEOUT)
            .call()
            .map_err(|e| format!("Couldn't fetch the build link: {e}"))?
            .into_string()
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("Couldn't read the build link: {e}")),
        None if input.starts_with("http") => {
            Err("Only pobb.in and pastebin.com links are supported. Paste the build code instead.".into())
        }
        None => Ok(input.to_string()),
    }
}

fn link_id(input: &str, host: &str) -> Option<String> {
    let rest = &input[input.find(host)? + host.len()..];
    let rest = rest.strip_prefix("raw/").or(rest.strip_prefix("pob/")).unwrap_or(rest);
    let id: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
    (!id.is_empty()).then_some(id)
}

pub fn decode(code: &str) -> Result<Build, String> {
    let bad = || "That doesn't look like a Path of Building code.".to_string();
    let cleaned: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned = cleaned.replace('+', "-").replace('/', "_");
    let padded = format!("{}{}", cleaned.trim_end_matches('='), "=".repeat((4 - cleaned.trim_end_matches('=').len() % 4) % 4));
    let bytes = URL_SAFE.decode(padded).map_err(|_| bad())?;
    let mut xml = String::new();
    flate2::read::ZlibDecoder::new(&bytes[..]).read_to_string(&mut xml).map_err(|_| bad())?;
    parse(&xml)
}

pub fn parse(xml: &str) -> Result<Build, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("The build is damaged: {e}"))?;
    let tree = doc
        .descendants()
        .find(|n| n.has_tag_name("Tree"))
        .ok_or("The build has no passive tree.")?;
    let mut specs = Vec::new();
    for s in tree.children().filter(|n| n.has_tag_name("Spec")) {
        let Some(nodes) = s.attribute("nodes") else {
            return Err("This build is too old: open it in Path of Building and re-export.".into());
        };
        let raw = s.attribute("title").unwrap_or("Tree");
        specs.push(Spec {
            title: clean_title(raw),
            raw_title: raw.to_string(),
            tree_version: s.attribute("treeVersion").unwrap_or("").to_string(),
            class_id: s.attribute("classId").and_then(|c| c.parse().ok()).unwrap_or(0),
            nodes: nodes.split(',').filter_map(|n| n.trim().parse().ok()).collect(),
            masteries: parse_masteries(s.attribute("masteryEffects").unwrap_or("")),
        });
    }
    if specs.is_empty() {
        return Err("The build has no tree specs.".into());
    }
    let active = tree
        .attribute("activeSpec")
        .and_then(|a| a.parse::<usize>().ok())
        .map(|a| a.saturating_sub(1).min(specs.len() - 1))
        .unwrap_or(0);
    let (skill_sets, active_skill_set) = parse_skills(&doc);
    Ok(Build { specs, active, skill_sets, active_skill_set })
}

/// `<Skills activeSkillSet=id>` › `<SkillSet id title>` › `<Skill>` › `<Gem>`. Builds from
/// before PoB had Skill sets keep their `<Skill>`s straight under `<Skills>`: one set.
fn parse_skills(doc: &roxmltree::Document) -> (Vec<SkillSet>, usize) {
    let Some(skills) = doc.descendants().find(|n| n.has_tag_name("Skills")) else { return (Vec::new(), 0) };
    let flag = |n: roxmltree::Node, a: &str| n.attribute(a).is_none_or(|v| v != "false");
    let num = |n: roxmltree::Node, a: &str| n.attribute(a).and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(0);
    let groups = |set: roxmltree::Node| -> Vec<Group> {
        set.children()
            .filter(|n| n.has_tag_name("Skill"))
            .map(|g| Group {
                label: g.attribute("label").unwrap_or("").to_string(),
                slot: g.attribute("slot").unwrap_or("").to_string(),
                enabled: flag(g, "enabled"),
                granted: g.attribute("source").is_some(),
                gems: g
                    .children()
                    .filter(|n| n.has_tag_name("Gem"))
                    .filter_map(|gem| {
                        let skill_id = gem.attribute("skillId").unwrap_or("");
                        let name = gem.attribute("nameSpec").filter(|n| !n.is_empty()).unwrap_or(skill_id);
                        (!name.is_empty()).then(|| Gem {
                            name: name.trim_end_matches(" Support").to_string(),
                            support: gem.attribute("gemId").unwrap_or("").contains("SupportGem")
                                || skill_id.starts_with("Support"),
                            level: num(gem, "level"),
                            quality: num(gem, "quality"),
                            enabled: flag(gem, "enabled"),
                            id: gem.attribute("gemId").unwrap_or("").to_string(),
                        })
                    })
                    .collect(),
            })
            .collect()
    };
    let sets: Vec<_> = skills.children().filter(|n| n.has_tag_name("SkillSet")).collect();
    if sets.is_empty() {
        let only = groups(skills);
        return if only.is_empty() { (Vec::new(), 0) } else { (vec![SkillSet { title: "Default".into(), groups: only }], 0) };
    }
    let active = skills.attribute("activeSkillSet");
    let index = sets.iter().position(|s| s.attribute("id") == active).unwrap_or(0);
    let out = sets
        .iter()
        .map(|s| SkillSet { title: s.attribute("title").unwrap_or("Default").to_string(), groups: groups(*s) })
        .collect();
    (out, index)
}

/// "{48385,12345},{...}" → [(48385, 12345), ...]
fn parse_masteries(s: &str) -> Vec<(u32, u32)> {
    s.split('}')
        .filter_map(|pair| {
            let (a, b) = pair.trim_start_matches([',', '{']).split_once(',')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        })
        .collect()
}

/// Strips PoB colour codes (^1, ^xRRGGBB) from spec titles.
pub fn clean_title(t: &str) -> String {
    let mut out = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '^' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some(d) if d.is_ascii_digit() => {
                chars.next();
            }
            Some('x') => {
                chars.next();
                for _ in 0..6 {
                    chars.next();
                }
            }
            _ => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const POHX: &str = include_str!("../tests/pohx-rf.txt");

    #[test]
    fn decodes_the_test_build() {
        let b = decode(POHX).unwrap();
        assert_eq!(b.specs.len(), 10);
        assert_eq!(b.active, 1);
        let s = &b.specs[1];
        assert_eq!(s.title, "Lvl 01-30 {1}");
        assert_eq!(s.tree_version, "3_29");
        assert_eq!(s.class_id, 1);
        assert_eq!(b.specs[0].title, "CHECK POB NOTES FOR INFORMATION");
    }

    #[test]
    fn reads_the_skill_sets() {
        let b = decode(POHX).unwrap();
        assert_eq!(b.skill_sets.len(), 8);
        assert_eq!(clean_title(&b.skill_sets[b.active_skill_set].title), "Level 1-30 {1}");
        let first = &b.skill_sets[0];
        let slam = first.groups.iter().find(|g| g.label == "Ground Slam").unwrap();
        assert_eq!(slam.gems[0], Gem { name: "Ground Slam".into(), support: false, level: 9, quality: 0, enabled: true, id: "Metadata/Items/Gems/SkillGemGroundSlam".into() });
        assert!(slam.gems[1].support);
        assert!(first.groups.iter().any(|g| g.gems.is_empty()), "label-only groups are kept as headings");
    }

    #[test]
    fn reads_flat_skills_and_granted_groups() {
        let xml = r#"<PathOfBuilding><Tree activeSpec="1"><Spec nodes="1"/></Tree><Skills>
            <Skill label="" enabled="true"><Gem nameSpec="Cleave" skillId="Cleave" gemId="Metadata/Items/Gems/SkillGemCleave" level="20" quality="20"/>
              <Gem nameSpec="Melee Physical Damage" skillId="SupportMeleePhysicalDamage" enabled="false" level="1"/></Skill>
            <Skill source="Item:3:Some Ring" enabled="true"><Gem skillId="Curse"/></Skill></Skills></PathOfBuilding>"#;
        let b = parse(xml).unwrap();
        assert_eq!(b.skill_sets.len(), 1);
        let g = &b.skill_sets[0].groups;
        assert_eq!(g[0].gems[0].quality, 20);
        assert!(g[0].gems[1].support && !g[0].gems[1].enabled);
        assert_eq!(g[0].gems[0].id, "Metadata/Items/Gems/SkillGemCleave");
        assert!(g[1].granted);
    }

    #[test]
    fn reads_link_ids() {
        assert_eq!(link_id("https://pobb.in/cYgy70giDjbo", "pobb.in/").unwrap(), "cYgy70giDjbo");
        assert_eq!(link_id("pastebin.com/raw/abc123", "pastebin.com/").unwrap(), "abc123");
    }

    #[test]
    fn refuses_legacy_specs() {
        let xml = r#"<PathOfBuilding><Tree activeSpec="1"><Spec><URL>https://x/AAAABAEA</URL></Spec></Tree></PathOfBuilding>"#;
        assert!(parse(xml).unwrap_err().contains("too old"));
    }

    #[test]
    fn parses_masteries_and_titles() {
        assert_eq!(parse_masteries("{1,2},{30,40}"), vec![(1, 2), (30, 40)]);
        assert_eq!(clean_title("Lvl 90 ^2Going Block ^7{6} ^xFF0000red"), "Lvl 90 Going Block {6} red");
    }
}
