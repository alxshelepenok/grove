use super::load_state;
use crate::templates::Templates;
use grove_core::{area_relevant_discoveries, Kind, State};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const LABEL_THRESHOLD: &str =
    "id labels hide above 200 nodes or below scale 0.4 so the WebGL path stays the bottleneck";

pub const ROOT_ID: &str = "Project";
pub const ROOT_TITLE: &str = "Project";
pub const ROOT_CLUSTER: &str = "root";

const KIND_ORDER: [Kind; 8] = [
    Kind::A,
    Kind::G,
    Kind::W,
    Kind::T,
    Kind::Q,
    Kind::B,
    Kind::Y,
    Kind::D,
];

const KIND_LABELS: [&str; 8] = [
    "Areas (A)",
    "Goals (G)",
    "Work (W)",
    "Themes (T)",
    "Questions (Q)",
    "Assumptions (B)",
    "Discovery (Y)",
    "Decisions (D)",
];

fn kind_from(param: &str) -> Option<Kind> {
    KIND_ORDER.iter().copied().find(|k| k.as_str() == param)
}

fn contains(from: &str, to: &str) -> Value {
    json!({ "from": from, "label": "contains", "to": to, "virtual": true })
}

fn full_graph(st: &State, include_archived: bool) -> (Vec<Value>, Vec<Value>) {
    let kept: BTreeSet<&str> = st
        .nodes
        .values()
        .filter(|n| include_archived || !n.archived)
        .map(|n| n.id.as_str())
        .collect();
    let nodes: Vec<Value> = st
        .nodes
        .values()
        .filter(|n| kept.contains(n.id.as_str()))
        .map(|n| {
            json!({
                "id": n.id,
                "kind": n.kind.as_str(),
                "status": n.status,
                "title": n.title,
                "wtype": n.wtype.clone().unwrap_or_default(),
                "archived": n.archived,
            })
        })
        .collect();
    let edges: Vec<Value> = st
        .edges
        .iter()
        .filter(|e| kept.contains(e.from.as_str()) && kept.contains(e.to.as_str()))
        .map(|e| json!({ "from": e.from, "label": e.label, "to": e.to }))
        .collect();
    let visible = |id: &str, kind: Kind| {
        st.nodes
            .get(id)
            .is_some_and(|n| n.kind == kind && kept.contains(n.id.as_str()))
    };
    let mut virtual_edges: Vec<Value> = Vec::new();
    let mut themed: BTreeSet<String> = BTreeSet::new();
    for n in st.nodes.values() {
        if n.kind == Kind::W && kept.contains(n.id.as_str()) {
            themed.extend(n.lines("theme").into_iter().filter(|t| visible(t, Kind::T)));
        }
    }
    for n in st.nodes.values() {
        if !kept.contains(n.id.as_str()) {
            continue;
        }
        match n.kind {
            Kind::A => virtual_edges.push(contains(ROOT_ID, &n.id)),
            Kind::G => {
                let area = n.single("area");
                if visible(&area, Kind::A) {
                    virtual_edges.push(contains(&area, &n.id));
                } else {
                    virtual_edges.push(contains(ROOT_ID, &n.id));
                }
            }
            Kind::W => {
                let goals: Vec<String> = n
                    .lines("goals")
                    .into_iter()
                    .filter(|g| visible(g, Kind::G))
                    .collect();
                if goals.is_empty() {
                    virtual_edges.push(contains(ROOT_ID, &n.id));
                } else {
                    for g in goals {
                        virtual_edges.push(contains(&g, &n.id));
                    }
                }
                for t in n.lines("theme") {
                    if visible(&t, Kind::T) {
                        virtual_edges.push(contains(&t, &n.id));
                    }
                }
            }
            Kind::T => {
                if !themed.contains(&n.id) {
                    virtual_edges.push(contains(ROOT_ID, &n.id));
                }
            }
            _ => {}
        }
    }
    let mut area_matches: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for z in st.nodes.values() {
        if z.kind == Kind::A && kept.contains(z.id.as_str()) {
            area_matches.insert(
                z.id.as_str(),
                area_relevant_discoveries(st, z).into_iter().collect(),
            );
        }
    }
    for n in st.nodes.values() {
        if n.kind != Kind::Y || !kept.contains(n.id.as_str()) {
            continue;
        }
        let area = area_matches
            .iter()
            .find(|(_, ys)| ys.contains(&n.id))
            .map(|(id, _)| *id);
        match area {
            Some(a) => virtual_edges.push(contains(a, &n.id)),
            None => virtual_edges.push(contains(ROOT_ID, &n.id)),
        }
    }
    let mut incident: BTreeSet<String> = BTreeSet::new();
    for e in &edges {
        incident.insert(e["from"].as_str().unwrap_or_default().to_string());
        incident.insert(e["to"].as_str().unwrap_or_default().to_string());
    }
    for e in &virtual_edges {
        incident.insert(e["from"].as_str().unwrap_or_default().to_string());
        incident.insert(e["to"].as_str().unwrap_or_default().to_string());
    }
    for n in st.nodes.values() {
        if !kept.contains(n.id.as_str()) {
            continue;
        }
        if matches!(n.kind, Kind::Q | Kind::B | Kind::D) && !incident.contains(n.id.as_str()) {
            virtual_edges.push(contains(ROOT_ID, &n.id));
        }
    }
    let mut all_edges = edges;
    all_edges.extend(virtual_edges);
    (nodes, all_edges)
}

