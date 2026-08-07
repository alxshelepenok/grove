use super::{goals, load_state, status_variant};
use crate::templates::Templates;
use grove_core::{
    area_goals, area_node_ids, area_relevant_discoveries, area_surface, area_tags, area_work,
    coverage, dor, is_terminal, listnodes, status_set, Kind, Node, State,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn health_tally(
    st: &State,
    n: &Node,
    c: &mut BTreeMap<&'static str, i64>,
    v: &mut BTreeMap<&'static str, i64>,
) {
    match n.kind {
        Kind::Q => {
            if n.status == "open" {
                *v.entry("q").or_insert(0) += 1;
            }
            if n.status == "answered" {
                *c.entry("q").or_insert(0) += 1;
            }
        }
        Kind::B => {
            if n.status == "proposed" || n.status == "testing" {
                *v.entry("b").or_insert(0) += 1;
            }
            if n.status == "validated" {
                *c.entry("b").or_insert(0) += 1;
            }
        }
        Kind::D => {
            if n.status == "accepted" {
                *c.entry("d").or_insert(0) += 1;
            }
        }
        Kind::Y => {
            if n.status == "active" {
                *c.entry("y").or_insert(0) += 1;
            }
        }
        Kind::W => {
            if !is_terminal(Kind::W, &n.status) {
                if !dor(st, n, false) {
                    *v.entry("w").or_insert(0) += 1;
                }
                if !n.lines("surface").is_empty() {
                    let (cov, _, _) = coverage(st, n);
                    if cov < 1.0 {
                        *v.entry("surf").or_insert(0) += 1;
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn area_health(
    st: &State,
    z: &Node,
) -> (BTreeMap<&'static str, i64>, BTreeMap<&'static str, i64>) {
    let mut c: BTreeMap<&'static str, i64> = [("b", 0), ("q", 0), ("d", 0), ("y", 0)].into();
    let mut v: BTreeMap<&'static str, i64> = [("q", 0), ("b", 0), ("w", 0)].into();
    for id in area_node_ids(st, z) {
        if let Some(n) = st.nodes.get(&id) {
            health_tally(st, n, &mut c, &mut v);
        }
    }
    c.insert("y", area_relevant_discoveries(st, z).len() as i64);
    (c, v)
}

fn tally(m: &BTreeMap<&'static str, i64>, k: &str) -> i64 {
    *m.get(k).unwrap_or(&0)
}

fn area_model(st: &State, z: &Node) -> Value {
    let goal_rows: Vec<Value> = area_goals(st, z)
        .into_iter()
        .map(|g| {
            let (_, _, fitness_label) = goals::fitness_view(g);
            json!({
                "id": g.id,
                "title": g.title,
                "fitness_label": fitness_label,
            })
        })
        .collect();
    let works = area_work(st, z);
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
    let (c, v) = area_health(st, z);
    let surface: Vec<String> = area_surface(st, z).into_iter().collect();
    let tags: Vec<String> = area_tags(st, z).into_iter().collect();
    json!({
        "id": z.id,
        "title": z.title,
        "status": z.status,
        "status_variant": status_variant(Kind::A, &z.status),
        "goals": goal_rows,
        "goals_empty": goal_rows.is_empty(),
        "counts": counts,
        "work_empty": counts.is_empty(),
        "c_total": c.values().sum::<i64>(),
        "v_total": v.values().sum::<i64>(),
        "c": {
            "b": tally(&c, "b"),
            "q": tally(&c, "q"),
            "d": tally(&c, "d"),
            "y": tally(&c, "y"),
        },
        "v": {
            "q": tally(&v, "q"),
            "b": tally(&v, "b"),
            "w": tally(&v, "w"),
            "surf": tally(&v, "surf"),
        },
        "has_y": tally(&c, "y") > 0,
        "has_surf": tally(&v, "surf") > 0,
        "surface": surface,
        "has_surface": !surface.is_empty(),
        "tags": tags,
        "has_tags": !tags.is_empty(),
    })
}

pub fn model(st: &State) -> Value {
    let areas: Vec<Value> = listnodes(st, Kind::A, false)
        .into_iter()
        .map(|z| area_model(st, z))
        .collect();
    json!({
        "areas": areas,
        "empty": areas.is_empty(),
    })
}

pub fn render(tpl: &Templates, root: &str) -> Result<String, String> {
    let st = load_state(root)?;
    tpl.render("areas", &model(&st))
}
