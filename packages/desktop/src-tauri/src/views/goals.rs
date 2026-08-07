use super::{load_state, status_variant};
use crate::templates::Templates;
use grove_core::{listnodes, Kind, State};
use serde_json::{json, Value};

pub fn fitness_view(goal: &grove_core::Node) -> (bool, i64, String) {
    let kind = goal.attr("fitness_kind");
    let target = goal.single("fitness_target");
    let current = goal.single("fitness_current");
    if kind == "boolean" {
        let on = current == "true";
        return (
            true,
            if on { 100 } else { 0 },
            if on {
                "boolean: true".to_string()
            } else {
                "boolean: false".to_string()
            },
        );
    }
    let target_n = target.parse::<f64>().ok().filter(|t| *t > 0.0);
    if let Some(t) = target_n {
        let c = current.parse::<f64>().unwrap_or(0.0);
        let percent = ((c / t) * 100.0).round().clamp(0.0, 100.0) as i64;
        let shown_current = if current.is_empty() {
            "0".to_string()
        } else {
            current
        };
        return (true, percent, format!("{shown_current} / {target}"));
    }
    let legacy = goal.attr("fitness");
    if !legacy.is_empty() {
        return (false, 0, legacy);
    }
    if !current.is_empty() || !target.is_empty() {
        let label = [current, target]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" / ");
        return (false, 0, label);
    }
    (false, 0, "n/a".to_string())
}

fn goal_works(st: &State, gid: &str) -> usize {
    listnodes(st, Kind::W, false)
        .into_iter()
        .filter(|w| w.lines("goals").iter().any(|g| g == gid))
        .count()
}

pub fn model(st: &State) -> Value {
    let goals: Vec<Value> = listnodes(st, Kind::G, false)
        .into_iter()
        .map(|g| {
            let (has_bar, percent, fitness_label) = fitness_view(g);
            json!({
                "id": g.id,
                "title": g.title,
                "status": g.status,
                "status_variant": status_variant(Kind::G, &g.status),
                "fitness_kind": g.attr("fitness_kind"),
                "has_bar": has_bar,
                "percent": percent,
                "fitness_label": fitness_label,
                "works": goal_works(st, &g.id),
            })
        })
        .collect();
    json!({
        "goals": goals,
        "empty": goals.is_empty(),
    })
}

pub fn render(tpl: &Templates, root: &str) -> Result<String, String> {
    let st = load_state(root)?;
    tpl.render("goals", &model(&st))
}
