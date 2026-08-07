use crate::model::{Kind, Node, State};
use crate::status::listnodes;
use crate::times::stamp_touch_node;
use std::collections::BTreeSet;

pub const GOAL_FITNESS_KINDS: [&str; 5] = ["count", "ratio", "boolean", "metric", "manual"];

pub fn goal_structured_kind(g: &Node) -> Option<&'static str> {
    if g.kind != Kind::G {
        return None;
    }
    let raw = g.attr("fitness_kind");
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    GOAL_FITNESS_KINDS.iter().copied().find(|k| *k == s)
}

pub fn parse_fitness_target(label: &str) -> Option<i64> {
    let b = label.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n {
        if b[i].is_ascii_digit() {
            let mut j = i;
            while j < n && b[j].is_ascii_digit() {
                j += 1;
            }
            let mut k = j;
            while k < n && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < n && b[k] == b'/' {
                let mut m = k + 1;
                while m < n && b[m].is_ascii_whitespace() {
                    m += 1;
                }
                let mut p = m;
                while p < n && b[p].is_ascii_digit() {
                    p += 1;
                }
                if p > m {
                    return label[m..p].parse::<i64>().ok();
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    None
}

pub fn aggregate_fitness_delta(st: &State, gid: &str) -> i64 {
    let mut t = 0i64;
    for ww in listnodes(st, Kind::W, false) {
        if ww.status != "done" {
            continue;
        }
        let fd = ww.fitness();
        if let Some(v) = fd.get(gid) {
            t += v;
        }
    }
    t
}

fn parse_nonneg_int(val: &str) -> Option<i64> {
    match val.trim().parse::<i64>() {
        Ok(v) if v >= 0 => Some(v),
        _ => None,
    }
}

fn sync_goal_fitness_current_field(g: &mut Node, kind: &str, total: i64) {
    if kind == "boolean" {
        g.set_single(
            "fitness_current",
            if total >= 1 {
                "true".to_string()
            } else {
                "false".to_string()
            },
        );
    } else {
        g.set_single("fitness_current", total.to_string());
    }
}

fn refresh_goal_legacy(g: &mut Node, total: i64) {
    let prev = g.status.clone();
    let label = g.attr("fitness");
    let target = parse_fitness_target(&label);
    let verified = match target {
        Some(t) => total >= t,
        None => false,
    };
    if verified {
        g.status = "verified".to_string();
    } else if total > 0 {
        g.status = "partial".to_string();
    }
    if g.status != prev {
        stamp_touch_node(g);
    }
}

pub fn refresh_goal_structured_fitness(st: &mut State, gid: &str) {
    let kind = match st.nodes.get(gid) {
        None => return,
        Some(g) => {
            if g.kind != Kind::G {
                return;
            }
            goal_structured_kind(g)
        }
    };
    let total = aggregate_fitness_delta(st, gid);
    let kind = match kind {
        None => {
            let g = st.nodes.get_mut(gid).expect("goal node exists");
            refresh_goal_legacy(g, total);
            return;
        }
        Some(k) => k,
    };
    if kind == "manual" {
        return;
    }
    let g = st.nodes.get_mut(gid).expect("goal node exists");
    sync_goal_fitness_current_field(g, kind, total);
    let prev = g.status.clone();
    let tgt_txt = g.single("fitness_target").trim().to_string();
    match kind {
        "boolean" => {
            if total >= 1 {
                g.status = "verified".to_string();
            }
        }
        "count" | "metric" => {
            let ntar = if tgt_txt.is_empty() {
                None
            } else {
                parse_nonneg_int(&tgt_txt)
            };
            if let Some(ntar) = ntar {
                if total >= ntar {
                    g.status = "verified".to_string();
                } else if total > 0 {
                    g.status = "partial".to_string();
                }
            }
        }
        "ratio" => {
            let mut ntar = parse_fitness_target(&tgt_txt);
            if ntar.is_none() && !tgt_txt.is_empty() {
                ntar = parse_nonneg_int(&tgt_txt);
            }
            if let Some(ntar) = ntar {
                if total >= ntar {
                    g.status = "verified".to_string();
                } else if total > 0 {
                    g.status = "partial".to_string();
                }
            }
        }
        _ => {}
    }
    if g.status != prev {
        stamp_touch_node(g);
    }
}

pub fn rederive_goals(st: &mut State, wid: &str) {
    let gids = match st.nodes.get(wid) {
        None => return,
        Some(w) => {
            if w.status != "done" {
                return;
            }
            w.lines("goals")
        }
    };
    let mut seen = BTreeSet::new();
    for gid0 in gids {
        let gid = gid0.trim().to_string();
        if gid.is_empty() {
            continue;
        }
        if !seen.insert(gid.clone()) {
            continue;
        }
        match st.nodes.get(&gid) {
            None => continue,
            Some(gg) => {
                if gg.kind != Kind::G {
                    continue;
                }
            }
        }
        refresh_goal_structured_fitness(st, &gid);
    }
}

pub fn goal_fitness_table_cell(g: &Node) -> String {
    let k = g.attr("fitness_kind").trim().to_string();
    if k.is_empty() {
        return g.attr("fitness").trim().to_string();
    }
    let cur = g.single("fitness_current").trim().to_string();
    let tgt = g.single("fitness_target").trim().to_string();
    let mut parts = vec![k];
    if !cur.is_empty() || !tgt.is_empty() {
        if tgt.is_empty() {
            parts.push(format!("current={}", cur));
        } else {
            parts.push(format!("current={} target={}", cur, tgt));
        }
    } else {
        let legacy = g.attr("fitness").trim().to_string();
        if !legacy.is_empty() {
            parts.push(legacy);
        }
    }
    parts.join("; ")
}
