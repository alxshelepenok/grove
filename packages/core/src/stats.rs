use crate::archive::exclusive_archive_ids;
use crate::cli::{json_cli_out, load, CliCtx};
use crate::journal::{journal_apply_inverse, journal_read_nonempty_pairs, journal_record_mutation};
use crate::json::{julia_float_repr, julia_num_repr, julia_round_digits2, JVal, Json, JuliaDict};
use crate::model::{Kind, State};
use crate::ops::OpResult;
use crate::render::content_health_sums;
use crate::status::listnodes;
use crate::times::{format_unix_utc, parse_rfc3339_utc_second, utc_stamp_second};
use std::collections::{BTreeMap, BTreeSet};

pub struct StatsOut {
    pub payload: JuliaDict,
    pub text: String,
}

pub type StatsIntervals = BTreeMap<String, Vec<(Option<i64>, i64, String)>>;

const STATS_STATUS_OPS: [&str; 3] = [
    "set_status_plain",
    "set_w_status_with_goals",
    "revalidate_restore",
];

fn jstr(v: &Json, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn rec_ts(rec: &Json) -> Option<i64> {
    parse_rfc3339_utc_second(&jstr(rec, "ts"))
}

fn inv_obj(rec: &Json) -> Option<&Json> {
    match rec.get("inv") {
        Some(v) if v.as_obj().is_some() => Some(v),
        _ => None,
    }
}

pub fn stats_median(xs: &mut Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.total_cmp(b));
    let n = xs.len();
    if n % 2 == 1 {
        xs[n / 2]
    } else {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    }
}

pub fn stats_intervals(st: &State, recs: &[Json], now_dt: i64) -> StatsIntervals {
    let mut tracked: BTreeMap<String, String> = BTreeMap::new();
    let mut cursor: BTreeMap<String, i64> = BTreeMap::new();
    let mut ivals: StatsIntervals = BTreeMap::new();
    for (id, n) in &st.nodes {
        tracked.insert(id.clone(), n.status.clone());
        cursor.insert(id.clone(), now_dt);
        ivals.insert(id.clone(), Vec::new());
    }
    let mut birth: BTreeMap<String, i64> = BTreeMap::new();
    let mut touched: BTreeSet<String> = BTreeSet::new();
    let mut oldest: Option<i64> = None;
    for rec in recs {
        let Some(ts) = rec_ts(rec) else {
            continue;
        };
        if oldest.is_none_or(|o| ts < o) {
            oldest = Some(ts);
        }
    }
    for rec in recs.iter().rev() {
        let Some(ts) = rec_ts(rec) else {
            continue;
        };
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        let op = jstr(inv, "op");
        let id = jstr(inv, "id");
        if STATS_STATUS_OPS.contains(&op.as_str()) && tracked.contains_key(&id) {
            let old = if op == "set_w_status_with_goals" {
                jstr(inv, "old_w_status")
            } else {
                jstr(inv, "old_status")
            };
            let stop = cursor[&id];
            let status = tracked[&id].clone();
            ivals
                .get_mut(&id)
                .expect("tracked id")
                .push((Some(ts), stop, status));
            tracked.insert(id.clone(), old);
            cursor.insert(id.clone(), ts);
            touched.insert(id.clone());
        } else if op == "rm_node" && jstr(rec, "cmd") == "add" && st.nodes.contains_key(&id) {
            birth.insert(id.clone(), ts);
            touched.insert(id.clone());
        }
    }
    for id in st.nodes.keys() {
        let start = if let Some(b) = birth.get(id) {
            Some(*b)
        } else if touched.contains(id) {
            oldest
        } else {
            None
        };
        let stop = cursor[id];
        let status = tracked[id].clone();
        let v = ivals.get_mut(id).expect("present");
        v.push((start, stop, status));
        v.reverse();
    }
    ivals
}

pub fn stats_cycle_time(
    st: &State,
    ivals: &StatsIntervals,
) -> (JuliaDict, Vec<i64>, Vec<(String, i64, f64, f64, f64)>) {
    let mut order: Vec<String> = Vec::new();
    let mut by_class: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut all_seconds: Vec<i64> = Vec::new();
    for n in listnodes(st, Kind::W, true) {
        let ivs = &ivals[&n.id];
        let tr: Vec<i64> = ivs
            .iter()
            .filter(|(start, _, status)| status == "ready" && start.is_some())
            .map(|(start, _, _)| start.expect("filtered"))
            .collect();
        let td: Vec<i64> = ivs
            .iter()
            .filter(|(start, _, status)| status == "done" && start.is_some())
            .map(|(start, _, _)| start.expect("filtered"))
            .collect();
        if tr.is_empty() || td.is_empty() {
            continue;
        }
        let t0 = tr.iter().copied().min().expect("nonempty");
        let t1 = td.iter().copied().min().expect("nonempty");
        if t1 < t0 {
            continue;
        }
        let secs = t1 - t0;
        let cls = n.cynefin.clone().unwrap_or_else(|| "none".to_string());
        if !by_class.contains_key(&cls) {
            order.push(cls.clone());
        }
        by_class.entry(cls).or_default().push(secs);
        all_seconds.push(secs);
    }
    let mut classes = JuliaDict::new();
    let mut rows: Vec<(String, i64, f64, f64, f64)> = Vec::new();
    for cls in &order {
        let secs = &by_class[cls];
        let mut hrs: Vec<f64> = secs.iter().map(|s| *s as f64 / 3600.0).collect();
        let n = hrs.len() as i64;
        let mean = hrs.iter().sum::<f64>() / n as f64;
        let median = stats_median(&mut hrs);
        let max = hrs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        rows.push((cls.clone(), n, mean, median, max));
        classes.insert(
            cls.clone(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("n".to_string(), JVal::Int(n)),
                ("mean_hours".to_string(), JVal::Float(mean)),
                ("median_hours".to_string(), JVal::Float(median)),
                ("max_hours".to_string(), JVal::Float(max)),
            ])),
        );
    }
    (classes, all_seconds, rows)
}

