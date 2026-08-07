use crate::cli::{eff_token, json_cli_out, load, CliCtx};
use crate::dor::{dor, dor_breakdown};
use crate::json::{julia_float_repr, parse_json, JVal, Json, JuliaDict};
use crate::model::{field_form, field_order, FieldValue, Form, Kind, Node, State};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_NOTFOUND};
use crate::serialize::serialize_node_string;
use crate::session::{
    progress_has_session_record, progress_session_display_stale, session_claim_age_stale,
    session_token_matches, SESSION_DISPLAY_STALE_AFTER_HOURS,
};
use crate::status::{alignment_triggers, listnodes};
use std::path::Path;

fn silent(code: i32) -> OpResult {
    OpResult {
        code,
        out: String::new(),
        err: String::new(),
        journal: Vec::new(),
    }
}

fn julia_dict_copy(pairs: impl Iterator<Item = (String, JVal)>) -> JuliaDict {
    let mut stage1 = JuliaDict::new();
    for (k, v) in pairs {
        stage1.insert(k, v);
    }
    let mut stage2 = JuliaDict::with_sizehint(stage1.len());
    for (k, v) in stage1.iter_pairs() {
        stage2.insert(k.clone(), v.clone());
    }
    stage2
}

fn json_field_value(kind: Kind, fname: &str, v: &FieldValue) -> JVal {
    match (field_form(kind, fname), v) {
        (Some(Form::Prose), FieldValue::Prose(lines))
        | (Some(Form::RefList), FieldValue::RefList(lines)) => {
            JVal::Arr(lines.iter().map(|s| JVal::Str(s.clone())).collect())
        }
        (Some(Form::Single), FieldValue::Single(s)) => JVal::Str(s.clone()),
        (Some(Form::Fitness), FieldValue::Fitness(m)) => JVal::Obj(julia_dict_copy(
            m.iter().map(|(k, d)| (k.clone(), JVal::Int(*d))),
        )),
        _ => match v {
            FieldValue::Prose(lines) | FieldValue::RefList(lines) => {
                JVal::Arr(lines.iter().map(|s| JVal::Str(s.clone())).collect())
            }
            FieldValue::Single(s) => JVal::Str(s.clone()),
            FieldValue::Fitness(m) => JVal::Obj(julia_dict_copy(
                m.iter().map(|(k, d)| (k.clone(), JVal::Int(*d))),
            )),
        },
    }
}

pub fn json_node_snapshot(n: &Node) -> JuliaDict {
    let mut fields = JuliaDict::new();
    for fname in field_order(n.kind) {
        let Some(v) = n.fields.get(*fname) else {
            continue;
        };
        fields.insert(fname.to_string(), json_field_value(n.kind, fname, v));
    }
    let attrs = julia_dict_copy(
        n.attrs
            .iter()
            .map(|(k, v)| (k.clone(), JVal::Str(v.clone()))),
    );
    let mut record = JuliaDict::from_pairs(vec![
        ("kind".to_string(), JVal::Str(n.kind.as_str().to_string())),
        ("id".to_string(), JVal::Str(n.id.clone())),
        ("title".to_string(), JVal::Str(n.title.clone())),
        ("status".to_string(), JVal::Str(n.status.clone())),
        ("archived".to_string(), JVal::Bool(n.archived)),
        ("attrs".to_string(), JVal::Obj(attrs)),
        ("fields".to_string(), JVal::Obj(fields)),
    ]);
    if let Some(t) = &n.wtype {
        record.insert("type".to_string(), JVal::Str(t.clone()));
    }
    if let Some(c) = &n.cynefin {
        record.insert("cynefin".to_string(), JVal::Str(c.clone()));
    }
    JuliaDict::from_pairs(vec![
        ("command".to_string(), JVal::Str("show".to_string())),
        ("record".to_string(), JVal::Obj(record)),
    ])
}

fn load_state(ctx: &CliCtx) -> Result<State, OpResult> {
    load(ctx, true)
}

pub fn cmd_deps(ctx: &CliCtx, pos: &[String], _kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return silent(EXIT_ERR);
    }
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let pred = crate::algebra::deps(&st, &pos[0]);
    if ctx.json {
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("deps".to_string())),
            ("id".to_string(), JVal::Str(pos[0].clone())),
            (
                "predecessors".to_string(),
                JVal::Arr(pred.into_iter().map(JVal::Str).collect()),
            ),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    for id in pred {
        r.out.push_str(&id);
        r.out.push('\n');
    }
    r
}

