use crate::algebra::{
    asks_of, backward_cone, bchain, contraction_order, critical_path, forward_cone,
    goal_fragility, goals_of, impact, implements_of, relevant_discoveries,
};
use crate::cli::{json_cli_out, load, CliCtx};
use crate::dor::{dor, dor_breakdown, ready};
use crate::json::{JVal, JuliaDict};
use crate::model::{Kind, Node, State};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_NOTFOUND};
use std::collections::BTreeSet;

fn jsym(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("nothing")
}

fn fitness_parts(fitness: &std::collections::BTreeMap<String, i64>) -> Vec<String> {
    let mut d = JuliaDict::new();
    for (k, v) in fitness {
        d.insert(k.clone(), JVal::Int(*v));
    }
    d.iter_pairs()
        .map(|(k, v)| {
            let v = match v {
                JVal::Int(i) => *i,
                _ => 0,
            };
            if v >= 0 {
                format!("{k}=+{v}")
            } else {
                format!("{k}={v}")
            }
        })
        .collect()
}

pub fn packet(st: &State, w: &Node) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Execution packet: {} ({})\n", w.id, w.title));
    out.push('\n');
    out.push_str(&format!(
        "type={}  status={}  cynefin={}\n",
        jsym(&w.wtype),
        w.status,
        jsym(&w.cynefin)
    ));
    out.push('\n');
    let goals = goals_of(w);
    if !goals.is_empty() {
        out.push_str(&format!("**Goals:** {}\n", goals.join(", ")));
    }
    let fitness = w.fitness();
    if !fitness.is_empty() {
        out.push_str(&format!(
            "**Fitness contribution:** {}\n",
            fitness_parts(&fitness).join(", ")
        ));
    }
    out.push('\n');
    for (label, fname) in [
        ("Why", "why"),
        ("Repro", "repro"),
        ("Hypothesis", "hypothesis"),
        ("Exit (spike)", "exit"),
        ("Acceptance criteria", "ac"),
        ("Evidence strategy", "evidence_strategy"),
        ("Plan", "plan"),
        ("Evidence", "evidence"),
    ] {
        let lines = w.lines(fname);
        if lines.is_empty() {
            continue;
        }
        out.push_str(&format!("## {label}\n"));
        out.push('\n');
        for ln in &lines {
            out.push_str(&format!("- {ln}\n"));
        }
        out.push('\n');
    }
    for did in implements_of(st, w) {
        let Some(d) = st.nodes.get(&did) else {
            continue;
        };
        out.push_str(&format!("## Decision {}: {}  ({})\n", d.id, d.title, d.status));
        out.push('\n');
        for fname in ["context", "options", "decision", "consequences", "validation"] {
            let lines = d.lines(fname);
            if lines.is_empty() {
                continue;
            }
            out.push_str(&format!("**{fname}:**\n"));
            for ln in &lines {
                out.push_str(&format!("- {ln}\n"));
            }
            out.push('\n');
        }
    }
    for bid in bchain(st, w) {
        let Some(b) = st.nodes.get(&bid) else {
            continue;
        };
        out.push_str(&format!(
            "## Assumption {}: {}  ({}, {})\n",
            b.id,
            b.title,
            b.status,
            jsym(&b.cynefin)
        ));
        for fname in ["vm", "threshold", "result"] {
            let lines = b.lines(fname);
            if lines.is_empty() {
                continue;
            }
            out.push_str(&format!("**{fname}:**\n"));
            for ln in &lines {
                out.push_str(&format!("- {ln}\n"));
            }
        }
        out.push('\n');
    }
    for qid in asks_of(st, w) {
        let Some(q) = st.nodes.get(&qid) else {
            continue;
        };
        out.push_str(&format!(
            "## Question {}: {}  ({}, {})\n",
            q.id,
            q.title,
            q.status,
            jsym(&q.cynefin)
        ));
        let outcome = q.lines("outcome");
        if !outcome.is_empty() {
            out.push_str("**outcome:**\n");
            for ln in &outcome {
                out.push_str(&format!("- {ln}\n"));
            }
        }
        out.push('\n');
    }
    out.push_str("## Definition of Ready\n");
    out.push('\n');
    for (label, ok, detail) in dor_breakdown(st, w, false) {
        let sym = if ok { "⊤" } else { "⊥" };
        if detail.is_empty() {
            out.push_str(&format!("- {sym}  {label}.\n"));
        } else {
            out.push_str(&format!("- {sym}  {label} ({detail}).\n"));
        }
    }
    let overall = if dor(st, w, false) { "⊤" } else { "⊥" };
    out.push('\n');
    out.push_str(&format!("**result: {overall}**\n"));
    out
}