pub fn stats_dor(
    st: &State,
    recs: &[Json],
    ivals: &StatsIntervals,
) -> (i64, JuliaDict, Vec<(String, i64)>, i64, i64, JVal) {
    let mut total = 0i64;
    let mut order: Vec<String> = Vec::new();
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut reject_ts: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for rec in recs {
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "dor_reject" {
            continue;
        }
        let id = jstr(inv, "id");
        total += 1;
        if !counts.contains_key(&id) {
            order.push(id.clone());
        }
        *counts.entry(id.clone()).or_insert(0) += 1;
        if let Some(ts) = rec_ts(rec) {
            reject_ts.entry(id).or_default().push(ts);
        }
    }
    let mut per_node = JuliaDict::new();
    for id in &order {
        per_node.insert(id.clone(), JVal::Int(counts[id]));
    }
    let sorted: Vec<(String, i64)> = counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let mut entries = 0i64;
    let mut first_pass = 0i64;
    for n in listnodes(st, Kind::W, true) {
        for (start, _, status) in &ivals[&n.id] {
            if status != "progress" || start.is_none() {
                continue;
            }
            entries += 1;
            let s = start.expect("checked");
            let rejected_before = reject_ts
                .get(&n.id)
                .map(|v| v.iter().any(|t| *t < s))
                .unwrap_or(false);
            if !rejected_before {
                first_pass += 1;
            }
        }
    }
    let rate = if entries == 0 {
        JVal::Null
    } else {
        JVal::Float(first_pass as f64 / entries as f64)
    };
    (total, per_node, sorted, entries, first_pass, rate)
}

pub fn stats_bets(st: &State, ivals: &StatsIntervals) -> ((i64, i64, i64), JVal) {
    let mut validated = 0i64;
    let mut acceptable = 0i64;
    let mut blocking = 0i64;
    for n in listnodes(st, Kind::B, true) {
        for (start, _, status) in &ivals[&n.id] {
            if start.is_none() {
                continue;
            }
            match status.as_str() {
                "validated" => validated += 1,
                "invalidated_acceptable" => acceptable += 1,
                "invalidated_blocking" => blocking += 1,
                _ => {}
            }
        }
    }
    let den = acceptable + blocking;
    let ratio = if den == 0 {
        JVal::Null
    } else {
        JVal::Float(validated as f64 / den as f64)
    };
    ((validated, acceptable, blocking), ratio)
}

pub fn stats_discovery(
    st: &State,
    recs: &[Json],
    ivals: &StatsIntervals,
) -> (i64, i64, i64, i64, i64, i64, Vec<JVal>) {
    let mut stale_entries = 0i64;
    for n in listnodes(st, Kind::Y, true) {
        for (start, _, status) in &ivals[&n.id] {
            if status == "stale" && start.is_some() {
                stale_entries += 1;
            }
        }
    }
    let mut revalidations = 0i64;
    let mut gate_runs = 0i64;
    let mut gate_empty = 0i64;
    let mut overflow_events = 0i64;
    let mut invalidated_events = 0i64;
    let mut gates: Vec<JVal> = Vec::new();
    for rec in recs {
        if jstr(rec, "cmd") == "revalidate" {
            revalidations += 1;
        }
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "gate" {
            continue;
        }
        gate_runs += 1;
        let empty = inv
            .get("empty")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if empty {
            gate_empty += 1;
        }
        let ov = inv
            .get("overflows")
            .and_then(|v| v.as_arr())
            .map(|a| a.len() as i64);
        if let Some(k) = ov {
            overflow_events += k;
        }
        let ivl = inv
            .get("invalidated")
            .and_then(|v| v.as_arr())
            .map(|a| a.len() as i64);
        if let Some(k) = ivl {
            invalidated_events += k;
        }
        let overflow_paths = match inv.get("overflow_counts") {
            Some(Json::Obj(pairs)) => {
                JVal::Int(pairs.iter().map(|(_, v)| v.as_i64().unwrap_or(0)).sum())
            }
            _ => JVal::Null,
        };
        gates.push(JVal::Obj(JuliaDict::from_pairs(vec![
            ("ts".to_string(), JVal::Str(jstr(rec, "ts"))),
            (
                "tw".to_string(),
                JVal::Int(inv.get("tw").and_then(|v| v.as_i64()).unwrap_or(0)),
            ),
            (
                "dones".to_string(),
                JVal::Int(inv.get("dones").and_then(|v| v.as_i64()).unwrap_or(0)),
            ),
            ("empty".to_string(), JVal::Bool(empty)),
            (
                "overflow_events".to_string(),
                JVal::Int(ov.unwrap_or(0)),
            ),
            ("overflow_paths".to_string(), overflow_paths),
            (
                "invalidated_events".to_string(),
                JVal::Int(ivl.unwrap_or(0)),
            ),
        ])));
    }
    (
        stale_entries,
        revalidations,
        gate_runs,
        gate_empty,
        overflow_events,
        invalidated_events,
        gates,
    )
}

