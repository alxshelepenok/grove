use crate::algebra::treewidth_upper;
use crate::cli::{abspath, journal_session_token, json_cli_out, load, CliCtx};
use crate::journal::{
    append_journal_record, journal_read_nonempty_pairs, stamp_journal_session,
    wrap_journal_record, JOURNAL_GATE_OP,
};
use crate::json::{JVal, JuliaDict, Json};
use crate::model::{Kind, Node, State};
use crate::ops::{kw_get, OpResult, EXIT_ERR};
use crate::status::listnodes;
use std::collections::{BTreeMap, BTreeSet};

pub struct GateBaseline {
    pub ts: String,
    pub tw: i64,
    pub dones: i64,
}

pub struct GateReport {
    pub baseline: Option<GateBaseline>,
    pub tw_now: usize,
    pub tw_delta: i64,
    pub dones: usize,
    pub due: bool,
    pub overflows: Vec<(String, Vec<String>)>,
    pub invalidated: Vec<Node>,
    pub accepted: Vec<Node>,
    pub empty: bool,
    pub theta: i64,
    pub n: i64,
}

pub fn gate_baseline(recs: &[Json]) -> Option<GateBaseline> {
    for rec in recs.iter().rev() {
        let Some(inv @ Json::Obj(_)) = rec.get("inv") else {
            continue;
        };
        let op = inv.get("op").and_then(|v| v.as_str()).unwrap_or("");
        if op != JOURNAL_GATE_OP {
            continue;
        }
        let ts = rec
            .get("ts")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if ts.is_empty() {
            continue;
        }
        let tw = inv.get("tw").and_then(|v| v.as_i64()).unwrap_or(0);
        let dones = inv.get("dones").and_then(|v| v.as_i64()).unwrap_or(0);
        return Some(GateBaseline { ts, tw, dones });
    }
    None
}

fn gate_time_cut(baseline: Option<&GateBaseline>) -> &str {
    baseline.map(|b| b.ts.as_str()).unwrap_or("")
}

fn gate_done_since<'a>(st: &'a State, cut: &str) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "done" {
            continue;
        }
        if w.attr("t_updated").as_str() < cut {
            continue;
        }
        out.push(w);
    }
    out
}

fn gate_git_root_ok(root: &str) -> bool {
    let has_git = std::process::Command::new("git")
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    has_git && crate::gitutil::git_repository_root(root)
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn scan_subject_ids(line: &str) -> Vec<String> {
    let b = line.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if b[i].is_ascii_uppercase()
            && (i == 0 || !is_word_byte(b[i - 1]))
            && i + 1 < n
            && b[i + 1] == b'-'
        {
            let d0 = i + 2;
            let mut k = d0;
            while k < n && b[k].is_ascii_digit() {
                k += 1;
            }
            let mut j = k;
            let mut hit = None;
            while j > d0 {
                if j == n || !is_word_byte(b[j]) {
                    hit = Some(j);
                    break;
                }
                j -= 1;
            }
            if let Some(j) = hit {
                out.push(line[i..j].to_string());
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub fn gate_git_scan_log(txt: &str, wids: &[String]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> =
        wids.iter().map(|w| (w.clone(), Vec::new())).collect();
    let want: BTreeSet<&str> = wids.iter().map(String::as_str).collect();
    let mut hits: Vec<String> = Vec::new();
    for line in txt.lines() {
        if line.starts_with('\x01') {
            hits = Vec::new();
            for id in scan_subject_ids(line) {
                if want.contains(id.as_str()) && !hits.contains(&id) {
                    hits.push(id);
                }
            }
            continue;
        }
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        for id in &hits {
            if let Some(v) = out.get_mut(id) {
                v.push(s.to_string());
            }
        }
    }
    for v in out.values_mut() {
        v.sort();
        v.dedup();
    }
    out
}

pub fn gate_git_files_by_w(root: &str, wids: &[String], cut: &str) -> BTreeMap<String, Vec<String>> {
    if wids.is_empty() || !gate_git_root_ok(root) {
        return wids.iter().map(|w| (w.clone(), Vec::new())).collect();
    }
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C")
        .arg(abspath(root))
        .arg("--no-pager")
        .arg("log")
        .arg("--name-only")
        .arg("--pretty=format:\x01%s")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stderr(std::process::Stdio::inherit());
    if !cut.is_empty() {
        cmd.arg(format!("--since={cut}"));
    }
    let txt = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => String::new(),
    };
    gate_git_scan_log(&txt, wids)
}

fn surface_overflows(
    st: &State,
    root: &str,
    baseline: Option<&GateBaseline>,
    theta: i64,
) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let cut = gate_time_cut(baseline);
    let dones = gate_done_since(st, cut);
    let wids: Vec<String> = dones.iter().map(|w| w.id.clone()).collect();
    let by_w = gate_git_files_by_w(root, &wids, cut);
    for w in dones {
        let actual = by_w.get(&w.id).cloned().unwrap_or_default();
        if actual.is_empty() {
            continue;
        }
        let declared: BTreeSet<String> = w.lines("surface").into_iter().collect();
        let overflow: Vec<String> = actual
            .iter()
            .filter(|x| !declared.contains(*x))
            .cloned()
            .collect();
        if overflow.len() as i64 > theta {
            out.push((w.id.clone(), overflow));
        }
    }
    out
}

fn gate_invalidated<'a>(st: &'a State, cut: &str) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for b in listnodes(st, Kind::B, false) {
        if b.status != "invalidated_acceptable" && b.status != "invalidated_blocking" {
            continue;
        }
        if b.attr("t_updated").as_str() < cut {
            continue;
        }
        out.push(b);
    }
    out
}

