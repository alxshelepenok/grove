use crate::cli::{abspath, json_cli_out, CliCtx};
use crate::gitutil::{git_repository_root, git_show_path, read_worktree_lock_text};
use crate::json::{JVal, JuliaDict};
use crate::model::{field_form, field_order, FieldValue, Form, Kind, Node, State};
use crate::ops::{kw_get, OpResult, EXIT_ERR};
use crate::parse::parse_strict;
use crate::status::listnodes;
use std::collections::{BTreeMap, BTreeSet};

type EdgeKey = (String, String, String);

fn edge_multiset_counts(st: &State) -> BTreeMap<EdgeKey, i64> {
    let mut d: BTreeMap<EdgeKey, i64> = BTreeMap::new();
    for e in &st.edges {
        *d.entry((e.from.clone(), e.label.clone(), e.to.clone()))
            .or_insert(0) += 1;
    }
    d
}

fn multiset_diff(
    a: &BTreeMap<EdgeKey, i64>,
    b: &BTreeMap<EdgeKey, i64>,
) -> Vec<(i32, EdgeKey)> {
    let mut outs: Vec<(i32, EdgeKey)> = Vec::new();
    let keys: BTreeSet<&EdgeKey> = a.keys().chain(b.keys()).collect();
    for k in keys {
        let da = a.get(k).copied().unwrap_or(0);
        let db = b.get(k).copied().unwrap_or(0);
        if da == db {
            continue;
        }
        if da > db {
            for _ in 0..(da - db) {
                outs.push((-1, k.clone()));
            }
        } else {
            for _ in 0..(db - da) {
                outs.push((1, k.clone()));
            }
        }
    }
    outs.sort();
    outs
}

fn field_lines(n: &Node, fname: &str) -> Vec<String> {
    match n.fields.get(fname) {
        Some(FieldValue::Prose(v)) | Some(FieldValue::RefList(v)) => v.clone(),
        _ => Vec::new(),
    }
}

fn field_single(n: &Node, fname: &str) -> String {
    match n.fields.get(fname) {
        Some(FieldValue::Single(s)) => s.clone(),
        _ => String::new(),
    }
}

fn field_fitness(n: &Node, fname: &str) -> BTreeMap<String, i64> {
    match n.fields.get(fname) {
        Some(FieldValue::Fitness(m)) => m.clone(),
        _ => BTreeMap::new(),
    }
}

fn field_semantically_equal(kind: Kind, fname: &str, a: &Node, b: &Node) -> bool {
    match field_form(kind, fname) {
        Some(Form::Prose) | Some(Form::RefList) => {
            let mut va = field_lines(a, fname);
            let mut vb = field_lines(b, fname);
            if va.is_empty() && vb.is_empty() {
                return true;
            }
            va.sort();
            vb.sort();
            va == vb
        }
        Some(Form::Single) => field_single(a, fname) == field_single(b, fname),
        Some(Form::Fitness) => field_fitness(a, fname) == field_fitness(b, fname),
        None => false,
    }
}

fn node_semantically_equal(a: &Node, b: &Node) -> bool {
    a.kind == b.kind
        && a.title == b.title
        && a.wtype == b.wtype
        && a.status == b.status
        && a.cynefin == b.cynefin
        && a.archived == b.archived
        && a.attrs == b.attrs
        && field_order(a.kind)
            .iter()
            .all(|fname| field_semantically_equal(a.kind, fname, a, b))
}

fn fmt_fitness_dict(d: &BTreeMap<String, i64>) -> String {
    if d.is_empty() {
        return String::new();
    }
    d.iter()
        .map(|(k, v)| format!("{k}={}{v}", if *v >= 0 { "+" } else { "" }))
        .collect::<Vec<_>>()
        .join(", ")
}

