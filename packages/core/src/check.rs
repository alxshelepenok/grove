use crate::cli::{info, json_cli_out, load, persist, CliCtx};
use crate::decay::discovery_decay_errors;
use crate::invariants::check_all;
use crate::json::{JVal, JuliaDict};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_INVARIANT, EXIT_OK};
use crate::parse::parse_strict;

pub fn cmd_check(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut errs = check_all(&st);
    errs.extend(discovery_decay_errors(&st, &ctx.root, &ctx.glossarypath()));
    if ctx.json {
        let ok = errs.is_empty();
        let mut r = OpResult {
            code: if ok { EXIT_OK } else { EXIT_INVARIANT },
            out: String::new(),
            err: String::new(),
            journal: Vec::new(),
        };
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("check".to_string())),
            ("ok".to_string(), JVal::Bool(ok)),
            (
                "errors".to_string(),
                JVal::Arr(errs.iter().map(|e| JVal::Str(e.clone())).collect()),
            ),
        ]));
        return r;
    }
    if errs.is_empty() {
        let mut r = OpResult::ok();
        info(ctx, &mut r, "ok");
        return r;
    }
    OpResult::fail_lines(EXIT_INVARIANT, &errs)
}

pub fn cmd_repair(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    if kw_get(kw, "confirm").is_none() {
        return OpResult::fail(EXIT_ERR, "refusing without --confirm");
    }
    let p = ctx.lockpath();
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => {
            return OpResult::fail(
                EXIT_ERR,
                &format!("lock not found: {} (run `grove init`)", p.display()),
            )
        }
    };
    let text = text.replace("\r\n", "\n");
    let mut st = match parse_strict(&text) {
        Ok(s) => s,
        Err(e) => return OpResult::fail(EXIT_ERR, &e.to_string()),
    };
    persist(ctx, &mut st, None);
    let mut r = OpResult::ok();
    info(ctx, &mut r, &format!("repaired: {}", p.display()));
    r
}
