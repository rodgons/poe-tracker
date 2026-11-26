// The Items section: the Build's Item set for the Tree spec, each item as PoB describes it,
// and a pathofexile.com/trade link to buy it. The app never calls the trade site: it only
// builds the link the player opens in their browser.
use crate::gems::{self, Chosen};
use crate::pob::Build;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;

/// Trade stat ids by the trade site's own mod text, and the base types (scripts/extract_trade_stats.py).
#[derive(Deserialize)]
struct Stats {
    explicit: HashMap<String, String>,
    enchant: HashMap<String, String>,
    bases: Vec<String>,
    /// lowercased texts, for PoB lines whose capitals differ from trade's ("Trap And Mine")
    #[serde(skip)]
    explicit_lower: HashMap<String, String>,
    #[serde(skip)]
    enchant_lower: HashMap<String, String>,
}

fn stats() -> &'static Stats {
    static STATS: OnceLock<Stats> = OnceLock::new();
    STATS.get_or_init(|| {
        let mut s: Stats = serde_json::from_str(include_str!("../data/trade-stats.json")).expect("bundled trade stats");
        let lower = |m: &HashMap<String, String>| m.iter().map(|(k, v)| (k.to_lowercase(), v.clone())).collect();
        s.explicit_lower = lower(&s.explicit);
        s.enchant_lower = lower(&s.enchant);
        s
    })
}

/// The table a lookup uses, by its kind: (exact texts, lowercased texts).
type Table = (&'static HashMap<String, String>, &'static HashMap<String, String>);
fn explicit() -> Table {
    (&stats().explicit, &stats().explicit_lower)
}
fn enchant() -> Table {
    (&stats().enchant, &stats().enchant_lower)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    Normal,
    Magic,
    Rare,
    Unique,
}

/// How PoB tagged a mod line.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Crafted,
    Enchant,
    Fractured,
    Custom,
    Exarch,
    Eater,
    Mutated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// the text as PoB shows it, tags stripped, ranges kept: "(15-20)% increased Cast Speed"
    pub text: String,
    pub kind: Option<Kind>,
    /// a variant line the author picked (Watcher's Eye and other alt-variant uniques)
    pub picked: bool,
}

/// One item's PoB text, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub rarity: Rarity,
    /// "Nebulis", "New Item" (an unnamed rare), or a magic item's full name
    pub name: String,
    /// the base type; a magic or normal item's is found in its name
    pub base: Option<String>,
    pub implicits: Vec<Line>,
    pub explicits: Vec<Line>,
    /// Corrupted, Mirrored, …
    pub flags: Vec<String>,
    /// Shaper, Searing Exarch, …
    pub influences: Vec<String>,
    /// the author picks variants (Watcher's Eye); its picked lines are what makes it
    pub picks_variants: bool,
}

const INFLUENCES: [&str; 8] = ["Shaper", "Elder", "Crusader", "Redeemer", "Hunter", "Warlord", "Searing Exarch", "Eater of Worlds"];
const FLAGS: [&str; 6] = ["Corrupted", "Mirrored", "Split", "Fractured Item", "Synthesised Item", "Unidentified"];

