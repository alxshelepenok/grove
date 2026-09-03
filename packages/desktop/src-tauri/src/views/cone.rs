use super::{load_state, status_variant};
use crate::templates::Templates;
use grove_core::{
    backward_cone, contraction_order, critical_path, forward_cone, goal_fragility, listnodes,
    relevant_discoveries, Kind, State,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const DEFAULT_DEPTH: usize = 4;
const DEFAULT_MAX: usize = 50;

fn param_usize(params: &Value, key: &str, default: usize) -> usize {
    params
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v.max(1) as usize)
        .unwrap_or(default)
}

pub fn params_depth_max(params: &Value) -> (usize, usize) {
    (
        param_usize(params, "depth", DEFAULT_DEPTH),
        param_usize(params, "max", DEFAULT_MAX),
    )
}

pub fn cone_walk(st: &State, seed: &str, depth: usize, max: usize) -> Value {
    let backward = backward_cone(st, seed, depth, max);
    let forward = forward_cone(st, seed, depth, max);
    let order = contraction_order(st, &backward.ids);
    let mut members: BTreeSet<&str> = BTreeSet::new();
    members.insert(seed);
    for id in backward.ids.iter().chain(forward.ids.iter()) {
        members.insert(id.as_str());
    }
    let nodes: Vec<Value> = members
        .iter()
        .filter_map(|id| st.nodes.get(*id))
        .map(|n| {
            json!({
                "id": n.id,
                "kind": n.kind.as_str(),
                "title": n.title,
                "status": n.status,
                "status_variant": status_variant(n.kind, &n.status),
                "archived": n.archived,
            })
        })
        .collect();
    let critical_path_ids = critical_path(st);
    let critical_set: BTreeSet<&str> = critical_path_ids.iter().map(|s| s.as_str()).collect();
    let nodes: Vec<Value> = nodes
        .into_iter()
        .map(|mut n| {
            n["critical"] = json!(critical_set.contains(n["id"].as_str().unwrap_or("")));
            n
        })
        .collect();
    let critical_in_cone: Vec<&str> = critical_path_ids
        .iter()
        .map(|s| s.as_str())
        .filter(|id| members.contains(*id))
        .collect();
    let edges: Vec<Value> = st
        .edges
        .iter()
        .filter(|e| {
            e.label == "blocks"
                && members.contains(e.from.as_str())
                && members.contains(e.to.as_str())
        })
        .map(|e| json!({"from": e.from, "to": e.to}))
        .collect();
    let mut goal_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut theme_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut file_touchers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut area_goals: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in &members {
        let Some(n) = st.nodes.get(*id) else {
            continue;
        };
        if n.kind != Kind::W {
            continue;
        }
        for g in n.lines("goals") {
            goal_members.entry(g).or_default().push((*id).to_string());
        }
        for t in n.lines("theme") {
            theme_members.entry(t).or_default().push((*id).to_string());
        }
        for s in n.lines("surface") {
            file_touchers.entry(s).or_default().push((*id).to_string());
        }
    }
    let goals: Vec<Value> = goal_members
        .iter()
        .filter_map(|(g, ms)| {
            let n = st.nodes.get(g)?;
            let area = n.single("area");
            if !area.is_empty() {
                area_goals.entry(area).or_default().push(g.clone());
            }
            Some(json!({
                "id": n.id,
                "kind": "g",
                "title": n.title,
                "status": n.status,
                "status_variant": status_variant(Kind::G, &n.status),
                "members": ms,
            }))
        })
        .collect();
    let areas: Vec<Value> = area_goals
        .iter()
        .filter_map(|(a, gs)| {
            let n = st.nodes.get(a)?;
            Some(json!({
                "id": n.id,
                "kind": "a",
                "title": n.title,
                "status": n.status,
                "goals": gs,
            }))
        })
        .collect();
    let themes: Vec<Value> = theme_members
        .iter()
        .filter_map(|(t, ms)| {
            let n = st.nodes.get(t)?;
            Some(json!({
                "id": n.id,
                "kind": "t",
                "title": n.title,
                "status": n.status,
                "status_variant": status_variant(Kind::T, &n.status),
                "members": ms,
            }))
        })
        .collect();
    let files: Vec<Value> = file_touchers
        .iter()
        .map(|(p, ids)| {
            let name = p.rsplit('/').next().unwrap_or(p);
            json!({"id": p, "kind": "f", "title": name, "touchers": ids})
        })
        .collect();
    json!({
        "seed": seed,
        "depth": depth,
        "max": max,
        "backward": backward.ids,
        "backward_count": backward.ids.len(),
        "order": order,
        "forward": forward.ids,
        "forward_count": forward.ids.len(),
        "truncated": backward.truncated || forward.truncated,
        "backward_truncated": backward.truncated,
        "forward_truncated": forward.truncated,
        "nodes": nodes,
        "edges": edges,
        "critical": critical_in_cone,
        "strata": {
            "goals": goals,
            "areas": areas,
            "themes": themes,
            "files": files,
        },
    })
}

