// The bundled passive tree and the plan for one tree spec: which points come next,
// in what order, and what the corner panel draws.
use crate::pob::Spec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

/// Cluster jewel nodes live at these hashes and up; no tree data has them.
const CLUSTER_MIN: u32 = 65536;
/// Non-spec nodes this far outside the spec's bounds are left off the Mini-tree.
const CONTEXT_MARGIN: i32 = 900;

#[derive(Deserialize)]
pub struct Node {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "k")]
    pub kind: String,
    pub x: i32,
    pub y: i32,
    #[serde(rename = "e")]
    pub edges: Vec<u32>,
    #[serde(rename = "st", default)]
    pub stats: Vec<String>,
    #[serde(rename = "c")]
    pub class_start: Option<u32>,
    /// mastery effect id → its stat lines
    #[serde(rename = "m", default)]
    pub masteries: HashMap<String, Vec<String>>,
    /// the Ascendancy an Ascendancy node belongs to
    #[serde(rename = "a", default)]
    pub ascendancy: Option<String>,
}

#[derive(Deserialize)]
pub struct Tree {
    pub version: String,
    skip: HashSet<u32>,
    /// the string ids Client.txt prints → node hash
    ids: HashMap<String, u32>,
    pub nodes: HashMap<u32, Node>,
    /// the class Ascendancies' nodes, apart from the main tree
    pub asc: HashMap<u32, Node>,
}

impl Tree {
    /// A main-tree or Ascendancy node.
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.get(&id).or_else(|| self.asc.get(&id))
    }

    /// A node the app tracks: on the main tree or in an Ascendancy (not a Bloodline).
    pub fn knows(&self, id: u32) -> bool {
        self.node(id).is_some()
    }

    /// The node a Client.txt passive line means: by its string id, or else by
    /// name among `candidates` (an id the bundled tree doesn't know yet).
    pub fn node_for_log(&self, id: &str, name: &str, candidates: impl IntoIterator<Item = u32>) -> Option<u32> {
        if let Some(hash) = self.ids.get(id) {
            return Some(*hash);
        }
        candidates.into_iter().find(|n| self.node(*n).is_some_and(|node| node.name == name))
    }
}

pub fn tree() -> &'static Tree {
    static TREE: OnceLock<Tree> = OnceLock::new();
    TREE.get_or_init(|| serde_json::from_str(include_str!("../data/tree.json")).expect("bundled tree"))
}

pub struct Plan {
    pub title: String,
    pub start: u32,
    /// spec nodes on the current tree, minus the class start
    pub nodes: HashSet<u32>,
    /// spec sockets inside a large cluster socket: only a cluster jewel opens them,
    /// so they're the spec's but never suggested or counted
    cluster_sockets: HashSet<u32>,
    masteries: HashMap<u32, u32>,
    /// spec nodes the current tree doesn't have (removed or moved by a patch)
    pub missing: usize,
    pub clusters: usize,
    pub tree_version: String,
    /// the spec's Ascendancy; None when the spec takes no Ascendancy nodes
    pub asc: Option<AscPlan>,
}

/// The Ascendancy part of a Tree spec, numbered and counted apart from the main tree.
pub struct AscPlan {
    pub name: String,
    pub start: u32,
    /// spec nodes of the Ascendancy, minus its start and chosen options
    pub nodes: HashSet<u32>,
    /// a multiple-choice node → the option the spec picks there (no point of its own)
    options: HashMap<u32, u32>,
}

/// The Ascendancy inset: the spec's Ascendancy drawn apart from the main tree.
#[derive(Clone, Serialize)]
pub struct AscView {
    pub name: String,
    pub done: usize,
    pub total: usize,
    nodes: Vec<DrawnNode>,
    edges: Vec<(u32, u32)>,
    bounds: (i32, i32, i32, i32),
}

#[derive(Clone, Serialize)]
pub struct DrawnNode {
    id: u32,
    x: i32,
    y: i32,
    k: &'static str,
    n: String,
    /// "alloc", "refund", "next", "plan" or "other"
    s: &'static str,
    /// position in the Next points, 1-based; 0 when not next
    i: usize,
    /// the node's stats (a mastery's chosen effect), for hover details
    d: Vec<String>,
}