pub fn stats_undo(recs: &[Json]) -> (i64, i64, i64, JVal) {
    let mut events = 0i64;
    let mut steps = 0i64;
    let mut mutations = 0i64;
    for rec in recs {
        if journal_record_mutation(rec) {
            mutations += 1;
        }
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "undo" {
            continue;
        }
        events += 1;
        steps += inv.get("steps").and_then(|v| v.as_i64()).unwrap_or(0);
    }
    let ratio = if mutations == 0 {
        JVal::Null
    } else {
        JVal::Float((100 * events) as f64 / mutations as f64)
    };
    (events, steps, mutations, ratio)
}

pub fn stats_sessions(recs: &[Json]) -> (Vec<(String, i64)>, JVal, JVal, JVal) {
    let mut per: BTreeMap<String, i64> = BTreeMap::new();
    for rec in recs {
        let raw = match rec.get("session") {
            Some(Json::Str(x)) => x.clone(),
            Some(Json::Int(i)) => i.to_string(),
            Some(Json::Float(f)) => julia_float_repr(*f),
            Some(Json::Bool(b)) => b.to_string(),
            _ => "unknown".to_string(),
        };
        let tok = raw.trim();
        let tok = if tok.is_empty() { "unknown" } else { tok };
        *per.entry(tok.to_string()).or_insert(0) += 1;
    }
    let rows: Vec<(String, i64)> = per.into_iter().collect();
    if rows.is_empty() {
        return (rows, JVal::Null, JVal::Null, JVal::Null);
    }
    let mut counts: Vec<f64> = rows.iter().map(|(_, c)| *c as f64).collect();
    let mean = counts.iter().sum::<f64>() / counts.len() as f64;
    let median = stats_median(&mut counts);
    let max = rows.iter().map(|(_, c)| *c).max().expect("nonempty");
    (rows, JVal::Float(mean), JVal::Float(median), JVal::Int(max))
}

fn stats_hours_summary(hrs: &[f64]) -> JuliaDict {
    let (mean, median, max) = if hrs.is_empty() {
        (JVal::Null, JVal::Null, JVal::Null)
    } else {
        let mut sorted = hrs.to_vec();
        let mean = hrs.iter().sum::<f64>() / hrs.len() as f64;
        let median = stats_median(&mut sorted);
        let max = hrs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (JVal::Float(mean), JVal::Float(median), JVal::Float(max))
    };
    JuliaDict::from_pairs(vec![
        ("n".to_string(), JVal::Int(hrs.len() as i64)),
        ("mean_hours".to_string(), mean),
        ("median_hours".to_string(), median),
        ("max_hours".to_string(), max),
    ])
}

pub fn stats_checkpoint_latency(
    st: &State,
    recs: &[Json],
    ivals: &StatsIntervals,
) -> (Vec<f64>, Vec<f64>) {
    let mut dor_hrs: Vec<f64> = Vec::new();
    for rec in recs {
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "dor_reject" {
            continue;
        }
        let id = jstr(inv, "id");
        let Some(ivs) = ivals.get(&id) else {
            continue;
        };
        let Some(rts) = rec_ts(rec) else {
            continue;
        };
        let starts: Vec<i64> = ivs
            .iter()
            .filter(|(start, _, status)| status == "progress" && start.is_some_and(|s| s > rts))
            .map(|(start, _, _)| start.expect("filtered"))
            .collect();
        let Some(t0) = starts.iter().copied().min() else {
            continue;
        };
        dor_hrs.push((t0 - rts) as f64 / 3600.0);
    }
    let mut disc_hrs: Vec<f64> = Vec::new();
    for n in listnodes(st, Kind::Y, true) {
        let ivs = &ivals[&n.id];
        let t0s: Vec<i64> = ivs
            .iter()
            .filter(|(start, _, status)| status == "proposed" && start.is_some())
            .map(|(start, _, _)| start.expect("filtered"))
            .collect();
        let t1s: Vec<i64> = ivs
            .iter()
            .filter(|(start, _, status)| status == "active" && start.is_some())
            .map(|(start, _, _)| start.expect("filtered"))
            .collect();
        if t0s.is_empty() || t1s.is_empty() {
            continue;
        }
        let t0 = t0s.iter().copied().min().expect("nonempty");
        let t1 = t1s.iter().copied().min().expect("nonempty");
        if t1 < t0 {
            continue;
        }
        disc_hrs.push((t1 - t0) as f64 / 3600.0);
    }
    (dor_hrs, disc_hrs)
}

