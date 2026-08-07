use crate::model::{Kind, Node, State};
use crate::status::{clears_blocks_predecessor, is_terminal, listnodes};
use crate::times::stamp_touch_node;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub fn blocked_by(st: &State, id: &str) -> Vec<String> {
    st.edges
        .iter()
        .filter(|e| e.label == "blocks" && e.to == id)
        .map(|e| e.from.clone())
        .collect()
}

pub fn blocks_of(st: &State, id: &str) -> Vec<String> {
    st.edges
        .iter()
        .filter(|e| e.label == "blocks" && e.from == id)
        .map(|e| e.to.clone())
        .collect()
}

pub fn deps(st: &State, id: &str) -> Vec<String> {
    fn visit(st: &State, x: &str, seen: &mut BTreeSet<String>, order: &mut Vec<String>) {
        for p in blocked_by(st, x) {
            if seen.insert(p.clone()) {
                visit(st, &p, seen, order);
                order.push(p);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    visit(st, id, &mut seen, &mut order);
    order
}

pub fn impact(st: &State, id: &str) -> Vec<String> {
    fn visit(st: &State, x: &str, seen: &mut BTreeSet<String>, order: &mut Vec<String>) {
        for s in blocks_of(st, x) {
            if seen.insert(s.clone()) {
                order.push(s.clone());
                visit(st, &s, seen, order);
            }
        }
    }
    let mut seen = BTreeSet::new();
    let mut order = Vec::new();
    visit(st, id, &mut seen, &mut order);
    order
}

pub fn preds_clear(st: &State, id: &str) -> bool {
    for p in blocked_by(st, id) {
        match st.nodes.get(&p) {
            None => return false,
            Some(n) => {
                if !clears_blocks_predecessor(n) {
                    return false;
                }
            }
        }
    }
    true
}

pub fn ac_of(n: &Node) -> Vec<String> {
    n.lines("ac")
}

pub fn goals_of(n: &Node) -> Vec<String> {
    n.lines("goals")
}

pub fn asks_of(st: &State, w: &Node) -> Vec<String> {
    st.edges
        .iter()
        .filter(|e| e.label == "asks" && e.to == w.id)
        .map(|e| e.from.clone())
        .collect()
}

pub fn implements_of(st: &State, w: &Node) -> Vec<String> {
    st.edges
        .iter()
        .filter(|e| e.label == "implements" && e.from == w.id)
        .map(|e| e.to.clone())
        .collect()
}

pub fn bchain(st: &State, w: &Node) -> Vec<String> {
    let mut out = BTreeSet::new();
    for e in &st.edges {
        if e.label != "targets" || e.to != w.id {
            continue;
        }
        if let Some(fromn) = st.nodes.get(&e.from) {
            if fromn.kind == Kind::B {
                out.insert(e.from.clone());
            }
        }
    }
    for e in &st.edges {
        if e.label != "tests" {
            continue;
        }
        let bf = st.nodes.get(&e.from);
        let qt = st.nodes.get(&e.to);
        let (bf, qt) = match (bf, qt) {
            (Some(bf), Some(qt)) => (bf, qt),
            _ => continue,
        };
        if !(bf.kind == Kind::B && qt.kind == Kind::Q) {
            continue;
        }
        let linked = st
            .edges
            .iter()
            .any(|ed| ed.label == "asks" && ed.from == e.to && ed.to == w.id);
        if linked {
            out.insert(e.from.clone());
        }
    }
    out.into_iter().collect()
}

pub fn rederive_artifacts(st: &mut State) {
    let themed: BTreeMap<String, Vec<String>> = {
        let mut m: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for w in listnodes(st, Kind::W, false) {
            let theme = w.single("theme");
            if !theme.is_empty() {
                m.entry(theme).or_default().push(w.id.clone());
            }
        }
        m
    };
    let t_ids: Vec<String> = listnodes(st, Kind::T, false)
        .into_iter()
        .map(|a| a.id.clone())
        .collect();
    for tid in t_ids {
        let ws: Vec<&Node> = match themed.get(&tid) {
            None => Vec::new(),
            Some(ids) => ids.iter().filter_map(|id| st.nodes.get(id)).collect(),
        };
        let new_status = if ws.is_empty() {
            "open"
        } else if ws.iter().all(|w| is_terminal(Kind::W, &w.status)) {
            "done"
        } else {
            "open"
        };
        let a = st.nodes.get_mut(&tid).expect("t node listed");
        if a.status != new_status {
            a.status = new_status.to_string();
            stamp_touch_node(a);
        }
    }
}

pub fn active_discovery_surfaces(st: &State) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for x in listnodes(st, Kind::Y, false) {
        if x.status != "active" {
            continue;
        }
        out.extend(x.lines("surface"));
    }
    out
}

pub fn coverage(st: &State, w: &Node) -> (f64, Vec<String>, Vec<String>) {
    let surface_w = w.lines("surface");
    if surface_w.is_empty() {
        return (0.0, Vec::new(), Vec::new());
    }
    let act = active_discovery_surfaces(st);
    let mut covered: Vec<String> = surface_w
        .iter()
        .filter(|p| act.contains(*p))
        .cloned()
        .collect();
    covered.sort();
    let mut uncovered: Vec<String> = surface_w
        .iter()
        .filter(|p| !act.contains(*p))
        .cloned()
        .collect();
    uncovered.sort();
    let ratio = covered.len() as f64 / surface_w.len() as f64;
    (ratio, covered, uncovered)
}

pub struct TriageRow {
    pub w: String,
    pub title: String,
    pub cov: f64,
    pub declared: bool,
    pub uncertainty: i64,
    pub fragile: bool,
    pub suggestion: String,
}

pub fn triage_rows(st: &State) -> Vec<TriageRow> {
    let mut rows = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if is_terminal(Kind::W, &w.status) {
            continue;
        }
        let declared = !w.lines("surface").is_empty();
        let (cov, _, _) = coverage(st, w);
        let mut chi: i64 = 0;
        for q in asks_of(st, w) {
            if let Some(n) = st.nodes.get(&q) {
                if n.status == "open" {
                    chi += 1;
                }
            }
        }
        for b in bchain(st, w) {
            let ok = match st.nodes.get(&b) {
                None => false,
                Some(n) => n.status == "validated" || n.status == "invalidated_acceptable",
            };
            if !ok {
                chi += 1;
            }
        }
        chi += crate::dor::dor_breakdown(st, w, false)
            .iter()
            .filter(|t| !t.1)
            .count() as i64;
        let fragile = goal_fragility(st, w).iter().any(|t| t.1 <= 1);
        let suggestion = if !declared {
            "declare surface"
        } else if cov == 0.0 {
            "spike to create coverage"
        } else if chi > 0 {
            "resolve open Q/B and DoR gaps"
        } else if fragile {
            "add a redundant path (blocks)"
        } else if cov < 0.5 {
            "deepen coverage"
        } else {
            "ready to deliver"
        };
        rows.push(TriageRow {
            w: w.id.clone(),
            title: w.title.clone(),
            cov,
            declared,
            uncertainty: chi,
            fragile,
            suggestion: suggestion.to_string(),
        });
    }
    rows.sort_by(|a, b| {
        a.cov
            .partial_cmp(&b.cov)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.uncertainty.cmp(&a.uncertainty))
            .then(a.w.cmp(&b.w))
    });
    rows
}

