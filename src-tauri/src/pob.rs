// Path of Building builds: fetch a pasted code or link, decode it, read its tree specs.
use base64::{engine::general_purpose::URL_SAFE, Engine};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Spec {
    pub title: String,
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
        specs.push(Spec {
            title: clean_title(s.attribute("title").unwrap_or("Tree")),
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
    Ok(Build { specs, active })
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
fn clean_title(t: &str) -> String {
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
