use crate::algebra::rederive_artifacts;
use crate::dor::{dor, dor_breakdown, parse_requires_coverage};
use crate::fitness::{goal_structured_kind, rederive_goals, refresh_goal_structured_fitness, GOAL_FITNESS_KINDS};
use crate::guards::{guard_status_transition, GuardVerdict};
use crate::json::{parse_json, JuliaDict};
use crate::journal::*;
use crate::model::{field_form, FieldValue, Form, Kind, Node, State};
use crate::renumber::{
    apply_renumber, glossary_rename_in_text, glossary_terms, renumber_blocked_by_done_evidence,
};
use crate::session::*;
use crate::status::{listnodes, prose_field_nonempty, EDGE_LABELS};
use crate::times::{stamp_new_node, stamp_touch_node};
use std::path::Path;

pub const EXIT_OK: i32 = 0;
pub const EXIT_ERR: i32 = 1;
pub const EXIT_INVARIANT: i32 = 3;
pub const EXIT_GUARD: i32 = 4;
pub const EXIT_NOTFOUND: i32 = 5;

pub struct OpResult {
    pub code: i32,
    pub out: String,
    pub err: String,
    pub journal: Vec<String>,
}

impl OpResult {
    pub fn ok() -> OpResult {
        OpResult {
            code: EXIT_OK,
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        }
    }

    pub fn fail(code: i32, msg: &str) -> OpResult {
        OpResult {
            code,
            out: String::new(),
            err: format!("{msg}\n"),
            journal: Vec::new(),
        }
    }

    pub fn fail_lines(code: i32, msgs: &[String]) -> OpResult {
        let mut err = String::new();
        for m in msgs {
            err.push_str(m);
            err.push('\n');
        }
        OpResult {
            code,
            out: String::new(),
            err,
            journal: Vec::new(),
        }
    }
}

pub fn kw_get<'a>(kw: &'a [(String, String)], key: &str) -> Option<&'a str> {
    kw.iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn kw_has(kw: &[(String, String)], key: &str) -> bool {
    kw.iter().any(|(k, _)| k == key)
}

fn csv_unstripped(raw: &str) -> Vec<String> {
    raw.split(',').map(|s| s.to_string()).collect()
}

