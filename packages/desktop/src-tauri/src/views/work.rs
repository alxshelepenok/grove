use super::{load_state, status_variant};
use crate::templates::Templates;
use grove_core::{critical_path, dor, listnodes, status_set, Kind, State};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const FILTERS: [&str; 6] = ["all", "proposed", "ready", "progress", "done", "rejected"];

fn normalize_filter(status: &str) -> &'static str {
    FILTERS
        .iter()
        .copied()
        .find(|s| *s == status)
        .unwrap_or("all")
}

pub fn model(st: &State, status: &str, include_archived: bool) -> Value {
    let filter = normalize_filter(status);
    let cp: BTreeSet<String> = critical_path(st).into_iter().collect();
    let all = listnodes(st, Kind::W, include_archived);
    let mut filters: Vec<Value> = vec![json!({
        "status": "all",
        "count": all.len(),
        "variant": "neutral",
        "active": filter == "all",
    })];
    for s in status_set(Kind::W).iter().filter(|s| **s != "archived") {
        filters.push(json!({
            "status": s,
            "count": all.iter().filter(|w| w.status == *s).count(),
            "variant": status_variant(Kind::W, s),
            "active": filter == *s,
        }));
    }
    let rows: Vec<Value> = all
        .iter()
        .filter(|w| filter == "all" || w.status == filter)
        .map(|w| {
            json!({
                "id": w.id,
                "wtype": w.wtype.as_deref().unwrap_or("nothing"),
                "title": w.title,
                "goals": w.lines("goals").join(", "),
                "cynefin": w.cynefin.as_deref().unwrap_or("nothing"),
                "dor": if dor(st, w, false) { "\u{22a4}" } else { "\u{22a5}" },
                "status": w.status,
                "status_variant": status_variant(Kind::W, &w.status),
                "critical": if cp.contains(&w.id) { "\u{2605}" } else { "" },
                "archived": w.archived,
            })
        })
        .collect();
    json!({
        "rows": rows,
        "shown": rows.len(),
        "total": all.len(),
        "empty": rows.is_empty(),
        "filter": filter,
        "filters": filters,
        "include_archived": include_archived,
    })
}

pub fn render(tpl: &Templates, root: &str, params: &Value) -> Result<String, String> {
    let st = load_state(root)?;
    let status = params
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("all");
    let include_archived = params
        .get("archived")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    tpl.render("work", &model(&st, status, include_archived))
}