pub fn stats_post_approval_invalidation(st: &State, ivals: &StatsIntervals) -> (i64, i64, JVal) {
    let mut ever_validated = 0i64;
    let mut invalidated = 0i64;
    for n in listnodes(st, Kind::B, true) {
        let ivs = &ivals[&n.id];
        let Some(k) = ivs.iter().position(|(_, _, status)| status == "validated") else {
            continue;
        };
        ever_validated += 1;
        if ivs[k + 1..]
            .iter()
            .any(|(_, _, status)| status == "invalidated_acceptable" || status == "invalidated_blocking")
        {
            invalidated += 1;
        }
    }
    let rate = if ever_validated == 0 {
        JVal::Null
    } else {
        JVal::Float(invalidated as f64 / ever_validated as f64)
    };
    (invalidated, ever_validated, rate)
}

pub fn stats_rework(st: &State, reject_per_node: &BTreeMap<String, i64>) -> JuliaDict {
    let mut covered_surfaces: BTreeSet<String> = BTreeSet::new();
    for y in listnodes(st, Kind::Y, false) {
        if y.status != "active" {
            continue;
        }
        for s in y.lines("surface") {
            covered_surfaces.insert(s);
        }
    }
    let mut out = JuliaDict::new();
    for (key, want_covered) in [("covered", true), ("uncovered", false)] {
        let mut rows: Vec<(String, i64)> = Vec::new();
        for w in listnodes(st, Kind::W, true) {
            let surf = w.lines("surface");
            let covered = !surf.is_empty() && surf.iter().any(|s| covered_surfaces.contains(s));
            if covered != want_covered {
                continue;
            }
            rows.push((w.id.clone(), reject_per_node.get(&w.id).copied().unwrap_or(0)));
        }
        let total: i64 = rows.iter().map(|(_, r)| r).sum();
        let mean = if rows.is_empty() {
            JVal::Null
        } else {
            JVal::Float(julia_round_digits2(total as f64 / rows.len() as f64))
        };
        let per_w: Vec<JVal> = rows
            .iter()
            .map(|(id, r)| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(id.clone())),
                    ("rejects".to_string(), JVal::Int(*r)),
                ]))
            })
            .collect();
        out.insert(
            key.to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("w".to_string(), JVal::Int(rows.len() as i64)),
                ("rejects".to_string(), JVal::Int(total)),
                ("mean_rejects".to_string(), mean),
                ("per_w".to_string(), JVal::Arr(per_w)),
            ])),
        );
    }
    out
}

pub fn stats_distill_yield(st: &State, recs: &[Json]) -> (i64, i64, i64, Vec<JVal>) {
    let goals: Vec<_> = listnodes(st, Kind::G, true)
        .into_iter()
        .filter(|g| g.archived)
        .collect();
    let mut null_attested: BTreeSet<String> = BTreeSet::new();
    for rec in recs {
        if jstr(rec, "cmd") != "distill" {
            continue;
        }
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        let gid = jstr(inv, "goal");
        if !gid.is_empty() {
            null_attested.insert(gid);
        }
    }
    let mut entries: Vec<JVal> = Vec::new();
    let mut real = 0i64;
    let mut null = 0i64;
    let mut none = 0i64;
    if !goals.is_empty() {
        let mut st_open = st.clone();
        for n in st_open.nodes.values_mut() {
            n.archived = false;
        }
        for g in &goals {
            let pool = exclusive_archive_ids(&st_open, &g.id);
            let mut yids: BTreeSet<String> = BTreeSet::new();
            for e in &st.edges {
                if e.label != "distills" {
                    continue;
                }
                if !pool.contains(&e.to) {
                    continue;
                }
                let Some(yn) = st.nodes.get(&e.from) else {
                    continue;
                };
                if yn.kind != Kind::Y {
                    continue;
                }
                yids.insert(e.from.clone());
            }
            let status = if !yids.is_empty() {
                real += 1;
                "real"
            } else if null_attested.contains(&g.id) {
                null += 1;
                "null"
            } else {
                none += 1;
                "none"
            };
            entries.push(JVal::Obj(JuliaDict::from_pairs(vec![
                ("goal".to_string(), JVal::Str(g.id.clone())),
                ("status".to_string(), JVal::Str(status.to_string())),
                (
                    "discoveries".to_string(),
                    JVal::Arr(yids.iter().map(|y| JVal::Str(y.clone())).collect()),
                ),
            ])));
        }
    }
    (real, null, none, entries)
}