/// One point of the Tree spec in the list in the Main window's Tree section.
#[derive(Clone, Serialize)]
pub struct ListedPoint {
    id: u32,
    name: String,
    taken: bool,
}

#[derive(Clone, Serialize)]
pub struct NextPoint {
    name: String,
    kind: String,
    detail: String,
}

#[derive(Clone, Serialize)]
pub struct PanelView {
    spec: String,
    pub done: usize,
    total: usize,
    nodes: Vec<DrawnNode>,
    edges: Vec<(u32, u32)>,
    next: Vec<NextPoint>,
    refunds: Vec<String>,
    /// Pending refunds: Allocated nodes clicked to refund in game, not yet settled
    pub pending: Vec<u32>,
    /// what the Pending refunds are and when they settle
    pub pending_note: Option<String>,
    /// side notes (a Re-sync's result, nodes left out); nothing the player must act on
    pub notices: Vec<String>,
    /// why points have stopped ticking, when they have: the panel puts it in a banner
    pub stopped: Option<String>,
    /// what puts ticking right again: "resync" or "log"
    pub stopped_fix: Option<&'static str>,
    /// the Character the points tick for
    pub character: String,
    /// the Tree spec couldn't be read, so there's nothing to draw
    pub error: Option<String>,
    /// the Build's following Tree spec, named when this one is complete
    pub next_spec: Option<String>,
    /// min x, min y, max x, max y of what's drawn
    bounds: (i32, i32, i32, i32),
    /// the spec's Ascendancy inset; None when the spec has no Ascendancy
    pub asc: Option<AscView>,
}

/// The order to take `nodes` in from `have`: walk to the nearest notable, keystone or
/// mastery through `nodes` only, take the path, repeat; small points nothing leads
/// past come last.
fn walk(tree: &Tree, nodes: &HashSet<u32>, mut have: HashSet<u32>) -> Vec<u32> {
    let mut order = Vec::new();
    loop {
        let path = path_to_nearest(tree, nodes, &have, |n| tree.node(n).unwrap().kind != "small")
            .or_else(|| path_to_nearest(tree, nodes, &have, |_| true));
        let Some(path) = path else { break };
        have.extend(&path);
        order.extend(path);
    }
    order
}

fn path_to_nearest(
    tree: &Tree,
    nodes: &HashSet<u32>,
    have: &HashSet<u32>,
    want: impl Fn(u32) -> bool,
) -> Option<Vec<u32>> {
    let mut prev: HashMap<u32, Option<u32>> = have.iter().map(|n| (*n, None)).collect();
    // sorted, so equally near targets always break the same way
    let mut start: Vec<u32> = have.iter().copied().collect();
    start.sort_unstable();
    let mut queue: VecDeque<u32> = start.into();
    while let Some(cur) = queue.pop_front() {
        if !have.contains(&cur) && want(cur) {
            let mut path = vec![cur];
            let mut at = cur;
            while let Some(Some(p)) = prev.get(&at) {
                if have.contains(p) {
                    break;
                }
                path.push(*p);
                at = *p;
            }
            path.reverse();
            return Some(path);
        }
        let Some(node) = tree.node(cur) else { continue };
        // a mastery is reached from its notables but doesn't connect them
        if !have.contains(&cur) && node.kind == "mastery" {
            continue;
        }
        for &next in &node.edges {
            if nodes.contains(&next) && !prev.contains_key(&next) {
                prev.insert(next, Some(cur));
                queue.push_back(next);
            }
        }
    }
    None
}