pub fn cmd_impact(ctx: &CliCtx, pos: &[String], _kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return silent(EXIT_ERR);
    }
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let succ = crate::algebra::impact(&st, &pos[0]);
    if ctx.json {
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("impact".to_string())),
            ("id".to_string(), JVal::Str(pos[0].clone())),
            (
                "successors".to_string(),
                JVal::Arr(succ.into_iter().map(JVal::Str).collect()),
            ),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    for id in succ {
        r.out.push_str(&id);
        r.out.push('\n');
    }
    r
}

pub fn cmd_path(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let chain = crate::algebra::critical_path(&st);
    if ctx.json {
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("path".to_string())),
            (
                "chain".to_string(),
                JVal::Arr(chain.into_iter().map(JVal::Str).collect()),
            ),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    for id in chain {
        r.out.push_str(&id);
        r.out.push('\n');
    }
    r
}

pub fn cmd_triage(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let rows = crate::algebra::triage_rows(&st);
    if ctx.json {
        let jr: Vec<JVal> = rows
            .iter()
            .map(|r| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("w".to_string(), JVal::Str(r.w.clone())),
                    ("title".to_string(), JVal::Str(r.title.clone())),
                    ("coverage".to_string(), JVal::Float(r.cov)),
                    ("declared".to_string(), JVal::Bool(r.declared)),
                    ("uncertainty".to_string(), JVal::Int(r.uncertainty)),
                    ("fragile".to_string(), JVal::Bool(r.fragile)),
                    ("suggestion".to_string(), JVal::Str(r.suggestion.clone())),
                ]))
            })
            .collect();
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("triage".to_string())),
            ("rows".to_string(), JVal::Arr(jr)),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    if rows.is_empty() {
        r.out.push_str("triage: no open work\n");
        return r;
    }
    r.out.push_str("W\tcov\t\u{03c7}\tfragile\tsuggestion\n");
    for row in rows {
        r.out.push_str(&format!(
            "{}\t{:.2}\t{}\t{}\t{}\n",
            row.w,
            row.cov,
            row.uncertainty,
            if row.fragile { "yes" } else { "no" },
            row.suggestion
        ));
    }
    r
}

pub fn cmd_dor(ctx: &CliCtx, pos: &[String], _kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return silent(EXIT_ERR);
    }
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(n) = st.nodes.get(&pos[0]) else {
        return silent(EXIT_NOTFOUND);
    };
    if ctx.json {
        let conj: Vec<JVal> = dor_breakdown(&st, n, false)
            .into_iter()
            .map(|(label, ok, detail)| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("label".to_string(), JVal::Str(label)),
                    ("ok".to_string(), JVal::Bool(ok)),
                    ("detail".to_string(), JVal::Str(detail)),
                ]))
            })
            .collect();
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("dor".to_string())),
            ("work".to_string(), JVal::Str(n.id.clone())),
            ("conjuncts".to_string(), JVal::Arr(conj)),
            ("dor".to_string(), JVal::Bool(dor(&st, n, false))),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut out = format!("{} DoR:\n", n.id);
    for (label, ok, detail) in dor_breakdown(&st, n, false) {
        let sym = if ok { "\u{22a4}" } else { "\u{22a5}" };
        if detail.is_empty() {
            out.push_str(&format!("  {}  {}\n", sym, label));
        } else {
            out.push_str(&format!("  {}  {}  \u{2192} {}\n", sym, label, detail));
        }
    }
    let overall = if dor(&st, n, false) {
        "\u{22a4}"
    } else {
        "\u{22a5}"
    };
    out.push_str(&format!("result: {}\n", overall));
    let mut r = OpResult::ok();
    r.out = out;
    r
}

pub fn cmd_show(ctx: &CliCtx, pos: &[String], _kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return silent(EXIT_ERR);
    }
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(n) = st.nodes.get(&pos[0]) else {
        return silent(EXIT_NOTFOUND);
    };
    if ctx.json {
        let mut r = OpResult::ok();
        r.out = json_cli_out(json_node_snapshot(n));
        return r;
    }
    let mut r = OpResult::ok();
    r.out = serialize_node_string(n);
    r
}

