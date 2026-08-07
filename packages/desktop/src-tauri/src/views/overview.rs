use super::{goals, load_state, status_variant};
use crate::templates::Templates;
use grove_core::{
    content_health, journal_read_nonempty_pairs, listnodes, stats_cv_series, utc_stamp_second,
    CliCtx, Json, Kind, State,
};
use serde_json::{json, Value};
use std::time::SystemTime;

pub const TREND_WINDOW: usize = 30;
pub const SPARK_W: usize = 120;
pub const SPARK_H: usize = 28;
pub const RECENT_DISCOVERIES: usize = 6;

const WORK_STATUSES: [&str; 5] = ["proposed", "ready", "progress", "done", "rejected"];

const NAV_CARDS: [(&str, &str, &str, &str); 7] = [
    (
        "areas",
        "Areas",
        "Coverage and health per knowledge area.",
        "layout",
    ),
    (
        "discovery",
        "Discovery",
        "Questions, assumptions, and probes in flight.",
        "sparkles",
    ),
    ("goals", "Goals", "Track fitness progress across all goals.", "target"),
    ("work", "Work", "Every work item by status.", "list-view"),
    (
        "themes",
        "Themes",
        "Track work by theme and the critical path.",
        "layers",
    ),
    ("graph", "Graph", "Explore nodes and edges as an interactive graph.", "share"),
    (
        "packet",
        "Packet",
        "Execution packet for a work item.",
        "file",
    ),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trend {
    pub points: usize,
    pub delta: i64,
    pub dir: &'static str,
    pub variant: &'static str,
    pub label: String,
    pub spark_c: String,
    pub spark_v: String,
}

fn spark_points(values: &[i64], max: i64) -> String {
    let padded: Vec<i64> = if values.len() == 1 {
        vec![values[0], values[0]]
    } else {
        values.to_vec()
    };
    let denom = (padded.len() - 1).max(1) as f64;
    padded
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = SPARK_W as f64 * i as f64 / denom;
            let frac = if max > 0 {
                (*v as f64 / max as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let y = (SPARK_H as f64 - 1.0) - frac * (SPARK_H as f64 - 2.0);
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn trend_from_series(series: &[(String, i64, i64)]) -> Trend {
    let window: Vec<(i64, i64)> = series
        .iter()
        .rev()
        .take(TREND_WINDOW)
        .rev()
        .map(|(_, c, v)| (*c, *v))
        .collect();
    if window.is_empty() {
        return Trend {
            points: 0,
            delta: 0,
            dir: "flat",
            variant: "neutral",
            label: "no history".to_string(),
            spark_c: String::new(),
            spark_v: String::new(),
        };
    }
    let n = window.len();
    let delta = window[n - 1].0 - window[0].0;
    let (dir, variant) = if delta > 0 {
        ("up", "success")
    } else if delta < 0 {
        ("down", "danger")
    } else {
        ("flat", "neutral")
    };
    let shown = if delta > 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    };
    let unit = if n == 1 { "point" } else { "points" };
    let max = window.iter().map(|(c, v)| (*c).max(*v)).max().unwrap_or(0);
    let cs: Vec<i64> = window.iter().map(|p| p.0).collect();
    let vs: Vec<i64> = window.iter().map(|p| p.1).collect();
    Trend {
        points: n,
        delta,
        dir,
        variant,
        label: format!("{shown} over last {n} {unit}"),
        spark_c: spark_points(&cs, max),
        spark_v: spark_points(&vs, max),
    }
}

pub fn open_work(st: &State) -> usize {
    listnodes(st, Kind::W, false)
        .iter()
        .filter(|w| w.status != "done" && w.status != "rejected")
        .count()
}

fn content_sums(st: &State) -> (i64, i64) {
    let (c, v) = content_health(st);
    (c.values().sum(), v.values().sum())
}

fn count_status(st: &State, kind: Kind, statuses: &[&str]) -> usize {
    listnodes(st, kind, false)
        .iter()
        .filter(|n| statuses.contains(&n.status.as_str()))
        .count()
}

fn work_model(st: &State) -> Value {
    let ws = listnodes(st, Kind::W, false);
    let total = ws.len();
    let segments: Vec<Value> = WORK_STATUSES
        .iter()
        .map(|s| {
            let count = ws.iter().filter(|w| w.status == *s).count();
            let pct = if total > 0 {
                ((count as f64 / total as f64) * 1000.0).round() / 10.0
            } else {
                0.0
            };
            json!({
                "status": s,
                "label": s,
                "count": count,
                "valueText": count.to_string(),
                "variant": status_variant(Kind::W, s),
                "widthPct": if count > 0 { json!(pct) } else { json!(0) },
                "title": format!("{s}: {count}"),
            })
        })
        .collect();
    json!({
        "open": open_work(st),
        "segments": segments,
    })
}

fn goal_rows(st: &State) -> Vec<Value> {
    listnodes(st, Kind::G, false)
        .into_iter()
        .map(|g| {
            let (_, _, fitness_label) = goals::fitness_view(g);
            json!({
                "id": g.id,
                "title": g.title,
                "fitness_label": fitness_label,
            })
        })
        .collect()
}

fn recent_discoveries(st: &State) -> Vec<Value> {
    let mut ys = listnodes(st, Kind::Y, false);
    ys.sort_by(|a, b| b.attr("t_updated").cmp(&a.attr("t_updated")));
    ys.into_iter()
        .take(RECENT_DISCOVERIES)
        .map(|y| {
            json!({
                "id": y.id,
                "title": y.title,
                "tags": y.lines("tags"),
            })
        })
        .collect()
}

pub fn model(st: &State, recs: &[Json], now_ts: &str) -> Value {
    let (c, v) = content_sums(st);
    let ratio = if v > 0 {
        format!("{:.2}", c as f64 / v as f64)
    } else {
        "n/a".to_string()
    };
    let (series, _replay_failures) = stats_cv_series(st, recs, now_ts);
    let trend = trend_from_series(&series);
    let goals = goal_rows(st);
    let discoveries = recent_discoveries(st);
    let nav_cards: Vec<Value> = NAV_CARDS
        .iter()
        .map(|(route, title, description, icon_name)| {
            json!({
                "route": route,
                "title": title,
                "description": description,
                "icon_name": icon_name,
            })
        })
        .collect();
    json!({
        "content": {
            "c": c,
            "v": v,
            "ratio": ratio,
            "trend_points": trend.points,
            "trend_delta": trend.delta,
            "trend_dir": trend.dir,
            "trend_variant": trend.variant,
            "trend_label": trend.label,
            "spark_w": SPARK_W,
            "spark_h": SPARK_H,
            "spark_c": trend.spark_c,
            "spark_v": trend.spark_v,
        },
        "work": work_model(st),
        "goals": goals,
        "goals_empty": goals.is_empty(),
        "goals_count": goals.len(),
        "discovery_items": discoveries,
        "discovery_empty": discoveries.is_empty(),
        "nav_cards": nav_cards,
    })
}

pub fn updated_label(now: SystemTime, lock_mtime: Option<SystemTime>) -> String {
    let Some(mtime) = lock_mtime else {
        return "n/a".to_string();
    };
    let age = now.duration_since(mtime).unwrap_or_default().as_secs();
    if age < 10 {
        "just now".to_string()
    } else if age < 60 {
        format!("{age}s ago")
    } else if age < 3600 {
        format!("{}m ago", age / 60)
    } else if age < 86400 {
        format!("{}h ago", age / 3600)
    } else {
        format!("{}d ago", age / 86400)
    }
}

pub fn lock_size_label(lock_bytes: Option<u64>) -> String {
    let Some(bytes) = lock_bytes else {
        return "n/a".to_string();
    };
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn status_bar_model(
    st: &State,
    lock_mtime: Option<SystemTime>,
    lock_bytes: Option<u64>,
) -> Value {
    let (c, v) = content_sums(st);
    json!({
        "c": c,
        "v": v,
        "g": listnodes(st, Kind::G, false).len(),
        "ready": count_status(st, Kind::W, &["ready"]),
        "done": count_status(st, Kind::W, &["done"]),
        "lock": lock_size_label(lock_bytes),
        "updated": updated_label(SystemTime::now(), lock_mtime),
    })
}

pub fn render(tpl: &Templates, root: &str) -> Result<String, String> {
    let st = load_state(root)?;
    let ctx = CliCtx::new(root.to_string());
    let (_, recs) = journal_read_nonempty_pairs(&ctx.journalpath());
    tpl.render("overview", &model(&st, &recs, &utc_stamp_second()))
}
