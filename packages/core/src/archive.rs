use crate::cli::{eff_token, journal_session_token, load, persist, CliCtx};
use crate::journal::{
    journal_read_nonempty_pairs, stamp_journal_session, wrap_journal_record, JOURNAL_ARCHIVE_OP,
    JOURNAL_DISTILL_OP,
};
use crate::json::{JVal, JuliaDict, Json};
use crate::model::{Kind, State};
use crate::ops::{OpResult, EXIT_ERR, EXIT_GUARD, EXIT_NOTFOUND};
use crate::session::session_denial_progress_mutate;
use crate::status::listnodes;
use crate::times::stamp_touch_node;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn merge_goal_refs(
    refs: &mut BTreeMap<String, BTreeSet<String>>,
    to: &str,
    fr: &str,
    changed: &mut bool,
) {
    let add: Vec<String> = match refs.get(fr) {
        Some(s) => s.iter().cloned().collect(),
        None => Vec::new(),
    };
    let entry = refs.entry(to.to_string()).or_insert_with(BTreeSet::new);
    for g in add {
        if entry.insert(g) {
            *changed = true;
        }
    }
}

pub fn goal_reference_sets(st: &State) -> BTreeMap<String, BTreeSet<String>> {
    let mut refs = BTreeMap::new();
    for id in st.nodes.keys() {
        refs.insert(id.clone(), BTreeSet::new());
    }
    for g in listnodes(st, Kind::G, true) {
        refs.entry(g.id.clone()).or_insert_with(BTreeSet::new).insert(g.id.clone());
    }
    for w in listnodes(st, Kind::W, true) {
        for gg in w.lines("goals") {
            refs.entry(w.id.clone()).or_insert_with(BTreeSet::new).insert(gg);
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for e in &st.edges {
            let Some(fk) = st.nodes.get(&e.from) else {
                continue;
            };
            let Some(tk) = st.nodes.get(&e.to) else {
                continue;
            };
            if e.label == "implements" && fk.kind == Kind::W && tk.kind == Kind::D {
                merge_goal_refs(&mut refs, &e.to, &e.from, &mut changed);
            } else if e.label == "produces" && fk.kind == Kind::W {
                if matches!(tk.kind, Kind::D | Kind::Q | Kind::B) {
                    merge_goal_refs(&mut refs, &e.to, &e.from, &mut changed);
                }
            } else if e.label == "asks" && fk.kind == Kind::Q && tk.kind == Kind::W {
                merge_goal_refs(&mut refs, &e.from, &e.to, &mut changed);
            } else if e.label == "tests" && fk.kind == Kind::B && tk.kind == Kind::Q {
                merge_goal_refs(&mut refs, &e.from, &e.to, &mut changed);
            } else if e.label == "targets" && fk.kind == Kind::B && tk.kind == Kind::W {
                merge_goal_refs(&mut refs, &e.from, &e.to, &mut changed);
            } else if e.label == "causes" && fk.kind == Kind::T && tk.kind == Kind::W {
                merge_goal_refs(&mut refs, &e.from, &e.to, &mut changed);
            } else if e.label == "supersedes" && fk.kind == Kind::D && tk.kind == Kind::D {
                merge_goal_refs(&mut refs, &e.from, &e.to, &mut changed);
                merge_goal_refs(&mut refs, &e.to, &e.from, &mut changed);
            }
        }
        for w in listnodes(st, Kind::W, true) {
            let tid = w.single("theme").trim().to_string();
            if tid.is_empty() {
                continue;
            }
            if !st.nodes.contains_key(&tid) {
                continue;
            }
            merge_goal_refs(&mut refs, &tid, &w.id, &mut changed);
        }
    }
    refs
}

fn exclusive_want(
    st: &State,
    refs: &BTreeMap<String, BTreeSet<String>>,
    gid: &str,
) -> BTreeSet<String> {
    let mut want = BTreeSet::new();
    let gset: BTreeSet<String> = [gid.to_string()].into_iter().collect();
    for (id, rs) in refs {
        let Some(n) = st.nodes.get(id) else {
            continue;
        };
        if n.archived {
            continue;
        }
        if *rs != gset {
            continue;
        }
        want.insert(id.clone());
    }
    want
}

fn affinity_neighbors(st: &State, u: &str, want: &BTreeSet<String>, gid: &str) -> Vec<String> {
    let mut out = BTreeSet::new();
    if u == gid {
        for w in listnodes(st, Kind::W, false) {
            if w.archived || !want.contains(&w.id) {
                continue;
            }
            if !w.lines("goals").iter().any(|g| g == gid) {
                continue;
            }
            out.insert(w.id.clone());
        }
    }
    if let Some(un) = st.nodes.get(u) {
        if un.kind == Kind::W
            && !un.archived
            && un.lines("goals").iter().any(|g| g == gid)
            && want.contains(gid)
        {
            out.insert(gid.to_string());
        }
    }
    for e in &st.edges {
        let other = if e.from == u {
            e.to.as_str()
        } else if e.to == u {
            e.from.as_str()
        } else {
            continue;
        };
        if !want.contains(other) {
            continue;
        }
        out.insert(other.to_string());
    }
    out.into_iter().collect()
}

pub fn exclusive_archive_ids(st: &State, gid: &str) -> BTreeSet<String> {
    let refs = goal_reference_sets(st);
    let want = exclusive_want(st, &refs, gid);
    if !want.contains(gid) {
        return BTreeSet::new();
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![gid.to_string()];
    while let Some(u) = stack.pop() {
        if seen.contains(&u) {
            continue;
        }
        if !want.contains(&u) {
            continue;
        }
        seen.insert(u.clone());
        for v in affinity_neighbors(st, &u, &want, gid) {
            if !seen.contains(&v) {
                stack.push(v);
            }
        }
    }
    seen
}

pub fn distill_linked_da_ids(st: &State, mass: &BTreeSet<String>) -> Vec<String> {
    let mut out = BTreeSet::new();
    for e in &st.edges {
        let xid = if e.label == "produces" && mass.contains(&e.from) {
            e.to.as_str()
        } else if e.label == "distills" && mass.contains(&e.to) {
            e.from.as_str()
        } else {
            continue;
        };
        let Some(n) = st.nodes.get(xid) else {
            continue;
        };
        if n.kind != Kind::Y {
            continue;
        }
        if n.status != "active" {
            continue;
        }
        out.insert(n.id.clone());
    }
    out.into_iter().collect()
}

pub fn distill_null_attested(journal_path: &Path, gid: &str) -> bool {
    let (_, recs) = journal_read_nonempty_pairs(journal_path);
    for rec in &recs {
        let Some(inv @ Json::Obj(_)) = rec.get("inv") else {
            continue;
        };
        let op = inv.get("op").and_then(|v| v.as_str()).unwrap_or("");
        if op != JOURNAL_DISTILL_OP {
            continue;
        }
        let goal = inv.get("goal").and_then(|v| v.as_str()).unwrap_or("");
        if goal != gid {
            continue;
        }
        return true;
    }
    false
}

pub fn cmd_archive(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove archive <G-NN>");
    }
    let gid = &pos[0];
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(g) = st.nodes.get(gid) else {
        return OpResult {
            code: EXIT_NOTFOUND,
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        };
    };
    if g.status != "verified" {
        return OpResult::fail(EXIT_GUARD, "goal must be verified");
    }
    let ids = exclusive_archive_ids(&st, gid);
    let distilled = !distill_linked_da_ids(&st, &ids).is_empty()
        || distill_null_attested(&ctx.journalpath(), gid);
    if !distilled {
        return OpResult::fail(
            EXIT_GUARD,
            &format!("archive: distill {gid} first (grove distill {gid}, or grove distill {gid} --null)"),
        );
    }
    let eff = eff_token(ctx, kw);
    for w in listnodes(&st, Kind::W, false) {
        if !w.lines("goals").iter().any(|g| g == gid) {
            continue;
        }
        if w.status != "progress" {
            continue;
        }
        if let Some(msg) = session_denial_progress_mutate(w, &eff) {
            return OpResult::fail(EXIT_GUARD, &msg);
        }
    }
    for id in &ids {
        let n = st.nodes.get_mut(id).expect("archive ids are nodes");
        n.archived = true;
        stamp_touch_node(n);
    }
    let line = stamp_journal_session(
        &wrap_journal_record(
            "archive",
            JuliaDict::from_pairs(vec![
                ("op".to_string(), JVal::Str(JOURNAL_ARCHIVE_OP.to_string())),
                ("id".to_string(), JVal::Str(gid.clone())),
                (
                    "ids".to_string(),
                    JVal::Arr(ids.iter().map(|i| JVal::Str(i.clone())).collect()),
                ),
            ]),
        ),
        &journal_session_token(ctx, kw),
    );
    persist(ctx, &mut st, Some(&line));
    OpResult::ok()
}