pub fn cmd_list(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove list <kind>");
    }
    let kind: Option<Kind> = pos[0].parse().ok();
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let rows: Vec<&Node> = match kind {
        Some(k) => listnodes(&st, k, false),
        None => Vec::new(),
    };
    let fstatus = kw_get(kw, "status").unwrap_or("");
    let fcynefin = kw_get(kw, "cynefin").unwrap_or("");
    let keep = |n: &Node| {
        (fstatus.is_empty() || n.status == fstatus)
            && (fcynefin.is_empty() || n.cynefin.as_deref() == Some(fcynefin))
    };
    if ctx.json {
        let outrows: Vec<JVal> = rows
            .iter()
            .filter(|n| keep(**n))
            .map(|n| {
                let mut row = JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(n.id.clone())),
                    ("status".to_string(), JVal::Str(n.status.clone())),
                    ("title".to_string(), JVal::Str(n.title.clone())),
                ]);
                if let Some(c) = &n.cynefin {
                    row.insert("cynefin".to_string(), JVal::Str(c.clone()));
                }
                JVal::Obj(row)
            })
            .collect();
        let mut d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("list".to_string())),
            ("kind".to_string(), JVal::Str(pos[0].clone())),
            ("rows".to_string(), JVal::Arr(outrows)),
        ]);
        if !fstatus.is_empty() {
            d.insert("filter_status".to_string(), JVal::Str(fstatus.to_string()));
        }
        if !fcynefin.is_empty() {
            d.insert("filter_cynefin".to_string(), JVal::Str(fcynefin.to_string()));
        }
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    for n in rows.into_iter().filter(|n| keep(*n)) {
        r.out.push_str(&format!("{}\t{}\t{}\n", n.id, n.status, n.title));
    }
    r
}

pub fn cmd_graph(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let text = crate::render::render_graph_section(&st);
    if ctx.json {
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("graph".to_string())),
            ("mermaid".to_string(), JVal::Str(text)),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    r.out = text;
    r
}

fn status_line2(w: &Node, tok: &str, eff: &str) -> String {
    if tok.is_empty() {
        return format!(
            "  (no session= on record; I11 broken: use `grove resume {}` or re-claim progress)",
            w.id
        );
    }
    let flag = if session_token_matches(w, eff) {
        ""
    } else {
        "  [!= this session]"
    };
    let age = if session_claim_age_stale(w) {
        format!("  (claimed >{}h ago)", SESSION_DISPLAY_STALE_AFTER_HOURS)
    } else {
        String::new()
    };
    format!("  session={}{}{}", tok, flag, age)
}

pub fn cmd_status(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let mut prog: Vec<&Node> = listnodes(&st, Kind::W, false)
        .into_iter()
        .filter(|w| w.status == "progress")
        .collect();
    prog.sort_by(|a, b| a.id.cmp(&b.id));
    if ctx.json {
        let items: Vec<JVal> = prog
            .iter()
            .map(|&w| {
                let tok = if progress_has_session_record(w) {
                    w.attr("session")
                } else {
                    String::new()
                };
                let stale = progress_session_display_stale(w, &eff);
                let line2 = status_line2(w, &tok, &eff);
                let opts = if stale {
                    format!(
                        "grove resume {0} | grove revert {0} | grove handoff {0} --to=<token>",
                        w.id
                    )
                } else {
                    String::new()
                };
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(w.id.clone())),
                    ("title".to_string(), JVal::Str(w.title.clone())),
                    ("session".to_string(), JVal::Str(tok)),
                    ("stale_for_agent".to_string(), JVal::Bool(stale)),
                    ("session_detail".to_string(), JVal::Str(line2)),
                    ("options_hint".to_string(), JVal::Str(opts)),
                ]))
            })
            .collect();
        let al = alignment_triggers(&st);
        let inv = crate::invariants::check_all(&st);
        let invd = JuliaDict::from_pairs(vec![
            ("ok".to_string(), JVal::Bool(inv.is_empty())),
            (
                "messages".to_string(),
                JVal::Arr(inv.into_iter().map(JVal::Str).collect()),
            ),
        ]);
        let d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("status".to_string())),
            ("progress".to_string(), JVal::Arr(items)),
            (
                "alignment_triggers".to_string(),
                JVal::Arr(al.into_iter().map(JVal::Str).collect()),
            ),
            ("invariants".to_string(), JVal::Obj(invd)),
        ]);
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut out = String::from("# grove status\n\n## Work in `progress`\n\n");
    if prog.is_empty() {
        out.push_str("(none)\n");
    } else {
        for &w in &prog {
            let tok = if progress_has_session_record(w) {
                w.attr("session")
            } else {
                String::new()
            };
            let stale = progress_session_display_stale(w, &eff);
            let line2 = status_line2(w, &tok, &eff);
            if stale {
                out.push_str(&format!(
                    "{}\t{}  (stale for this agent)\n{}\n",
                    w.id, w.title, line2
                ));
                out.push_str(&format!(
                    "  options: `grove resume {0}` | `grove revert {0}` | `grove handoff {0} --to=<token>`\n",
                    w.id
                ));
            } else {
                out.push_str(&format!("{}\t{}\n{}\n", w.id, w.title, line2));
            }
        }
    }
    out.push_str("\n## Alignment triggers (protocol 2.5)\n\n");
    let al = alignment_triggers(&st);
    if al.is_empty() {
        out.push_str("(none)\n");
    } else {
        for line in al {
            out.push_str("- ");
            out.push_str(&line);
            out.push('\n');
        }
    }
    out.push_str("\n## Structure / invariants (same as `check`, non-blocking here)\n\n");
    let inv = crate::invariants::check_all(&st);
    if inv.is_empty() {
        out.push_str("ok\n");
    } else {
        for e in inv {
            out.push_str("- ");
            out.push_str(&e);
            out.push('\n');
        }
    }
    let mut r = OpResult::ok();
    r.out = out;
    r
}

