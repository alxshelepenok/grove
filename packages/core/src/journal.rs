use crate::fitness::rederive_goals;
use crate::ids::reconcile_counters;
use crate::invariants::validate_and_push_edge;
use crate::json::{emit_jval, parse_json, JVal, JuliaDict, Json};
use crate::model::{field_form, FieldValue, Form, Node, State};
use crate::renumber::apply_renumber;
use crate::session::progress_has_session_record;
use crate::times::{stamp_touch_node, utc_stamp_second};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;

pub const JOURNAL_GATE_OP: &str = "gate";
pub const JOURNAL_DISTILL_OP: &str = "distill";
pub const JOURNAL_ARCHIVE_OP: &str = "archive";
pub const JOURNAL_NONMUTATION_OPS: [&str; 5] = [
    JOURNAL_GATE_OP,
    JOURNAL_DISTILL_OP,
    JOURNAL_ARCHIVE_OP,
    "dor_reject",
    "undo",
];

fn s(v: &str) -> JVal {
    JVal::Str(v.to_string())
}

pub fn wrap_journal_record(cmd: &str, inv: JuliaDict) -> String {
    let rec = JuliaDict::from_pairs(vec![
        ("v".to_string(), JVal::Int(1)),
        ("ts".to_string(), s(&utc_stamp_second())),
        ("cmd".to_string(), s(cmd)),
        ("inv".to_string(), JVal::Obj(inv)),
    ]);
    emit_jval(&JVal::Obj(rec))
}

fn scan_str_span(b: &[u8], mut i: usize) -> Option<usize> {
    i += 1;
    while let Some(&c) = b.get(i) {
        match c {
            b'"' => return Some(i + 1),
            b'\\' => i += 2,
            _ => i += 1,
        }
    }
    None
}

fn scan_value_span(b: &[u8], mut i: usize) -> Option<usize> {
    match *b.get(i)? {
        b'"' => scan_str_span(b, i),
        b'{' | b'[' => {
            let mut depth = 0usize;
            while let Some(&c) = b.get(i) {
                match c {
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    b'"' => {
                        i = scan_str_span(b, i)?;
                        continue;
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        _ => {
            while let Some(&c) = b.get(i) {
                if matches!(c, b',' | b'}' | b']' | b' ' | b'\t' | b'\r' | b'\n') {
                    break;
                }
                i += 1;
            }
            Some(i)
        }
    }
}

fn shallow_json_fields(text: &str) -> Option<Vec<(String, String)>> {
    let b = text.as_bytes();
    let n = b.len();
    let mut i = 0;
    while i < n && b[i].is_ascii_whitespace() {
        i += 1;
    }
    if b.get(i) != Some(&b'{') {
        return None;
    }
    i += 1;
    let mut fields = Vec::new();
    loop {
        while i < n && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b.get(i) == Some(&b'}') {
            i += 1;
            while i < n && b[i].is_ascii_whitespace() {
                i += 1;
            }
            return if i == n { Some(fields) } else { None };
        }
        if b.get(i) != Some(&b'"') {
            return None;
        }
        let kend = scan_str_span(b, i)?;
        let key = text[i + 1..kend - 1].to_string();
        i = kend;
        while i < n && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b.get(i) != Some(&b':') {
            return None;
        }
        i += 1;
        while i < n && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let vstart = i;
        i = scan_value_span(b, i)?;
        fields.push((key, text[vstart..i].to_string()));
        while i < n && b[i].is_ascii_whitespace() {
            i += 1;
        }
        match *b.get(i)? {
            b',' => i += 1,
            b'}' => {
                i += 1;
                while i < n && b[i].is_ascii_whitespace() {
                    i += 1;
                }
                return if i == n { Some(fields) } else { None };
            }
            _ => return None,
        }
    }
}

pub fn stamp_journal_session(line: &str, token: &str) -> String {
    let Some(fields) = shallow_json_fields(line) else {
        return line.to_string();
    };
    if fields.iter().any(|(k, _)| k == "session") {
        return line.to_string();
    }
    let mut d = JuliaDict::from_pairs(
        fields
            .iter()
            .map(|(k, _)| (k.clone(), JVal::Null))
            .collect(),
    );
    d.insert("session".to_string(), JVal::Null);
    let mut out = String::with_capacity(line.len() + token.len() + 14);
    out.push('{');
    let mut first = true;
    for (k, _) in d.iter_pairs() {
        if !first {
            out.push(',');
        }
        first = false;
        if k == "session" {
            out.push_str("\"session\":");
            out.push_str(&emit_jval(&s(token)));
        } else {
            let v = fields
                .iter()
                .find(|(fk, _)| fk == k)
                .map(|(_, v)| v)
                .expect("field");
            out.push('"');
            out.push_str(k);
            out.push_str("\":");
            out.push_str(v);
        }
    }
    out.push('}');
    out
}

pub fn jinv_rm_node(id: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("rm_node")),
        ("id".to_string(), s(id)),
    ])
}

pub fn jinv_unlink_edge(from: &str, label: &str, to: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("unlink_edge")),
        ("from".to_string(), s(from)),
        ("label".to_string(), s(label)),
        ("to".to_string(), s(to)),
    ])
}