fn gate_accepted<'a>(st: &'a State, cut: &str) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for d in listnodes(st, Kind::D, false) {
        if d.status != "accepted" {
            continue;
        }
        if d.attr("t_updated").as_str() < cut {
            continue;
        }
        out.push(d);
    }
    out
}

pub fn gate_report(st: &State, recs: &[Json], root: &str, theta: i64, n: i64) -> GateReport {
    let baseline = gate_baseline(recs);
    let cut = gate_time_cut(baseline.as_ref()).to_string();
    let tw_now = treewidth_upper(st);
    let tw_delta = baseline
        .as_ref()
        .map(|b| tw_now as i64 - b.tw)
        .unwrap_or(0);
    let dones = gate_done_since(st, &cut).len();
    let overflows = surface_overflows(st, root, baseline.as_ref(), theta);
    let invalidated = gate_invalidated(st, &cut);
    let accepted = gate_accepted(st, &cut);
    let empty =
        tw_delta == 0 && overflows.is_empty() && invalidated.is_empty() && accepted.is_empty();
    let due = dones as i64 >= n;
    GateReport {
        baseline,
        tw_now,
        tw_delta,
        dones,
        due,
        overflows,
        invalidated: invalidated.into_iter().cloned().collect(),
        accepted: accepted.into_iter().cloned().collect(),
        empty,
        theta,
        n,
    }
}

pub fn gate_json_payload(rep: &GateReport) -> JuliaDict {
    let baseline = match &rep.baseline {
        None => JVal::Null,
        Some(b) => JVal::Obj(JuliaDict::from_pairs(vec![
            ("ts".to_string(), JVal::Str(b.ts.clone())),
            ("tw".to_string(), JVal::Int(b.tw)),
            ("dones".to_string(), JVal::Int(b.dones)),
        ])),
    };
    let overflows = JVal::Arr(
        rep.overflows
            .iter()
            .map(|(wid, paths)| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("w".to_string(), JVal::Str(wid.clone())),
                    (
                        "paths".to_string(),
                        JVal::Arr(paths.iter().map(|p| JVal::Str(p.clone())).collect()),
                    ),
                ]))
            })
            .collect(),
    );
    let invalidated = JVal::Arr(
        rep.invalidated
            .iter()
            .map(|b| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(b.id.clone())),
                    ("title".to_string(), JVal::Str(b.title.clone())),
                    ("status".to_string(), JVal::Str(b.status.clone())),
                ]))
            })
            .collect(),
    );
    let accepted = JVal::Arr(
        rep.accepted
            .iter()
            .map(|d| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(d.id.clone())),
                    ("title".to_string(), JVal::Str(d.title.clone())),
                ]))
            })
            .collect(),
    );
    JuliaDict::from_pairs(vec![
        ("command".to_string(), JVal::Str("gate".to_string())),
        ("baseline".to_string(), baseline),
        ("tw_now".to_string(), JVal::Int(rep.tw_now as i64)),
        ("tw_delta".to_string(), JVal::Int(rep.tw_delta)),
        ("dones".to_string(), JVal::Int(rep.dones as i64)),
        ("due".to_string(), JVal::Bool(rep.due)),
        ("overflows".to_string(), overflows),
        ("invalidated".to_string(), invalidated),
        ("accepted".to_string(), accepted),
        ("empty".to_string(), JVal::Bool(rep.empty)),
        ("theta".to_string(), JVal::Int(rep.theta)),
        ("n".to_string(), JVal::Int(rep.n)),
    ])
}

