use crate::cli::{json_cli_out, load, persist, CliCtx};
use crate::decay::discovery_decay_errors;
use crate::invariants::check_all;
use crate::json::{JVal, JuliaDict};
use crate::message::{Code, Diagnostic};
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
            diags: Vec::new(),
            raw_err: String::new(),
            journal: Vec::new(),
        };
        r.out = json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("check".to_string())),
            ("ok".to_string(), JVal::Bool(ok)),
            (
                "errors".to_string(),
                JVal::Arr(errs.iter().map(|e| e.to_jval()).collect()),
            ),
        ]));
        return r;
    }
    if errs.is_empty() {
        let mut r = OpResult::ok();
        if !ctx.quiet {
            r.push_info(Code::Operational, "ok");
        }
        return r;
    }
    OpResult {
        code: EXIT_INVARIANT,
        out: String::new(),
        diags: errs,
        raw_err: String::new(),
        journal: Vec::new(),
    }
}

pub fn cmd_repair(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    if kw_get(kw, "confirm").is_none() {
        return OpResult::fail_diag(
            EXIT_ERR,
            Diagnostic::error(Code::Interface, "refusing without --confirm"),
        );
    }
    let p = ctx.lockpath();
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => {
            return OpResult::fail_diag(
                EXIT_ERR,
                Diagnostic::error_details(
                    Code::System,
                    "lock file is absent, initialize the project first",
                    vec![("path".to_string(), p.display().to_string())],
                ),
            )
        }
    };
    let text = text.replace("\r\n", "\n");
    let mut st = match parse_strict(&text) {
        Ok(s) => s,
        Err(e) => return OpResult::fail(EXIT_ERR, Code::System, &e.to_string()),
    };
    persist(ctx, &mut st, None);
    let mut r = OpResult::ok();
    if !ctx.quiet {
        r.push_diag(Diagnostic::info_details(
            Code::Operational,
            "recanonicalized the lock",
            vec![("path".to_string(), p.display().to_string())],
        ));
    }
    r
}