fn fmt_single_field(kind: Kind, fname: &str, n: &Node) -> String {
    match field_form(kind, fname) {
        Some(Form::Prose) => {
            let lines = field_lines(n, fname);
            if lines.is_empty() {
                "(empty)".to_string()
            } else {
                format!("{} prose lines", lines.len())
            }
        }
        Some(Form::RefList) => {
            let mut xs = field_lines(n, fname);
            if xs.is_empty() {
                "(empty)".to_string()
            } else {
                xs.sort();
                xs.join(",")
            }
        }
        Some(Form::Single) => field_single(n, fname),
        Some(Form::Fitness) => fmt_fitness_dict(&field_fitness(n, fname)),
        None => String::new(),
    }
}

fn node_field_snap(kind: Kind, n: &Node, fname: &str) -> String {
    if !n.fields.contains_key(fname) {
        return match field_form(kind, fname) {
            Some(Form::Prose) | Some(Form::RefList) => "(empty)".to_string(),
            _ => String::new(),
        };
    }
    fmt_single_field(kind, fname, n)
}

fn discrete_header_line(n: &Node) -> String {
    let mut parts = vec![format!("{} {}", n.kind.as_str(), n.id)];
    match n.kind {
        Kind::W => {
            parts.push(n.status.clone());
            if let Some(t) = &n.wtype {
                parts.push(t.clone());
            }
        }
        Kind::G | Kind::D | Kind::T | Kind::Y => parts.push(n.status.clone()),
        Kind::Q | Kind::B => {
            parts.push(n.status.clone());
            if let Some(c) = &n.cynefin {
                parts.push(c.clone());
            }
        }
        Kind::A => {}
    }
    parts.join(" ")
}

fn julia_repr_str(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::from("\"");
    for (i, &c) in chars.iter().enumerate() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            '\0' => {
                if chars.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                    out.push_str("\\x00");
                } else {
                    out.push_str("\\0");
                }
            }
            '\u{07}' => out.push_str("\\a"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0b}' => out.push_str("\\v"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            '\u{1b}' => out.push_str("\\e"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn describe_node_changes(a: &Node, b: &Node) -> Vec<String> {
    let mut out = Vec::new();
    let na = discrete_header_line(a);
    let nb = discrete_header_line(b);
    if na != nb {
        out.push(format!("  header: {na} -> {nb}"));
    }
    if a.attrs != b.attrs {
        out.push("  attrs: changed".to_string());
    }
    for fname in field_order(a.kind) {
        if field_semantically_equal(a.kind, fname, a, b) {
            continue;
        }
        let sa = node_field_snap(a.kind, a, fname);
        let sb = node_field_snap(b.kind, b, fname);
        out.push(format!(
            "  {fname}: {} -> {}",
            julia_repr_str(&sa),
            julia_repr_str(&sb)
        ));
    }
    out
}

fn kind_ids(st: &State, kind: Kind) -> BTreeSet<String> {
    listnodes(st, kind, false)
        .iter()
        .map(|n| n.id.clone())
        .collect()
}

fn changed_pair<'a>(ref_st: &'a State, wt_st: &'a State, cid: &str) -> Option<(&'a Node, &'a Node)> {
    let nr = &ref_st.nodes[cid];
    let nw = &wt_st.nodes[cid];
    if nr.kind == nw.kind && node_semantically_equal(nr, nw) {
        None
    } else {
        Some((nr, nw))
    }
}

