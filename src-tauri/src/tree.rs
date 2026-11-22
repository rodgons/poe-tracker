// The bundled passive tree and the plan for one tree spec: which points come next,
// in what order, and what the corner panel draws.
use crate::pob::Spec;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

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
}

#[derive(Deserialize)]
pub struct Tree {
    pub version: String,
    skip: HashSet<u32>,
    /// the string ids Client.txt prints → node hash
    ids: HashMap<String, u32>,
    pub nodes: HashMap<u32, Node>,
}

impl Tree {
    /// The node a Client.txt passive line means: by its string id, or else by
    /// name among `candidates` (an id the bundled tree doesn't know yet).
    pub fn node_for_log(&self, id: &str, name: &str, candidates: impl IntoIterator<Item = u32>) -> Option<u32> {
        if let Some(hash) = self.ids.get(id) {
            return Some(*hash);
        }
        candidates.into_iter().find(|n| self.nodes.get(n).is_some_and(|node| node.name == name))
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
    masteries: HashMap<u32, u32>,
    /// spec nodes the current tree doesn't have (removed or moved by a patch)
    pub missing: usize,
    pub clusters: usize,
    pub tree_version: String,
}

#[derive(Clone, Serialize)]
pub struct DrawnNode {
    id: u32,
    x: i32,
    y: i32,
    k: &'static str,
    n: String,
    /// "alloc", "next", "plan" or "other"
    s: &'static str,
    /// position in the Next points, 1-based; 0 when not next
    i: usize,
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
    done: usize,
    total: usize,
    nodes: Vec<DrawnNode>,
    edges: Vec<(u32, u32)>,
    next: Vec<NextPoint>,
    refunds: Vec<String>,
    pub notices: Vec<String>,
    /// min x, min y, max x, max y of what's drawn
    bounds: (i32, i32, i32, i32),
}

impl Plan {
    pub fn new(tree: &Tree, spec: &Spec) -> Result<Plan, String> {
        let start = tree
            .nodes
            .iter()
            .find(|(_, n)| n.class_start == Some(spec.class_id))
            .map(|(id, _)| *id)
            .ok_or("The build's class isn't on the passive tree.")?;
        let mut nodes = HashSet::new();
        let (mut missing, mut clusters) = (0, 0);
        for &id in &spec.nodes {
            if id >= CLUSTER_MIN {
                clusters += 1;
            } else if tree.nodes.contains_key(&id) {
                if id != start {
                    nodes.insert(id);
                }
            } else if !tree.skip.contains(&id) {
                missing += 1;
            }
        }
        Ok(Plan {
            title: spec.title.clone(),
            start,
            nodes,
            masteries: spec.masteries.iter().copied().collect(),
            missing,
            clusters,
            tree_version: spec.tree_version.replace('_', "."),
        })
    }

    /// The spec's remaining points in the order to take them: walk to the nearest
    /// notable, keystone or mastery through spec nodes only, take the path, repeat;
    /// small points nothing leads past come last.
    pub fn order(&self, tree: &Tree, allocated: &HashSet<u32>) -> Vec<u32> {
        let mut have: HashSet<u32> = allocated.iter().copied().filter(|n| self.is_mine(*n)).collect();
        have.insert(self.start);
        let mut order = Vec::new();
        loop {
            let path = self
                .path_to_nearest(tree, &have, |n| tree.nodes[&n].kind != "small")
                .or_else(|| self.path_to_nearest(tree, &have, |_| true));
            let Some(path) = path else { break };
            have.extend(&path);
            order.extend(path);
        }
        order
    }

    fn is_mine(&self, id: u32) -> bool {
        id == self.start || self.nodes.contains(&id)
    }

    fn path_to_nearest(&self, tree: &Tree, have: &HashSet<u32>, want: impl Fn(u32) -> bool) -> Option<Vec<u32>> {
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
            // a mastery is reached from its notables but doesn't connect them
            if !have.contains(&cur) && tree.nodes[&cur].kind == "mastery" {
                continue;
            }
            for &next in &tree.nodes[&cur].edges {
                if self.nodes.contains(&next) && !prev.contains_key(&next) {
                    prev.insert(next, Some(cur));
                    queue.push_back(next);
                }
            }
        }
        None
    }

    /// Allocated nodes after taking the first `taken` points in order.
    pub fn seed(&self, tree: &Tree, taken: usize) -> HashSet<u32> {
        let mut have: HashSet<u32> = self.order(tree, &HashSet::new()).into_iter().take(taken).collect();
        have.insert(self.start);
        have
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
            let state = if allocated.contains(&id) || id == self.start {
                "alloc"
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
            notices,
            bounds,
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
        let have = plan.seed(t, 10);
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
        let mut have = plan.seed(t, 3);
        let stray = *t.nodes.keys().find(|n| !plan.nodes.contains(n) && **n != plan.start && t.nodes[n].kind == "notable").unwrap();
        have.insert(stray);
        let view = plan.view(t, &have, 5);
        assert_eq!(view.refunds, vec![t.nodes[&stray].name.clone()]);
        assert_eq!(view.done, 3);
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
    fn late_specs_skip_clusters_and_ascendancy() {
        let build = crate::pob::decode(include_str!("../tests/pohx-rf.txt")).unwrap();
        let plan = Plan::new(tree(), &build.specs[8]).unwrap();
        assert_eq!(plan.missing, 0);
        assert!(plan.clusters > 0);
        let view = plan.view(tree(), &plan.seed(tree(), 0), 5);
        assert!(view.notices.iter().any(|n| n.contains("cluster")));
    }
}