pub fn jinv_restore_edge(from: &str, label: &str, to: &str, t_created: Option<&str>) -> JuliaDict {
    let mut d = JuliaDict::from_pairs(vec![
        ("op".to_string(), s("restore_edge")),
        ("from".to_string(), s(from)),
        ("label".to_string(), s(label)),
        ("to".to_string(), s(to)),
    ]);
    let tc = match t_created {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => String::new(),
    };
    d.insert("t_created".to_string(), s(&tc));
    d
}

pub fn jinv_restore_fitness_key(
    wid: &str,
    gid: &str,
    had_key: bool,
    previous: Option<i64>,
) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("restore_fitness_key")),
        ("wid".to_string(), s(wid)),
        ("gid".to_string(), s(gid)),
        ("had_key".to_string(), JVal::Bool(had_key)),
        (
            "previous".to_string(),
            previous.map(JVal::Int).unwrap_or(JVal::Null),
        ),
    ])
}

pub fn session_journal_snap(w: &Node) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        (
            "had_session_before".to_string(),
            JVal::Bool(progress_has_session_record(w)),
        ),
        ("old_session".to_string(), s(&w.attr("session"))),
        (
            "had_session_at_before".to_string(),
            JVal::Bool(w.attrs.contains_key("session_at")),
        ),
        ("old_session_at".to_string(), s(&w.attr("session_at"))),
    ])
}

pub fn jinv_merge_snap(mut inv: JuliaDict, w: &Node) -> JuliaDict {
    inv.merge_from(&session_journal_snap(w));
    inv
}

pub fn jinv_set_w_status_with_goals(
    id: &str,
    old_w_status: &str,
    goal_statuses: JuliaDict,
    w: &Node,
) -> JuliaDict {
    let inv = JuliaDict::from_pairs(vec![
        ("op".to_string(), s("set_w_status_with_goals")),
        ("id".to_string(), s(id)),
        ("old_w_status".to_string(), s(old_w_status)),
        ("goal_statuses".to_string(), JVal::Obj(goal_statuses)),
    ]);
    jinv_merge_snap(inv, w)
}

pub fn jinv_session_restore_claim(id: &str, w: &Node) -> JuliaDict {
    let base = JuliaDict::from_pairs(vec![
        ("op".to_string(), s("session_restore_claim")),
        ("id".to_string(), s(id)),
    ]);
    let mut merged = JuliaDict::slot_copy(&base);
    merged.merge_from(&session_journal_snap(w));
    merged
}

