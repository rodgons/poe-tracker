// pathofexile.com character import: the public character list and passive tree
// of the player's own account (needs the Characters tab public; no login).
use serde::{Deserialize, Serialize};

const SITE: &str = "https://www.pathofexile.com/character-window";
/// a stalled answer gives up instead of leaving the Build section on "Importing…"
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const USER_AGENT: &str = concat!("poe-tracker/", env!("CARGO_PKG_VERSION"), " (github.com/rodgons/poe-tracker)");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: String,
    pub league: String,
    pub level: u32,
    /// in a league of the current season, not Standard or another permanent one
    #[serde(default)]
    pub current: bool,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Passives {
    /// same numbering as a PoB spec's classId
    #[serde(rename = "character")]
    pub class_id: u32,
    /// every allocated node, ascendancy included
    pub hashes: Vec<u32>,
}

/// "Name#1234", or the "Name-1234" a profile URL shows, as the query value.
pub fn account_param(account: &str) -> String {
    let account = account.trim();
    let account = match account.rsplit_once('-') {
        Some((name, tag)) if tag.len() == 4 && tag.chars().all(|c| c.is_ascii_digit()) => format!("{name}#{tag}"),
        _ => account.to_string(),
    };
    account
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The account's characters, each marked with whether its league is this season's.
pub fn characters(account: &str) -> Result<Vec<Character>, String> {
    let mut chars = parse_characters(&get(&format!("{SITE}/get-characters?accountName={}&realm=pc", account_param(account)))?)?;
    let current = get("https://www.pathofexile.com/api/leagues?type=main&realm=pc").ok().and_then(|b| current_leagues(&b));
    for c in &mut chars {
        c.current = match &current {
            Some(leagues) => leagues.iter().any(|l| *l == c.league),
            // the league list is down: anything that isn't a permanent league
            None => !PERMANENT.contains(&c.league.as_str()),
        };
    }
    Ok(chars)
}

const PERMANENT: [&str; 8] = [
    "Standard",
    "Hardcore",
    "Solo Self-Found",
    "Hardcore SSF",
    "Ruthless",
    "Hardcore Ruthless",
    "SSF Ruthless",
    "Hardcore SSF Ruthless",
];

/// This season's softcore trade league ("Keepers"), for Trade links when no Character's league is known.
pub fn season_league() -> Option<String> {
    let leagues = current_leagues(&get("https://www.pathofexile.com/api/leagues?type=main&realm=pc").ok()?)?;
    leagues.into_iter().find(|l| !["Hardcore", "HC", "SSF", "Solo", "Ruthless"].iter().any(|w| l.contains(w)))
}

/// The leagues GGG marks as the current season (`category.current`).
fn current_leagues(body: &str) -> Option<Vec<String>> {
    #[derive(Deserialize)]
    struct League {
        id: String,
        #[serde(default)]
        category: Category,
    }
    #[derive(Default, Deserialize)]
    struct Category {
        #[serde(default)]
        current: bool,
    }
    let leagues: Vec<League> = serde_json::from_str(body).ok()?;
    Some(leagues.into_iter().filter(|l| l.category.current).map(|l| l.id).collect())
}

pub fn passives(account: &str, character: &str) -> Result<Passives, String> {
    let url = format!(
        "{SITE}/get-passive-skills?accountName={}&character={}&realm=pc",
        account_param(account),
        account_param(character)
    );
    parse_passives(&get(&url)?)
}

/// A gem socketed in the Character's equipped gear (weapon swap left out).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnedGem {
    /// the in-game name, " Support" kept
    pub name: String,
    /// a Vaal gem's base skill ("Molten Shell" for Vaal Molten Shell)
    pub vaal_base: Option<String>,
    pub support: bool,
    pub level: u32,
    pub quality: u32,
    /// the item's slot ("Helm", "Weapon"…)
    pub slot: String,
    /// gems with the same number are linked: the item's position and its socket's group
    pub link: u32,
    /// the gem's picture on GGG's image server, as the items import gives it
    #[serde(default)]
    pub icon: String,
}

/// The Character's socketed gems, and its level and league when the answer gives them.
pub fn items(account: &str, character: &str) -> Result<(Vec<OwnedGem>, Option<u32>, Option<String>), String> {
    let url = format!(
        "{SITE}/get-items?accountName={}&character={}&realm=pc",
        account_param(account),
        account_param(character)
    );
    parse_items(&get(&url)?)
}

