use crate::algebra::{bchain, blocked_by};
use crate::dor::dor;
use crate::model::{Edge, Kind, Node, State};
use crate::session::progress_has_session_record;
use crate::status::{clears_blocks_predecessor, listnodes, prose_field_nonempty, EDGE_LABELS};
use crate::times::{stamp_new_edge, stamp_touch_node};
use std::collections::{BTreeMap, BTreeSet};

pub const WIP_LIMIT_DEFAULT: i64 = 2;

pub fn i1_dor_on_progress(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "progress" {
            continue;
        }
        if !dor(st, w, true) {
            out.push(format!("I1: {} is `progress` but DoR ≢ ⊤", w.id));
        }
    }
    out
}

pub fn i2_spike_outputs(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if !(w.wtype.as_deref() == Some("spike") && w.status == "done") {
            continue;
        }
        let has_any = st
            .edges
            .iter()
            .any(|e| e.label == "produces" && e.from == w.id);
        if !has_any {
            out.push(format!(
                "I2: {} is a done spike but `produces` is empty (no outgoing `produces` edges)",
                w.id
            ));
        }
    }
    out
}

pub fn i3_done_has_evidence(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "done" {
            continue;
        }
        if w.lines("evidence").is_empty() {
            out.push(format!("I3: {} is `done` but `evidence` is empty", w.id));
        }
    }
    out
}

pub fn i4_wip_limit(st: &State) -> Vec<String> {
    i4_wip_limit_with(st, WIP_LIMIT_DEFAULT)
}

pub fn i4_wip_limit_with(st: &State, limit: i64) -> Vec<String> {
    let n = listnodes(st, Kind::W, false)
        .iter()
        .filter(|w| w.status == "progress")
        .count() as i64;
    if n > limit {
        vec![format!("I4: WIP {} exceeds limit {}", n, limit)]
    } else {
        Vec::new()
    }
}

pub fn i5_blocks_terminal(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "progress" {
            continue;
        }
        for p in blocked_by(st, &w.id) {
            match st.nodes.get(&p) {
                None => {
                    out.push(format!("I5: {} blocked by missing {}", w.id, p));
                    continue;
                }
                Some(np) => {
                    if !clears_blocks_predecessor(np) {
                        out.push(format!(
                            "I5: {} is `progress` but blocker {} ({}) does not satisfy blocks clearance (goals must be verified)",
                            w.id, p, np.status
                        ));
                    }
                }
            }
        }
    }
    out
}

pub fn i7_blocks_dag(st: &State) -> Vec<String> {
    let mut succ: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    for e in &st.edges {
        if e.label != "blocks" {
            continue;
        }
        succ.entry(e.from.clone()).or_default().push(e.to.clone());
        nodes.insert(e.from.clone());
        nodes.insert(e.to.clone());
    }
    let mut indeg: BTreeMap<String, i64> = nodes.iter().map(|id| (id.clone(), 0)).collect();
    for vs in succ.values() {
        for v in vs {
            *indeg.entry(v.clone()).or_insert(0) += 1;
        }
    }
    let mut q: Vec<String> = indeg
        .iter()
        .filter(|(_, d)| **d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(x) = q.pop() {
        visited += 1;
        if let Some(ss) = succ.get(&x) {
            for s in ss {
                if let Some(d) = indeg.get_mut(s) {
                    *d -= 1;
                    if *d == 0 {
                        q.push(s.clone());
                    }
                }
            }
        }
    }
    if visited == nodes.len() {
        Vec::new()
    } else {
        vec!["I7: blocks graph contains a cycle".to_string()]
    }
}

pub fn i9_feature_bchain(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if !(w.wtype.as_deref() == Some("feature")
            && matches!(w.status.as_str(), "ready" | "progress"))
        {
            continue;
        }
        for b in bchain(st, w) {
            let n = match st.nodes.get(&b) {
                Some(n) => n,
                None => continue,
            };
            if !matches!(n.status.as_str(), "validated" | "invalidated_acceptable") {
                out.push(format!(
                    "I9: {} is `{}` but {} is `{}`",
                    w.id, w.status, b, n.status
                ));
            }
        }
    }
    out
}

pub fn i10_done_fitness(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "done" {
            continue;
        }
        let f = w.fitness();
        for g in w.lines("goals") {
            if !f.contains_key(&g) {
                out.push(format!(
                    "I10: {} is `done` but no fitness delta for {}",
                    w.id, g
                ));
            }
        }
    }
    out
}

pub fn i11_progress_has_session_claim(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "progress" {
            continue;
        }
        if !progress_has_session_record(w) {
            out.push(format!(
                "I11: {} is `progress` but has no session token",
                w.id
            ));
        }
    }
    out
}

pub fn check_orphan_edges(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for e in &st.edges {
        if !st.nodes.contains_key(&e.from) {
            out.push(format!("edge endpoint missing: {}", e.from));
        }
        if !st.nodes.contains_key(&e.to) {
            out.push(format!("edge endpoint missing: {}", e.to));
        }
    }
    out
}