/// Reads PoB's item text (`Item:BuildRaw` in PoB's source; docs/research/pob-items.md).
pub fn parse(text: &str) -> Option<Parsed> {
    let lines: Vec<String> = text.lines().map(|l| crate::pob::clean_title(l)).filter(|l| !l.is_empty()).collect();
    let rarity = match lines.first()?.strip_prefix("Rarity: ")?.trim() {
        "MAGIC" => Rarity::Magic,
        "RARE" => Rarity::Rare,
        "UNIQUE" | "RELIC" => Rarity::Unique,
        _ => Rarity::Normal,
    };
    let name = lines.get(1)?.clone();
    let mut i = 2;
    let mut base = None;
    if matches!(rarity, Rarity::Rare | Rarity::Unique) {
        base = Some(lines.get(2)?.clone());
        i = 3;
    }
    let header_value = |key: &str| -> Option<u32> {
        lines.iter().find_map(|l| l.strip_prefix(key).and_then(|v| v.trim().parse().ok()))
    };
    let mut picked = vec![header_value("Selected Variant:")];
    for n in ["", " Two", " Three", " Four", " Five"] {
        picked.push(header_value(&format!("Selected Alt Variant{n}:")));
    }
    let picked: Vec<u32> = picked.into_iter().flatten().collect();
    let picks_variants = lines.iter().any(|l| l.starts_with("Has Alt Variant"));
    let mut influences = Vec::new();
    let mut implicit_count = None;
    while let Some(l) = lines.get(i) {
        i += 1;
        if let Some(n) = l.strip_prefix("Implicits:") {
            implicit_count = n.trim().parse::<usize>().ok();
            break;
        }
        if let Some(inf) = l.strip_suffix(" Item").filter(|s| INFLUENCES.contains(s)) {
            influences.push(inf.to_string());
        }
    }
    // an item with no "Implicits:" line has nothing PoB would show as mods
    let count = implicit_count?;
    let mut flags = Vec::new();
    let mut mods = Vec::new();
    for l in &lines[i..] {
        if FLAGS.contains(&l.as_str()) {
            flags.push(l.clone());
            continue;
        }
        let (tags, text) = strip_tags(l);
        let variants: Option<Vec<u32>> = tags
            .iter()
            .find_map(|(k, v)| (k == "variant").then(|| v.split(',').filter_map(|n| n.trim().parse().ok()).collect()));
        if let Some(vs) = &variants {
            if !picked.iter().any(|p| vs.contains(p)) {
                continue;
            }
        }
        let has = |t: &str| tags.iter().any(|(k, _)| k == t);
        let kind = if has("enchant") {
            Some(Kind::Enchant)
        } else if has("crafted") {
            Some(Kind::Crafted)
        } else if has("fractured") {
            Some(Kind::Fractured)
        } else if has("exarch") {
            Some(Kind::Exarch)
        } else if has("eater") {
            Some(Kind::Eater)
        } else if has("mutated") {
            Some(Kind::Mutated)
        } else if has("custom") {
            Some(Kind::Custom)
        } else {
            None
        };
        mods.push(Line { text, kind, picked: variants.is_some() && picks_variants });
    }
    let split = count.min(mods.len());
    let explicits = mods.split_off(split);
    // inside the implicit count, a crafted line is an enchant (anoints, cluster jewel enchants)
    let implicits = mods
        .into_iter()
        .map(|mut l| {
            if l.kind == Some(Kind::Crafted) {
                l.kind = Some(Kind::Enchant);
            }
            l
        })
        .collect();
    if base.is_none() {
        base = base_in(&name);
    }
    Some(Parsed { rarity, name, base, implicits, explicits, flags, influences, picks_variants })
}

/// "{variant:2}{range:0.5}(1-2)% x" → ([("variant","2"),("range","0.5")], "(1-2)% x")
fn strip_tags(line: &str) -> (Vec<(String, String)>, String) {
    let mut tags = Vec::new();
    let mut rest = line;
    while let Some(after) = rest.strip_prefix('{') {
        let Some(end) = after.find('}') else { break };
        let tag = &after[..end];
        let (k, v) = tag.split_once(':').unwrap_or((tag, ""));
        tags.push((k.to_string(), v.to_string()));
        rest = &after[end + 1..];
    }
    (tags, rest.trim().to_string())
}

/// The longest known base type inside a magic or normal item's name.
fn base_in(name: &str) -> Option<String> {
    stats().bases.iter().filter(|b| name.contains(b.as_str())).max_by_key(|b| b.len()).cloned()
}