struct LogRow {
    ts: String,
    tb: String,
    line: String,
}

fn log_ts(n: &Node, key: &str) -> String {
    n.attr(key).trim().to_string()
}

fn json_value_strings(x: &Json, acc: &mut Vec<String>) {
    match x {
        Json::Obj(pairs) => {
            for (_, v) in pairs {
                json_value_strings(v, acc);
            }
        }
        Json::Arr(items) => {
            for v in items {
                json_value_strings(v, acc);
            }
        }
        Json::Str(s) => acc.push(s.trim().to_string()),
        Json::Int(i) => acc.push(i.to_string()),
        Json::Float(f) => acc.push(julia_float_repr(*f)),
        Json::Bool(b) => acc.push(if *b { "true" } else { "false" }.to_string()),
        Json::Null => {}
    }
}

fn journal_inv_mentions_id(inv: &Json, needle: &str) -> bool {
    let mut acc = Vec::new();
    json_value_strings(inv, &mut acc);
    let n = needle.trim();
    acc.iter().any(|x| x == n)
}

fn journal_file_mentions_id(journal_path: &Path, needle: &str) -> bool {
    if !journal_path.is_file() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(journal_path) else {
        return false;
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        let Ok(rec) = parse_json(s) else {
            continue;
        };
        if !matches!(rec, Json::Obj(_)) {
            continue;
        }
        let Some(inv) = rec.get("inv") else {
            continue;
        };
        if !matches!(inv, Json::Obj(_)) {
            continue;
        }
        if journal_inv_mentions_id(inv, needle) {
            return true;
        }
    }
    false
}

fn json_interp_string(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        Json::Int(i) => i.to_string(),
        Json::Float(f) => julia_float_repr(*f),
        Json::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        other => crate::json::emit_jval(&json_to_jval(other)),
    }
}

fn json_to_jval(v: &Json) -> JVal {
    match v {
        Json::Null => JVal::Null,
        Json::Bool(b) => JVal::Bool(*b),
        Json::Int(i) => JVal::Int(*i),
        Json::Float(f) => JVal::Float(*f),
        Json::Str(s) => JVal::Str(s.clone()),
        Json::Arr(items) => JVal::Arr(items.iter().map(json_to_jval).collect()),
        Json::Obj(pairs) => JVal::Obj(JuliaDict::from_pairs(
            pairs
                .iter()
                .map(|(k, v)| (k.clone(), json_to_jval(v)))
                .collect(),
        )),
    }
}

