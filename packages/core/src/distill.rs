use crate::archive::{distill_linked_da_ids, distill_null_attested, exclusive_archive_ids, goal_reference_sets};
use crate::cli::{info, journal_session_token, json_cli_out, load, CliCtx};
use crate::journal::{append_journal_record, stamp_journal_session, wrap_journal_record, JOURNAL_DISTILL_OP};
use crate::json::{JVal, JuliaDict};
use crate::model::{Kind, State};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_GUARD, EXIT_NOTFOUND};
use std::collections::BTreeSet;

pub fn distill_skeleton(id: &str) -> String {
    format!("grove add y --from={id} --title=\"…\" --tags=<glossary-term> --surface=<path>  # xor --why=\"…\"")
}

pub fn distill_candidates(st: &State, pool: &BTreeSet<String>) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for id in pool {
        let Some(n) = st.nodes.get(id) else {
            continue;
        };
        if n.archived {
            continue;
        }
        let ok = (n.kind == Kind::B && n.status == "validated")
            || (n.kind == Kind::Q && n.status == "answered")
            || (n.kind == Kind::D && n.status == "accepted");
        if !ok {
            continue;
        }
        out.push((id.clone(), n.kind.as_str().to_string(), n.title.clone()));
    }
    out
}

pub fn cmd_distill(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove distill <G-NN> [--null]");
    }
    let gid = &pos[0];
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(g) = st.nodes.get(gid) else {
        return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {gid}"));
    };
    if g.kind != Kind::G {
        return OpResult::fail(EXIT_ERR, &format!("distill: {gid} is not a goal"));
    }
    if g.status != "verified" {
        return OpResult::fail(
            EXIT_GUARD,
            &format!(
                "distill: {gid} is `{}`; distillation happens at `verified`",
                g.status
            ),
        );
    }
    let mass = exclusive_archive_ids(&st, gid);
    let linked = distill_linked_da_ids(&st, &mass);
    let attested = distill_null_attested(&ctx.journalpath(), gid);
    if kw_get(kw, "null").is_some() {
        let jr = wrap_journal_record(
            "distill",
            JuliaDict::from_pairs(vec![
                ("op".to_string(), JVal::Str(JOURNAL_DISTILL_OP.to_string())),
                ("goal".to_string(), JVal::Str(gid.clone())),
                ("empty".to_string(), JVal::Bool(true)),
            ]),
        );
        let _ = append_journal_record(
            &ctx.journalpath(),
            &stamp_journal_session(&jr, &journal_session_token(ctx, kw)),
        );
        let mut r = OpResult::ok();
        info(ctx, &mut r, &format!("null-distill attested for {gid}"));
        if ctx.json {
            r.out = json_cli_out(JuliaDict::from_pairs(vec![
                ("command".to_string(), JVal::Str("distill".to_string())),
                ("goal".to_string(), JVal::Str(gid.clone())),
                ("null".to_string(), JVal::Bool(true)),
                ("empty".to_string(), JVal::Bool(true)),
            ]));
        }
        return r;
    }
    let mut pool = mass;
    if pool.len() == 1 && pool.contains(gid) {
        let refs = goal_reference_sets(&st);
        pool = refs
            .iter()
            .filter(|(_, rs)| rs.contains(gid))
            .map(|(id, _)| id.clone())
            .collect();
    }
    let cands = distill_candidates(&st, &pool);
    let met = !linked.is_empty() || attested;
    if ctx.json {
        let cands_arr: Vec<JVal> = cands
            .iter()
            .map(|(id, k, t)| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("id".to_string(), JVal::Str(id.clone())),
                    ("kind".to_string(), JVal::Str(k.clone())),
                    ("title".to_string(), JVal::Str(t.clone())),
                    ("skeleton".to_string(), JVal::Str(distill_skeleton(id))),
                ]))
            })
            .collect();
        let mut r = OpResult::ok();
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("distill".to_string())),
            ("goal".to_string(), JVal::Str(gid.clone())),
            ("precondition_met".to_string(), JVal::Bool(met)),
            (
                "linked_discoveries".to_string(),
                JVal::Arr(linked.iter().map(|x| JVal::Str(x.clone())).collect()),
            ),
            ("null_attested".to_string(), JVal::Bool(attested)),
            ("candidates".to_string(), JVal::Arr(cands_arr)),
        ]));
        return r;
    }
    let mut out = String::new();
    let title = if g.title.is_empty() {
        String::new()
    } else {
        format!(" ({})", g.title)
    };
    out.push_str(&format!("distillation worksheet for {gid}{title}\n"));
    if met {
        let how = if !linked.is_empty() {
            format!("linked Discovery: {}", linked.join(", "))
        } else {
            "null-distill attested".to_string()
        };
        out.push_str(&format!("archive precondition: met ({how})\n"));
    } else {
        out.push_str(&format!(
            "archive precondition: not met; `grove archive {gid}` refuses until a Discovery is linked or a null-distill attestation exists\n"
        ));
    }
    if cands.is_empty() {
        out.push_str("no validated B / answered Q / accepted D in the goal's mass\n");
    } else {
        out.push_str("candidates:\n");
        for (id, k, t) in &cands {
            let label = if k == "b" {
                "validated B"
            } else if k == "q" {
                "answered Q"
            } else {
                "accepted D"
            };
            out.push_str(&format!("- {id} ({label}): {t}\n"));
            out.push_str(&format!("    {}\n", distill_skeleton(id)));
        }
    }
    out.push_str(&format!("nothing worth distilling? `grove distill {gid} --null`\n"));
    let mut r = OpResult::ok();
    r.out = out;
    r
}
