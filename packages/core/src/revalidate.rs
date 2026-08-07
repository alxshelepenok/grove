use crate::cli::{journal_session_token, json_cli_out, load, persist, CliCtx};
use crate::invariants::validate_and_push_edge;
use crate::journal::{stamp_journal_session, wrap_journal_record};
use crate::json::{JVal, JuliaDict};
use crate::model::{FieldValue, Kind};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_GUARD, EXIT_NOTFOUND};
use crate::times::{stamp_touch_node, utc_stamp_second};
use std::path::{Path, PathBuf};

pub fn cmd_revalidate(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(
            EXIT_ERR,
            "usage: grove revalidate <Y-NN> [--surface=p1,p2] [--from=ID,...]",
        );
    }
    let id = &pos[0];
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(n) = st.nodes.get(id) else {
        return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {id}"));
    };
    if n.kind != Kind::Y {
        return OpResult::fail(EXIT_ERR, &format!("revalidate: {id} is not a discovery"));
    }
    if n.status != "stale" {
        return OpResult::fail(
            EXIT_GUARD,
            &format!("revalidate: {id} is `{}`, not `stale`", n.status),
        );
    }
    let has_surface = kw_get(kw, "surface").is_some();
    let has_from = kw_get(kw, "from").is_some();
    if !has_surface && !has_from {
        return OpResult::fail(
            EXIT_GUARD,
            "revalidate: refusing without payment; pass --surface=<paths> and/or --from=<ID>",
        );
    }
    let surface: Vec<String> = kw_get(kw, "surface")
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    let froms: Vec<String> = kw_get(kw, "from")
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    if has_surface && surface.is_empty() {
        return OpResult::fail(EXIT_ERR, "revalidate: --surface given but empty");
    }
    if has_from && froms.is_empty() {
        return OpResult::fail(EXIT_ERR, "revalidate: --from given but empty");
    }
    for p in &surface {
        let pth: PathBuf = if Path::new(p).is_absolute() {
            PathBuf::from(p)
        } else {
            Path::new(&ctx.root).join(p)
        };
        if !pth.exists() {
            return OpResult::fail(
                EXIT_GUARD,
                &format!("revalidate: surface path does not exist under root: {p}"),
            );
        }
    }
    for oid in &froms {
        let Some(src) = st.nodes.get(oid) else {
            return OpResult::fail(EXIT_GUARD, &format!("revalidate: unknown --from id: {oid}"));
        };
        if !matches!(src.kind, Kind::W | Kind::D | Kind::Q | Kind::B) {
            return OpResult::fail(
                EXIT_GUARD,
                &format!("revalidate: --from {oid} must reference W or D/Q/B"),
            );
        }
        if src.kind == Kind::D && src.status == "superseded" {
            return OpResult::fail(EXIT_GUARD, &format!("revalidate: --from {oid} is superseded"));
        }
        if src.kind == Kind::B
            && (src.status == "invalidated_acceptable" || src.status == "invalidated_blocking")
        {
            return OpResult::fail(EXIT_GUARD, &format!("revalidate: --from {oid} is invalidated"));
        }
    }
    let mut paid: Vec<String> = Vec::new();
    if has_surface {
        paid.push(format!("surface={}", surface.join(",")));
    }
    if has_from {
        paid.push(format!("from={}", froms.join(",")));
    }
    let line = format!("{} {}", utc_stamp_second(), paid.join(" "));
    let mut added: Vec<JVal> = Vec::new();
    for oid in &froms {
        let src_kind = st.nodes.get(oid).expect("from ids checked").kind;
        let (f0, l0, t0) = if src_kind == Kind::W {
            (oid.clone(), "produces".to_string(), id.clone())
        } else {
            (id.clone(), "distills".to_string(), oid.clone())
        };
        let already = st
            .edges
            .iter()
            .any(|e| e.from == f0 && e.label == l0 && e.to == t0);
        if let Some(msg) = validate_and_push_edge(&mut st, &f0, &l0, &t0, true) {
            return OpResult::fail(EXIT_GUARD, &msg);
        }
        if !already {
            added.push(JVal::Obj(JuliaDict::from_pairs(vec![
                ("from".to_string(), JVal::Str(f0)),
                ("label".to_string(), JVal::Str(l0)),
                ("to".to_string(), JVal::Str(t0)),
            ])));
        }
    }
    let jr = {
        let n = st.nodes.get(id).expect("checked");
        let old_status = n.status.clone();
        let had_surface = n.fields.contains_key("surface");
        let old_surface = n.lines("surface");
        wrap_journal_record(
            "revalidate",
            JuliaDict::from_pairs(vec![
                ("op".to_string(), JVal::Str("revalidate_restore".to_string())),
                ("id".to_string(), JVal::Str(id.clone())),
                ("old_status".to_string(), JVal::Str(old_status)),
                ("had_surface".to_string(), JVal::Bool(had_surface)),
                (
                    "old_surface".to_string(),
                    JVal::Arr(old_surface.iter().map(|s| JVal::Str(s.clone())).collect()),
                ),
                ("added_edges".to_string(), JVal::Arr(added)),
            ]),
        )
    };
    {
        let n = st.nodes.get_mut(id).expect("checked");
        n.status = "active".to_string();
        if has_surface {
            n.fields
                .insert("surface".to_string(), FieldValue::RefList(surface.clone()));
        }
        if !matches!(n.fields.get("revalidation"), Some(FieldValue::Prose(_))) {
            n.fields
                .insert("revalidation".to_string(), FieldValue::Prose(Vec::new()));
        }
        if let Some(FieldValue::Prose(rv)) = n.fields.get_mut("revalidation") {
            rv.push(line.clone());
        }
        stamp_touch_node(n);
    }
    persist(ctx, &mut st, Some(&stamp_journal_session(&jr, &journal_session_token(ctx, kw))));
    let mut r = OpResult::ok();
    if ctx.json {
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("revalidate".to_string())),
            ("id".to_string(), JVal::Str(id.clone())),
            ("status".to_string(), JVal::Str("active".to_string())),
            ("line".to_string(), JVal::Str(line)),
        ]));
    }
    r
}