pub fn packet_cone(st: &State, w: &Node, depth: usize, maxcount: usize) -> String {
    let back = backward_cone(st, &w.id, depth, maxcount);
    let fwd = forward_cone(st, &w.id, depth, maxcount);
    let mut out = String::new();
    out.push('\n');
    out.push_str("## Contraction order\n");
    out.push('\n');
    for (i, id) in contraction_order(st, &back.ids).iter().enumerate() {
        let Some(n) = st.nodes.get(id) else {
            continue;
        };
        out.push_str(&format!("{}. {}  {}  {}\n", i + 1, id, n.status, n.title));
    }
    out.push('\n');
    out.push_str("## Forward cone (impact)\n");
    out.push('\n');
    for id in &fwd.ids {
        let Some(n) = st.nodes.get(id) else {
            continue;
        };
        out.push_str(&format!("- {}  {}  {}\n", id, n.status, n.title));
    }
    out.push('\n');
    out.push_str("## Fragility\n");
    out.push('\n');
    for (g, k) in goal_fragility(st, w) {
        if k == 0 {
            out.push_str(&format!("- {g}: no blocks-path\n"));
        } else if k == 1 {
            out.push_str(&format!("- {g}: 1 (brittle)\n"));
        } else {
            out.push_str(&format!("- {g}: {k} disjoint blocks-paths\n"));
        }
    }
    let arts = relevant_discoveries(st, w, &back.ids, maxcount);
    if !arts.is_empty() {
        out.push('\n');
        out.push_str("## Relevant discoveries\n");
        out.push('\n');
        for id in &arts {
            let Some(n) = st.nodes.get(id) else {
                continue;
            };
            out.push_str(&format!("- {}  {}\n", id, n.title));
        }
    }
    if back.truncated || fwd.truncated {
        out.push('\n');
        out.push_str(&format!("> cone truncated (depth={depth}, max={maxcount})\n"));
    }
    out
}

fn str_arr(ids: Vec<String>) -> JVal {
    JVal::Arr(ids.into_iter().map(JVal::Str).collect())
}

pub fn cmd_ready(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let cp: BTreeSet<String> = critical_path(&st).into_iter().collect();
    let mut rs = ready(&st);
    rs.sort_by_cached_key(|w| {
        (
            if cp.contains(&w.id) { 0 } else { 1 },
            -(impact(&st, &w.id).len() as i64),
            w.id.clone(),
        )
    });
    let mut r = OpResult::ok();
    if ctx.json {
        let items: Vec<JVal> = rs
            .iter()
            .map(|w| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(w.id.clone())),
                    ("title".to_string(), JVal::Str(w.title.clone())),
                    ("critical".to_string(), JVal::Bool(cp.contains(&w.id))),
                ]))
            })
            .collect();
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("ready".to_string())),
            ("items".to_string(), JVal::Arr(items)),
        ]));
        return r;
    }
    for w in &rs {
        let flag = if cp.contains(&w.id) { " [crit]" } else { "" };
        r.out.push_str(&format!("{}  {}{}\n", w.id, w.title, flag));
    }
    r
}