pub fn stats_dor_split(st: &State, recs: &[Json], ivals: &StatsIntervals) -> ((i64, i64, i64), JVal) {
    let mut reject_ts: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for rec in recs {
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "dor_reject" {
            continue;
        }
        let Some(ts) = rec_ts(rec) else {
            continue;
        };
        reject_ts.entry(jstr(inv, "id")).or_default().push(ts);
    }
    let mut mutations: Vec<(i64, &Json)> = Vec::new();
    for rec in recs {
        if !journal_record_mutation(rec) {
            continue;
        }
        let Some(ts) = rec_ts(rec) else {
            continue;
        };
        mutations.push((ts, rec));
    }
    let mut no_reject = 0i64;
    let mut reject_discovery = 0i64;
    let mut reject_plain = 0i64;
    for n in listnodes(st, Kind::W, true) {
        for (start, _, status) in &ivals[&n.id] {
            if status != "progress" || start.is_none() {
                continue;
            }
            let s = start.expect("checked");
            let latest = reject_ts
                .get(&n.id)
                .and_then(|v| v.iter().copied().filter(|t| *t < s).max());
            let Some(latest) = latest else {
                no_reject += 1;
                continue;
            };
            let discovery = mutations.iter().any(|(ts, rec)| {
                if *ts <= latest || *ts >= s {
                    return false;
                }
                let Some(inv) = inv_obj(rec) else {
                    return false;
                };
                let id = jstr(inv, "id");
                if id.is_empty() {
                    return false;
                }
                match st.nodes.get(&id) {
                    Some(nd) => matches!(nd.kind, Kind::Q | Kind::B | Kind::Y),
                    None => false,
                }
            });
            if discovery {
                reject_discovery += 1;
            } else {
                reject_plain += 1;
            }
        }
    }
    let den = reject_discovery + reject_plain;
    let rate = if den == 0 {
        JVal::Null
    } else {
        JVal::Float(reject_discovery as f64 / den as f64)
    };
    ((no_reject, reject_discovery, reject_plain), rate)
}

pub fn stats_surprise_series(
    st: &State,
    recs: &[Json],
    ivals: &StatsIntervals,
    series: &[(String, i64, i64)],
) -> Vec<JVal> {
    let mut events: Vec<i64> = Vec::new();
    for n in listnodes(st, Kind::B, true) {
        for (start, _, status) in &ivals[&n.id] {
            if status != "invalidated_acceptable" && status != "invalidated_blocking" {
                continue;
            }
            let Some(s) = start else {
                continue;
            };
            events.push(*s);
        }
    }
    for rec in recs {
        let Some(inv) = inv_obj(rec) else {
            continue;
        };
        if jstr(inv, "op") != "gate" {
            continue;
        }
        let Some(ov) = inv.get("overflows").and_then(|v| v.as_arr()) else {
            continue;
        };
        if ov.is_empty() {
            continue;
        }
        let Some(ts) = rec_ts(rec) else {
            continue;
        };
        for _ in 0..ov.len() {
            events.push(ts);
        }
    }
    events.sort();
    let mut cv: Vec<(i64, i64)> = Vec::new();
    for (ts_str, c, _) in series {
        let Some(ts) = parse_rfc3339_utc_second(ts_str) else {
            continue;
        };
        cv.push((ts, *c));
    }
    cv.sort_by(|a, b| a.0.cmp(&b.0));
    let mut dones: Vec<(String, i64)> = Vec::new();
    for n in listnodes(st, Kind::W, true) {
        for (start, _, status) in &ivals[&n.id] {
            if status != "done" || start.is_none() {
                continue;
            }
            dones.push((n.id.clone(), start.expect("checked")));
        }
    }
    dones.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let mut out: Vec<JVal> = Vec::new();
    let mut prev: Option<i64> = None;
    for (wid, ts) in &dones {
        let delta = events
            .iter()
            .filter(|t| prev.is_none_or(|p| **t > p) && **t <= *ts)
            .count() as i64;
        let mut c = 0i64;
        for (cts, cc) in &cv {
            if *cts > *ts {
                break;
            }
            c = *cc;
        }
        out.push(JVal::Obj(JuliaDict::from_pairs(vec![
            ("id".to_string(), JVal::Str(wid.clone())),
            ("ts".to_string(), JVal::Str(format_unix_utc(*ts))),
            ("delta".to_string(), JVal::Int(delta)),
            ("c".to_string(), JVal::Int(c)),
        ])));
        prev = Some(*ts);
    }
    out
}

pub fn stats_cv_series(
    st: &State,
    recs: &[Json],
    now_ts: &str,
) -> (Vec<(String, i64, i64)>, i64) {
    let mut r = st.clone();
    let (c0, v0) = content_health_sums(st);
    let mut series: Vec<(String, i64, i64)> = vec![(now_ts.to_string(), c0, v0)];
    let mut failures = 0i64;
    for rec in recs.iter().rev() {
        if !journal_record_mutation(rec) {
            continue;
        }
        let inv = rec.get("inv").cloned().unwrap_or(Json::Null);
        match journal_apply_inverse(&mut r, &inv) {
            Some(_) => failures += 1,
            None => {
                let (c, v) = content_health_sums(&r);
                series.push((jstr(rec, "ts"), c, v));
            }
        }
    }
    series.reverse();
    (series, failures)
}

pub fn stats_fmt_num(v: &JVal) -> String {
    match v {
        JVal::Null => "\u{2013}".to_string(),
        JVal::Float(x) => julia_num_repr(*x),
        JVal::Int(i) => i.to_string(),
        JVal::Str(s) => s.clone(),
        _ => String::new(),
    }
}