fn lock_structural_lines(ref_st: &State, wt_st: &State) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for kind in Kind::ALL {
        let ids_ref = kind_ids(ref_st, kind);
        let ids_wt = kind_ids(wt_st, kind);
        let added: Vec<String> = ids_wt.difference(&ids_ref).cloned().collect();
        let removed: Vec<String> = ids_ref.difference(&ids_wt).cloned().collect();
        let common: Vec<String> = ids_ref.intersection(&ids_wt).cloned().collect();
        let chlines: Vec<(&Node, &Node)> = common
            .iter()
            .filter_map(|cid| changed_pair(ref_st, wt_st, cid))
            .collect();
        if added.is_empty() && removed.is_empty() && chlines.is_empty() {
            continue;
        }
        out.push(format!("## {}", kind.as_str().to_uppercase()));
        if !added.is_empty() {
            out.push("### added (+)".to_string());
            for id in &added {
                let n = &wt_st.nodes[id];
                let ttl = if n.title.is_empty() {
                    "(no title)"
                } else {
                    n.title.as_str()
                };
                out.push(format!("+ {}  {ttl}", discrete_header_line(n)));
            }
        }
        if !removed.is_empty() {
            out.push("### removed (-)".to_string());
            for id in &removed {
                let n = &ref_st.nodes[id];
                let ttl = if n.title.is_empty() {
                    "(no title)"
                } else {
                    n.title.as_str()
                };
                out.push(format!("- {}  {ttl}", discrete_header_line(n)));
            }
        }
        if !chlines.is_empty() {
            out.push("### changed (~)".to_string());
            for (nr, nw) in &chlines {
                out.push(format!("~ {}", nw.id));
                out.extend(describe_node_changes(nr, nw));
            }
        }
        out.push(String::new());
    }
    let ere = multiset_diff(&edge_multiset_counts(ref_st), &edge_multiset_counts(wt_st));
    if !ere.is_empty() {
        out.push("## EDGES".to_string());
        for (sig, (f, lbl, t)) in &ere {
            if *sig < 0 {
                out.push(format!("- e {f} {lbl} {t}"));
            } else {
                out.push(format!("+ e {f} {lbl} {t}"));
            }
        }
        out.push(String::new());
    }
    if out.is_empty() {
        out.push("(no semantic changes)".to_string());
        out.push(String::new());
    }
    out
}

