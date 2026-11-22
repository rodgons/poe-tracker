// pathofexile.com character import: the public character list and passive tree
// of the player's own account (needs the Characters tab public; no login).
use serde::{Deserialize, Serialize};

const SITE: &str = "https://www.pathofexile.com/character-window";
const USER_AGENT: &str = concat!("poe-tracker/", env!("CARGO_PKG_VERSION"), " (github.com/rodgons/poe-tracker)");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub name: String,
    pub class: String,
    pub league: String,
    pub level: u32,
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

pub fn characters(account: &str) -> Result<Vec<Character>, String> {
    parse_characters(&get(&format!("{SITE}/get-characters?accountName={}&realm=pc", account_param(account)))?)
}

pub fn passives(account: &str, character: &str) -> Result<Passives, String> {
    let url = format!(
        "{SITE}/get-passive-skills?accountName={}&character={}&realm=pc",
        account_param(account),
        account_param(character)
    );
    parse_passives(&get(&url)?)
}

fn get(url: &str) -> Result<String, String> {
    match ureq::get(url).set("User-Agent", USER_AGENT).call() {
        Ok(resp) => resp.into_string().map_err(|e| format!("Couldn't read pathofexile.com's answer: {e}")),
        Err(ureq::Error::Status(429, resp)) => Err(format!(
            "pathofexile.com asked us to slow down. Try again in {} seconds.",
            resp.header("Retry-After").unwrap_or("60")
        )),
        Err(ureq::Error::Status(403, _)) => {
            Err("That profile is private. Make your Characters tab public on pathofexile.com (My Account → Privacy), or set the points by hand.".into())
        }
        Err(ureq::Error::Status(404, _)) => Err("Account or character not found. Check the spelling and the #1234 tag.".into()),
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
            vec![Character { name: "shiefire".into(), class: "Chieftain".into(), league: "Covens".into(), level: 75 }]
        );
        assert_eq!(parse_characters("<!DOCTYPE html><html>").unwrap_err(), NOT_FOUND);
    }

    #[test]
    fn reads_passives() {
        let body = r#"{"character":1,"ascendancy":3,"alternate_ascendancy":0,"hashes":[1731,2219],"hashes_ex":[],
            "mastery_effects":{"2828":11521},"skill_overrides":{},"items":[],"jewel_data":{}}"#;
        assert_eq!(parse_passives(body).unwrap(), Passives { class_id: 1, hashes: vec![1731, 2219] });
    }
}