fn dget<'a>(d: &'a JuliaDict, key: &str) -> &'a JVal {
    d.iter_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("stats payload missing key {key}"))
}

pub fn compute_stats(st: &State, recs: &[Json], now_ts: &str) -> StatsOut {
    let now_dt = parse_rfc3339_utc_second(now_ts).expect("stats: unparseable now_ts");
    let ivals = stats_intervals(st, recs, now_dt);
    let (cycle_classes, cycle_seconds, cycle_rows) = stats_cycle_time(st, &ivals);
    let (reject_total, reject_per_node, reject_sorted, progress_entries, first_pass, rate) =
        stats_dor(st, recs, &ivals);
    let ((bet_validated, bet_acceptable, bet_blocking), bet_ratio) = stats_bets(st, &ivals);
    let (stale_entries, revalidations, gate_runs, gate_empty, overflow_events, invalidated_events, gates) =
        stats_discovery(st, recs, &ivals);
    let (undo_events, undone_steps, mutations, undos_per_100) = stats_undo(recs);
    let surprise_total = bet_acceptable + bet_blocking + overflow_events;
    let done_w = listnodes(st, Kind::W, true)
        .iter()
        .filter(|n| ivals[&n.id].iter().any(|(_, _, status)| status == "done"))
        .count() as i64;
    let per_done = if done_w == 0 {
        JVal::Null
    } else {
        JVal::Float(surprise_total as f64 / done_w as f64)
    };
    let (series, replay_failures) = stats_cv_series(st, recs, now_ts);
    let (session_rows, session_mean, session_median, session_max) = stats_sessions(recs);
    let session_entries: Vec<JVal> = session_rows
        .iter()
        .map(|(t, c)| {
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("session".to_string(), JVal::Str(t.clone())),
                ("commands".to_string(), JVal::Int(*c)),
            ]))
        })
        .collect();
    let (latency_dor, latency_discovery) = stats_checkpoint_latency(st, recs, &ivals);
    let latency_dor_summary = stats_hours_summary(&latency_dor);
    let latency_discovery_summary = stats_hours_summary(&latency_discovery);
    let (pai_invalidated, pai_ever, pai_rate) = stats_post_approval_invalidation(st, &ivals);
    let ((split_no_reject, split_reject_discovery, split_reject_plain), split_rate) =
        stats_dor_split(st, recs, &ivals);
    let reject_counts: BTreeMap<String, i64> = reject_sorted.iter().cloned().collect();
    let rework = stats_rework(st, &reject_counts);
    let (yield_real, yield_null, yield_none, yield_goals) = stats_distill_yield(st, recs);
    let surprise_series = stats_surprise_series(st, recs, &ivals, &series);
    let mut text = String::new();
    text.push_str(&format!("records: {}\n", recs.len()));
    text.push_str(&format!("mutations: {mutations}\n"));
    text.push('\n');
    text.push_str("cycle time (ready -> done):\n");
    if cycle_rows.is_empty() {
        text.push_str("  (no W with ready and done intervals)\n");
    } else {
        text.push_str(&format!(
            "  {:<10} {:>5} {:>8} {:>9} {:>8}\n",
            "class", "n", "mean h", "median h", "max h"
        ));
        let mut rows = cycle_rows;
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        for (cls, n, mean, median, max) in &rows {
            text.push_str(&format!(
                "  {:<10} {:>5} {:>8} {:>9} {:>8}\n",
                cls,
                n,
                stats_fmt_num(&JVal::Float(*mean)),
                stats_fmt_num(&JVal::Float(*median)),
                stats_fmt_num(&JVal::Float(*max))
            ));
        }
    }
    text.push('\n');
    text.push_str("DoR:\n");
    text.push_str(&format!("  reject events: {reject_total}\n"));
    for (id, k) in &reject_sorted {
        text.push_str(&format!("    {id}: {k}\n"));
    }
    text.push_str(&format!("  progress entries: {progress_entries}\n"));
    text.push_str(&format!("  first pass: {first_pass}\n"));
    text.push_str(&format!("  first pass rate: {}\n", stats_fmt_num(&rate)));
    text.push_str("  first pass split:\n");
    text.push_str(&format!("    no reject: {split_no_reject}\n"));
    text.push_str(&format!("    reject + discovery: {split_reject_discovery}\n"));
    text.push_str(&format!("    reject plain: {split_reject_plain}\n"));
    text.push_str(&format!(
        "    discovery rate: {}\n",
        stats_fmt_num(&split_rate)
    ));
    text.push('\n');
    text.push_str("bets:\n");
    text.push_str(&format!("  validated: {bet_validated}\n"));
    text.push_str(&format!("  invalidated acceptable: {bet_acceptable}\n"));
    text.push_str(&format!("  invalidated blocking: {bet_blocking}\n"));
    text.push_str(&format!("  ratio: {}\n", stats_fmt_num(&bet_ratio)));
    text.push('\n');
    text.push_str("discovery:\n");
    text.push_str(&format!("  stale entries: {stale_entries}\n"));
    text.push_str(&format!("  revalidations: {revalidations}\n"));
    text.push_str(&format!("  gate runs: {gate_runs}\n"));
    text.push_str(&format!("  gate empty: {gate_empty}\n"));
    text.push_str(&format!("  gate overflow events: {overflow_events}\n"));
    text.push_str(&format!(
        "  gate invalidated events: {invalidated_events}\n"
    ));
    text.push('\n');
    text.push_str("undo:\n");
    text.push_str(&format!("  events: {undo_events}\n"));
    text.push_str(&format!("  undone steps: {undone_steps}\n"));
    text.push_str(&format!(
        "  per 100 mutations: {}\n",
        stats_fmt_num(&undos_per_100)
    ));
    text.push('\n');
    text.push_str("audit:\n");
    text.push_str("  commands per session:\n");
    for e in &session_entries {
        let JVal::Obj(d) = e else {
            continue;
        };
        let tok = stats_fmt_num(dget(d, "session"));
        let tok: String = tok.chars().take(24).collect();
        text.push_str(&format!("    {tok} {}\n", stats_fmt_num(dget(d, "commands"))));
    }
    text.push_str(&format!(
        "  sessions: {} mean {} median {} max {}\n",
        session_entries.len(),
        stats_fmt_num(&session_mean),
        stats_fmt_num(&session_median),
        stats_fmt_num(&session_max)
    ));
    text.push_str("  checkpoint latency (hours):\n");
    for (label, d) in [
        ("dor reject -> progress", &latency_dor_summary),
        ("discovery proposed -> active", &latency_discovery_summary),
    ] {
        text.push_str(&format!(
            "    {label}: n {} mean {} median {} max {}\n",
            stats_fmt_num(dget(d, "n")),
            stats_fmt_num(dget(d, "mean_hours")),
            stats_fmt_num(dget(d, "median_hours")),
            stats_fmt_num(dget(d, "max_hours"))
        ));
    }
    text.push_str(&format!(
        "  post-approval invalidation: {pai_invalidated} / {pai_ever} (rate {})\n",
        stats_fmt_num(&pai_rate)
    ));
    text.push('\n');
    text.push_str("rework:\n");
    for key in ["covered", "uncovered"] {
        let JVal::Obj(g) = dget(&rework, key) else {
            continue;
        };
        text.push_str(&format!(
            "  {key}: {} W, {} rejects, mean {} per W\n",
            stats_fmt_num(dget(g, "w")),
            stats_fmt_num(dget(g, "rejects")),
            stats_fmt_num(dget(g, "mean_rejects"))
        ));
        let JVal::Arr(rows) = dget(g, "per_w") else {
            continue;
        };
        for r in rows {
            let JVal::Obj(r) = r else {
                continue;
            };
            text.push_str(&format!(
                "    {}: {}\n",
                stats_fmt_num(dget(r, "id")),
                stats_fmt_num(dget(r, "rejects"))
            ));
        }
    }
    text.push_str("  note: undo events are global; undone journal lines are dropped\n");
    text.push('\n');
    text.push_str("distill yield:\n");
    text.push_str(&format!(
        "  real: {yield_real} null: {yield_null} none: {yield_none}\n"
    ));
    for e in &yield_goals {
        let JVal::Obj(d) = e else {
            continue;
        };
        let goal = stats_fmt_num(dget(d, "goal"));
        let status = stats_fmt_num(dget(d, "status"));
        if status == "real" {
            let JVal::Arr(ys) = dget(d, "discoveries") else {
                continue;
            };
            let names: Vec<String> = ys.iter().map(stats_fmt_num).collect();
            text.push_str(&format!("  {goal} real {}\n", names.join(" ")));
        } else {
            text.push_str(&format!("  {goal} {status}\n"));
        }
    }
    text.push('\n');
    text.push_str("surprise:\n");
    text.push_str(&format!("  total: {surprise_total}\n"));
    text.push_str(&format!("  done W: {done_w}\n"));
    text.push_str(&format!("  per done: {}\n", stats_fmt_num(&per_done)));
    text.push('\n');
    text.push_str("surprise series:\n");
    if surprise_series.is_empty() {
        text.push_str("  \u{2013}\n");
    } else {
        for e in &surprise_series {
            let JVal::Obj(d) = e else {
                continue;
            };
            text.push_str(&format!(
                "  {} {} +{} C={}\n",
                stats_fmt_num(dget(d, "id")),
                stats_fmt_num(dget(d, "ts")),
                stats_fmt_num(dget(d, "delta")),
                stats_fmt_num(dget(d, "c"))
            ));
        }
    }
    text.push('\n');
    text.push_str(&format!(
        "cv series: {} points (replay failures: {replay_failures})\n",
        series.len()
    ));
    if let (Some(f), Some(l)) = (series.first(), series.last()) {
        text.push_str(&format!("  first: {} C={} V={}\n", f.0, f.1, f.2));
        text.push_str(&format!("  last:  {} C={} V={}\n", l.0, l.1, l.2));
    }
    let payload = JuliaDict::from_pairs(vec![
        ("command".to_string(), JVal::Str("stats".to_string())),
        ("records".to_string(), JVal::Int(recs.len() as i64)),
        ("mutations".to_string(), JVal::Int(mutations)),
        (
            "cycle_time".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("by_cynefin".to_string(), JVal::Obj(cycle_classes)),
                (
                    "durations_seconds".to_string(),
                    JVal::Arr(cycle_seconds.iter().map(|s| JVal::Int(*s)).collect()),
                ),
            ])),
        ),
        (
            "dor".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("reject_events".to_string(), JVal::Int(reject_total)),
                ("reject_per_node".to_string(), JVal::Obj(reject_per_node)),
                ("progress_entries".to_string(), JVal::Int(progress_entries)),
                ("first_pass".to_string(), JVal::Int(first_pass)),
                ("first_pass_rate".to_string(), rate),
                (
                    "first_pass_split".to_string(),
                    JVal::Obj(JuliaDict::from_pairs(vec![
                        ("no_reject".to_string(), JVal::Int(split_no_reject)),
                        (
                            "reject_discovery".to_string(),
                            JVal::Int(split_reject_discovery),
                        ),
                        ("reject_plain".to_string(), JVal::Int(split_reject_plain)),
                        ("discovery_rate".to_string(), split_rate),
                    ])),
                ),
            ])),
        ),
        (
            "bets".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("validated".to_string(), JVal::Int(bet_validated)),
                (
                    "invalidated_acceptable".to_string(),
                    JVal::Int(bet_acceptable),
                ),
                (
                    "invalidated_blocking".to_string(),
                    JVal::Int(bet_blocking),
                ),
                ("ratio".to_string(), bet_ratio),
            ])),
        ),
        (
            "discovery".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("stale_entries".to_string(), JVal::Int(stale_entries)),
                ("revalidations".to_string(), JVal::Int(revalidations)),
                ("gate_runs".to_string(), JVal::Int(gate_runs)),
                ("gate_empty".to_string(), JVal::Int(gate_empty)),
                (
                    "gate_overflow_events".to_string(),
                    JVal::Int(overflow_events),
                ),
                (
                    "gate_invalidated_events".to_string(),
                    JVal::Int(invalidated_events),
                ),
            ])),
        ),
        ("gates".to_string(), JVal::Arr(gates)),
        (
            "undo".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("undo_events".to_string(), JVal::Int(undo_events)),
                ("undone_steps".to_string(), JVal::Int(undone_steps)),
                ("undos_per_100_mutations".to_string(), undos_per_100),
            ])),
        ),
        (
            "audit".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                (
                    "sessions".to_string(),
                    JVal::Obj(JuliaDict::from_pairs(vec![
                        ("count".to_string(), JVal::Int(session_entries.len() as i64)),
                        ("per_session".to_string(), JVal::Arr(session_entries)),
                        ("mean".to_string(), session_mean),
                        ("median".to_string(), session_median),
                        ("max".to_string(), session_max),
                    ])),
                ),
                (
                    "checkpoint_latency".to_string(),
                    JVal::Obj(JuliaDict::from_pairs(vec![
                        ("dor".to_string(), JVal::Obj(latency_dor_summary)),
                        (
                            "discovery".to_string(),
                            JVal::Obj(latency_discovery_summary),
                        ),
                    ])),
                ),
                (
                    "post_approval_invalidation".to_string(),
                    JVal::Obj(JuliaDict::from_pairs(vec![
                        ("invalidated".to_string(), JVal::Int(pai_invalidated)),
                        ("ever_validated".to_string(), JVal::Int(pai_ever)),
                        ("rate".to_string(), pai_rate),
                    ])),
                ),
            ])),
        ),
        ("rework".to_string(), JVal::Obj(rework)),
        (
            "distill_yield".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("goals_with_real".to_string(), JVal::Int(yield_real)),
                ("goals_null_attested".to_string(), JVal::Int(yield_null)),
                ("goals_without".to_string(), JVal::Int(yield_none)),
                ("goals".to_string(), JVal::Arr(yield_goals)),
            ])),
        ),
        (
            "surprise".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("total".to_string(), JVal::Int(surprise_total)),
                ("done_w".to_string(), JVal::Int(done_w)),
                ("per_done".to_string(), per_done),
            ])),
        ),
        ("surprise_series".to_string(), JVal::Arr(surprise_series)),
        (
            "cv_series".to_string(),
            JVal::Arr(
                series
                    .iter()
                    .map(|(ts, c, v)| {
                        JVal::Obj(JuliaDict::from_pairs(vec![
                            ("ts".to_string(), JVal::Str(ts.clone())),
                            ("c".to_string(), JVal::Int(*c)),
                            ("v".to_string(), JVal::Int(*v)),
                        ]))
                    })
                    .collect(),
            ),
        ),
        ("replay_failures".to_string(), JVal::Int(replay_failures)),
    ]);
    StatsOut { payload, text }
}

pub fn cmd_stats(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let (_, recs) = journal_read_nonempty_pairs(&ctx.journalpath());
    let out = compute_stats(&st, &recs, &utc_stamp_second());
    let mut r = OpResult::ok();
    r.out = if ctx.json {
        json_cli_out(out.payload)
    } else {
        out.text
    };
    r
}