/// Slots the gem comparison ignores: weapon swap, flasks and anything not worn.
const SKIPPED_SLOTS: [&str; 4] = ["Weapon2", "Offhand2", "Flask", "MainInventory"];

fn parse_items(body: &str) -> Result<(Vec<OwnedGem>, Option<u32>, Option<String>), String> {
    #[derive(Deserialize)]
    struct Items {
        items: Vec<Item>,
        #[serde(default)]
        character: Option<Who>,
    }
    #[derive(Deserialize)]
    struct Who {
        #[serde(default)]
        level: Option<u32>,
        #[serde(default)]
        league: Option<String>,
    }
    #[derive(Deserialize)]
    struct Item {
        #[serde(rename = "inventoryId", default)]
        slot: String,
        #[serde(default)]
        sockets: Vec<Socket>,
        #[serde(rename = "socketedItems", default)]
        socketed: Vec<Socketed>,
    }
    #[derive(Deserialize)]
    struct Socket {
        #[serde(default)]
        group: u32,
    }
    #[derive(Deserialize)]
    struct Socketed {
        #[serde(default)]
        socket: usize,
        #[serde(rename = "typeLine", default)]
        type_line: String,
        #[serde(rename = "baseType", default)]
        base_type: String,
        #[serde(default)]
        support: Option<bool>,
        #[serde(rename = "abyssJewel", default)]
        abyss: bool,
        #[serde(default)]
        properties: Vec<Property>,
        #[serde(default)]
        hybrid: Option<Hybrid>,
        #[serde(default)]
        icon: String,
    }
    #[derive(Deserialize)]
    struct Property {
        name: String,
        #[serde(default)]
        values: Vec<(String, serde_json::Value)>,
    }
    #[derive(Deserialize)]
    struct Hybrid {
        #[serde(rename = "isVaalGem", default)]
        vaal: bool,
        #[serde(rename = "baseTypeName", default)]
        base: String,
    }
    let parsed: Items = serde_json::from_str(body).map_err(|_| NOT_FOUND.to_string())?;
    // "21 (Max)" → 21, "+23%" → 23; a missing Quality is 0
    let number = |p: &[Property], name: &str| -> u32 {
        p.iter()
            .find(|p| p.name == name)
            .and_then(|p| p.values.first())
            .and_then(|(v, _)| v.trim().trim_start_matches('+').chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
            .unwrap_or(0)
    };
    let mut gems = Vec::new();
    for (i, item) in parsed.items.iter().enumerate() {
        if SKIPPED_SLOTS.contains(&item.slot.as_str()) {
            continue;
        }
        for g in item.socketed.iter().filter(|g| !g.abyss && g.support.is_some()) {
            let name = if g.base_type.is_empty() { &g.type_line } else { &g.base_type };
            let group = item.sockets.get(g.socket).map_or(g.socket as u32 + 100, |s| s.group);
            gems.push(OwnedGem {
                name: name.clone(),
                vaal_base: g.hybrid.as_ref().filter(|h| h.vaal && !h.base.is_empty()).map(|h| h.base.clone()),
                support: g.support == Some(true),
                level: number(&g.properties, "Level"),
                quality: number(&g.properties, "Quality"),
                slot: item.slot.clone(),
                link: (i as u32) * 1000 + group,
                icon: g.icon.clone(),
            });
        }
    }
    let (level, league) = parsed.character.map(|c| (c.level, c.league)).unwrap_or_default();
    Ok((gems, level, league))
}

fn get(url: &str) -> Result<String, String> {
    match ureq::get(url).set("User-Agent", USER_AGENT).timeout(TIMEOUT).call() {
        Ok(resp) => resp.into_string().map_err(|e| format!("Couldn't read pathofexile.com's answer: {e}")),
        Err(ureq::Error::Status(429, resp)) => Err(format!(
            "pathofexile.com asked us to slow down. Try again in {} seconds.",
            resp.header("Retry-After").unwrap_or("60")
        )),
        Err(ureq::Error::Status(403, _)) => {
            Err("That profile is private. Make your Characters tab public on pathofexile.com (My Account → Privacy), or set the points by hand.".into())
        }
        Err(ureq::Error::Status(404, _)) => Err("Account or character not found. Check the spelling and the #1234 tag.".into()),
        Err(ureq::Error::Status(code, _)) => Err(format!("pathofexile.com answered with an error ({code}). Try again later.")),
        Err(e) => Err(format!("Couldn't reach pathofexile.com: {e}")),
    }
}

/// An unknown account answers with the site's HTML page, not JSON.
const NOT_FOUND: &str = "Account not found. Use the full name with its tag, like Name#1234.";

fn parse_characters(body: &str) -> Result<Vec<Character>, String> {
    serde_json::from_str(body).map_err(|_| NOT_FOUND.to_string())
}

fn parse_passives(body: &str) -> Result<Passives, String> {
    serde_json::from_str(body).map_err(|_| NOT_FOUND.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_account_tags() {
        assert_eq!(account_param("Some_Name#1234"), "Some_Name%231234");
        assert_eq!(account_param(" Some_Name-1234 "), "Some_Name%231234");
        assert_eq!(account_param("my char"), "my%20char");
    }

    #[test]
    fn reads_the_character_list() {
        let body = r#"[{"name":"shiefire","realm":"pc","class":"Chieftain","league":"Covens","level":75,"lastLoginTime":1,"pinnable":false}]"#;
        assert_eq!(
            parse_characters(body).unwrap(),
            vec![Character { name: "shiefire".into(), class: "Chieftain".into(), league: "Covens".into(), level: 75, current: false }]
        );
        assert_eq!(parse_characters("<!DOCTYPE html><html>").unwrap_err(), NOT_FOUND);
    }

    #[test]
    fn finds_the_current_season() {
        let body = r#"[{"id":"Standard","category":{"id":"Standard"}},{"id":"Covens","category":{"id":"Coven","current":true}},
            {"id":"Hardcore Covens","category":{"id":"Coven","current":true}},{"id":"Old","endAt":"2025-01-01"}]"#;
        assert_eq!(current_leagues(body).unwrap(), vec!["Covens", "Hardcore Covens"]);
    }

    #[test]
    fn reads_socketed_gems() {
        let body = r#"{"items":[
            {"inventoryId":"Boots","sockets":[{"group":0},{"group":0},{"group":1}],"socketedItems":[
              {"socket":0,"support":true,"typeLine":"More Duration Support","baseType":"More Duration Support",
               "properties":[{"name":"Duration, Support","values":[]},{"name":"Level","values":[["20 (Max)",0]]},{"name":"Quality","values":[["+23%",1]]}]},
              {"socket":1,"support":false,"typeLine":"Vaal Molten Shell","baseType":"Vaal Molten Shell",
               "properties":[{"name":"Level","values":[["21 (Max)",0]]}],"hybrid":{"isVaalGem":true,"baseTypeName":"Molten Shell"}},
              {"socket":2,"support":false,"typeLine":"Herald of Ash","icon":"https://web.poecdn.com/gen/image/x/HeraldOfAsh.png","properties":[{"name":"Level","values":[["7",0]]}]}]},
            {"inventoryId":"Belt","sockets":[{"group":0}],"socketedItems":[{"socket":0,"abyssJewel":true,"typeLine":"Murderous Eye Jewel"}]},
            {"inventoryId":"Weapon2","sockets":[{"group":0}],"socketedItems":[{"socket":0,"support":false,"typeLine":"Dash"}]}],
            "character":{"name":"x","level":12,"league":"Keepers"}}"#;
        let (gems, level, league) = parse_items(body).unwrap();
        assert_eq!(level, Some(12));
        assert_eq!(league.as_deref(), Some("Keepers"));
        assert_eq!(gems.len(), 3, "abyss jewels and weapon swap are left out");
        assert_eq!((gems[0].level, gems[0].quality, gems[0].support), (20, 23, true));
        assert_eq!(gems[1].vaal_base.as_deref(), Some("Molten Shell"));
        assert_eq!(gems[1].quality, 0);
        assert_eq!(gems[0].link, gems[1].link);
        assert_ne!(gems[1].link, gems[2].link);
        assert_eq!(gems[2].name, "Herald of Ash");
        assert_eq!(gems[2].icon, "https://web.poecdn.com/gen/image/x/HeraldOfAsh.png");
    }

    #[test]
    fn reads_passives() {
        let body = r#"{"character":1,"ascendancy":3,"alternate_ascendancy":0,"hashes":[1731,2219],"hashes_ex":[],
            "mastery_effects":{"2828":11521},"skill_overrides":{},"items":[],"jewel_data":{}}"#;
        assert_eq!(parse_passives(body).unwrap(), Passives { class_id: 1, hashes: vec![1731, 2219] });
    }
}