pub fn print_lock_structural_diff(ref_label: &str, ref_st: &State, wt_st: &State) -> String {
    let mut out = String::from("# grove diff (ref -> worktree)\n\n");
    out.push_str(&format!("baseline: `{ref_label}`\n\n"));
    for line in lock_structural_lines(ref_st, wt_st) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn payload_title(n: &Node) -> JVal {
    JVal::Str(if n.title.is_empty() {
        "(no title)".to_string()
    } else {
        n.title.clone()
    })
}

fn payload_node_entry(n: &Node) -> JVal {
    JVal::Obj(JuliaDict::from_pairs(vec![
        ("id".to_string(), JVal::Str(n.id.clone())),
        ("header".to_string(), JVal::Str(discrete_header_line(n))),
        ("title".to_string(), payload_title(n)),
    ]))
}

pub fn lock_structural_diff_payload(ref_st: &State, wt_st: &State) -> JuliaDict {
    let mut nodes_payload = JuliaDict::new();
    for kind in Kind::ALL {
        let ids_ref = kind_ids(ref_st, kind);
        let ids_wt = kind_ids(wt_st, kind);
        let added: Vec<String> = ids_wt.difference(&ids_ref).cloned().collect();
        let removed: Vec<String> = ids_ref.difference(&ids_wt).cloned().collect();
        let common: Vec<String> = ids_ref.intersection(&ids_wt).cloned().collect();
        let chlines: Vec<(&Node, &Node)> = common
            .iter()
            .filter_map(|cid| changed_pair(ref_st, wt_st, cid))
            .collect();
        if added.is_empty() && removed.is_empty() && chlines.is_empty() {
            continue;
        }
        let mut block = JuliaDict::new();
        block.insert(
            "added".to_string(),
            JVal::Arr(
                added
                    .iter()
                    .map(|id| payload_node_entry(&wt_st.nodes[id]))
                    .collect(),
            ),
        );
        block.insert(
            "removed".to_string(),
            JVal::Arr(
                removed
                    .iter()
                    .map(|id| payload_node_entry(&ref_st.nodes[id]))
                    .collect(),
            ),
        );
        block.insert(
            "changed".to_string(),
            JVal::Arr(
                chlines
                    .iter()
                    .map(|(nr, nw)| {
                        JVal::Obj(JuliaDict::from_pairs(vec![
                            ("id".to_string(), JVal::Str(nw.id.clone())),
                            (
                                "detail_lines".to_string(),
                                JVal::Arr(
                                    describe_node_changes(nr, nw)
                                        .into_iter()
                                        .map(JVal::Str)
                                        .collect(),
                                ),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        );
        nodes_payload.insert(kind.as_str().to_string(), JVal::Obj(block));
    }
    let ere = multiset_diff(&edge_multiset_counts(ref_st), &edge_multiset_counts(wt_st));
    let mut edges_added: Vec<JVal> = Vec::new();
    let mut edges_removed: Vec<JVal> = Vec::new();
    for (sig, (f, lbl, t)) in &ere {
        let d = JVal::Obj(JuliaDict::from_pairs(vec![
            ("from".to_string(), JVal::Str(f.clone())),
            ("label".to_string(), JVal::Str(lbl.clone())),
            ("to".to_string(), JVal::Str(t.clone())),
        ]));
        if *sig < 0 {
            edges_removed.push(d);
        } else {
            edges_added.push(d);
        }
    }
    let sem = nodes_payload.len() > 0 || !edges_added.is_empty() || !edges_removed.is_empty();
    JuliaDict::from_pairs(vec![
        ("nodes".to_string(), JVal::Obj(nodes_payload)),
        (
            "edges".to_string(),
            JVal::Obj(JuliaDict::from_pairs(vec![
                ("added".to_string(), JVal::Arr(edges_added)),
                ("removed".to_string(), JVal::Arr(edges_removed)),
            ])),
        ),
        ("semantic_change".to_string(), JVal::Bool(sem)),
    ])
}

pub fn cmd_diff(_ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    let git_ref = kw_get(kw, "since").unwrap_or("HEAD");
    let rp = abspath(&_ctx.root);
    if !git_repository_root(&rp) {
        return OpResult::fail(
            EXIT_ERR,
            &format!(
                "grove diff: not a git repository (--root=`{rp}`): cannot resolve `{git_ref}:.grove/state.lock` via git"
            ),
        );
    }
    let wt_path = _ctx.lockpath();
    if !wt_path.is_file() {
        return OpResult::fail(EXIT_ERR, &format!("lock not found: {}", wt_path.display()));
    }
    let wt_text = match read_worktree_lock_text(&wt_path) {
        Ok(t) => t,
        Err(_) => {
            return OpResult::fail(EXIT_ERR, &format!("lock not found: {}", wt_path.display()))
        }
    };
    let st_wt = match parse_strict(&wt_text) {
        Ok(st) => st,
        Err(e) => return OpResult::fail(EXIT_ERR, &e.to_string()),
    };
    let blob = match git_show_path(&rp, git_ref, ".grove/state.lock") {
        Ok(b) => b,
        Err(e) => return OpResult::fail(EXIT_ERR, &format!("grove diff: {e}")),
    };
    let st_ref = match parse_strict(&blob) {
        Ok(st) => st,
        Err(e) => {
            return OpResult::fail_lines(
                EXIT_ERR,
                &[
                    e.to_string(),
                    format!(" (while parsing `{git_ref}:.grove/state.lock`)"),
                ],
            )
        }
    };
    if _ctx.json {
        let mut pl = lock_structural_diff_payload(&st_ref, &st_wt);
        pl.insert("command".to_string(), JVal::Str("diff".to_string()));
        pl.insert("since".to_string(), JVal::Str(git_ref.to_string()));
        let mut r = OpResult::ok();
        r.out = json_cli_out(pl);
        return r;
    }
    let mut r = OpResult::ok();
    r.out = print_lock_structural_diff(git_ref, &st_ref, &st_wt);
    r
}