impl AscPlan {
    /// The Ascendancy the spec's Ascendancy nodes belong to (the most of them, should
    /// a build mix two), with its points.
    fn new(tree: &Tree, ids: &[u32]) -> Option<AscPlan> {
        let mut count: HashMap<&str, usize> = HashMap::new();
        for id in ids {
            if let Some(a) = tree.asc[id].ascendancy.as_deref() {
                *count.entry(a).or_default() += 1;
            }
        }
        let name = count.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))?.0.to_string();
        let start = tree
            .asc
            .iter()
            .find(|(_, n)| n.kind == "start" && n.ascendancy.as_deref() == Some(&name))
            .map(|(id, _)| *id)?;
        let mut nodes = HashSet::new();
        let mut options = HashMap::new();
        for &id in ids {
            let node = &tree.asc[&id];
            if id == start || node.ascendancy.as_deref() != Some(&name) {
                continue;
            }
            if node.kind == "option" {
                // its one edge leads to the node it's chosen at
                if let Some(&parent) = node.edges.first() {
                    options.insert(parent, id);
                }
            } else {
                nodes.insert(id);
            }
        }
        Some(AscPlan { name, start, nodes, options })
    }

    fn is_mine(&self, id: u32) -> bool {
        id == self.start || self.nodes.contains(&id)
    }

    /// Whether `id` is a node of this Ascendancy.
    fn owns(&self, tree: &Tree, id: u32) -> bool {
        tree.asc.get(&id).is_some_and(|n| n.ascendancy.as_deref() == Some(&self.name))
    }

    /// The Ascendancy's remaining points in the order to take them.
    pub fn order(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<u32> {
        let mut have: HashSet<u32> = allocated.iter().copied().filter(|n| self.is_mine(*n)).collect();
        have.insert(self.start);
        walk(tree, &self.nodes, have)
    }

    /// Another Ascendancy the Character has nodes in, when theirs isn't the spec's.
    pub fn other<'t>(&self, tree: &'t Tree, allocated: &HashSet<u32>) -> Option<&'t str> {
        let mut others: Vec<&str> = allocated
            .iter()
            .filter_map(|n| tree.asc.get(n)?.ascendancy.as_deref())
            .filter(|a| *a != self.name)
            .collect();
        others.sort_unstable();
        others.first().copied()
    }

    fn label(&self, tree: &Tree, id: u32) -> String {
        let node = &tree.asc[&id];
        match self.options.get(&id) {
            Some(o) => format!("{} → {}", node.name, tree.asc[o].name),
            None => node.name.clone(),
        }
    }

    fn details(&self, tree: &Tree, id: u32) -> Vec<String> {
        let id = self.options.get(&id).copied().unwrap_or(id);
        tree.asc[&id].stats.clone()
    }

    /// Allocated Ascendancy nodes the spec doesn't contain, while the Character's
    /// Ascendancy is the spec's.
    fn refunds(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<String> {
        if self.other(tree, allocated).is_some() {
            return Vec::new();
        }
        allocated
            .iter()
            .filter(|n| self.owns(tree, **n) && !self.is_mine(**n) && tree.asc[n].kind != "option")
            .map(|n| self.label(tree, *n))
            .collect()
    }

    fn done(&self, allocated: &HashSet<u32>) -> usize {
        self.nodes.iter().filter(|n| allocated.contains(n)).count()
    }

    /// Every point of the Ascendancy: the taken ones first, then the rest in order.
    pub fn points(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<ListedPoint> {
        let listed = |id: u32, taken| ListedPoint { id, name: self.label(tree, id), taken };
        let mut taken: Vec<u32> = self.order(tree, &HashSet::new()).into_iter().filter(|n| allocated.contains(n)).collect();
        let mut rest: Vec<u32> = self.nodes.iter().copied().filter(|n| allocated.contains(n) && !taken.contains(n)).collect();
        rest.sort_unstable();
        taken.extend(rest);
        taken
            .into_iter()
            .map(|id| listed(id, true))
            .chain(self.order(tree, allocated).into_iter().map(|id| listed(id, false)))
            .collect()
    }

    /// Allocated Ascendancy nodes after taking its first `taken` points in order.
    pub fn seed(&self, tree: &Tree, taken: usize) -> HashSet<u32> {
        let mut have: HashSet<u32> = self.order(tree, &HashSet::new()).into_iter().take(taken).collect();
        if taken > 0 {
            have.insert(self.start);
        }
        have
    }

    fn view(&self, tree: &Tree, allocated: &HashSet<u32>) -> AscView {
        let order = self.order(tree, allocated);
        let mut drawn = Vec::new();
        let mut shown = HashSet::new();
        for (&id, node) in &tree.asc {
            if node.ascendancy.as_deref() != Some(&self.name) || node.kind == "option" {
                continue;
            }
            let state = if id == self.start || (allocated.contains(&id) && self.is_mine(id)) {
                "alloc"
            } else if allocated.contains(&id) {
                "refund"
            } else if self.nodes.contains(&id) {
                "next"
            } else {
                "other"
            };
            shown.insert(id);
            drawn.push(DrawnNode {
                id,
                x: node.x,
                y: node.y,
                k: kind_name(&node.kind),
                n: self.label(tree, id),
                s: state,
                i: order.iter().position(|n| *n == id).map_or(0, |p| p + 1),
                d: self.details(tree, id),
            });
        }
        drawn.sort_unstable_by_key(|n| n.id);
        let mut edges: Vec<(u32, u32)> = shown
            .iter()
            .flat_map(|&a| tree.asc[&a].edges.iter().map(move |&b| (a.min(b), a.max(b))))
            .filter(|(a, b)| shown.contains(a) && shown.contains(b))
            .collect();
        edges.sort_unstable();
        edges.dedup();
        let pad = 120;
        let bounds = (
            drawn.iter().map(|n| n.x).min().unwrap_or(0) - pad,
            drawn.iter().map(|n| n.y).min().unwrap_or(0) - pad,
            drawn.iter().map(|n| n.x).max().unwrap_or(0) + pad,
            drawn.iter().map(|n| n.y).max().unwrap_or(0) + pad,
        );
        AscView {
            name: self.name.clone(),
            done: self.done(allocated),
            total: self.nodes.len(),
            nodes: drawn,
            edges,
            bounds,
        }
    }
}

impl Plan {
    /// `Plan::new` on the bundled tree, remembered per spec: every tick, view and Tree spec
    /// list needs the plans, and a spec's plan never changes while the app runs.
    pub fn cached(spec: &Spec) -> Result<Arc<Plan>, String> {
        type Plans = Mutex<HashMap<u64, Result<Arc<Plan>, String>>>;
        static PLANS: OnceLock<Plans> = OnceLock::new();
        let key = {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&spec.title, &spec.tree_version, spec.class_id, &spec.nodes, &spec.masteries).hash(&mut h);
            h.finish()
        };
        let plans = PLANS.get_or_init(Default::default);
        if let Some(plan) = plans.lock().unwrap().get(&key) {
            return plan.clone();
        }
        let plan = Plan::new(tree(), spec).map(Arc::new);
        let mut plans = plans.lock().unwrap();
        // a few builds' specs at most; pasting many builds in one session starts it afresh
        if plans.len() >= 64 {
            plans.clear();
        }
        plans.insert(key, plan.clone());
        plan
    }

    pub fn new(tree: &Tree, spec: &Spec) -> Result<Plan, String> {
        let start = tree
            .nodes
            .iter()
            .find(|(_, n)| n.class_start == Some(spec.class_id))
            .map(|(id, _)| *id)
            .ok_or("The build's class isn't on the passive tree.")?;
        let mut nodes = HashSet::new();
        let mut asc = Vec::new();
        let (mut missing, mut clusters) = (0, 0);
        for &id in &spec.nodes {
            if id >= CLUSTER_MIN {
                clusters += 1;
            } else if tree.nodes.contains_key(&id) {
                if id != start {
                    nodes.insert(id);
                }
            } else if tree.asc.contains_key(&id) {
                asc.push(id);
            } else if !tree.skip.contains(&id) {
                missing += 1;
            }
        }
        let mut plan = Plan {
            title: spec.title.clone(),
            start,
            nodes,
            cluster_sockets: HashSet::new(),
            masteries: spec.masteries.iter().copied().collect(),
            missing,
            clusters,
            tree_version: spec.tree_version.replace('_', "."),
            asc: AscPlan::new(tree, &asc),
        };
        // a socket no path of spec nodes leads to sits inside a cluster jewel
        let reached: HashSet<u32> = plan.order(tree, &HashSet::new()).into_iter().collect();
        let stranded: Vec<u32> =
            plan.nodes.iter().copied().filter(|n| !reached.contains(n) && tree.nodes[n].kind == "jewel").collect();
        for n in stranded {
            plan.nodes.remove(&n);
            plan.cluster_sockets.insert(n);
            plan.clusters += 1;
        }
        Ok(plan)
    }

    /// The spec's remaining main-tree points in the order to take them.
    pub fn order(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<u32> {
        let mut have: HashSet<u32> = allocated.iter().copied().filter(|n| self.is_mine(*n)).collect();
        have.insert(self.start);
        walk(tree, &self.nodes, have)
    }

    fn is_mine(&self, id: u32) -> bool {
        id == self.start || self.nodes.contains(&id) || self.cluster_sockets.contains(&id)
    }

    /// Names a Client.txt line could mean: the next main-tree and Ascendancy points.
    pub fn candidates(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<u32> {
        let mut c = self.order(tree, allocated);
        if let Some(a) = &self.asc {
            c.extend(a.order(tree, allocated));
        }
        c
    }

    /// Allocated nodes after taking the first `taken` main-tree points and the first
    /// `asc_taken` Ascendancy points in order.
    pub fn seed(&self, tree: &Tree, taken: usize, asc_taken: usize) -> HashSet<u32> {
        let mut have: HashSet<u32> = self.order(tree, &HashSet::new()).into_iter().take(taken).collect();
        have.insert(self.start);
        if let Some(a) = &self.asc {
            have.extend(a.seed(tree, asc_taken));
        }
        have
    }

    /// Allocated nodes from a pathofexile.com import: every main-tree and Ascendancy
    /// node the character has (off-plan ones become Refunds), plus the class start.
    pub fn from_import(&self, tree: &Tree, hashes: &[u32]) -> HashSet<u32> {
        let mut have: HashSet<u32> = hashes.iter().copied().filter(|n| tree.knows(*n)).collect();
        have.insert(self.start);
        have
    }

    /// Every point of the spec: the taken ones first, then the rest in the order to take them.
    pub fn points(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<ListedPoint> {
        let listed = |id: u32, taken| ListedPoint { id, name: self.label(tree, id), taken };
        let mut taken: Vec<u32> = self.order(tree, &HashSet::new()).into_iter().filter(|n| allocated.contains(n)).collect();
        // taken points the from-scratch order skips (it only lists what's reachable)
        let mut rest: Vec<u32> = self.nodes.iter().copied().filter(|n| allocated.contains(n) && !taken.contains(n)).collect();
        rest.sort_unstable();
        taken.extend(rest);
        taken
            .into_iter()
            .map(|id| listed(id, true))
            .chain(self.order(tree, allocated).into_iter().map(|id| listed(id, false)))
            .collect()
    }

    fn details(&self, tree: &Tree, id: u32) -> Vec<String> {
        let node = &tree.nodes[&id];
        match self.masteries.get(&id).and_then(|e| node.masteries.get(&e.to_string())) {
            Some(stats) => stats.clone(),
            None => node.stats.clone(),
        }
    }

    fn label(&self, tree: &Tree, id: u32) -> String {
        let node = &tree.nodes[&id];
        match self.masteries.get(&id).and_then(|e| node.masteries.get(&e.to_string())) {
            Some(stats) => format!("{} → {}", node.name, stats.join(" / ")),
            None => node.name.clone(),
        }
    }

    pub fn view(&self, tree: &Tree, allocated: &HashSet<u32>, count: usize) -> PanelView {
        let order = self.order(tree, allocated);
        let next: Vec<u32> = order.iter().take(count).copied().collect();
        let done = self.nodes.iter().filter(|n| allocated.contains(n)).count();

        let mine: Vec<&Node> = self.nodes.iter().chain([&self.start]).map(|n| &tree.nodes[n]).collect();
        let (x0, x1) = (mine.iter().map(|n| n.x).min().unwrap(), mine.iter().map(|n| n.x).max().unwrap());
        let (y0, y1) = (mine.iter().map(|n| n.y).min().unwrap(), mine.iter().map(|n| n.y).max().unwrap());
        let bounds = (x0 - CONTEXT_MARGIN, y0 - CONTEXT_MARGIN, x1 + CONTEXT_MARGIN, y1 + CONTEXT_MARGIN);
        let inside = |n: &Node| n.x >= bounds.0 && n.x <= bounds.2 && n.y >= bounds.1 && n.y <= bounds.3;

        let mut drawn = Vec::new();
        let mut shown = HashSet::new();
        for (&id, node) in &tree.nodes {
            let state = if id == self.start || (allocated.contains(&id) && self.is_mine(id)) {
                "alloc"
            } else if allocated.contains(&id) {
                "refund"
            } else if next.contains(&id) {
                "next"
            } else if self.nodes.contains(&id) {
                "plan"
            } else if inside(node) && node.kind != "mastery" {
                "other"
            } else {
                continue;
            };
            shown.insert(id);
            drawn.push(DrawnNode {
                id,
                x: node.x,
                y: node.y,
                k: kind_name(&node.kind),
                n: self.label(tree, id),
                s: state,
                i: next.iter().position(|n| *n == id).map_or(0, |p| p + 1),
                d: self.details(tree, id),
            });
        }
        let mut edges: Vec<(u32, u32)> = shown
            .iter()
            .flat_map(|&a| tree.nodes[&a].edges.iter().map(move |&b| (a.min(b), a.max(b))))
            .filter(|(a, b)| shown.contains(a) && shown.contains(b))
            .filter(|(a, b)| tree.nodes[a].kind != "mastery" && tree.nodes[b].kind != "mastery")
            .collect();
        edges.sort_unstable();
        edges.dedup();

        let mut refunds: Vec<String> = allocated
            .iter()
            .filter(|n| !self.is_mine(**n) && tree.nodes.contains_key(n))
            .map(|n| self.label(tree, *n))
            .collect();
        if let Some(a) = &self.asc {
            refunds.extend(a.refunds(tree, allocated));
        }
        refunds.sort();

        let mut notices = Vec::new();
        if self.missing > 0 && self.tree_version != tree.version {
            notices.push(format!(
                "Built on {}, shown on {}: {} nodes not in this tree",
                self.tree_version, tree.version, self.missing
            ));
        }
        if self.clusters > 0 {
            notices.push(format!("{} cluster nodes not shown", self.clusters));
        }
        if let Some((a, other)) = self.asc.as_ref().and_then(|a| Some((a, a.other(tree, allocated)?))) {
            notices.push(format!("Your Character is a {other}, but this tree spec is for a {}", a.name));
        }

        PanelView {
            spec: self.title.clone(),
            done,
            total: self.nodes.len(),
            nodes: drawn,
            edges,
            next: next
                .iter()
                .map(|&id| NextPoint {
                    name: self.label(tree, id),
                    kind: tree.nodes[&id].kind.clone(),
                    detail: tree.nodes[&id].stats.join(" · "),
                })
                .collect(),
            refunds,
            pending: Vec::new(),
            pending_note: None,
            notices,
            stopped: None,
            stopped_fix: None,
            character: String::new(),
            error: None,
            next_spec: None,
            bounds,
            asc: self.asc.as_ref().map(|a| a.view(tree, allocated)),
        }
    }
}

impl PanelView {
    /// A Tree spec that can't be drawn: the panel says why instead of claiming there's no build.
    pub fn unreadable(spec: &str, error: String) -> PanelView {
        PanelView {
            spec: spec.to_string(),
            done: 0,
            total: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
            next: Vec::new(),
            refunds: Vec::new(),
            pending: Vec::new(),
            pending_note: None,
            notices: Vec::new(),
            stopped: None,
            stopped_fix: None,
            character: String::new(),
            error: Some(error),
            next_spec: None,
            bounds: (0, 0, 0, 0),
            asc: None,
        }
    }
}

fn kind_name(kind: &str) -> &'static str {
    match kind {
        "start" => "start",
        "keystone" => "keystone",
        "mastery" => "mastery",
        "jewel" => "jewel",
        "notable" => "notable",
        _ => "small",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lvl_01_30() -> Plan {
        let build = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        Plan::new(tree(), &build.specs[1]).unwrap()
    }

    #[test]
    fn orders_every_point_of_the_spec() {
        let plan = lvl_01_30();
        let order = plan.order(tree(), &HashSet::new());
        assert_eq!(order.len(), plan.nodes.len());
        assert_eq!(plan.nodes.len(), 33);
        assert_eq!(plan.missing, 0);
    }

    #[test]
    fn next_points_follow_the_seed() {
        let t = tree();
        let plan = lvl_01_30();
        let full = plan.order(t, &HashSet::new());
        let have = plan.seed(t, 10, 0);
        assert_eq!(have.len(), 11);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.done, 10);
        assert_eq!(view.next.len(), 5);
        assert_eq!(view.next[0].name, t.nodes[&full[10]].name);
        assert!(view.refunds.is_empty());
    }

    #[test]
    fn off_plan_nodes_are_refunds() {
        let t = tree();
        let plan = lvl_01_30();
        let mut have = plan.seed(t, 3, 0);
        let stray = *t.nodes.keys().find(|n| !plan.nodes.contains(n) && **n != plan.start && t.nodes[n].kind == "notable").unwrap();
        have.insert(stray);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.refunds, vec![t.nodes[&stray].name.clone()]);
        assert_eq!(view.done, 3);
        assert_eq!(view.nodes.iter().find(|n| n.id == stray).unwrap().s, "refund");
    }

    #[test]
    fn imports_keep_main_tree_nodes() {
        let t = tree();
        let plan = lvl_01_30();
        let first: Vec<u32> = plan.order(t, &HashSet::new()).into_iter().take(4).collect();
        let bloodline = *t.skip.iter().next().unwrap();
        let stray = *t.nodes.keys().find(|n| !plan.nodes.contains(n) && **n != plan.start && t.nodes[n].kind == "notable").unwrap();
        let mut hashes = first.clone();
        hashes.extend([bloodline, stray, 70000]);
        let have = plan.from_import(t, &hashes);
        assert_eq!(have.len(), 6);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.done, 4);
        assert_eq!(view.refunds, vec![t.nodes[&stray].name.clone()]);
    }

    #[test]
    fn lists_taken_points_then_the_rest_in_order() {
        let t = tree();
        let plan = lvl_01_30();
        let have = plan.seed(t, 10, 0);
        let points = plan.points(t, &have);
        assert_eq!(points.len(), plan.nodes.len());
        assert!(points[..10].iter().all(|p| p.taken));
        assert!(points[10..].iter().all(|p| !p.taken));
        assert_eq!(points[10].id, plan.order(t, &have)[0]);
    }

    #[test]
    fn log_ids_map_to_nodes() {
        let t = tree();
        assert_eq!(t.node_for_log("life_regeneration1087", "Life Regeneration", []), Some(55906));
        let plan = lvl_01_30();
        let next = plan.order(t, &HashSet::new());
        let name = t.nodes[&next[0]].name.clone();
        assert_eq!(t.node_for_log("renamed_in_a_patch1", &name, next.iter().copied()), Some(next[0]));
        assert_eq!(t.node_for_log("unknown1", "No Such Node", next), None);
    }

    #[test]
    fn late_specs_skip_clusters() {
        let build = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        let plan = Plan::new(tree(), &build.specs[8]).unwrap();
        assert_eq!(plan.missing, 0);
        assert!(plan.clusters > 0);
        let view = plan.view(tree(), &plan.seed(tree(), 0, 0), 5);
        assert!(view.notices.iter().any(|n| n.contains("cluster")));
    }

    #[test]
    fn cluster_sockets_are_neither_counted_nor_refunds() {
        let t = tree();
        let build = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        let plan = Plan::new(t, &build.specs[8]).unwrap();
        assert_eq!(plan.cluster_sockets.len(), 2);
        // every counted point can be walked to, so the spec can finish
        let all = plan.seed(t, usize::MAX, 0);
        assert_eq!(all.len(), plan.nodes.len() + 1);
        let mut have = all.clone();
        have.extend(&plan.cluster_sockets);
        let view = plan.view(t, &have, 5);
        assert_eq!((view.done, view.total), (plan.nodes.len(), plan.nodes.len()));
        assert!(view.next.is_empty() && view.refunds.is_empty());
    }

    fn spec(i: usize) -> Plan {
        let build = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        Plan::new(tree(), &build.specs[i]).unwrap()
    }

    /// a Chieftain node the spec doesn't take, and one of another Ascendancy
    fn strays(plan: &Plan) -> (u32, u32) {
        let t = tree();
        let a = plan.asc.as_ref().unwrap();
        let mut off: Vec<u32> = t.asc.keys().copied().filter(|n| a.owns(t, *n) && !a.is_mine(*n)).collect();
        off.sort_unstable();
        let mut jugg: Vec<u32> = t.asc.iter().filter(|(_, n)| n.ascendancy.as_deref() == Some("Juggernaut") && n.kind != "start").map(|(id, _)| *id).collect();
        jugg.sort_unstable();
        (off[0], jugg[0])
    }

    #[test]
    fn specs_without_ascendancy_nodes_have_no_ascendancy() {
        let t = tree();
        let plan = lvl_01_30();
        assert!(plan.asc.is_none());
        // a Character that has ascended already isn't told to refund it
        let mut have = plan.seed(t, 3, 0);
        let (stray, _) = strays(&spec(8));
        have.insert(stray);
        let view = plan.view(t, &have, 5);
        assert!(view.asc.is_none() && view.refunds.is_empty());
    }

    #[test]
    fn the_ascendancy_is_counted_and_numbered_apart() {
        let t = tree();
        let plan = spec(8);
        let a = plan.asc.as_ref().unwrap();
        assert_eq!(a.name, "Chieftain");
        assert_eq!(a.nodes.len(), 8);
        assert_eq!(a.order(t, &HashSet::new()).len(), 8);
        let have = plan.seed(t, 20, 2);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.done, 20);
        assert_eq!(view.total, plan.nodes.len());
        assert!(view.next.iter().all(|n| !a.nodes.iter().any(|m| t.asc[m].name == n.name)));
        let asc = view.asc.unwrap();
        assert_eq!((asc.done, asc.total), (2, 8));
        // every remaining Ascendancy point is numbered, 1 to 6
        let mut nums: Vec<usize> = asc.nodes.iter().filter(|n| n.s == "next").map(|n| n.i).collect();
        nums.sort_unstable();
        assert_eq!(nums, (1..=6).collect::<Vec<_>>());
        assert_eq!(asc.nodes.iter().find(|n| n.id == a.start).unwrap().s, "alloc");
        let points = a.points(t, &have);
        assert_eq!(points.len(), 8);
        assert!(points[..2].iter().all(|p| p.taken) && points[2..].iter().all(|p| !p.taken));
    }

    #[test]
    fn before_the_first_lab_the_ascendancy_shows_from_zero() {
        let t = tree();
        let plan = spec(2);
        let view = plan.view(t, &plan.seed(t, 0, 0), 5);
        let asc = view.asc.unwrap();
        assert_eq!((asc.name.as_str(), asc.done, asc.total), ("Chieftain", 0, 2));
    }

    #[test]
    fn off_plan_ascendancy_nodes_are_refunds() {
        let t = tree();
        let plan = spec(8);
        let (stray, _) = strays(&plan);
        let mut have = plan.seed(t, 5, 2);
        have.insert(stray);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.refunds, vec![t.asc[&stray].name.clone()]);
        assert_eq!(view.asc.unwrap().nodes.iter().find(|n| n.id == stray).unwrap().s, "refund");
    }

    #[test]
    fn another_ascendancy_gets_one_notice_not_refunds() {
        let t = tree();
        let plan = spec(8);
        let (_, jugg) = strays(&plan);
        let mut have = plan.seed(t, 5, 0);
        have.insert(jugg);
        let view = plan.view(t, &have, 5);
        assert!(view.refunds.is_empty());
        assert!(view.notices.iter().any(|n| n.contains("Juggernaut") && n.contains("Chieftain")));
        assert_eq!(view.asc.unwrap().done, 0);
    }

    #[test]
    fn imports_keep_ascendancy_nodes() {
        let t = tree();
        let plan = spec(8);
        let a = plan.asc.as_ref().unwrap();
        let first: Vec<u32> = a.order(t, &HashSet::new()).into_iter().take(4).collect();
        let bloodline = *t.skip.iter().next().unwrap();
        let mut hashes = first.clone();
        hashes.extend([a.start, bloodline]);
        let have = plan.from_import(t, &hashes);
        assert!(first.iter().all(|n| have.contains(n)) && !have.contains(&bloodline));
        assert_eq!(plan.view(t, &have, 5).asc.unwrap().done, 4);
    }

    #[test]
    fn log_lines_find_ascendancy_nodes() {
        let t = tree();
        let plan = spec(8);
        let a = plan.asc.as_ref().unwrap();
        let first = a.order(t, &HashSet::new())[0];
        let id = t.ids.iter().find(|(_, h)| **h == first).unwrap().0.clone();
        assert_eq!(t.node_for_log(&id, "", []), Some(first));
        let name = t.asc[&first].name.clone();
        assert_eq!(t.node_for_log("renamed1", &name, plan.candidates(t, &HashSet::new())), Some(first));
    }
}