/// A mod line as a trade template and its minimums: "+(80-89) to maximum Life" → ("+# to maximum Life", [80]).
/// A range counts from its bottom; a fixed value from 90% of it, as Awakened PoE Trade does.
fn template(text: &str) -> (String, Vec<f64>) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut mins = Vec::new();
    let number = |at: usize| -> Option<(f64, usize)> {
        let mut j = at;
        while j < chars.len() && (chars[j].is_ascii_digit() || (chars[j] == '.' && j > at)) {
            j += 1;
        }
        if j == at {
            return None;
        }
        chars[at..j].iter().collect::<String>().parse().ok().map(|v| (v, j))
    };
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            if let Some((a, j)) = number(i + 1) {
                if chars.get(j) == Some(&'-') {
                    if let Some((b, k)) = number(j + 1) {
                        if chars.get(k) == Some(&')') {
                            out.push('#');
                            mins.push(a.min(b));
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
        }
        if chars[i].is_ascii_digit() {
            if let Some((v, j)) = number(i) {
                out.push('#');
                mins.push((v * 0.9).floor());
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out, mins)
}

/// Trade's own words for a stat on a weapon or armour piece, where it differs from the global one.
fn local_slot(slot: &str) -> bool {
    matches!(slot, "Weapon 1" | "Weapon 2" | "Helmet" | "Body Armour" | "Gloves" | "Boots")
}

/// A filter for one mod line: its id, and a minimum when the line has values to compare.
fn filter_for(map: Table, text: &str, local: bool) -> Option<Value> {
    if let Some(f) = lookup(map, text, local, true) {
        return Some(f);
    }
    // PoB keeps older flask wording: "during Flask effect" is trade's "during Effect"
    let flask = text.replace("during Flask effect", "during Effect").replace("during Flask Effect", "during Effect");
    if flask != text {
        if let Some(f) = lookup(map, &flask, local, true) {
            return Some(f);
        }
    }
    // trade lists "reduced" stats as "increased" ones with negative values: only ask for the stat
    let increased = flask.replace(" reduced ", " increased ");
    if increased != flask {
        return lookup(map, &increased, local, false);
    }
    None
}

fn lookup((map, lower): Table, text: &str, local: bool, with_min: bool) -> Option<Value> {
    let get = |key: &str| map.get(key).or_else(|| lower.get(&key.to_lowercase()));
    let find = |key: &str| -> Option<&String> { local.then(|| get(&format!("{key} (Local)"))).flatten().or_else(|| get(key)) };
    // exact text first: cluster notables ("1 Added Passive Skill is Burning Bright"), small passive enchants
    if let Some(id) = find(text) {
        return Some(stat(id, None));
    }
    let (tpl, mins) = template(text);
    let (id, mins) = match find(&tpl) {
        Some(id) => (id, mins),
        // a fixed number that's part of the stat's wording: "for 4 seconds on Kill"
        None => {
            let (tpl, mins) = ranges_only(text);
            (find(&tpl)?, mins)
        }
    };
    let min = match mins.len() {
        _ if !with_min => None,
        0 => None,
        1 => Some(mins[0]),
        // "Adds # to # Fire Damage": trade compares the average
        _ => Some((mins.iter().sum::<f64>() / mins.len() as f64).floor()),
    };
    Some(stat(id, min))
}

/// Like `template`, but fixed numbers stay as written: "(8-12)% for 4 seconds" → "#% for 4 seconds".
fn ranges_only(text: &str) -> (String, Vec<f64>) {
    let mut out = String::new();
    let mut mins = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')').map(|c| open + c) else { break };
        let inner = &rest[open + 1..close];
        match inner.split_once('-').and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?))) {
            Some((a, b)) => {
                out.push_str(&rest[..open]);
                out.push('#');
                mins.push(a.min(b));
            }
            None => out.push_str(&rest[..=close]),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    (out, mins)
}

/// `{id, value}`; ids like "enchant.stat_3948993189|17" carry an option instead of a number.
fn stat(id: &str, min: Option<f64>) -> Value {
    match id.split_once('|') {
        Some((id, option)) => json!({ "id": id, "value": { "option": option.parse::<u64>().map(Value::from).unwrap_or(Value::from(option)) } }),
        None => match min {
            Some(m) => json!({ "id": id, "value": { "min": m } }),
            None => json!({ "id": id }),
        },
    }
}

const ELE: &str = "pseudo.pseudo_total_elemental_resistance";
const CHAOS: &str = "pseudo.pseudo_total_chaos_resistance";
const STR: &str = "pseudo.pseudo_total_strength";
const DEX: &str = "pseudo.pseudo_total_dexterity";
const INT: &str = "pseudo.pseudo_total_intelligence";

/// Life, resistance and attribute lines count toward trade's pseudo totals, so any mix of mods reaching them will do.
fn pseudo(tpl: &str) -> &'static [(&'static str, f64)] {
    match tpl {
        "+# to maximum Life" => &[("pseudo.pseudo_total_life", 1.0)],
        "+#% to Fire Resistance" | "+#% to Cold Resistance" | "+#% to Lightning Resistance" => &[(ELE, 1.0)],
        "+#% to Fire and Cold Resistances" | "+#% to Fire and Lightning Resistances" | "+#% to Cold and Lightning Resistances" => &[(ELE, 2.0)],
        "+#% to all Elemental Resistances" => &[(ELE, 3.0)],
        "+#% to Chaos Resistance" => &[(CHAOS, 1.0)],
        "+#% to Fire and Chaos Resistances" | "+#% to Cold and Chaos Resistances" | "+#% to Lightning and Chaos Resistances" => &[(ELE, 1.0), (CHAOS, 1.0)],
        "+# to Strength" => &[(STR, 1.0)],
        "+# to Dexterity" => &[(DEX, 1.0)],
        "+# to Intelligence" => &[(INT, 1.0)],
        "+# to Strength and Dexterity" => &[(STR, 1.0), (DEX, 1.0)],
        "+# to Strength and Intelligence" => &[(STR, 1.0), (INT, 1.0)],
        "+# to Dexterity and Intelligence" => &[(DEX, 1.0), (INT, 1.0)],
        "+# to all Attributes" => &[(STR, 1.0), (DEX, 1.0), (INT, 1.0)],
        _ => &[],
    }
}