pub fn goal_statuses_jdict(st: &State, w: &Node) -> JuliaDict {
    let mut gs = JuliaDict::new();
    for gid in w.lines("goals") {
        let Some(g) = st.nodes.get(&gid) else {
            continue;
        };
        gs.insert(gid, s(&g.status));
    }
    gs
}

pub fn fitness_map_jval(map: &BTreeMap<String, i64>) -> JVal {
    let mut src = JuliaDict::new();
    for k in map.keys() {
        src.insert(k.clone(), JVal::Null);
    }
    let mut out = JuliaDict::with_sizehint(map.len());
    for (k, _) in src.iter_pairs() {
        let v = map.get(k).copied().unwrap_or(0);
        out.insert(k.clone(), JVal::Int(v));
    }
    JVal::Obj(out)
}

pub fn jinv_field_pop_last(id: &str, field: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("field_pop_last")),
        ("id".to_string(), s(id)),
        ("field".to_string(), s(field)),
    ])
}

pub fn jinv_field_restore_lines(id: &str, field: &str, lines: &[String]) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("field_restore_lines")),
        ("id".to_string(), s(id)),
        ("field".to_string(), s(field)),
        (
            "lines".to_string(),
            JVal::Arr(lines.iter().map(|l| s(l)).collect()),
        ),
    ])
}

pub fn jinv_field_restore_fitness(
    id: &str,
    field: &str,
    map: &BTreeMap<String, i64>,
) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("field_restore_fitness")),
        ("id".to_string(), s(id)),
        ("field".to_string(), s(field)),
        ("map".to_string(), fitness_map_jval(map)),
    ])
}

pub fn jinv_field_restore_single(id: &str, field: &str, value: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("field_restore_single")),
        ("id".to_string(), s(id)),
        ("field".to_string(), s(field)),
        ("value".to_string(), s(value)),
    ])
}

pub fn jinv_field_insert_line(id: &str, field: &str, index: i64, line: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("field_insert_line")),
        ("id".to_string(), s(id)),
        ("field".to_string(), s(field)),
        ("index".to_string(), JVal::Int(index)),
        ("line".to_string(), s(line)),
    ])
}

pub fn jinv_set_simple_old(op: &str, id: &str, old: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s(op)),
        ("id".to_string(), s(id)),
        ("old".to_string(), s(old)),
    ])
}

pub fn jinv_set_g_attr_fitness_kind(
    id: &str,
    had_before: bool,
    old: &str,
    new: &str,
) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("set_g_attr_fitness_kind")),
        ("id".to_string(), s(id)),
        ("had_before".to_string(), JVal::Bool(had_before)),
        ("old".to_string(), s(old)),
        ("new".to_string(), s(new)),
    ])
}

pub fn jinv_set_requires_coverage(id: &str, had_before: bool, old: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("set_requires_coverage")),
        ("id".to_string(), s(id)),
        ("had_before".to_string(), JVal::Bool(had_before)),
        ("old".to_string(), s(old)),
    ])
}

pub fn jinv_set_g_area(id: &str, had_before: bool, old: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("set_g_area")),
        ("id".to_string(), s(id)),
        ("had_before".to_string(), JVal::Bool(had_before)),
        ("old".to_string(), s(old)),
    ])
}

pub fn jinv_set_status_plain(id: &str, old_status: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("set_status_plain")),
        ("id".to_string(), s(id)),
        ("old_status".to_string(), s(old_status)),
    ])
}

pub fn jinv_renumber_swap(from_new: &str, to_old: &str) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("renumber_swap")),
        ("from".to_string(), s(from_new)),
        ("to".to_string(), s(to_old)),
    ])
}

pub fn jinv_glossary_rename_restore(
    tags: JuliaDict,
    old: &str,
    new: &str,
    glossary_changed: bool,
) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("glossary_rename_restore")),
        ("tags".to_string(), JVal::Obj(tags)),
        ("old".to_string(), s(old)),
        ("new".to_string(), s(new)),
        (
            "glossary_changed".to_string(),
            JVal::Bool(glossary_changed),
        ),
    ])
}