pub fn critical_path(st: &State) -> Vec<String> {
    let active: BTreeSet<String> = listnodes(st, Kind::W, false)
        .into_iter()
        .filter(|w| !is_terminal(Kind::W, &w.status))
        .map(|w| w.id.clone())
        .collect();
    let mut succ: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut indeg: BTreeMap<String, i64> = active.iter().map(|id| (id.clone(), 0)).collect();
    for e in &st.edges {
        if e.label != "blocks" {
            continue;
        }
        if !active.contains(&e.from) || !active.contains(&e.to) {
            continue;
        }
        succ.entry(e.from.clone()).or_default().push(e.to.clone());
        *indeg.entry(e.to.clone()).or_insert(0) += 1;
    }
    let mut queue: BTreeSet<String> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut topo = Vec::new();
    while let Some(x) = queue.iter().next().cloned() {
        queue.remove(&x);
        topo.push(x.clone());
        if let Some(ss) = succ.get(&x) {
            for s in ss {
                if let Some(d) = indeg.get_mut(s) {
                    *d -= 1;
                    if *d == 0 {
                        queue.insert(s.clone());
                    }
                }
            }
        }
    }
    let mut dist: BTreeMap<String, i64> = active.iter().map(|id| (id.clone(), 1)).collect();
    let mut parent: BTreeMap<String, Option<String>> =
        active.iter().map(|id| (id.clone(), None)).collect();
    for x in &topo {
        if let Some(ss) = succ.get(x) {
            for s in ss {
                let cand = dist[x] + 1;
                if cand > dist[s] {
                    dist.insert(s.clone(), cand);
                    parent.insert(s.clone(), Some(x.clone()));
                }
            }
        }
    }
    if dist.is_empty() {
        return Vec::new();
    }
    let tail = active
        .iter()
        .min_by_key(|id| (-dist[*id], (*id).clone()))
        .cloned()
        .expect("active non-empty");
    let mut chain = Vec::new();
    let mut cur = Some(tail);
    while let Some(c) = cur {
        chain.push(c.clone());
        cur = parent[&c].clone();
    }
    chain.reverse();
    chain
}