/// The trade search for one item, and which of its lines it searches (None: not a line that could be).
pub struct Search {
    pub query: Value,
    pub implicits: Vec<Option<bool>>,
    pub explicits: Vec<Option<bool>>,
}

/// The decisions on map #64: uniques by name (plus picked variant lines); anything else by its
/// exact base and explicit lines, life/res/attributes as required pseudo totals, the rest any N−1.
pub fn search(item: &Parsed, slot: &str) -> Search {
    let mut required: Vec<Value> = Vec::new();
    let mut misc = serde_json::Map::new();
    let mut implicits = vec![None; item.implicits.len()];
    let mut explicits = vec![None; item.explicits.len()];
    let mut query = json!({ "status": { "option": "available" } });
    if item.rarity == Rarity::Unique {
        let name = item.name.strip_prefix("Foulborn ").unwrap_or(&item.name);
        if name.len() != item.name.len() {
            misc.insert("mutated".into(), json!({ "option": "true" }));
        }
        query["name"] = json!(name);
        if let Some(base) = &item.base {
            query["type"] = json!(base);
        }
        for (i, l) in item.explicits.iter().enumerate().filter(|(_, l)| l.picked) {
            let f = filter_for(explicit(), &l.text, false).map(|mut f| {
                // a picked line only needs to be there; its roll isn't the point
                if f["value"].get("option").is_none() {
                    if let Some(o) = f.as_object_mut() {
                        o.remove("value");
                    }
                }
                f
            });
            explicits[i] = Some(f.is_some());
            required.extend(f);
        }
    } else {
        if let Some(base) = &item.base {
            query["type"] = json!(base);
        }
        let cluster = item.base.as_deref().is_some_and(|b| b.ends_with("Cluster Jewel"));
        if cluster {
            for (i, l) in item.implicits.iter().enumerate() {
                let f = filter_for(enchant(), &l.text, false);
                implicits[i] = Some(f.is_some());
                required.extend(f);
            }
        }
        let local = local_slot(slot);
        let mut totals: Vec<(&str, f64)> = Vec::new();
        let mut rest = Vec::new();
        for (i, l) in item.explicits.iter().enumerate() {
            let (tpl, mins) = template(&l.text);
            let parts = pseudo(&tpl);
            if !parts.is_empty() && mins.len() == 1 {
                for (id, times) in parts {
                    match totals.iter_mut().find(|(t, _)| t == id) {
                        Some(t) => t.1 += mins[0] * times,
                        None => totals.push((id, mins[0] * times)),
                    }
                }
                explicits[i] = Some(true);
                continue;
            }
            let f = filter_for(explicit(), &l.text, local);
            explicits[i] = Some(f.is_some());
            rest.extend(f);
        }
        required.extend(totals.into_iter().map(|(id, v)| json!({ "id": id, "value": { "min": v.floor() } })));
        for inf in &item.influences {
            match inf.as_str() {
                "Searing Exarch" => drop(misc.insert("searing_item".into(), json!({ "option": "true" }))),
                "Eater of Worlds" => drop(misc.insert("tangled_item".into(), json!({ "option": "true" }))),
                other => required.push(json!({ "id": format!("pseudo.pseudo_has_{}_influence", other.to_lowercase()) })),
            }
        }
        let mut groups = Vec::new();
        if rest.len() > 2 {
            let n = rest.len() - 1;
            groups.push(json!({ "type": "count", "value": { "min": n }, "filters": rest }));
        } else {
            required.extend(rest);
        }
        groups.insert(0, json!({ "type": "and", "filters": required }));
        query["stats"] = json!(groups);
        query["filters"] = json!({ "type_filters": { "filters": { "rarity": { "option": "nonunique" } } } });
        if !misc.is_empty() {
            query["filters"]["misc_filters"] = json!({ "filters": misc });
        }
        return Search { query, implicits, explicits };
    }
    query["stats"] = json!([{ "type": "and", "filters": required }]);
    if !misc.is_empty() {
        query["filters"] = json!({ "misc_filters": { "filters": misc } });
    }
    Search { query, implicits, explicits }
}

/// The trade site's own short link: gzip of the query, base64url, after the league.
pub fn trade_url(league: &str, query: &Value) -> String {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gz.write_all(query.to_string().as_bytes()).expect("gzip to memory");
    let bytes = gz.finish().expect("gzip to memory");
    let league: String = league
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect();
    format!("https://www.pathofexile.com/trade/search/{league}/{}", URL_SAFE_NO_PAD.encode(bytes))
}