fn csv_filtered(raw: &str) -> Vec<String> {
    raw.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn op_add(st: &mut State, kind_str: &str, kw: &[(String, String)]) -> OpResult {
    let kind: Kind = match kind_str.parse() {
        Ok(k) => k,
        Err(()) => return OpResult::fail(EXIT_ERR, &format!("unknown kind: {kind_str}")),
    };
    let backup = st.clone();
    let id = crate::ids::next_id(st, kind);
    let mut n = Node::new(kind, id.clone());
    n.title = kw_get(kw, "title").unwrap_or("").to_string();
    macro_rules! bail {
        ($code:expr, $msg:expr) => {{
            *st = backup;
            return OpResult::fail($code, $msg);
        }};
    }
    match kind {
        Kind::W => {
            n.wtype = Some(kw_get(kw, "type").unwrap_or("feature").to_string());
            n.cynefin = Some(kw_get(kw, "cynefin").unwrap_or("complicated").to_string());
            n.status = kw_get(kw, "status").unwrap_or("proposed").to_string();
            if let Some(g) = kw_get(kw, "goals") {
                n.fields
                    .insert("goals".to_string(), FieldValue::RefList(csv_unstripped(g)));
            }
            if let Some(t) = kw_get(kw, "theme") {
                n.set_single("theme", t.to_string());
            }
            let surface = csv_filtered(kw_get(kw, "surface").unwrap_or(""));
            if !surface.is_empty() {
                n.fields
                    .insert("surface".to_string(), FieldValue::RefList(surface));
            }
        }
        Kind::G => {
            n.status = kw_get(kw, "status").unwrap_or("unverified").to_string();
            if let Some(f) = kw_get(kw, "fitness") {
                n.attrs.insert("fitness".to_string(), f.to_string());
            }
            if let Some(fk_raw) = kw_get(kw, "fitness-kind") {
                let fk = fk_raw.trim().to_lowercase();
                if !GOAL_FITNESS_KINDS.contains(&fk.as_str()) {
                    bail!(EXIT_ERR, "bad --fitness-kind");
                }
                n.attrs.insert("fitness_kind".to_string(), fk);
            }
            if let Some(ft) = kw_get(kw, "fitness-target") {
                n.set_single("fitness_target", ft.to_string());
            }
            let aref = kw_get(kw, "area").unwrap_or("").trim().to_string();
            if aref.is_empty() {
                bail!(
                    EXIT_ERR,
                    "add g: --area=A-NN is required (create one with grove add a --title=...)"
                );
            }
            let ok_area = st
                .nodes
                .get(&aref)
                .map(|an| an.kind == Kind::A)
                .unwrap_or(false);
            if !ok_area {
                bail!(EXIT_ERR, &format!("add g: unknown --area id: {aref}"));
            }
            n.set_single("area", aref);
        }
        Kind::D => {
            n.status = kw_get(kw, "status").unwrap_or("proposed").to_string();
        }
        Kind::Q => {
            n.status = kw_get(kw, "status").unwrap_or("open").to_string();
            n.cynefin = Some(kw_get(kw, "cynefin").unwrap_or("complicated").to_string());
        }
        Kind::B => {
            n.status = kw_get(kw, "status").unwrap_or("proposed").to_string();
            n.cynefin = Some(kw_get(kw, "cynefin").unwrap_or("complicated").to_string());
        }
        Kind::T => {
            n.status = kw_get(kw, "status").unwrap_or("open").to_string();
        }
        Kind::Y => {
            n.status = "proposed".to_string();
            if n.title.trim().is_empty() {
                bail!(EXIT_ERR, "add y: --title is required");
            }
            let tags = csv_filtered(kw_get(kw, "tags").unwrap_or(""));
            if tags.is_empty() {
                bail!(
                    EXIT_ERR,
                    "add y: --tags=<t1,t2> is required (≥1 glossary term)"
                );
            }
            n.fields.insert("tags".to_string(), FieldValue::RefList(tags));
            let surface = csv_filtered(kw_get(kw, "surface").unwrap_or(""));
            if !surface.is_empty() {
                n.fields
                    .insert("surface".to_string(), FieldValue::RefList(surface));
            }
            if let Some(w) = kw_get(kw, "why") {
                n.fields
                    .insert("why".to_string(), FieldValue::Prose(vec![w.to_string()]));
            }
            if !n.fields.contains_key("surface") {
                let ok_why = match n.fields.get("why") {
                    Some(FieldValue::Prose(lines)) => prose_field_nonempty(lines),
                    _ => false,
                };
                if !ok_why {
                    bail!(EXIT_ERR, "add y: --surface absent requires --why prose");
                }
            }
            let from = csv_filtered(kw_get(kw, "from").unwrap_or(""));
            if from.is_empty() {
                bail!(
                    EXIT_ERR,
                    "add y: --from=<W-NN|D-NN|Q-NN|B-NN> is required (≥1 provenance record)"
                );
            }
        }
        Kind::A => {
            n.status = "present".to_string();
            if n.title.trim().is_empty() {
                bail!(EXIT_ERR, "add a: --title is required");
            }
            let surface = csv_filtered(kw_get(kw, "surface").unwrap_or(""));
            if !surface.is_empty() {
                n.fields
                    .insert("surface".to_string(), FieldValue::RefList(surface));
            }
        }
    }
    st.nodes.insert(id.clone(), n);
    stamp_new_node(st.nodes.get_mut(&id).expect("just inserted"));
    if let Err((code, msg)) = flush_add_edges(st, kind, &id, kw) {
        *st = backup;
        return OpResult::fail(code, &msg);
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.out = format!("{id}\n");
    r.journal.push(wrap_journal_record("add", jinv_rm_node(&id)));
    r
}

fn flush_add_edges(
    st: &mut State,
    kind: Kind,
    id: &str,
    kw: &[(String, String)],
) -> Result<(), (i32, String)> {
    let push = |st: &mut State, from: &str, label: &str, to: &str| -> Result<(), (i32, String)> {
        match crate::invariants::validate_and_push_edge(st, from, label, to, true) {
            Some(msg) => Err((EXIT_GUARD, msg)),
            None => Ok(()),
        }
    };
    if kind == Kind::D && kw_has(kw, "supersedes") {
        for oid in kw_get(kw, "supersedes").unwrap_or("").split(',') {
            let oid = oid.trim();
            if oid.is_empty() {
                continue;
            }
            push(st, id, "supersedes", oid)?;
        }
    } else if kind == Kind::Q && kw_has(kw, "targets") {
        for tid in kw_get(kw, "targets").unwrap_or("").split(',') {
            let tid = tid.trim();
            if tid.is_empty() {
                continue;
            }
            push(st, id, "asks", tid)?;
        }
    } else if kind == Kind::B {
        if kw_has(kw, "tests") {
            for qid in kw_get(kw, "tests").unwrap_or("").split(',') {
                let qid = qid.trim();
                if qid.is_empty() {
                    continue;
                }
                push(st, id, "tests", qid)?;
            }
        }
        if kw_has(kw, "targets") {
            for wid in kw_get(kw, "targets").unwrap_or("").split(',') {
                let wid = wid.trim();
                if wid.is_empty() {
                    continue;
                }
                push(st, id, "targets", wid)?;
            }
        }
    } else if kind == Kind::Y && kw_has(kw, "from") {
        for oid in kw_get(kw, "from").unwrap_or("").split(',') {
            let oid = oid.trim();
            if oid.is_empty() {
                continue;
            }
            let src_kind = st.nodes.get(oid).map(|n| n.kind);
            let Some(src_kind) = src_kind else {
                return Err((EXIT_GUARD, format!("add y: unknown --from id: {oid}")));
            };
            match src_kind {
                Kind::W => push(st, oid, "produces", id)?,
                Kind::D | Kind::Q | Kind::B => push(st, id, "distills", oid)?,
                _ => {
                    return Err((
                        EXIT_GUARD,
                        format!("add y: --from {oid} must reference W or D/Q/B"),
                    ))
                }
            }
        }
    }
    Ok(())
}

fn goal_notes_distill_deferred(g: &Node) -> bool {
    if g.kind != Kind::G {
        return false;
    }
    g.lines("notes")
        .iter()
        .any(|ln| ln.contains("--distill-deferred"))
}

fn lazy_distill_prompts(st: &State, w: &Node, old_goal_status: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    if w.kind != Kind::W {
        return out;
    }
    for gid_raw in w.lines("goals") {
        let gid = gid_raw.trim().to_string();
        if gid.is_empty() {
            continue;
        }
        let Some(ost) = old_goal_status
            .iter()
            .find(|(k, _)| *k == gid)
            .map(|(_, v)| v.clone())
        else {
            continue;
        };
        if ost == "verified" {
            continue;
        }
        let Some(g) = st.nodes.get(&gid) else {
            continue;
        };
        if g.kind != Kind::G || g.status != "verified" {
            continue;
        }
        if goal_notes_distill_deferred(g) {
            continue;
        }
        out.push(format!(
            "grove: goal {} ({}) verified, distill content: `grove distill {}` (or `grove distill {} --null` when nothing is worth keeping; lazy distill, see rules.md). To skip: add a `notes` prose line containing `--distill-deferred`.",
            gid, g.title, gid, gid
        ));
    }
    out
}

fn guard_sessions_for_progress_endpoints(
    st: &State,
    eff: &str,
    from: &str,
    to: &str,
) -> Option<String> {
    for id in [from, to] {
        let Some(n) = st.nodes.get(id) else {
            continue;
        };
        if n.kind != Kind::W || n.status != "progress" {
            continue;
        }
        if let Some(msg) = session_denial_progress_mutate(n, eff) {
            return Some(msg);
        }
    }
    None
}

pub fn op_set(st: &mut State, id: &str, key: &str, val: &str, eff: &str) -> OpResult {
    if !st.nodes.contains_key(id) {
        return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {id}"));
    }
    if key == "status" {
        let new_status = val;
        {
            let n = st.nodes.get(id).expect("checked");
            if n.kind == Kind::W && n.status == "progress" && new_status != "progress" {
                if let Some(msg) = session_denial_progress_release(n, eff) {
                    return OpResult::fail(EXIT_GUARD, &msg);
                }
            }
        }
        {
            let n = st.nodes.get(id).expect("checked");
            if n.kind == Kind::W && new_status == "progress" && !dor(st, n, false) {
                let missing: Vec<String> = dor_breakdown(st, n, false)
                    .iter()
                    .filter(|(_, ok, _)| !ok)
                    .map(|(label, _, _)| label.clone())
                    .collect();
                let mut r = OpResult::fail(
                    EXIT_GUARD,
                    &format!("DoR ≢ ⊤ for {}; see `grove dor {}`", n.id, n.id),
                );
                r.journal
                    .push(wrap_journal_record("set", jinv_dor_reject(&n.id, &missing)));
                return r;
            }
        }
        let verdict = {
            let n = st.nodes.get(id).expect("checked");
            guard_status_transition(st, n, new_status)
        };
        match verdict {
            GuardVerdict::Ok => {}
            GuardVerdict::Invalid(msgs) => return OpResult::fail_lines(EXIT_ERR, &msgs),
            GuardVerdict::Reject(msgs) => return OpResult::fail_lines(EXIT_GUARD, &msgs),
        }
        let (line, prompts): (String, Vec<String>);
        {
            let n = st.nodes.get(id).expect("checked");
            let old_status = n.status.clone();
            if n.kind == Kind::W {
                let gs = goal_statuses_jdict(st, n);
                let inv = jinv_set_w_status_with_goals(id, &old_status, gs, n);
                line = wrap_journal_record("set", inv);
            } else {
                line = wrap_journal_record("set", jinv_set_status_plain(id, &old_status));
            }
        }
        let old_status;
        let old_goal_pairs: Vec<(String, String)>;
        {
            let n = st.nodes.get_mut(id).expect("checked");
            old_status = n.status.clone();
            n.status = new_status.to_string();
        }
        let is_w = st.nodes.get(id).map(|n| n.kind) == Some(Kind::W);
        if is_w {
            if new_status == "progress" {
                let n = st.nodes.get_mut(id).expect("checked");
                assign_w_claim_session(n, eff);
            } else if old_status == "progress" {
                let n = st.nodes.get_mut(id).expect("checked");
                clear_w_session_attrs(n);
            }
            rederive_goals(st, id);
        }
        old_goal_pairs = if new_status == "done" && is_w {
            let parsed = parse_json(&line).ok();
            let mut pairs = Vec::new();
            if let Some(rec) = parsed {
                if let Some(crate::json::Json::Obj(gs)) = rec
                    .get("inv")
                    .and_then(|i| i.get("goal_statuses"))
                {
                    for (k, v) in gs {
                        if let Some(sv) = v.as_str() {
                            pairs.push((k.clone(), sv.to_string()));
                        }
                    }
                }
            }
            pairs
        } else {
            Vec::new()
        };
        if new_status == "done" && is_w {
            let n = st.nodes.get(id).expect("checked");
            let prompts2 = lazy_distill_prompts(st, n, &old_goal_pairs);
            prompts = prompts2;
        } else {
            prompts = Vec::new();
        }
        stamp_touch_node(st.nodes.get_mut(id).expect("checked"));
        rederive_artifacts(st);
        let mut r = OpResult::ok();
        for p in prompts {
            r.err.push_str(&p);
            r.err.push('\n');
        }
        r.journal.push(line);
        return r;
    }
    {
        let n = st.nodes.get(id).expect("checked");
        if n.kind == Kind::W && n.status == "progress" {
            if let Some(msg) = session_denial_progress_mutate(n, eff) {
                return OpResult::fail(EXIT_GUARD, &msg);
            }
        }
    }
    let kind = st.nodes.get(id).map(|n| n.kind).expect("checked");
    let line: String;
    match key {
        "cynefin" => {
            let n = st.nodes.get_mut(id).expect("checked");
            let old = n.cynefin.clone().unwrap_or_default();
            line = wrap_journal_record("set", jinv_set_simple_old("set_cynefin", id, &old));
            n.cynefin = Some(val.to_string());
        }
        "type" => {
            let n = st.nodes.get_mut(id).expect("checked");
            let old = n.wtype.clone().unwrap_or_default();
            line = wrap_journal_record("set", jinv_set_simple_old("set_type", id, &old));
            n.wtype = Some(val.to_string());
        }
        "title" => {
            let n = st.nodes.get_mut(id).expect("checked");
            let old = n.title.clone();
            line = wrap_journal_record("set", jinv_set_simple_old("set_title", id, &old));
            n.title = val.to_string();
        }
        "fitness" if kind == Kind::G => {
            {
                let n = st.nodes.get_mut(id).expect("checked");
                let old = n.attr("fitness");
                line = wrap_journal_record(
                    "set",
                    jinv_set_simple_old("set_g_attr_fitness", id, &old),
                );
                n.attrs.insert("fitness".to_string(), val.to_string());
            }
            refresh_goal_structured_fitness(st, id);
        }
        "fitness_kind" if kind == Kind::G => {
            let ks = val.trim().to_lowercase();
            if !GOAL_FITNESS_KINDS.contains(&ks.as_str()) {
                return OpResult::fail(
                    EXIT_ERR,
                    &format!(
                        "bad fitness_kind (expected one of: {})",
                        GOAL_FITNESS_KINDS.join(", ")
                    ),
                );
            }
            {
                let n = st.nodes.get_mut(id).expect("checked");
                let hb = n.attrs.contains_key("fitness_kind");
                let oldk = n.attr("fitness_kind");
                line = wrap_journal_record(
                    "set",
                    jinv_set_g_attr_fitness_kind(id, hb, &oldk, &ks),
                );
                n.attrs.insert("fitness_kind".to_string(), ks);
            }
            refresh_goal_structured_fitness(st, id);
        }
        "area" if kind == Kind::G => {
            let aref = val.trim().to_string();
            let ok_area = st
                .nodes
                .get(&aref)
                .map(|an| an.kind == Kind::A)
                .unwrap_or(false);
            if !ok_area {
                return OpResult::fail(
                    EXIT_ERR,
                    &format!("set: unknown area: {aref} (expected an existing A-NN node)"),
                );
            }
            let n = st.nodes.get_mut(id).expect("checked");
            let hb = n.fields.contains_key("area");
            let old = if hb { n.single("area") } else { String::new() };
            line = wrap_journal_record("set", jinv_set_g_area(id, hb, &old));
            n.set_single("area", aref);
        }
        "requires_coverage" if kind == Kind::G || kind == Kind::T => {
            if parse_requires_coverage(Some(val)).is_none() {
                return OpResult::fail(
                    EXIT_ERR,
                    "bad requires_coverage (expected `true` or a float in (0,1])",
                );
            }
            let n = st.nodes.get_mut(id).expect("checked");
            let hb = n.attrs.contains_key("requires_coverage");
            let old = n.attr("requires_coverage");
            line = wrap_journal_record("set", jinv_set_requires_coverage(id, hb, &old));
            n.attrs
                .insert("requires_coverage".to_string(), val.to_string());
        }
        _ => {
            return OpResult::fail(EXIT_ERR, &format!("unsupported key: {key}"));
        }
    }
    stamp_touch_node(st.nodes.get_mut(id).expect("checked"));
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_field(
    st: &mut State,
    id: &str,
    fname: &str,
    op: &str,
    value: Option<&str>,
    eff: &str,
) -> OpResult {
    if !st.nodes.contains_key(id) {
        return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {id}"));
    }
    {
        let n = st.nodes.get(id).expect("checked");
        if n.kind == Kind::W && n.status == "progress" {
            if let Some(msg) = session_denial_progress_mutate(n, eff) {
                return OpResult::fail(EXIT_GUARD, &msg);
            }
        }
    }
    let kind = st.nodes.get(id).map(|n| n.kind).expect("checked");
    let Some(form) = field_form(kind, fname) else {
        return OpResult::fail(
            EXIT_ERR,
            &format!("unknown field {fname} on {}", kind.as_str()),
        );
    };
    if kind == Kind::G && fname == "fitness_current" {
        let n = st.nodes.get(id).expect("checked");
        if let Some(kg) = goal_structured_kind(n) {
            if kg != "manual" {
                return OpResult::fail(
                    EXIT_GUARD,
                    "grove field: `fitness_current` is derived for structured goals; use kind=manual to author it",
                );
            }
        }
    }
    let line: String;
    match op {
        "clear" => match form {
            Form::Prose | Form::RefList => {
                let n = st.nodes.get_mut(id).expect("checked");
                let oldv = n.lines(fname);
                line = wrap_journal_record("field", jinv_field_restore_lines(id, fname, &oldv));
                let fv = if form == Form::Prose {
                    FieldValue::Prose(Vec::new())
                } else {
                    FieldValue::RefList(Vec::new())
                };
                n.fields.insert(fname.to_string(), fv);
            }
            Form::Fitness => {
                let n = st.nodes.get_mut(id).expect("checked");
                let oldd = n.fitness();
                line = wrap_journal_record("field", jinv_field_restore_fitness(id, fname, &oldd));
                n.fields
                    .insert(fname.to_string(), FieldValue::Fitness(Default::default()));
            }
            Form::Single => {
                let n = st.nodes.get_mut(id).expect("checked");
                let prev = n.single(fname);
                line = wrap_journal_record("field", jinv_field_restore_single(id, fname, &prev));
                n.set_single(fname, String::new());
            }
        },
        "add" => {
            let Some(val) = value else {
                return OpResult::fail(EXIT_ERR, "missing value");
            };
            match form {
                Form::Prose | Form::RefList => {
                    let n = st.nodes.get_mut(id).expect("checked");
                    line = wrap_journal_record("field", jinv_field_pop_last(id, fname));
                    let needs_insert =
                        !matches!(n.fields.get(fname), Some(FieldValue::Prose(_)) | Some(FieldValue::RefList(_)));
                    if needs_insert {
                        let fv = if form == Form::Prose {
                            FieldValue::Prose(Vec::new())
                        } else {
                            FieldValue::RefList(Vec::new())
                        };
                        n.fields.insert(fname.to_string(), fv);
                    }
                    match n.fields.get_mut(fname) {
                        Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => {
                            v.push(val.to_string())
                        }
                        _ => unreachable!(),
                    }
                }
                Form::Single => {
                    let n = st.nodes.get_mut(id).expect("checked");
                    let prev = n.single(fname);
                    line =
                        wrap_journal_record("field", jinv_field_restore_single(id, fname, &prev));
                    n.set_single(fname, val.to_string());
                }
                Form::Fitness => {
                    return OpResult::fail(EXIT_ERR, &format!("field {fname} not addable"));
                }
            }
        }
        "rm" => {
            let Some(raw_idx) = value else {
                return OpResult::fail(EXIT_ERR, "missing index");
            };
            let idx: i64 = match raw_idx.parse() {
                Ok(v) => v,
                Err(_) => {
                    return OpResult {
                        code: EXIT_ERR,
                        out: String::new(),
                        err: String::new(),
                        journal: Vec::new(),
                    }
                }
            };
            let n = st.nodes.get_mut(id).expect("checked");
            let v: Vec<String> = match n.fields.get(fname) {
                Some(FieldValue::Prose(lines)) | Some(FieldValue::RefList(lines)) => lines.clone(),
                _ => Vec::new(),
            };
            if idx < 1 || idx > v.len() as i64 {
                return OpResult::fail(EXIT_ERR, "index out of range");
            }
            let removed = v[(idx - 1) as usize].clone();
            line = wrap_journal_record(
                "field",
                jinv_field_insert_line(id, fname, idx, &removed),
            );
            match n.fields.get_mut(fname) {
                Some(FieldValue::Prose(lines)) | Some(FieldValue::RefList(lines)) => {
                    lines.remove((idx - 1) as usize);
                }
                _ => {}
            }
        }
        _ => {
            return OpResult::fail(EXIT_ERR, &format!("unknown op: {op}"));
        }
    }
    if kind == Kind::G && fname == "fitness_target" && (op == "add" || op == "clear") {
        refresh_goal_structured_fitness(st, id);
    }
    stamp_touch_node(st.nodes.get_mut(id).expect("checked"));
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_evidence(st: &mut State, id: &str, text: &str, eff: &str) -> OpResult {
    op_field(st, id, "evidence", "add", Some(text), eff)
}

pub fn op_link(st: &mut State, from: &str, label: &str, to: &str, eff: &str) -> OpResult {
    if !EDGE_LABELS.contains(&label) {
        return OpResult::fail(EXIT_ERR, &format!("unknown label: {label}"));
    }
    if let Some(msg) = guard_sessions_for_progress_endpoints(st, eff, from, to) {
        return OpResult::fail(EXIT_GUARD, &msg);
    }
    if let Some(msg) = crate::invariants::validate_and_push_edge(st, from, label, to, true) {
        return OpResult::fail(EXIT_GUARD, &msg);
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal
        .push(wrap_journal_record("link", jinv_unlink_edge(from, label, to)));
    r
}

pub fn op_unlink(st: &mut State, from: &str, label: &str, to: &str, eff: &str) -> OpResult {
    let t_created = match st
        .edges
        .iter()
        .find(|e| e.from == from && e.label == label && e.to == to)
    {
        None => return OpResult::fail(EXIT_NOTFOUND, "no such edge"),
        Some(e) => e.t_created.clone(),
    };
    if let Some(msg) = guard_sessions_for_progress_endpoints(st, eff, from, to) {
        return OpResult::fail(EXIT_GUARD, &msg);
    }
    let line = wrap_journal_record(
        "unlink",
        jinv_restore_edge(from, label, to, t_created.as_deref()),
    );
    st.edges
        .retain(|e| !(e.from == from && e.label == label && e.to == to));
    if let Some(n) = st.nodes.get_mut(from) {
        stamp_touch_node(n);
    }
    if let Some(n) = st.nodes.get_mut(to) {
        stamp_touch_node(n);
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_fitness(st: &mut State, wid: &str, gid: &str, delta: i64, eff: &str) -> OpResult {
    if !st.nodes.contains_key(wid) {
        return OpResult::fail(EXIT_NOTFOUND, &format!("missing: {wid}"));
    }
    if !st.nodes.contains_key(gid) {
        return OpResult::fail(EXIT_NOTFOUND, &format!("missing: {gid}"));
    }
    {
        let w = st.nodes.get(wid).expect("checked");
        if let Some(msg) = session_denial_progress_mutate(w, eff) {
            return OpResult::fail(EXIT_GUARD, &msg);
        }
    }
    let w = st.nodes.get_mut(wid).expect("checked");
    if !matches!(w.fields.get("fitness"), Some(FieldValue::Fitness(_))) {
        w.fields
            .insert("fitness".to_string(), FieldValue::Fitness(Default::default()));
    }
    let Some(FieldValue::Fitness(f)) = w.fields.get_mut("fitness") else {
        unreachable!()
    };
    let had_key = f.contains_key(gid);
    let previous = f.get(gid).copied();
    f.insert(gid.to_string(), delta);
    stamp_touch_node(w);
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(wrap_journal_record(
        "fitness",
        jinv_restore_fitness_key(wid, gid, had_key, previous),
    ));
    r
}

pub fn op_renumber(st: &mut State, old_id: &str, new_id: &str, eff: &str) -> OpResult {
    let old = old_id.trim();
    let new = new_id.trim();
    if new.is_empty() {
        return OpResult::fail(EXIT_ERR, "bad --to");
    }
    if old == new {
        return OpResult::ok();
    }
    if let Some(ow) = st.nodes.get(old) {
        if ow.kind == Kind::W && ow.status == "progress" {
            if let Some(msg) = session_denial_progress_mutate(ow, eff) {
                return OpResult::fail(EXIT_GUARD, &msg);
            }
        }
    }
    if renumber_blocked_by_done_evidence(st, old) {
        return OpResult::fail(
            EXIT_GUARD,
            "grove renumber: refusing; id occurs in evidence on a done W",
        );
    }
    if let Err(msg) = apply_renumber(st, old, new) {
        return OpResult::fail(EXIT_ERR, &msg);
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal
        .push(wrap_journal_record("renumber", jinv_renumber_swap(new, old)));
    r
}

pub fn op_glossary_rename(
    st: &mut State,
    glossary: &mut Option<String>,
    old_raw: &str,
    new_raw: &str,
) -> OpResult {
    let old = old_raw.trim().to_string();
    let new = new_raw.trim().to_string();
    if old.is_empty() || new.is_empty() {
        return OpResult::fail(EXIT_ERR, "glossary rename: empty term");
    }
    if old == new {
        return OpResult::fail(EXIT_ERR, "glossary rename: old and new are identical");
    }
    let gtext = glossary.as_deref().unwrap_or("");
    let terms = glossary_terms(gtext);
    let mut users: Vec<String> = listnodes(st, Kind::Y, false)
        .iter()
        .filter(|x| x.lines("tags").iter().any(|t| *t == old))
        .map(|x| x.id.clone())
        .collect();
    users.sort();
    let in_glossary = terms.contains(&old);
    if !in_glossary && users.is_empty() {
        return OpResult::fail(
            EXIT_NOTFOUND,
            &format!("glossary rename: `{old}` is neither in glossary.md nor used by any discovery"),
        );
    }
    if terms.contains(&new) {
        return OpResult::fail(
            EXIT_GUARD,
            &format!("glossary rename: `{new}` already present in glossary.md"),
        );
    }
    let mut changed_in_glossary = false;
    if in_glossary {
        let text = glossary.as_deref().unwrap_or("");
        let (renamed, changed) = glossary_rename_in_text(text, &old, &new);
        if !changed {
            return OpResult::fail(
                EXIT_NOTFOUND,
                &format!("glossary rename: `{old}` not found in glossary.md"),
            );
        }
        *glossary = Some(renamed);
        changed_in_glossary = true;
    }
    let mut snap = JuliaDict::new();
    for xid in &users {
        let x = st.nodes.get(xid).expect("user listed");
        let tags = x.lines("tags");
        snap.insert(
            xid.clone(),
            crate::json::JVal::Arr(tags.iter().map(|t| crate::json::JVal::Str(t.clone())).collect()),
        );
    }
    for xid in &users {
        let x = st.nodes.get_mut(xid).expect("user listed");
        if let Some(FieldValue::RefList(tags)) = x.fields.get_mut("tags") {
            let mut seen: Vec<String> = Vec::new();
            for t in tags.iter() {
                let mapped = if *t == old { new.clone() } else { t.clone() };
                if !seen.contains(&mapped) {
                    seen.push(mapped);
                }
            }
            *tags = seen;
        }
        stamp_touch_node(x);
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(wrap_journal_record(
        "glossary",
        jinv_glossary_rename_restore(snap, &old, &new, changed_in_glossary),
    ));
    r
}

pub fn op_resume(st: &mut State, id: &str, eff: &str) -> OpResult {
    let Some(w) = st.nodes.get(id) else {
        return OpResult {
            code: EXIT_NOTFOUND,
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        };
    };
    if w.kind != Kind::W {
        return OpResult::fail(EXIT_ERR, "not a work item");
    }
    if w.status != "progress" {
        return OpResult::fail(EXIT_GUARD, &format!("{id} is not in progress"));
    }
    let line = wrap_journal_record("resume", jinv_session_restore_claim(id, w));
    let w = st.nodes.get_mut(id).expect("checked");
    assign_w_claim_session(w, eff);
    stamp_touch_node(w);
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_handoff(st: &mut State, id: &str, to: Option<&str>, eff: &str) -> OpResult {
    let Some(to_raw) = to else {
        return OpResult::fail(EXIT_ERR, "missing --to=<session-token>");
    };
    let to_tok = to_raw.trim().to_string();
    if to_tok.is_empty() {
        return OpResult::fail(EXIT_ERR, "empty --to");
    }
    let Some(w) = st.nodes.get(id) else {
        return OpResult {
            code: EXIT_NOTFOUND,
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        };
    };
    if w.kind != Kind::W {
        return OpResult::fail(EXIT_ERR, "not a work item");
    }
    if w.status != "progress" {
        return OpResult::fail(EXIT_GUARD, &format!("{id} is not in progress"));
    }
    if !progress_has_session_record(w) {
        return OpResult::fail(
            EXIT_GUARD,
            &format!("{id} has no session claim; use `grove resume`"),
        );
    }
    if !session_token_matches(w, eff) {
        return OpResult::fail(
            EXIT_GUARD,
            "only the holding session can hand off; use `grove resume` first",
        );
    }
    let line = wrap_journal_record("handoff", jinv_session_restore_claim(id, w));
    let w = st.nodes.get_mut(id).expect("checked");
    assign_w_claim_session(w, &to_tok);
    stamp_touch_node(w);
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_revert(st: &mut State, id: &str, eff: &str) -> OpResult {
    let Some(w) = st.nodes.get(id) else {
        return OpResult {
            code: EXIT_NOTFOUND,
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        };
    };
    if w.kind != Kind::W {
        return OpResult::fail(EXIT_ERR, "not a work item");
    }
    if w.status != "progress" {
        return OpResult::fail(EXIT_GUARD, &format!("{id} is not in progress"));
    }
    if let Some(msg) = session_denial_progress_release(w, eff) {
        return OpResult::fail(EXIT_GUARD, &msg);
    }
    let gs = goal_statuses_jdict(st, w);
    let inv = jinv_set_w_status_with_goals(id, "progress", gs, w);
    let line = wrap_journal_record("revert", inv);
    {
        let w = st.nodes.get_mut(id).expect("checked");
        w.status = "ready".to_string();
        clear_w_session_attrs(w);
    }
    rederive_goals(st, id);
    stamp_touch_node(st.nodes.get_mut(id).expect("checked"));
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    r.journal.push(line);
    r
}

pub fn op_undo(
    st: &mut State,
    journal_path: &Path,
    mut glossary: Option<&mut String>,
    steps_arg: Option<&str>,
    session: &str,
) -> OpResult {
    let meta_ok = journal_path
        .metadata()
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false);
    if !meta_ok {
        return OpResult::fail(
            EXIT_ERR,
            &format!("grove undo: no journal at {}", journal_path.display()),
        );
    }
    let steps: i64 = match steps_arg {
        Some(v) => match v.parse::<i64>() {
            Ok(n) => n.max(0),
            Err(_) => return OpResult::fail(EXIT_ERR, "grove undo: bad --steps"),
        },
        None => 1,
    };
    if steps == 0 {
        return OpResult::ok();
    }
    let (_, recs) = journal_read_nonempty_pairs(journal_path);
    let Some(idxs) = journal_tail_mutation_view(&recs, steps as usize) else {
        return OpResult::fail(
            EXIT_ERR,
            &format!(
                "grove undo: journal has fewer than {} mutation entr{}",
                steps,
                if steps == 1 { "y" } else { "ies" }
            ),
        );
    };
    for i in idxs.iter().rev() {
        let rec = &recs[*i];
        let Some(inv) = rec.get("inv") else {
            return OpResult::fail(EXIT_ERR, "grove undo: record missing inverse");
        };
        if let Some(msg) = journal_apply_inverse(st, inv) {
            return OpResult::fail(EXIT_INVARIANT, &msg);
        }
        let is_glossary = inv.get("op").and_then(|v| v.as_str()) == Some("glossary_rename_restore")
            && inv.get("glossary_changed").and_then(|v| v.as_bool()) == Some(true);
        if is_glossary {
            if let (Some(old), Some(new)) = (
                inv.get("old").and_then(|v| v.as_str()),
                inv.get("new").and_then(|v| v.as_str()),
            ) {
                if let Some(gtext) = glossary.as_deref() {
                    let (reversed, gchanged) = glossary_rename_in_text(gtext, new, old);
                    if gchanged {
                        if let Some(g) = glossary.as_deref_mut() {
                            *g = reversed;
                        }
                    }
                }
            }
        }
    }
    crate::ids::reconcile_counters(st);
    if journal_drop_lines_inplace(journal_path, &idxs).is_err() {
        return OpResult::fail(EXIT_ERR, "grove undo: could not truncate journal");
    }
    rederive_artifacts(st);
    let mut r = OpResult::ok();
    let line = stamp_journal_session(&wrap_journal_record("undo", jinv_undo(steps)), session);
    let _ = append_journal_record(journal_path, &line);
    r.journal.push(line);
    r
}