pub struct ConeWalk {
    pub ids: Vec<String>,
    pub truncated: bool,
}

pub fn bounded_cone_walk(
    st: &State,
    id: &str,
    step: fn(&State, &str) -> Vec<String>,
    depth: usize,
    maxcount: usize,
) -> ConeWalk {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    seen.insert(id.to_string());
    let mut ids = Vec::new();
    let mut truncated = false;
    let mut frontier: Vec<String> = vec![id.to_string()];
    let mut hops = 0;
    while hops < depth && !frontier.is_empty() && !truncated {
        hops += 1;
        let mut level = Vec::new();
        for x in &frontier {
            for y in step(st, x) {
                if seen.contains(&y) {
                    continue;
                }
                seen.insert(y.clone());
                level.push(y);
            }
        }
        level.sort();
        let room = maxcount.saturating_sub(ids.len());
        if level.len() > room {
            ids.extend(level.into_iter().take(room));
            truncated = true;
        } else {
            ids.extend(level.iter().cloned());
            frontier = level;
        }
    }
    if !truncated && !frontier.is_empty() {
        truncated = frontier
            .iter()
            .any(|x| step(st, x).iter().any(|y| !seen.contains(y)));
    }
    ConeWalk { ids, truncated }
}

pub fn backward_cone(st: &State, id: &str, depth: usize, maxcount: usize) -> ConeWalk {
    bounded_cone_walk(st, id, blocked_by, depth, maxcount)
}

pub fn forward_cone(st: &State, id: &str, depth: usize, maxcount: usize) -> ConeWalk {
    bounded_cone_walk(st, id, blocks_of, depth, maxcount)
}

pub fn contraction_order(st: &State, ids: &[String]) -> Vec<String> {
    let keep: BTreeSet<String> = ids.iter().cloned().collect();
    let mut succ: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut indeg: BTreeMap<String, i64> = keep.iter().map(|id| (id.clone(), 0)).collect();
    for e in &st.edges {
        if e.label != "blocks" {
            continue;
        }
        if !keep.contains(&e.from) || !keep.contains(&e.to) {
            continue;
        }
        succ.entry(e.from.clone()).or_default().push(e.to.clone());
        *indeg.entry(e.to.clone()).or_insert(0) += 1;
    }
    let mut queue: BTreeSet<String> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::new();
    while let Some(x) = queue.iter().next().cloned() {
        queue.remove(&x);
        order.push(x.clone());
        if let Some(ss) = succ.get(&x) {
            for s in ss {
                if let Some(d) = indeg.get_mut(s) {
                    *d -= 1;
                    if *d == 0 {
                        queue.insert(s.clone());
                    }
                }
            }
        }
    }
    order
}