/// A line the guide's author typed as advice, not a mod ("Note: …", "T2 Suffixes + Catalysts").
/// PoB keeps them as mod lines that match nothing. Only for lines no trade stat matched: prose has
/// no number, or a sentence's punctuation, or a shouted word ("WARNING"), or starts with "Note".
fn is_note(line: &Line) -> bool {
    if line.kind.is_some() {
        return false;
    }
    let t = line.text.as_str();
    let shouted = t.split(|c: char| !c.is_alphanumeric()).any(|w| w.len() >= 3 && w.chars().all(|c| c.is_ascii_uppercase()));
    // "T2", "Ilvl" don't count as a mod's number; "25", "+1", "(8-12)" do
    let number = t.split_whitespace().any(|w| w.trim_start_matches(['+', '-', '(']).starts_with(|c: char| c.is_ascii_digit()));
    // a mod's brackets hold a range, "(8-12)"; the author's hold words, "(pure Arm Pref but not required)"
    let worded_brackets = t.split('(').skip(1).any(|r| r.split(')').next().is_some_and(|inside| inside.chars().any(|c| c.is_alphabetic())));
    t.to_lowercase().starts_with("note") || shouted || !number || worded_brackets || t.contains(['.', ',', '|', '?', '!', ':', '='])
}

#[derive(Clone, Debug, Serialize)]
pub struct ModRow {
    pub text: String,
    pub kind: Option<Kind>,
    /// Some(false): could have been searched but isn't (no trade stat matches it)
    pub searched: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Row {
    pub slot: String,
    /// the same item in this many slots or sockets
    pub copies: usize,
    pub rarity: Rarity,
    /// an unnamed rare goes by its base
    pub title: String,
    pub subtitle: String,
    /// uniques show no mods, only what's picked on them
    pub implicits: Vec<ModRow>,
    pub mods: Vec<ModRow>,
    pub flags: Vec<String>,
    pub influences: Vec<String>,
    /// the guide author's advice typed among the mods, shown apart from them
    pub notes: Vec<String>,
    pub trade: String,
}

fn row(slot: &str, copies: usize, item: &Parsed, league: &str) -> Row {
    let s = search(item, slot);
    let rows = |lines: &[Line], searched: &[Option<bool>]| -> Vec<ModRow> {
        lines.iter().zip(searched).map(|(l, s)| ModRow { text: l.text.clone(), kind: l.kind, searched: *s }).collect()
    };
    let unique = item.rarity == Rarity::Unique;
    let (title, subtitle) = match item.rarity {
        Rarity::Unique => (item.name.clone(), item.base.clone().unwrap_or_default()),
        Rarity::Rare if item.name == "New Item" => (item.base.clone().unwrap_or_else(|| item.name.clone()), "Rare".into()),
        Rarity::Rare => (item.name.clone(), item.base.clone().unwrap_or_default()),
        Rarity::Magic => (item.name.clone(), "Magic".into()),
        Rarity::Normal => (item.name.clone(), "Normal".into()),
    };
    // a unique's own lines go unsearched by design, so only an outright "Note…" counts there
    let note = |l: &Line, searched: &Option<bool>| {
        if unique { !l.picked && l.kind.is_none() && l.text.to_lowercase().starts_with("note") } else { *searched == Some(false) && is_note(l) }
    };
    let notes = item.explicits.iter().zip(&s.explicits).filter(|(l, s)| note(l, s)).map(|(l, _)| l.text.clone()).collect();
    let kept: Vec<(&Line, &Option<bool>)> = item.explicits.iter().zip(&s.explicits).filter(|(l, s)| !note(l, s)).collect();
    Row {
        slot: slot.to_string(),
        copies,
        rarity: item.rarity,
        title,
        subtitle,
        implicits: if unique { Vec::new() } else { rows(&item.implicits, &s.implicits) },
        mods: kept
            .into_iter()
            .filter(|(l, _)| !unique || l.picked)
            .map(|(l, s)| ModRow { text: l.text.clone(), kind: l.kind, searched: *s })
            .collect(),
        notes,
        flags: item.flags.clone(),
        influences: item.influences.clone(),
        trade: trade_url(league, &s.query),
    }
}

/// The Item set to show: the player's pick, else the one paired with the Tree spec, else the active one.
pub fn choose(build: &Build, spec: usize, picked: Option<usize>) -> Option<(usize, Chosen)> {
    if build.item_sets.is_empty() {
        return None;
    }
    if let Some(p) = picked.filter(|p| *p < build.item_sets.len()) {
        return Some((p, Chosen::Picked));
    }
    let titles: Vec<&str> = build.item_sets.iter().map(|s| s.title.as_str()).collect();
    match gems::pair(build, spec, &titles) {
        Some(i) => Some((i, Chosen::Paired)),
        None => Some((build.active_item_set.min(build.item_sets.len() - 1), Chosen::Active)),
    }
}

/// The section's three groups.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Groups {
    pub gear: Vec<Row>,
    pub flasks: Vec<Row>,
    pub jewels: Vec<Row>,
}