pub fn cmd_gate(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    let mut theta: i64 = 0;
    if let Some(v) = kw_get(kw, "theta") {
        match v.parse::<i64>() {
            Ok(x) if x >= 0 => theta = x,
            Ok(_) => return OpResult::fail(EXIT_ERR, "--theta must be ≥ 0"),
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --theta (expected integer)"),
        }
    }
    let mut n: i64 = 5;
    if let Some(v) = kw_get(kw, "n") {
        match v.parse::<i64>() {
            Ok(x) if x >= 1 => n = x,
            Ok(_) => return OpResult::fail(EXIT_ERR, "--n must be ≥ 1"),
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --n (expected integer)"),
        }
    }
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let jp = ctx.journalpath();
    let (_, recs) = journal_read_nonempty_pairs(&jp);
    let rep = gate_report(&st, &recs, &ctx.root, theta, n);
    let overflow_counts = JuliaDict::from_pairs(
        rep.overflows
            .iter()
            .map(|(wid, paths)| (wid.clone(), JVal::Int(paths.len() as i64)))
            .collect(),
    );
    let jr = wrap_journal_record(
        "gate",
        JuliaDict::from_pairs(vec![
            ("op".to_string(), JVal::Str(JOURNAL_GATE_OP.to_string())),
            ("tw".to_string(), JVal::Int(rep.tw_now as i64)),
            ("dones".to_string(), JVal::Int(rep.dones as i64)),
            ("empty".to_string(), JVal::Bool(rep.empty)),
            (
                "overflows".to_string(),
                JVal::Arr(
                    rep.overflows
                        .iter()
                        .map(|(wid, _)| JVal::Str(wid.clone()))
                        .collect(),
                ),
            ),
            ("overflow_counts".to_string(), JVal::Obj(overflow_counts)),
            (
                "invalidated".to_string(),
                JVal::Arr(
                    rep.invalidated
                        .iter()
                        .map(|b| JVal::Str(b.id.clone()))
                        .collect(),
                ),
            ),
        ]),
    );
    let _ = append_journal_record(&jp, &stamp_journal_session(&jr, &journal_session_token(ctx, kw)));
    let mut r = OpResult::ok();
    if ctx.json {
        r.out = json_cli_out(gate_json_payload(&rep));
        return r;
    }
    let baseline = rep
        .baseline
        .as_ref()
        .map(|b| b.ts.as_str())
        .unwrap_or("none");
    r.out.push_str(&format!("baseline: {baseline}\n"));
    let sign = if rep.tw_delta >= 0 { "+" } else { "" };
    r.out.push_str(&format!(
        "treewidth: {} (Δ {sign}{})\n",
        rep.tw_now, rep.tw_delta
    ));
    r.out.push_str(&format!("done since baseline: {}\n", rep.dones));
    r.out.push_str(&format!("due: {}\n", rep.due));
    if rep.overflows.is_empty() && rep.invalidated.is_empty() && rep.accepted.is_empty() {
        r.out.push_str("would distill: none\n");
        return r;
    }
    r.out.push_str("would distill:\n");
    for (wid, paths) in &rep.overflows {
        r.out.push_str(&format!("- overflow {wid}: {}\n", paths.join(", ")));
    }
    for b in &rep.invalidated {
        r.out.push_str(&format!("- invalidated {}: {}\n", b.id, b.title));
    }
    for d in &rep.accepted {
        r.out.push_str(&format!("- accepted {}: {}\n", d.id, d.title));
    }
    r
}