pub fn node_connectivity(st: &State, src: &str, dst: &str) -> i64 {
    if src == dst {
        return 0;
    }
    let sn = st.nodes.get(src);
    let dn = st.nodes.get(dst);
    let (sn, dn) = match (sn, dn) {
        (Some(sn), Some(dn)) => (sn, dn),
        _ => return 0,
    };
    if sn.archived || dn.archived {
        return 0;
    }
    let ids: Vec<String> = st
        .nodes
        .iter()
        .filter(|(_, n)| !n.archived)
        .map(|(id, _)| id.clone())
        .collect();
    let slot: BTreeMap<String, usize> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();
    let n = 2 * ids.len();
    let mut cap = vec![vec![0i64; n]; n];
    let unbounded = ids.len() as i64 + 1;
    for (id, i) in &slot {
        cap[2 * i][2 * i + 1] = if id == src || id == dst { unbounded } else { 1 };
    }
    for e in &st.edges {
        if e.label != "blocks" {
            continue;
        }
        let (u, v) = match (slot.get(&e.from), slot.get(&e.to)) {
            (Some(u), Some(v)) => (*u, *v),
            _ => continue,
        };
        cap[2 * u + 1][2 * v] = unbounded;
    }
    let source = 2 * slot[src] + 1;
    let sink = 2 * slot[dst];
    let mut flow = 0i64;
    loop {
        let mut prev = vec![0usize; n];
        let mut seen = vec![false; n];
        seen[source] = true;
        let mut queue = VecDeque::new();
        queue.push_back(source);
        while !queue.is_empty() && !seen[sink] {
            let u = queue.pop_front().expect("queue non-empty");
            for v in 0..n {
                if cap[u][v] <= 0 || seen[v] {
                    continue;
                }
                seen[v] = true;
                prev[v] = u;
                queue.push_back(v);
            }
        }
        if !seen[sink] {
            break;
        }
        let mut add = unbounded;
        let mut v = sink;
        while v != source {
            let u = prev[v];
            add = add.min(cap[u][v]);
            v = u;
        }
        let mut v = sink;
        while v != source {
            let u = prev[v];
            cap[u][v] -= add;
            cap[v][u] += add;
            v = u;
        }
        flow += add;
    }
    flow
}

pub fn goal_fragility(st: &State, w: &Node) -> Vec<(String, i64)> {
    let goals: BTreeSet<String> = goals_of(w).into_iter().collect();
    goals
        .into_iter()
        .map(|g| {
            let k = node_connectivity(st, &g, &w.id);
            (g, k)
        })
        .collect()
}

pub fn treewidth_upper(st: &State) -> usize {
    fn fill_missing(nbrs: &BTreeSet<String>, adj: &BTreeMap<String, BTreeSet<String>>) -> usize {
        let mut c = 0;
        for x in nbrs {
            for y in nbrs {
                if x < y && !adj[x].contains(y) {
                    c += 1;
                }
            }
        }
        c
    }
    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, n) in &st.nodes {
        if n.archived {
            continue;
        }
        adj.insert(id.clone(), BTreeSet::new());
    }
    for e in &st.edges {
        if e.from == e.to {
            continue;
        }
        if !adj.contains_key(&e.from) || !adj.contains_key(&e.to) {
            continue;
        }
        adj.get_mut(&e.from).expect("checked").insert(e.to.clone());
        adj.get_mut(&e.to).expect("checked").insert(e.from.clone());
    }
    let mut width = 0usize;
    while !adj.is_empty() {
        let pick = adj
            .keys()
            .min_by_key(|id| (fill_missing(&adj[id.as_str()], &adj), (*id).clone()))
            .expect("adj non-empty")
            .clone();
        let nbrs: Vec<String> = adj[&pick].iter().cloned().collect();
        width = width.max(nbrs.len());
        for i in 0..nbrs.len() {
            for j in i + 1..nbrs.len() {
                adj.get_mut(&nbrs[i])
                    .expect("neighbor in adj")
                    .insert(nbrs[j].clone());
                adj.get_mut(&nbrs[j])
                    .expect("neighbor in adj")
                    .insert(nbrs[i].clone());
            }
        }
        for x in &nbrs {
            adj.get_mut(x).expect("neighbor in adj").remove(&pick);
        }
        adj.remove(&pick);
    }
    width
}