const GEAR: [&str; 13] = ["Weapon 1", "Weapon 2", "Helmet", "Body Armour", "Gloves", "Boots", "Amulet", "Ring 1", "Ring 2", "Ring 3", "Belt", "Graft 1", "Graft 2"];

/// The Item set's gear and flasks and the Tree spec's jewels, the same item in several places shown once.
pub fn groups(build: &Build, set: usize, spec: usize, league: &str) -> Groups {
    let items: HashMap<u32, Parsed> = build.items.iter().filter_map(|i| Some((i.id, parse(&i.text)?))).collect();
    let mut out = Groups::default();
    let Some(set) = build.item_sets.get(set) else { return out };
    let slot = |name: &str| set.slots.iter().find(|(n, _)| n == name).map(|(_, id)| *id);
    // gear in paper-doll order, abyss jewels after their gear piece; weapon swap is left out
    let mut gear: Vec<(String, u32)> = Vec::new();
    for g in GEAR {
        if let Some(id) = slot(g) {
            gear.push((g.to_string(), id));
        }
        for n in 1..=6 {
            if let Some(id) = slot(&format!("{g} Abyssal Socket {n}")) {
                gear.push((format!("{g} abyss"), id));
            }
        }
    }
    let flasks: Vec<(String, u32)> = (1..=5).filter_map(|n| slot(&format!("Flask {n}")).map(|id| (format!("Flask {n}"), id))).collect();
    let jewels: Vec<(String, u32)> = build
        .specs
        .get(spec)
        .map(|s| s.sockets.iter().filter(|(node, _)| s.nodes.contains(node)).map(|(_, id)| ("Jewel".to_string(), *id)).collect())
        .unwrap_or_default();
    let fold = |places: Vec<(String, u32)>| -> Vec<Row> {
        let mut seen: Vec<(u32, usize, String)> = Vec::new();
        for (name, id) in places {
            match seen.iter_mut().find(|(i, _, _)| *i == id) {
                Some(s) => s.1 += 1,
                None => seen.push((id, 1, name)),
            }
        }
        seen.into_iter().filter_map(|(id, n, name)| Some(row(&name, n, items.get(&id)?, league))).collect()
    };
    out.gear = fold(gear);
    out.flasks = fold(flasks);
    out.jewels = fold(jewels);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pob::decode;
    use std::io::Read;

    fn pohx() -> Build {
        decode(include_str!("../tests/pohx-rf.txt")).unwrap()
    }

    fn unzip(url: &str) -> Value {
        let code = url.rsplit('/').next().unwrap();
        let bytes = URL_SAFE_NO_PAD.decode(code).unwrap();
        let mut s = String::new();
        flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut s).unwrap();
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn reads_item_sets_and_sockets() {
        let b = pohx();
        assert_eq!(b.item_sets.len(), 9);
        assert_eq!(crate::pob::clean_title(&b.item_sets[b.active_item_set].title), "Level 1-30 {1}");
        assert!(b.items.len() > 100);
        let s10 = &b.specs[9];
        assert_eq!(s10.sockets.len(), 12);
    }

    #[test]
    fn item_sets_pair_with_tree_specs() {
        let b = pohx();
        let title = |spec: usize| crate::pob::clean_title(&b.item_sets[choose(&b, spec, None).unwrap().0].title);
        assert_eq!(title(1), "Level 1-30 {1}");
        assert_eq!(title(5), "Level 80-90 {5}");
        assert_eq!(title(9), "Lvl 90 Cogwork + Kalandra {10}");
        assert_eq!(choose(&b, 9, Some(0)).unwrap(), (0, Chosen::Picked));
    }

    #[test]
    fn templates_take_range_bottoms_and_90_percent() {
        assert_eq!(template("+120 to maximum Life"), ("+# to maximum Life".into(), vec![108.0]));
        assert_eq!(template("(14-16)% increased Area of Effect"), ("#% increased Area of Effect".into(), vec![14.0]));
        assert_eq!(template("Adds (10-12) to (20-24) Fire Damage"), ("Adds # to # Fire Damage".into(), vec![10.0, 20.0]));
        assert_eq!(template("Ignites you inflict spread to other Enemies within 1.2 metres").1, vec![1.0]);
    }

    #[test]
    fn parses_a_rare_with_tags() {
        let p = parse("Rarity: RARE\nNew Item\nVaal Gauntlets\nEater of Worlds Item\nImplicits: 1\n{eater}{range:0.5}(10-12)% increased Damage\n{fractured}+120 to maximum Life\n{crafted}(24-28)% increased Damage during any Flask Effect\nCorrupted").unwrap();
        assert_eq!(p.base.as_deref(), Some("Vaal Gauntlets"));
        assert_eq!(p.implicits[0].kind, Some(Kind::Eater));
        assert_eq!(p.explicits[0], Line { text: "+120 to maximum Life".into(), kind: Some(Kind::Fractured), picked: false });
        assert_eq!(p.explicits[1].kind, Some(Kind::Crafted));
        assert_eq!(p.flags, vec!["Corrupted"]);
        assert_eq!(p.influences, vec!["Eater of Worlds"]);
    }

    #[test]
    fn magic_items_are_searched_by_their_base() {
        let p = parse("Rarity: MAGIC\nFlagellant's Quartz Flask of the Curlew\nImplicits: 0\n(10-14)% increased Duration").unwrap();
        assert_eq!(p.base.as_deref(), Some("Quartz Flask"));
    }

    #[test]
    fn rare_search_folds_totals_and_wants_all_but_one_of_the_rest() {
        let p = parse("Rarity: RARE\nNew Item\nVaal Gauntlets\nImplicits: 0\n+120 to maximum Life\n+40% to Fire Resistance\n+(30-35)% to Cold and Lightning Resistances\n+33% to Chaos Resistance\n(14-16)% increased Area of Effect\n20% increased Life Regeneration rate\n(5-8)% increased Attack Speed\nT2 Suffixes + Catalysts").unwrap();
        let s = search(&p, "Gloves");
        let q = &s.query;
        assert_eq!(q["type"], "Vaal Gauntlets");
        assert_eq!(q["status"]["option"], "available");
        assert_eq!(q["filters"]["type_filters"]["filters"]["rarity"]["option"], "nonunique");
        let and = q["stats"][0]["filters"].as_array().unwrap();
        let min = |id: &str| and.iter().find(|f| f["id"] == id).map(|f| f["value"]["min"].as_f64().unwrap());
        assert_eq!(min("pseudo.pseudo_total_life"), Some(108.0));
        // 36 fire + 2 × 30 cold/lightning
        assert_eq!(min(ELE), Some(96.0));
        assert_eq!(min(CHAOS), Some(29.0));
        let count = &q["stats"][1];
        assert_eq!(count["type"], "count");
        assert_eq!(count["value"]["min"], 2);
        assert_eq!(count["filters"].as_array().unwrap().len(), 3);
        // gloves: attack speed is the local stat
        assert!(count["filters"].as_array().unwrap().iter().any(|f| f["id"] == stats().explicit["#% increased Attack Speed (Local)"]));
        assert_eq!(s.explicits.last(), Some(&Some(false)), "the author's note isn't searched");
    }

    #[test]
    fn cluster_jewels_need_their_enchants() {
        let p = parse("Rarity: RARE\nNew Item\nLarge Cluster Jewel\nImplicits: 3\n{crafted}Adds 8 Passive Skills\n{crafted}2 Added Passive Skills are Jewel Sockets\n{crafted}Added Small Passive Skills grant: 12% increased Fire Damage\n1 Added Passive Skill is Burning Bright\n1 Added Passive Skill is Prismatic Heart\n1 Added Passive Skill is Widespread Destruction").unwrap();
        let s = search(&p, "Jewel");
        assert_eq!(s.implicits, vec![Some(true); 3]);
        assert_eq!(s.explicits, vec![Some(true); 3]);
        let and = s.query["stats"][0]["filters"].as_array().unwrap();
        assert!(and.iter().any(|f| f["id"] == "enchant.stat_3948993189" && f["value"]["option"].is_number()));
        assert_eq!(s.query["stats"][1]["value"]["min"], 2);
    }

    #[test]
    fn uniques_go_by_name_and_picked_variants() {
        let b = pohx();
        let eye = b.items.iter().find(|i| i.text.contains("Watcher's Eye")).unwrap();
        let p = parse(&eye.text).unwrap();
        assert!(p.picks_variants);
        let picked: Vec<_> = p.explicits.iter().filter(|l| l.picked).collect();
        assert!(!picked.is_empty() && picked.len() <= 3, "{picked:?}");
        let s = search(&p, "Jewel");
        assert_eq!(s.query["name"], "Watcher's Eye");
        assert_eq!(s.query["type"], "Prismatic Jewel");
        assert!(s.query["stats"][0]["filters"].as_array().unwrap().iter().all(|f| f.get("value").is_none()));

        let red = parse("Rarity: UNIQUE\nFoulborn The Red Dream\nCrimson Jewel\nImplicits: 0\n{mutated}Gain 5% of Fire Damage as Extra Chaos Damage").unwrap();
        let s = search(&red, "Jewel");
        assert_eq!(s.query["name"], "The Red Dream");
        assert_eq!(s.query["filters"]["misc_filters"]["filters"]["mutated"]["option"], "true");
        assert!(s.query["stats"][0]["filters"].as_array().unwrap().is_empty());
    }

    #[test]
    fn shows_the_cogwork_set_with_jewels_from_the_tree() {
        let b = pohx();
        let set = choose(&b, 9, None).unwrap().0;
        let g = groups(&b, set, 9, "Keepers");
        let titles: Vec<&str> = g.gear.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"Nebulis") && titles.contains(&"Archdemon Crown") && titles.contains(&"Kalandra's Touch"), "{titles:?}");
        let crown = g.gear.iter().find(|r| r.title == "Archdemon Crown").unwrap();
        assert_eq!((crown.subtitle.as_str(), crown.slot.as_str()), ("Rare", "Helmet"));
        let nebulis = g.gear.iter().find(|r| r.title == "Nebulis").unwrap();
        assert!(nebulis.mods.is_empty() && nebulis.implicits.is_empty());
        assert_eq!(unzip(&nebulis.trade)["name"], "Nebulis");
        assert!(nebulis.trade.starts_with("https://www.pathofexile.com/trade/search/Keepers/"));
        assert_eq!(g.flasks.len(), 5);
        // 6 of spec {10}'s 12 sockets are taken
        assert_eq!(g.jewels.iter().map(|r| r.copies).sum::<usize>(), 6);
        assert!(g.jewels.iter().any(|r| r.title == "Watcher's Eye" && !r.mods.is_empty()));
    }

    #[test]
    fn copies_fold_into_one_row() {
        let b = pohx();
        let set = b.item_sets.iter().position(|s| {
            let r1 = s.slots.iter().find(|(n, _)| n == "Ring 1").map(|x| x.1);
            r1.is_some() && r1 == s.slots.iter().find(|(n, _)| n == "Ring 2").map(|x| x.1)
        });
        let g = groups(&b, set.unwrap(), 0, "Standard");
        assert!(g.gear.iter().any(|r| r.slot == "Ring 1" && r.copies == 2));
        // spec 1 (index 0) has no jewels
        assert!(g.jewels.is_empty());
    }

    #[test]
    fn falls_back_for_fixed_wording_old_flask_text_and_reduced() {
        let ex = explicit();
        let f = filter_for(ex, "(8-12)% chance to gain Onslaught for 4 seconds on Kill", false).unwrap();
        assert_eq!(f["value"]["min"], 8.0);
        assert!(filter_for(ex, "45% increased Armour during Flask effect", false).is_some());
        let r = filter_for(ex, "46% reduced Amount Recovered", false).unwrap();
        assert!(r.get("value").is_none());
    }

    #[test]
    fn the_authors_notes_stand_apart_from_mods() {
        let note = |t: &str| is_note(&Line { text: t.into(), kind: None, picked: false });
        for t in ["T2 Suffixes + Catalysts", "Note: If at 89/90% Max Res go with Nebulis.", "Double Multi", "+1 Fire | Fire Dmg or Multi are solid",
            "WARNING - If your chaos res is below 15% I'd use an Amethyst Flask", "1-2 Mod Sceptres are pretty solid, Combos of like", "Purchase Elder Helmet Ilvl 82+ (pure Arm Pref but not required)"] {
            assert!(note(t), "{t}");
        }
        assert!(!note("Gain 3 Charges when you are Hit by an Enemy"));
        assert!(!note("(8-12)% chance to gain Onslaught for 4 seconds on Kill"));
        assert!(!is_note(&Line { text: "Grants Level 30 Aspect of the Spider Skill".into(), kind: Some(Kind::Custom), picked: false }));

        let p = parse("Rarity: RARE\nNew Item\nCogwork Ring\nImplicits: 0\n+110 to maximum Life\nGain 3 Charges when you are Hit by an Enemy\nT2 Suffixes + Catalysts").unwrap();
        let r = row("Ring 1", 1, &p, "Keepers");
        assert_eq!(r.notes, vec!["T2 Suffixes + Catalysts"]);
        assert_eq!(r.mods.len(), 2);
        assert_eq!(r.mods[1].searched, Some(false));
    }

    #[test]
    fn capitals_dont_matter() {
        assert!(filter_for(explicit(), "Socketed Gems are Supported by Level 25 Trap And Mine Damage", false).is_some());
    }

    #[test]
    fn league_names_are_escaped() {
        assert!(trade_url("Hardcore Keepers", &json!({})).contains("/Hardcore%20Keepers/"));
    }
}