pub fn check_edge_types(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for e in &st.edges {
        let from = match st.nodes.get(&e.from) {
            Some(n) => n,
            None => continue,
        };
        let to = match st.nodes.get(&e.to) {
            Some(n) => n,
            None => continue,
        };
        let ok = match e.label.as_str() {
            "blocks" => to.kind == Kind::W,
            "causes" => from.kind == Kind::T && to.kind == Kind::W,
            "implements" => from.kind == Kind::W && to.kind == Kind::D,
            "asks" => from.kind == Kind::Q,
            "tests" => from.kind == Kind::B && to.kind == Kind::Q,
            "targets" => from.kind == Kind::B && to.kind == Kind::W,
            "produces" => {
                from.kind == Kind::W && matches!(to.kind, Kind::D | Kind::Q | Kind::B | Kind::Y)
            }
            "supersedes" => {
                (from.kind == Kind::D && to.kind == Kind::D)
                    || (from.kind == Kind::Y && to.kind == Kind::Y)
            }
            "distills" => from.kind == Kind::Y && matches!(to.kind, Kind::D | Kind::Q | Kind::B),
            _ => false,
        };
        if !ok {
            out.push(format!(
                "edge type mismatch: {} -{}-> {}",
                e.from, e.label, e.to
            ));
        }
    }
    out
}

pub fn discovery_anchor_issues(st: &State, x: &Node) -> Vec<String> {
    let mut out = Vec::new();
    if x.kind != Kind::Y {
        return out;
    }
    if x.archived {
        return out;
    }
    let mut has_origin = false;
    for e in &st.edges {
        if e.label == "produces" && e.to == x.id {
            if let Some(src) = st.nodes.get(&e.from) {
                if src.kind == Kind::W {
                    has_origin = true;
                }
            }
        } else if e.label == "distills" && e.from == x.id {
            if let Some(dst) = st.nodes.get(&e.to) {
                if matches!(dst.kind, Kind::D | Kind::Q | Kind::B) {
                    has_origin = true;
                }
            }
        }
        if has_origin {
            break;
        }
    }
    if !has_origin {
        out.push(format!(
            "I12: {} has no provenance edge (needs `produces` from a W or `distills` to a D/Q/B)",
            x.id
        ));
    }
    if x.lines("surface").is_empty() && !prose_field_nonempty(&x.lines("why")) {
        out.push(format!(
            "I12: {} has empty `surface` and empty `why` (≥1 anchor required)",
            x.id
        ));
    }
    if x.lines("tags").is_empty() {
        out.push(format!(
            "I12: {} has empty `tags` (≥1 glossary term required)",
            x.id
        ));
    }
    out
}

pub fn check_discovery_anchors(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for x in listnodes(st, Kind::Y, false) {
        out.extend(discovery_anchor_issues(st, x));
    }
    out
}

pub fn check_area_membership(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for g in listnodes(st, Kind::G, true) {
        let aref = g.single("area");
        if aref.trim().is_empty() {
            out.push(format!(
                "I13: {} has no `area` field (every goal belongs to an area: `grove set {} area=A-NN`)",
                g.id, g.id
            ));
            continue;
        }
        match st.nodes.get(&aref) {
            Some(zn) if zn.kind == Kind::A => {}
            _ => out.push(format!(
                "I13: {} area {} does not reference an existing area (a) node",
                g.id, aref
            )),
        }
    }
    out
}

pub fn check_all(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    out.extend(i1_dor_on_progress(st));
    out.extend(i2_spike_outputs(st));
    out.extend(i3_done_has_evidence(st));
    out.extend(i4_wip_limit(st));
    out.extend(i5_blocks_terminal(st));
    out.extend(i7_blocks_dag(st));
    out.extend(i9_feature_bchain(st));
    out.extend(i10_done_fitness(st));
    out.extend(i11_progress_has_session_claim(st));
    out.extend(check_orphan_edges(st));
    out.extend(check_edge_types(st));
    out.extend(check_discovery_anchors(st));
    out.extend(check_area_membership(st));
    out
}

pub fn validate_and_push_edge(
    st: &mut State,
    from: &str,
    label: &str,
    to: &str,
    bump_nodes: bool,
) -> Option<String> {
    let from = from.trim().to_string();
    let to = to.trim().to_string();
    if !EDGE_LABELS.contains(&label) {
        return Some(format!("unknown edge label: {}", label));
    }
    if !st.nodes.contains_key(&from) {
        return Some(format!("missing node {}", from));
    }
    if !st.nodes.contains_key(&to) {
        return Some(format!("missing node {}", to));
    }
    if st
        .edges
        .iter()
        .any(|e| e.from == from && e.label == label && e.to == to)
    {
        return None;
    }
    let mut e = Edge::new(&from, label, &to);
    stamp_new_edge(&mut e);
    st.edges.push(e);
    if label == "blocks" && !i7_blocks_dag(st).is_empty() {
        st.edges.pop();
        return Some("I7: blocks introduces a cycle".to_string());
    }
    let et = check_edge_types(st);
    if !et.is_empty() {
        st.edges.pop();
        return et.into_iter().last();
    }
    if bump_nodes {
        if let Some(n) = st.nodes.get_mut(&from) {
            stamp_touch_node(n);
        }
        if let Some(n) = st.nodes.get_mut(&to) {
            stamp_touch_node(n);
        }
    }
    None
}