pub fn discovery_anchor_count(
    st: &State,
    discovery: &Node,
    surfaces: &BTreeSet<String>,
    tags: &BTreeSet<String>,
    cone: &BTreeSet<String>,
) -> i64 {
    let mut anchors = 0i64;
    if discovery.lines("surface").iter().any(|s| surfaces.contains(s)) {
        anchors += 1;
    }
    if discovery.lines("tags").iter().any(|t| tags.contains(t)) {
        anchors += 1;
    }
    let linked = st.edges.iter().any(|e| {
        (e.from == discovery.id && cone.contains(&e.to))
            || (e.to == discovery.id && cone.contains(&e.from))
    });
    if linked {
        anchors += 1;
    }
    anchors
}

pub fn discovery_anchor_matches(
    st: &State,
    discovery: &Node,
    surfaces: &BTreeSet<String>,
    tags: &BTreeSet<String>,
    cone: &BTreeSet<String>,
) -> bool {
    discovery_anchor_count(st, discovery, surfaces, tags, cone) > 0
}

pub fn relevant_discoveries(st: &State, w: &Node, cone_ids: &[String], maxcount: usize) -> Vec<String> {
    let cone: BTreeSet<String> = cone_ids.iter().cloned().collect();
    let w_surface: BTreeSet<String> = w.lines("surface").into_iter().collect();
    let mut cone_tags: BTreeSet<String> = w.lines("tags").into_iter().collect();
    for id in &cone {
        if let Some(n) = st.nodes.get(id) {
            cone_tags.extend(n.lines("tags"));
        }
    }
    let mut scored: Vec<(i64, String)> = Vec::new();
    for discovery in listnodes(st, Kind::Y, false) {
        if discovery.status != "active" {
            continue;
        }
        let anchors = discovery_anchor_count(st, discovery, &w_surface, &cone_tags, &cone);
        if anchors > 0 {
            scored.push((-anchors, discovery.id.clone()));
        }
    }
    scored.sort();
    scored.into_iter().take(maxcount).map(|(_, id)| id).collect()
}

pub fn area_goals<'a>(st: &'a State, z: &Node) -> Vec<&'a Node> {
    listnodes(st, Kind::G, false)
        .into_iter()
        .filter(|g| g.single("area") == z.id)
        .collect()
}

pub fn area_work<'a>(st: &'a State, z: &Node) -> Vec<&'a Node> {
    let gids: BTreeSet<String> = area_goals(st, z).iter().map(|g| g.id.clone()).collect();
    listnodes(st, Kind::W, false)
        .into_iter()
        .filter(|w| goals_of(w).iter().any(|g| gids.contains(g)))
        .collect()
}

pub fn area_surface(st: &State, z: &Node) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = z.lines("surface").into_iter().collect();
    for w in area_work(st, z) {
        out.extend(w.lines("surface"));
    }
    out
}

pub fn area_tags(st: &State, z: &Node) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for n in area_goals(st, z).into_iter().chain(area_work(st, z)) {
        out.extend(n.lines("tags"));
    }
    out
}

pub fn area_node_ids(st: &State, z: &Node) -> BTreeSet<String> {
    let wids: BTreeSet<String> = area_work(st, z).iter().map(|w| w.id.clone()).collect();
    let mut out: BTreeSet<String> = area_goals(st, z).iter().map(|g| g.id.clone()).collect();
    out.extend(wids.iter().cloned());
    for n in st.nodes.values() {
        if n.archived {
            continue;
        }
        if !matches!(n.kind, Kind::Q | Kind::B | Kind::D) {
            continue;
        }
        let linked = st.edges.iter().any(|e| {
            (e.from == n.id && wids.contains(&e.to)) || (e.to == n.id && wids.contains(&e.from))
        });
        if linked {
            out.insert(n.id.clone());
        }
    }
    out
}

pub fn area_relevant_discoveries(st: &State, z: &Node) -> Vec<String> {
    let surfaces = area_surface(st, z);
    let tags = area_tags(st, z);
    let cone = area_node_ids(st, z);
    let mut scored: Vec<(i64, String)> = Vec::new();
    for discovery in listnodes(st, Kind::Y, false) {
        if discovery.status != "active" {
            continue;
        }
        let anchors = discovery_anchor_count(st, discovery, &surfaces, &tags, &cone);
        if anchors > 0 {
            scored.push((-anchors, discovery.id.clone()));
        }
    }
    scored.sort();
    scored.into_iter().map(|(_, id)| id).collect()
}