pub fn jinv_dor_reject(id: &str, missing: &[String]) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("dor_reject")),
        ("id".to_string(), s(id)),
        (
            "missing".to_string(),
            JVal::Arr(missing.iter().map(|m| s(m)).collect()),
        ),
    ])
}

pub fn jinv_undo(steps: i64) -> JuliaDict {
    JuliaDict::from_pairs(vec![
        ("op".to_string(), s("undo")),
        ("steps".to_string(), JVal::Int(steps)),
    ])
}

pub fn append_journal_record(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

pub fn journal_read_nonempty_pairs(path: &Path) -> (Vec<String>, Vec<Json>) {
    let mut rawlines = Vec::new();
    let mut recs = Vec::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return (rawlines, recs);
    };
    for line in text.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        rawlines.push(s.to_string());
        if let Ok(rec) = parse_json(s) {
            recs.push(rec);
        } else {
            recs.push(Json::Null);
        }
    }
    (rawlines, recs)
}

pub fn journal_record_mutation(rec: &Json) -> bool {
    match rec.get("inv") {
        Some(inv @ Json::Obj(_)) => {
            let op = inv.get("op").and_then(|o| o.as_str()).unwrap_or("");
            !JOURNAL_NONMUTATION_OPS.contains(&op)
        }
        _ => true,
    }
}

pub fn journal_tail_mutation_view(recs: &[Json], n: usize) -> Option<Vec<usize>> {
    if n == 0 {
        return None;
    }
    let mut idxs = Vec::new();
    for i in (0..recs.len()).rev() {
        if !journal_record_mutation(&recs[i]) {
            continue;
        }
        idxs.push(i);
        if idxs.len() == n {
            break;
        }
    }
    if idxs.len() < n {
        return None;
    }
    idxs.reverse();
    Some(idxs)
}

pub fn journal_drop_lines_inplace(path: &Path, idxs: &[usize]) -> std::io::Result<()> {
    if idxs.is_empty() {
        return Ok(());
    }
    let (rawlines, _) = journal_read_nonempty_pairs(path);
    let drop: BTreeSet<usize> = idxs.iter().copied().collect();
    let keep: Vec<String> = rawlines
        .iter()
        .enumerate()
        .filter(|(i, _)| !drop.contains(i))
        .map(|(_, l)| l.clone())
        .collect();
    if keep.is_empty() {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        std::fs::write(path, keep.join("\n") + "\n")
    }
}

fn inv_str<'a>(inv: &'a Json, key: &str) -> Option<&'a str> {
    inv.get(key).and_then(|v| v.as_str())
}

fn restore_session_attrs_if_present(w: &mut Node, inv: &Json) {
    let Some(had) = inv.get("had_session_before") else {
        return;
    };
    if had.as_bool().unwrap_or(false) {
        let v = inv_str(inv, "old_session").unwrap_or("").to_string();
        w.attrs.insert("session".to_string(), v);
    } else {
        w.attrs.remove("session");
    }
    if let Some(had_at) = inv.get("had_session_at_before") {
        if had_at.as_bool().unwrap_or(false) {
            let v = inv_str(inv, "old_session_at").unwrap_or("").to_string();
            w.attrs.insert("session_at".to_string(), v);
        } else {
            w.attrs.remove("session_at");
        }
    }
}

fn fail(msg: impl Into<String>) -> Option<String> {
    Some(format!("journal undo: {}", msg.into()))
}

macro_rules! req {
    ($e:expr, $key:expr) => {
        match $e {
            Some(v) => v,
            None => return fail(format!("malformed record: missing `{}`", $key)),
        }
    };
}