pub fn model(st: &State, include_archived: bool, root_title: &str) -> Value {
    model_filtered(st, include_archived, "all", "", root_title)
}

fn containment_ancestors<'a>(
    parents: &BTreeMap<&'a str, Vec<&'a str>>,
    seed: BTreeSet<&'a str>,
) -> BTreeSet<&'a str> {
    let mut out = seed;
    let mut queue: VecDeque<&'a str> = out.iter().copied().collect();
    while let Some(id) = queue.pop_front() {
        if let Some(ps) = parents.get(id) {
            for p in ps {
                if out.insert(*p) {
                    queue.push_back(*p);
                }
            }
        }
    }
    out
}

fn cluster_map(st: &State, parents: &BTreeMap<&str, Vec<&str>>) -> BTreeMap<String, String> {
    let is_kind = |id: &str, kind: Kind| st.nodes.get(id).is_some_and(|n| n.kind == kind);
    let area_of = |id: &str| -> Option<String> {
        for p in parents.get(id).into_iter().flatten() {
            if is_kind(p, Kind::A) {
                return Some(p.to_string());
            }
        }
        None
    };
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for n in st.nodes.values() {
        let cluster = match n.kind {
            Kind::A => n.id.clone(),
            Kind::G | Kind::Y => area_of(&n.id).unwrap_or_else(|| ROOT_CLUSTER.to_string()),
            Kind::W => {
                let mut goals: Vec<&str> = Vec::new();
                if let Some(ps) = parents.get(n.id.as_str()) {
                    for p in ps {
                        if is_kind(p, Kind::G) {
                            goals.push(*p);
                        }
                    }
                }
                goals.sort_unstable();
                goals
                    .first()
                    .and_then(|g| area_of(g))
                    .unwrap_or_else(|| ROOT_CLUSTER.to_string())
            }
            _ => ROOT_CLUSTER.to_string(),
        };
        out.insert(n.id.clone(), cluster);
    }
    out
}

