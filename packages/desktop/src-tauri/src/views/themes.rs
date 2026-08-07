use super::{load_state, status_variant};
use crate::templates::Templates;
use grove_core::{critical_path, listnodes, status_set, Kind, Node, State};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn work_view(w: &Node) -> Value {
    json!({
        "id": w.id,
        "title": w.title,
        "status": w.status,
        "status_variant": status_variant(Kind::W, &w.status),
    })
}

pub fn model(st: &State) -> Value {
    let mut by_theme: BTreeMap<String, Vec<&Node>> = BTreeMap::new();
    for w in listnodes(st, Kind::W, false) {
        by_theme.entry(w.single("theme")).or_default().push(w);
    }
    let mut themes: Vec<Value> = Vec::new();
    for t in listnodes(st, Kind::T, false) {
        let works = by_theme.remove(&t.id).unwrap_or_default();
        let counts: Vec<Value> = status_set(Kind::W)
            .iter()
            .filter_map(|s| {
                let n = works.iter().filter(|w| w.status == *s).count();
                (n > 0).then(|| {
                    json!({
                        "status": s,
                        "count": n,
                        "variant": status_variant(Kind::W, s),
                    })
                })
            })
            .collect();
        themes.push(json!({
            "id": t.id,
            "title": t.title,
            "status": t.status,
            "status_variant": status_variant(Kind::T, &t.status),
            "counts": counts,
            "works": works.iter().map(|w| work_view(w)).collect::<Vec<_>>(),
            "unthemed": false,
        }));
    }
    let mut unthemed: Vec<&Node> = by_theme.remove("").unwrap_or_default();
    for (_, works) in by_theme {
        unthemed.extend(works);
    }
    unthemed.sort_by(|a, b| a.id.cmp(&b.id));
    if !unthemed.is_empty() {
        let counts: Vec<Value> = status_set(Kind::W)
            .iter()
            .filter_map(|s| {
                let n = unthemed.iter().filter(|w| w.status == *s).count();
                (n > 0).then(|| {
                    json!({
                        "status": s,
                        "count": n,
                        "variant": status_variant(Kind::W, s),
                    })
                })
            })
            .collect();
        themes.push(json!({
            "id": "",
            "title": "Unthemed",
            "status": "",
            "counts": counts,
            "works": unthemed.iter().map(|w| work_view(w)).collect::<Vec<_>>(),
            "unthemed": true,
        }));
    }
    let chain = critical_path(st);
    let critical: Vec<Value> = if chain.len() < 2 {
        Vec::new()
    } else {
        chain
            .iter()
            .map(|id| {
                let title = st.nodes.get(id).map(|n| n.title.clone()).unwrap_or_default();
                json!({ "id": id, "title": title })
            })
            .collect()
    };
    let mut cloud: Vec<Value> = Vec::new();
    for kind in [Kind::Q, Kind::B] {
        for n in listnodes(st, kind, false) {
            let open = match kind {
                Kind::Q => n.status == "open",
                _ => n.status == "proposed" || n.status == "testing",
            };
            if !open {
                continue;
            }
            cloud.push(json!({
                "id": n.id,
                "title": n.title,
                "status": n.status,
                "cynefin": n.cynefin.clone().unwrap_or_default(),
            }));
        }
    }
    cloud.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    let questions: Vec<Value> = cloud
        .iter()
        .filter(|n| n["id"].as_str().unwrap_or_default().starts_with('Q'))
        .cloned()
        .collect();
    let assumptions: Vec<Value> = cloud
        .iter()
        .filter(|n| n["id"].as_str().unwrap_or_default().starts_with('B'))
        .cloned()
        .collect();
    json!({
        "themes": themes,
        "critical_path": critical,
        "critical_len": critical.len(),
        "questions": questions,
        "questions_count": questions.len(),
        "assumptions": assumptions,
        "assumptions_count": assumptions.len(),
        "cloud_empty": questions.is_empty() && assumptions.is_empty(),
    })
}

pub fn render(tpl: &Templates, root: &str) -> Result<String, String> {
    let st = load_state(root)?;
    tpl.render("themes", &model(&st))
}
