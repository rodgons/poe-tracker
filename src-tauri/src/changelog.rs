// What's new: CHANGELOG.md, bundled, shown once after the app updates to a new version.
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
pub struct Entry {
    pub version: String,
    pub changes: Vec<String>,
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    let mut parts = v.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u32>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

/// Every version's changes in `text`, newest first as written.
pub fn parse(text: &str) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    for line in text.lines() {
        if let Some(head) = line.strip_prefix("## ") {
            entries.push(Entry { version: head.trim().to_string(), changes: Vec::new() });
        } else if let (Some(bullet), Some(e)) = (line.strip_prefix("- "), entries.last_mut()) {
            e.changes.push(bullet.trim().to_string());
        } else if let (Some(e), true) = (entries.last_mut(), line.starts_with("  ") && !line.trim().is_empty()) {
            // a bullet wrapped onto the next line
            if let Some(last) = e.changes.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        }
    }
    entries
}

/// What's new since `seen` up to and including `current`; just `current`'s when `seen` is unknown.
pub fn since(text: &str, seen: &str, current: &str) -> Vec<Entry> {
    let Some(cur) = parse_version(current) else { return Vec::new() };
    let from = parse_version(seen);
    parse(text)
        .into_iter()
        .filter(|e| match (parse_version(&e.version), from) {
            (Some(v), Some(from)) => v > from && v <= cur,
            (Some(v), None) => v == cur,
            _ => false,
        })
        .filter(|e| !e.changes.is_empty())
        .collect()
}

pub fn bundled() -> &'static str {
    include_str!("../../CHANGELOG.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# Changelog\n\nintro\n\n## 0.3.0\n\n- three\n  wrapped\n- more\n\n## 0.2.0\n\n- two\n\n## 0.1.0\n\n- one\n";

    #[test]
    fn shows_every_version_since_the_last_seen_one() {
        let got = since(TEXT, "0.1.0", "0.3.0");
        assert_eq!(got.iter().map(|e| e.version.as_str()).collect::<Vec<_>>(), ["0.3.0", "0.2.0"]);
        assert_eq!(got[0].changes, ["three wrapped", "more"]);
    }

    #[test]
    fn an_unknown_last_version_shows_only_the_current_one_and_nothing_newer() {
        assert_eq!(since(TEXT, "", "0.2.0"), [Entry { version: "0.2.0".into(), changes: vec!["two".into()] }]);
        assert!(since(TEXT, "0.3.0", "0.3.0").is_empty());
    }

    #[test]
    fn the_bundled_changelog_covers_this_version() {
        let v = env!("CARGO_PKG_VERSION");
        assert!(!since(bundled(), "", v).is_empty(), "CHANGELOG.md has no entry for {v}");
    }
}