pub fn cmd_next(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let rs = ready(&st);
    if rs.is_empty() {
        let mut r = OpResult::ok();
        r.err = "no ready work items\n".to_string();
        return r;
    }
    let cp: BTreeSet<String> = critical_path(&st).into_iter().collect();
    let pick = match rs.iter().find(|w| cp.contains(&w.id)) {
        Some(w) => *w,
        None => rs[0],
    };
    let pkt = packet(&st, pick);
    let mut r = OpResult::ok();
    if ctx.json {
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("next".to_string())),
            ("work".to_string(), JVal::Str(pick.id.clone())),
            ("packet_markdown".to_string(), JVal::Str(pkt)),
        ]));
        return r;
    }
    r.out = pkt;
    r
}

pub fn cmd_packet(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove packet <W-NN>");
    }
    let mut depth = 4usize;
    let mut maxcount = 50usize;
    if let Some(raw) = kw_get(kw, "cone-depth") {
        let v: i64 = match raw.parse() {
            Ok(v) => v,
            Err(_) => {
                return OpResult::fail(EXIT_ERR, "bad --cone-depth (expected integer)");
            }
        };
        if v < 1 {
            return OpResult::fail(EXIT_ERR, "--cone-depth must be ≥ 1");
        }
        depth = v as usize;
    }
    if let Some(raw) = kw_get(kw, "cone-max") {
        let v: i64 = match raw.parse() {
            Ok(v) => v,
            Err(_) => {
                return OpResult::fail(EXIT_ERR, "bad --cone-max (expected integer)");
            }
        };
        if v < 1 {
            return OpResult::fail(EXIT_ERR, "--cone-max must be ≥ 1");
        }
        maxcount = v as usize;
    }
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(n) = st.nodes.get(&pos[0]) else {
        return OpResult::fail(EXIT_NOTFOUND, "not found");
    };
    if n.kind != Kind::W {
        return OpResult::fail(EXIT_ERR, "not a work item");
    }
    let cone = kw_get(kw, "cone").is_some();
    let mut pkt = packet(&st, n);
    if cone {
        pkt.push_str(&packet_cone(&st, n, depth, maxcount));
    }
    let mut r = OpResult::ok();
    if ctx.json {
        let mut out = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("packet".to_string())),
            ("work".to_string(), JVal::Str(n.id.clone())),
            ("packet_markdown".to_string(), JVal::Str(pkt)),
        ]);
        if cone {
            let back = backward_cone(&st, &n.id, depth, maxcount);
            let fwd = forward_cone(&st, &n.id, depth, maxcount);
            let fragility: Vec<JVal> = goal_fragility(&st, n)
                .into_iter()
                .map(|(g, k)| {
                    JVal::Obj(JuliaDict::from_pairs(vec![
                        ("goal".to_string(), JVal::Str(g)),
                        ("paths".to_string(), JVal::Int(k)),
                    ]))
                })
                .collect();
            let cone_dict = JuliaDict::from_pairs(vec![
                ("backward".to_string(), str_arr(back.ids.clone())),
                ("order".to_string(), str_arr(contraction_order(&st, &back.ids))),
                ("forward".to_string(), str_arr(fwd.ids.clone())),
                ("fragility".to_string(), JVal::Arr(fragility)),
                (
                    "relevant_discoveries".to_string(),
                    str_arr(relevant_discoveries(&st, n, &back.ids, maxcount)),
                ),
                (
                    "truncated".to_string(),
                    JVal::Bool(back.truncated || fwd.truncated),
                ),
                ("depth".to_string(), JVal::Int(depth as i64)),
                ("max".to_string(), JVal::Int(maxcount as i64)),
            ]);
            out.insert("cone".to_string(), JVal::Obj(cone_dict));
        }
        r.out = json_cli_out(out);
        return r;
    }
    r.out = pkt;
    r
}