pub fn model_filtered(
    st: &State,
    include_archived: bool,
    kind: &str,
    focus: &str,
    root_title: &str,
) -> Value {
    let (nodes, edges) = full_graph(st, include_archived);

    let mut parents: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut neighbors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in &edges {
        let from = e["from"].as_str().unwrap_or_default();
        let to = e["to"].as_str().unwrap_or_default();
        if e["virtual"].as_bool().unwrap_or(false) {
            parents.entry(to).or_default().push(from);
            children.entry(from).or_default().push(to);
        } else {
            neighbors.entry(from).or_default().push(to);
            neighbors.entry(to).or_default().push(from);
        }
    }
    let visible: BTreeSet<&str> = nodes.iter().filter_map(|n| n["id"].as_str()).collect();
    let mut kind_eff = if kind_from(kind).is_some() { kind } else { "all" };
    let focus_eff = if visible.contains(focus) { focus } else { "" };

    let mut kept: BTreeSet<&str> = if kind_eff == "all" {
        visible.iter().copied().chain([ROOT_ID]).collect()
    } else {
        containment_ancestors(
            &parents,
            nodes
                .iter()
                .filter(|n| n["kind"].as_str() == Some(kind_eff))
                .filter_map(|n| n["id"].as_str())
                .collect(),
        )
    };
    if !focus_eff.is_empty() && !kept.contains(focus_eff) {
        kind_eff = "all";
        kept = visible.iter().copied().chain([ROOT_ID]).collect();
    }
    if !focus_eff.is_empty() {
        let mut set: BTreeSet<&str> = [focus_eff].into_iter().collect();
        let mut queue: VecDeque<&str> = [focus_eff].into_iter().collect();
        while let Some(id) = queue.pop_front() {
            if let Some(cs) = children.get(id) {
                for c in cs {
                    if kept.contains(c) && set.insert(c) {
                        queue.push_back(c);
                    }
                }
            }
        }
        if let Some(ns) = neighbors.get(focus_eff) {
            for n in ns {
                if kept.contains(n) {
                    set.insert(n);
                }
            }
        }
        kept = containment_ancestors(&parents, set);
    }

    let clusters = cluster_map(st, &parents);
    let node_count = nodes
        .iter()
        .filter(|n| kept.contains(n["id"].as_str().unwrap_or_default()))
        .count();
    let mut final_nodes: Vec<Value> = nodes
        .iter()
        .filter(|n| kept.contains(n["id"].as_str().unwrap_or_default()))
        .map(|n| {
            let mut n = n.clone();
            let id = n["id"].as_str().unwrap_or_default();
            n["cluster"] = json!(clusters.get(id).map(String::as_str).unwrap_or(ROOT_CLUSTER));
            n
        })
        .collect();
    let mut edge_count = 0;
    let mut virtual_edge_count = 0;
    let mut final_edges: Vec<Value> = Vec::new();
    for e in &edges {
        let from = e["from"].as_str().unwrap_or_default();
        let to = e["to"].as_str().unwrap_or_default();
        if !kept.contains(from) || !kept.contains(to) {
            continue;
        }
        if e["virtual"].as_bool().unwrap_or(false) {
            virtual_edge_count += 1;
        } else {
            edge_count += 1;
        }
        final_edges.push(e.clone());
    }

    let mut incident: BTreeSet<&str> = BTreeSet::new();
    for e in &final_edges {
        incident.insert(e["from"].as_str().unwrap_or_default());
        incident.insert(e["to"].as_str().unwrap_or_default());
    }
    let orphans: Vec<String> = final_nodes
        .iter()
        .filter_map(|n| n["id"].as_str())
        .filter(|id| !incident.contains(*id))
        .map(str::to_string)
        .collect();
    let mut has_root = kept.contains(ROOT_ID);
    for id in &orphans {
        final_edges.push(contains(ROOT_ID, id));
        virtual_edge_count += 1;
        has_root = true;
    }
    if has_root {
        final_nodes.push(json!({
            "id": ROOT_ID,
            "kind": "root",
            "status": "",
            "title": root_title,
            "wtype": "",
            "archived": false,
            "virtual": true,
            "cluster": ROOT_CLUSTER,
        }));
    }

    let mut filters: Vec<Value> = vec![json!({
        "status": "all",
        "label": "All",
        "count": visible.len(),
        "active": kind_eff == "all",
    })];
    for (k, label) in KIND_ORDER.into_iter().zip(KIND_LABELS) {
        let count = nodes
            .iter()
            .filter(|n| n["kind"].as_str() == Some(k.as_str()))
            .count();
        if count == 0 {
            continue;
        }
        filters.push(json!({
            "status": k.as_str(),
            "label": label,
            "count": count,
            "active": kind_eff == k.as_str(),
        }));
    }
    let focus_items: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let id = n["id"].as_str().unwrap_or_default();
            json!({
                "id": id,
                "label": format!(
                    "{} - {} ({})",
                    id,
                    n["title"].as_str().unwrap_or_default(),
                    n["kind"].as_str().unwrap_or_default()
                ),
            })
        })
        .collect();
    let focus_label = focus_items
        .iter()
        .find(|i| i["id"].as_str() == Some(focus_eff))
        .and_then(|i| i["label"].as_str())
        .unwrap_or_default();

    json!({
        "include_archived": include_archived,
        "kind": kind_eff,
        "focus": focus_eff,
        "focus_label": focus_label,
        "focus_items": focus_items,
        "filters": filters,
        "node_count": node_count,
        "edge_count": edge_count,
        "virtual_node_count": if has_root { 1 } else { 0 },
        "virtual_edge_count": virtual_edge_count,
        "total_nodes": st.nodes.len(),
        "total_edges": st.edges.len(),
        "label_threshold": LABEL_THRESHOLD,
        "graph": {
            "nodes": final_nodes,
            "edges": final_edges,
        },
    })
}

pub fn render(tpl: &Templates, root: &str, params: &Value) -> Result<String, String> {
    let st = load_state(root)?;
    let include_archived = params
        .get("archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let kind = params
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("all");
    let focus = params
        .get("focus")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let root_title = crate::projects::display_name(root);
    tpl.render(
        "graph",
        &model_filtered(&st, include_archived, kind, focus, &root_title),
    )
}