fn vector_field_for_insert<'a>(n: &'a mut Node, field: &str) -> Option<&'a mut Vec<String>> {
    let form = field_form(n.kind, field);
    let make_prose = form != Some(Form::RefList);
    if !n.fields.contains_key(field) {
        let fv = if make_prose {
            FieldValue::Prose(Vec::new())
        } else {
            FieldValue::RefList(Vec::new())
        };
        n.fields.insert(field.to_string(), fv);
    }
    match n.fields.get_mut(field) {
        Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => Some(v),
        _ => None,
    }
}

pub fn journal_apply_inverse(st: &mut State, inv: &Json) -> Option<String> {
    let op = inv_str(inv, "op").unwrap_or("");
    match op {
        "rm_node" => {
            let id = req!(inv_str(inv, "id"), "id");
            if !st.nodes.contains_key(id) {
                return None;
            }
            st.nodes.remove(id);
            st.edges.retain(|e| e.from != id && e.to != id);
            reconcile_counters(st);
            None
        }
        "unlink_edge" => {
            let from = req!(inv_str(inv, "from"), "from");
            let label = req!(inv_str(inv, "label"), "label");
            let to = req!(inv_str(inv, "to"), "to");
            let n0 = st.edges.len();
            st.edges
                .retain(|e| !(e.from == from && e.label == label && e.to == to));
            if st.edges.len() == n0 {
                return fail(format!("unlink_edge: missing edge {from} {label} {to}"));
            }
            if let Some(n) = st.nodes.get_mut(from) {
                stamp_touch_node(n);
            }
            if let Some(n) = st.nodes.get_mut(to) {
                stamp_touch_node(n);
            }
            None
        }
        "restore_edge" => {
            let from = req!(inv_str(inv, "from"), "from");
            let label = req!(inv_str(inv, "label"), "label");
            let to = req!(inv_str(inv, "to"), "to");
            if st
                .edges
                .iter()
                .any(|e| e.from == from && e.label == label && e.to == to)
            {
                return None;
            }
            if let Some(r) = validate_and_push_edge(st, from, label, to, true) {
                return fail(r);
            }
            let tc = inv_str(inv, "t_created").unwrap_or("").trim().to_string();
            let Some(ee) = st
                .edges
                .iter_mut()
                .rev()
                .find(|e| e.from == from && e.label == label && e.to == to)
            else {
                return fail("restore_edge: edge missing after validate");
            };
            ee.t_created = if tc.is_empty() { None } else { Some(tc) };
            None
        }
        "set_cynefin" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.cynefin = match inv.get("old") {
                Some(Json::Str(o)) if !o.is_empty() => Some(o.clone()),
                _ => None,
            };
            stamp_touch_node(n);
            None
        }
        "set_type" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.wtype = match inv.get("old") {
                Some(Json::Str(o)) if !o.is_empty() => Some(o.clone()),
                _ => None,
            };
            stamp_touch_node(n);
            None
        }
        "set_title" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.title = inv_str(inv, "old").unwrap_or("").to_string();
            stamp_touch_node(n);
            None
        }
        "set_g_attr_fitness" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let old = inv_str(inv, "old").unwrap_or("").to_string();
            n.attrs.insert("fitness".to_string(), old);
            stamp_touch_node(n);
            None
        }
        "set_g_attr_fitness_kind" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let had = inv
                .get("had_before")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if had {
                let old = inv_str(inv, "old").unwrap_or("").to_string();
                n.attrs.insert("fitness_kind".to_string(), old);
            } else {
                n.attrs.remove("fitness_kind");
            }
            stamp_touch_node(n);
            None
        }
        "set_requires_coverage" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let had = inv
                .get("had_before")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if had {
                let old = inv_str(inv, "old").unwrap_or("").to_string();
                n.attrs.insert("requires_coverage".to_string(), old);
            } else {
                n.attrs.remove("requires_coverage");
            }
            stamp_touch_node(n);
            None
        }
        "set_g_area" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let had = inv
                .get("had_before")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if had {
                let old = inv_str(inv, "old").unwrap_or("").to_string();
                n.fields
                    .insert("area".to_string(), FieldValue::Single(old));
            } else {
                n.fields.remove("area");
            }
            stamp_touch_node(n);
            None
        }
        "set_status_plain" => {
            let id = req!(inv_str(inv, "id"), "id");
            let old_status = req!(inv_str(inv, "old_status"), "old_status");
            let is_w = st.nodes.get(id).map(|n| n.kind) == Some(crate::model::Kind::W);
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.status = old_status.to_string();
            stamp_touch_node(n);
            if is_w {
                rederive_goals(st, id);
            }
            None
        }
        "set_w_status_with_goals" => {
            let Some(Json::Obj(gs)) = inv.get("goal_statuses") else {
                return fail("missing goal_statuses");
            };
            for (gid, sv) in gs {
                let status = sv.as_str().unwrap_or("");
                let Some(g) = st.nodes.get_mut(gid) else {
                    return fail(format!("goal node missing {gid}"));
                };
                g.status = status.to_string();
                stamp_touch_node(g);
            }
            let id = req!(inv_str(inv, "id"), "id");
            let old_w_status = req!(inv_str(inv, "old_w_status"), "old_w_status");
            let Some(w) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            w.status = old_w_status.to_string();
            restore_session_attrs_if_present(w, inv);
            stamp_touch_node(w);
            rederive_goals(st, id);
            None
        }
        "session_restore_claim" => {
            let id = req!(inv_str(inv, "id"), "id");
            let Some(w) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            restore_session_attrs_if_present(w, inv);
            stamp_touch_node(w);
            None
        }
        "field_pop_last" => {
            let id = req!(inv_str(inv, "id"), "id");
            let field = req!(inv_str(inv, "field"), "field");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let empty = match n.fields.get_mut(field) {
                Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => {
                    if v.is_empty() {
                        true
                    } else {
                        v.pop();
                        false
                    }
                }
                _ => true,
            };
            if empty {
                return fail(format!("field_pop_last empty {field}"));
            }
            stamp_touch_node(n);
            None
        }
        "field_restore_lines" => {
            let id = req!(inv_str(inv, "id"), "id");
            let field = req!(inv_str(inv, "field"), "field");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let form = field_form(n.kind, field);
            let lines: Vec<String> = inv
                .get("lines")
                .and_then(|v| v.as_arr())
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default();
            match form {
                Some(Form::Prose) => {
                    n.fields.insert(field.to_string(), FieldValue::Prose(lines));
                }
                Some(Form::RefList) => {
                    n.fields
                        .insert(field.to_string(), FieldValue::RefList(lines));
                }
                _ => return fail("field_restore_lines wrong form"),
            }
            let n = st.nodes.get_mut(id).expect("present");
            stamp_touch_node(n);
            None
        }
        "field_restore_fitness" => {
            let id = req!(inv_str(inv, "id"), "id");
            let field = req!(inv_str(inv, "field"), "field");
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let mut d = BTreeMap::new();
            if let Some(Json::Obj(entries)) = inv.get("map") {
                for (k, vv) in entries {
                    if let Some(v) = vv.as_i64() {
                        d.insert(k.clone(), v);
                    }
                }
            }
            n.fields.insert(field.to_string(), FieldValue::Fitness(d));
            stamp_touch_node(n);
            None
        }
        "field_restore_single" => {
            let id = req!(inv_str(inv, "id"), "id");
            let field = req!(inv_str(inv, "field"), "field");
            let value = inv_str(inv, "value").unwrap_or("").to_string();
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.fields.insert(field.to_string(), FieldValue::Single(value));
            stamp_touch_node(n);
            None
        }
        "field_insert_line" => {
            let id = req!(inv_str(inv, "id"), "id");
            let field = req!(inv_str(inv, "field"), "field");
            let idx = req!(inv.get("index").and_then(|v| v.as_i64()), "index");
            let line = req!(inv_str(inv, "line"), "line").to_string();
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            let Some(v) = vector_field_for_insert(n, field) else {
                return fail(format!("field_insert_line bad field {field}"));
            };
            if idx < 1 || idx > v.len() as i64 + 1 {
                return fail(format!("field_insert_line bad index {idx}"));
            }
            v.insert((idx - 1) as usize, line);
            stamp_touch_node(n);
            None
        }
        "restore_fitness_key" => {
            let wid = req!(inv_str(inv, "wid"), "wid");
            let gid = req!(inv_str(inv, "gid"), "gid");
            let had_key = inv
                .get("had_key")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let Some(w) = st.nodes.get_mut(wid) else {
                return fail(format!("missing node {wid}"));
            };
            if !matches!(w.fields.get("fitness"), Some(FieldValue::Fitness(_))) {
                w.fields
                    .insert("fitness".to_string(), FieldValue::Fitness(BTreeMap::new()));
            }
            let Some(FieldValue::Fitness(fid)) = w.fields.get_mut("fitness") else {
                unreachable!()
            };
            if had_key {
                let Some(prev) = inv.get("previous").and_then(|v| v.as_i64()) else {
                    return fail("restore_fitness_key missing previous");
                };
                fid.insert(gid.to_string(), prev);
            } else {
                fid.remove(gid);
            }
            stamp_touch_node(w);
            None
        }
        "renumber_swap" => {
            let from = req!(inv_str(inv, "from"), "from");
            let to = req!(inv_str(inv, "to"), "to");
            match apply_renumber(st, from, to) {
                Ok(()) => None,
                Err(e) => fail(e),
            }
        }
        "revalidate_restore" => {
            let id = req!(inv_str(inv, "id"), "id");
            let old_status = req!(inv_str(inv, "old_status"), "old_status");
            let had_surface = inv
                .get("had_surface")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let old_surface: Vec<String> = inv
                .get("old_surface")
                .and_then(|v| v.as_arr())
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().unwrap_or("").to_string())
                        .collect()
                })
                .unwrap_or_default();
            let added: Vec<(String, String, String)> = inv
                .get("added_edges")
                .and_then(|v| v.as_arr())
                .map(|a| {
                    a.iter()
                        .filter_map(|e| {
                            Some((
                                inv_str(e, "from")?.to_string(),
                                inv_str(e, "label")?.to_string(),
                                inv_str(e, "to")?.to_string(),
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let Some(n) = st.nodes.get_mut(id) else {
                return fail(format!("missing node {id}"));
            };
            n.status = old_status.to_string();
            if had_surface {
                n.fields
                    .insert("surface".to_string(), FieldValue::RefList(old_surface));
            } else {
                n.fields.remove("surface");
            }
            if let Some(FieldValue::Prose(rv)) = n.fields.get_mut("revalidation") {
                if !rv.is_empty() {
                    rv.pop();
                }
            }
            for (f0, l0, t0) in added {
                st.edges
                    .retain(|e| !(e.from == f0 && e.label == l0 && e.to == t0));
            }
            stamp_touch_node(n);
            None
        }
        "glossary_rename_restore" => {
            let Some(Json::Obj(tags)) = inv.get("tags") else {
                return fail("glossary_rename_restore missing tags");
            };
            let updates: Vec<(String, Vec<String>)> = tags
                .iter()
                .map(|(id, lines)| {
                    let lines = lines
                        .as_arr()
                        .map(|a| {
                            a.iter()
                                .map(|x| x.as_str().unwrap_or("").to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    (id.clone(), lines)
                })
                .collect();
            for (id, lines) in updates {
                let Some(n) = st.nodes.get_mut(&id) else {
                    continue;
                };
                n.fields
                    .insert("tags".to_string(), FieldValue::RefList(lines));
                stamp_touch_node(n);
            }
            None
        }
        other => fail(format!("unknown inverse op `{other}`")),
    }
}