fn append_journal_timeline(rows: &mut Vec<LogRow>, journal_path: &Path, filt: Option<&str>) {
    if !journal_path.is_file() {
        return;
    }
    let Ok(text) = std::fs::read_to_string(journal_path) else {
        return;
    };
    let mut li: i64 = 0;
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        let Ok(rec) = parse_json(s) else {
            continue;
        };
        if !matches!(rec, Json::Obj(_)) {
            continue;
        }
        let Some(inv) = rec.get("inv") else {
            continue;
        };
        if !matches!(inv, Json::Obj(_)) {
            continue;
        }
        if let Some(f) = filt {
            if !journal_inv_mentions_id(inv, f) {
                continue;
            }
        }
        li += 1;
        let cmd = match rec.get("cmd") {
            Some(Json::Str(s)) => s.clone(),
            _ => "?".to_string(),
        };
        let ts = match rec.get("ts") {
            Some(Json::Str(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => "1980-01-01T00:00:00Z".to_string(),
        };
        let tb = format!("journal {:09}", li);
        let invop = match inv.get("op") {
            Some(Json::Str(s)) => s.clone(),
            _ => String::new(),
        };
        let mut parts = vec![invop];
        for k in ["id", "wid", "from", "to", "gid", "goal"] {
            let Some(v) = inv.get(k) else {
                continue;
            };
            if matches!(v, Json::Null) {
                continue;
            }
            parts.push(format!("{}={}", k, json_interp_string(v)));
        }
        let brief = parts.join(" ");
        rows.push(LogRow {
            line: format!("{}\tjournal\t{}\t{}", ts, cmd, brief),
            ts,
            tb,
        });
    }
}

fn log_timeline(st: &State, idfilt: Option<&str>, limit: i64, journal_path: &Path) -> Vec<LogRow> {
    let mut rows = Vec::new();
    let filt = idfilt.map(|s| s.trim().to_string());
    for n in st.nodes.values() {
        if let Some(f) = &filt {
            if &n.id != f {
                continue;
            }
        }
        let mut tc = log_ts(n, "t_created");
        let mut tu = log_ts(n, "t_updated");
        if tc.is_empty() && tu.is_empty() {
            continue;
        }
        if tc.is_empty() {
            tc = tu.clone();
        }
        if tu.is_empty() {
            tu = tc.clone();
        }
        let ttl = if n.title.is_empty() {
            "(no title)"
        } else {
            n.title.as_str()
        };
        rows.push(LogRow {
            line: format!(
                "{}\tnode\t{}\t{}\tcreated\t{} status={}",
                tc,
                n.kind.as_str(),
                n.id,
                ttl,
                n.status
            ),
            ts: tc.clone(),
            tb: format!("{} {} tc", n.kind.as_str(), n.id),
        });
        if tu != tc {
            rows.push(LogRow {
                line: format!(
                    "{}\tnode\t{}\t{}\tupdated\t{} status={}",
                    tu,
                    n.kind.as_str(),
                    n.id,
                    ttl,
                    n.status
                ),
                ts: tu.clone(),
                tb: format!("{} {} tu", n.kind.as_str(), n.id),
            });
        }
    }
    for e in &st.edges {
        if let Some(f) = &filt {
            if f != &e.from && f != &e.to {
                continue;
            }
        }
        let ts = e.t_created.as_deref().unwrap_or("").trim().to_string();
        if ts.is_empty() {
            continue;
        }
        rows.push(LogRow {
            line: format!("{}\tedge\t{}\t{}\t{}", ts, e.from, e.label, e.to),
            ts: ts.clone(),
            tb: format!("edge {} {} {}", e.from, e.label, e.to),
        });
    }
    append_journal_timeline(&mut rows, journal_path, filt.as_deref());
    rows.sort_by(|a, b| (&b.ts, &b.tb).cmp(&(&a.ts, &a.tb)));
    if limit > 0 && rows.len() as i64 > limit {
        rows.truncate(limit as usize);
    }
    rows
}

pub fn cmd_log(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    let st = match load_state(ctx) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut lfilt: Option<String> = None;
    if !pos.is_empty() {
        let id0 = &pos[0];
        let jp = ctx.journalpath();
        let mut ok = st.nodes.contains_key(id0)
            || st.edges.iter().any(|e| &e.from == id0 || &e.to == id0);
        if !ok && journal_file_mentions_id(&jp, id0) {
            ok = true;
        }
        if !ok {
            return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {id0}"));
        }
        lfilt = Some(id0.clone());
    }
    let lim: i64 = if let Some(v) = kw_get(kw, "limit") {
        match v.parse::<i64>() {
            Ok(x) => x,
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --limit (expected integer)"),
        }
    } else {
        200
    };
    let rows = log_timeline(&st, lfilt.as_deref(), lim, &ctx.journalpath());
    if ctx.json {
        let jr: Vec<JVal> = rows
            .iter()
            .map(|r| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("ts".to_string(), JVal::Str(r.ts.clone())),
                    ("sort".to_string(), JVal::Str(r.tb.clone())),
                    ("line".to_string(), JVal::Str(r.line.clone())),
                ]))
            })
            .collect();
        let mut d = JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("log".to_string())),
            ("limit".to_string(), JVal::Int(lim)),
            ("rows".to_string(), JVal::Arr(jr)),
        ]);
        if let Some(f) = &lfilt {
            d.insert("id_filter".to_string(), JVal::Str(f.clone()));
        }
        let mut r = OpResult::ok();
        r.out = json_cli_out(d);
        return r;
    }
    let mut r = OpResult::ok();
    for row in rows {
        r.out.push_str(&row.line);
        r.out.push('\n');
    }
    r
}