pub fn model(st: &State, seed: &str, depth: usize, max: usize) -> Value {
    let works: Vec<Value> = listnodes(st, Kind::W, false)
        .into_iter()
        .map(|w| {
            json!({
                "id": w.id,
                "label": format!("{} - {}", w.id, w.title),
            })
        })
        .collect();
    let selected = seed.trim().to_string();
    let selected_text = works
        .iter()
        .find(|w| w["id"].as_str() == Some(selected.as_str()))
        .and_then(|w| w["label"].as_str())
        .map(String::from);
    let mut m = json!({
        "works": works,
        "selected": selected,
    });
    if let Some(text) = selected_text {
        m["selectedText"] = json!(text);
    }
    if let Some(node) = st.nodes.get(&selected).filter(|n| n.kind == Kind::W) {
        let has_edges = st.edges.iter().any(|e| {
            e.label == "blocks" && (e.from == selected || e.to == selected)
        });
        m["seed"] = json!({
            "id": node.id,
            "title": node.title,
            "status": node.status,
            "status_variant": status_variant(Kind::W, &node.status),
        });
        m["seed_empty"] = json!(!has_edges);
        let fragility: Vec<Value> = goal_fragility(st, node)
            .into_iter()
            .map(|(g, k)| {
                let (label, variant) = if k == 0 {
                    ("no path".to_string(), "neutral")
                } else if k == 1 {
                    ("brittle".to_string(), "warning")
                } else {
                    (format!("{} disjoint paths", k), "info")
                };
                json!({
                    "id": g,
                    "title": st.nodes.get(&g).map(|n| n.title.clone()).unwrap_or_default(),
                    "connectivity": k,
                    "label": label,
                    "variant": variant,
                })
            })
            .collect();
        let fragility_len = fragility.len();
        m["fragility"] = json!(fragility);
        m["cone"] = cone_walk(st, &selected, depth, max);
        let backward = backward_cone(st, &selected, depth, max);
        let forward = forward_cone(st, &selected, depth, max);
        let member_row = |id: &String| {
            st.nodes.get(id).map(|n| json!({"id": n.id, "title": n.title}))
        };
        let dependencies: Vec<Value> =
            contraction_order(st, &backward.ids).iter().filter_map(member_row).collect();
        let impact: Vec<Value> = forward.ids.iter().filter_map(member_row).collect();
        m["dependencies"] = json!(dependencies);
        m["impact"] = json!(impact);
        let mut cone_ids = vec![selected.clone()];
        cone_ids.extend(backward.ids.iter().cloned());
        cone_ids.extend(forward.ids.iter().cloned());
        let cone_set: BTreeSet<String> = cone_ids.iter().cloned().collect();
        let w_surface: BTreeSet<String> = node.lines("surface").into_iter().collect();
        let mut cone_tags: BTreeSet<String> = node.lines("tags").into_iter().collect();
        for id in &cone_set {
            if let Some(n) = st.nodes.get(id) {
                cone_tags.extend(n.lines("tags"));
            }
        }
        let discoveries: Vec<Value> = relevant_discoveries(st, node, &cone_ids, 8)
            .into_iter()
            .filter_map(|id| {
                let y = st.nodes.get(&id)?;
                let surfaces: Vec<String> = y
                    .lines("surface")
                    .into_iter()
                    .filter(|s| w_surface.contains(s))
                    .collect();
                let tags: Vec<String> = y
                    .lines("tags")
                    .into_iter()
                    .filter(|t| cone_tags.contains(t))
                    .collect();
                let linked = st.edges.iter().find_map(|e| {
                    if e.from == id && cone_set.contains(&e.to) {
                        Some(e.to.clone())
                    } else if e.to == id && cone_set.contains(&e.from) {
                        Some(e.from.clone())
                    } else {
                        None
                    }
                });
                let mut reasons = Vec::new();
                if !surfaces.is_empty() {
                    reasons.push(format!("surface: {}", surfaces.join(", ")));
                }
                if !tags.is_empty() {
                    reasons.push(format!("tags: {}", tags.join(", ")));
                }
                if let Some(other) = linked {
                    reasons.push(format!("cone: {}", other));
                }
                Some(json!({"id": id, "title": y.title, "reason": reasons.join("; ")}))
            })
            .collect();
        m["discoveries"] = json!(discoveries);
        m["fragility_count"] = json!(fragility_len);
        m["discoveries_count"] = json!(discoveries.len());
        let mut file_touchers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for id in &cone_set {
            let Some(n) = st.nodes.get(id) else {
                continue;
            };
            if n.kind != Kind::W {
                continue;
            }
            for s in n.lines("surface") {
                file_touchers.entry(s).or_default().push((*id).to_string());
            }
        }
        let surface: Vec<Value> = file_touchers
            .iter()
            .map(|(p, ids)| {
                json!({
                    "path": p,
                    "name": p.rsplit('/').next().unwrap_or(p),
                    "count": ids.len(),
                    "touchers": ids,
                })
            })
            .collect();
        let surface_len = surface.len();
        m["surface"] = json!(surface);
        m["surface_count"] = json!(surface_len);
    }
    m
}

pub fn render(tpl: &Templates, root: &str, params: &Value) -> Result<String, String> {
    let st = load_state(root)?;
    let seed = params
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let (depth, max) = params_depth_max(params);
    tpl.render("cone", &model(&st, seed, depth, max))
}
