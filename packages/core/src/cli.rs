use crate::decay::dashboard_decay_count;
use crate::json::{emit_jval, JVal, JuliaDict};
use crate::model::State;
use crate::ops::*;
use crate::render::render_index;
use crate::serialize::serialize;
use crate::session::effective_session_token;
use std::path::{Path, PathBuf};

pub const EXIT_CHECKSUM: i32 = 2;

pub const HELP: &str = "grove (graph-driven reasoning over verified evidence)\n\nRead:\n  ready              list work items ready to start (critical first)\n  next               propose single next W with full execution packet\n  packet  <W-NN>     full execution packet for a W [--cone --cone-depth=N --cone-max=N]\n  deps    <ID>       transitive blocks-predecessors\n  impact  <ID>       transitive blocks-successors\n  path               critical path (longest unfinished blocks chain)\n  triage             rank open W by discovery need (cov, \u{03c7}, fragility; read-only advisory)\n  dor     <W-NN>     DoR conjunct breakdown\n  show    <ID>       record dump\n  list    <kind>     list nodes (g|w|d|q|b|t|y|a) [--status= --cynefin=]\n  check              verify lock checksum and invariants\n  graph              print mermaid block\n  status             summary: progress work, alignment triggers, invariant notes\n  stats              read-only telemetry from journal + lock (cycle time, DoR, bets, discovery, undo, surprise, C/V)\n  diff               structural diff vs git ref (--since=REF, default HEAD)\n  projects           registry table: name, path, last opened\n  log   [<ID>]      timeline from t_* on nodes/edges + journal.log (--limit=N, default 200; 0=unlimited)\n  gate               report-only distillation gate: tw delta, surface overflows, invalidated B, accepted D [--theta=N] [--n=N]\n\nMutate:\n  init                            create .grove/state.lock + index.md + glossary.md [--id-stride=N] [--id-offset=K] [--id-width=W]\n  add <kind> --title=\"\u{2026}\" [...]    create node; prints assigned ID\n  set <ID> <key>=<value>          guarded transitions\n  field <ID> <field> add|rm|clear \"\u{2026}\"\n  link <from> <label> <to>        labels: blocks|implements|asks|tests|targets|produces|causes|supersedes|distills\n  unlink <from> <label> <to>\n  evidence <W-NN> \"\u{2026}\"             append evidence line\n  fitness  <W-NN> <G-NN> <\u{00b1}N>     set per-goal delta\n  archive  <G-NN>                 archive G + exclusive w/d/q/b/t (requires distillation: a linked Discovery or `grove distill G-NN --null`)\n  distill  <G-NN> [--null]        distillation worksheet for a verified goal; --null writes a null-distill attestation (journal, non-mutation)\n  renumber <ID> --to=<NEW-ID>      rewrite record + refs (not if id in done evidence)\n  undo [--steps=N]                revert last N mutations (truncates `.grove/journal.log`)\n  resume  <W-NN>                   adopt session token on a `progress` W (journal undo restores prior claim)\n  handoff <W-NN> --to=<token>      transfer ownership (holder only)\n  revert  <W-NN>                   `progress` -> `ready`, clear session (holder or stale claim)\n  revalidate <Y-NN> --surface=\u{2026}|--from=ID   `stale` Discovery -> `active`, paid with a fresh anchor\n  promote <Y-NN> --to=<project>     copy a Discovery into another project with origin provenance (D13); copy starts `proposed`\n  glossary rename <old> <new>      rewrite glossary.md term + Discovery tags atomically\n  render                          regenerate index.md\n  repair --confirm                accept current lock contents (recompute checksum)\n\nGlobal flags: --root=<path> --project=<name|path> --quiet --json --no-render [--session=<token>]  (--since for diff; --limit for log; --steps for undo)\nRoot resolution: --root wins; else --project / GROVE_PROJECT (directory or registry name); else walk up from cwd to the first dir containing .grove/state.lock.\n";

#[derive(Clone, Debug)]
pub struct CliCtx {
    pub root: String,
    pub quiet: bool,
    pub json: bool,
    pub no_render: bool,
}

impl CliCtx {
    pub fn new(root: String) -> CliCtx {
        CliCtx {
            root,
            quiet: false,
            json: false,
            no_render: false,
        }
    }

    pub fn devdir(&self) -> PathBuf {
        Path::new(&self.root).join(".grove")
    }

    pub fn lockpath(&self) -> PathBuf {
        self.devdir().join("state.lock")
    }

    pub fn indexpath(&self) -> PathBuf {
        self.devdir().join("index.md")
    }

    pub fn glossarypath(&self) -> PathBuf {
        self.devdir().join("glossary.md")
    }

    pub fn journalpath(&self) -> PathBuf {
        self.devdir().join("journal.log")
    }
}

fn is_abs_path(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/') || (b.len() >= 3 && b[1] == b':' && b[2] == b'/')
}

pub fn abspath(p: &str) -> String {
    let p = p.replace('\\', "/");
    let joined = if is_abs_path(&p) {
        p
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        format!("{cwd}/{p}")
    };
    let mut out: Vec<&str> = Vec::new();
    for comp in joined.split('/') {
        match comp {
            "" | "." => continue,
            ".." => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    let mut s = String::new();
    let bytes = joined.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        s.push_str(&joined[..2]);
        s.push('/');
        s.push_str(&out[1..].join("/"));
    } else {
        s.push('/');
        s.push_str(&out.join("/"));
    }
    if cfg!(windows) {
        s = s.replace('/', "\\");
    }
    s
}

pub fn parse_args(args: &[String]) -> (CliCtx, Vec<String>, Vec<(String, String)>) {
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut ctx = CliCtx::new(cwd);
    let mut pos = Vec::new();
    let mut kw: Vec<(String, String)> = Vec::new();
    for a in args {
        if let Some(rest) = a.strip_prefix("--") {
            let (key, val) = match rest.find('=') {
                Some(eq) => (&rest[..eq], &rest[eq + 1..]),
                None => (rest, "true"),
            };
            match key {
                "root" => ctx.root = abspath(val),
                "quiet" => ctx.quiet = val == "true",
                "json" => ctx.json = val == "true",
                "no-render" => ctx.no_render = val == "true",
                _ => kw.push((key.to_string(), val.to_string())),
            }
        } else {
            pos.push(a.clone());
        }
    }
    (ctx, pos, kw)
}

pub fn info(ctx: &CliCtx, r: &mut OpResult, msg: &str) {
    if !ctx.quiet {
        r.err.push_str(msg);
        r.err.push('\n');
    }
}

pub fn json_cli_out(obj: JuliaDict) -> String {
    let mut s = emit_jval(&JVal::Obj(obj));
    s.push('\n');
    s
}

pub fn atomic_write_same_dir(path: &Path, payload: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let fname = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| "lock".to_string());
    let tmp = path.with_file_name(format!(".{fname}.tmp-{}", std::process::id()));
    std::fs::write(&tmp, payload)?;
    let moved = if path.exists() {
        std::fs::remove_file(path).and_then(|_| std::fs::rename(&tmp, path))
    } else {
        std::fs::rename(&tmp, path)
    };
    if moved.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    moved
}

pub fn load(ctx: &CliCtx, verify: bool) -> Result<State, OpResult> {
    let p = ctx.lockpath();
    if !p.is_file() {
        return Err(OpResult::fail(
            EXIT_ERR,
            &format!("lock not found: {} (run `grove init`)", p.display()),
        ));
    }
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => {
            return Err(OpResult::fail(
                EXIT_ERR,
                &format!("lock not found: {} (run `grove init`)", p.display()),
            ))
        }
    };
    let text = text.replace("\r\n", "\n");
    match crate::parse::parse_internal(&text, crate::parse::ParseMode::Strict) {
        Ok((st, body, expected)) => {
            if verify {
                let actual = crate::checksum::checksum_of(&body);
                if actual != expected {
                    return Err(OpResult::fail(EXIT_CHECKSUM, &format!(
                        "lock checksum mismatch (expected {expected}, got {actual}). Did you edit state.lock by hand? Run `grove repair --confirm` to accept the current contents."
                    )));
                }
            }
            Ok(st)
        }
        Err(e) => Err(OpResult::fail(EXIT_ERR, &e.to_string())),
    }
}

pub fn persist(ctx: &CliCtx, st: &mut State, journal: Option<&str>) {
    crate::algebra::rederive_artifacts(st);
    let payload = serialize(st);
    let _ = atomic_write_same_dir(&ctx.lockpath(), &payload);
    if !ctx.no_render {
        let decay = dashboard_decay_count(ctx, st);
        let idx = render_index(st, decay);
        let _ = atomic_write_same_dir(&ctx.indexpath(), &idx);
    }
    if let Some(line) = journal {
        let _ = crate::journal::append_journal_record(&ctx.journalpath(), line);
    }
}

pub fn persist_result(ctx: &CliCtx, st: &mut State, r: &OpResult, session: &str) {
    let stamped: Vec<String> = r
        .journal
        .iter()
        .map(|l| crate::journal::stamp_journal_session(l, session))
        .collect();
    if r.code != EXIT_OK {
        for line in &stamped {
            let _ = crate::journal::append_journal_record(&ctx.journalpath(), line);
        }
        return;
    }
    persist(ctx, st, stamped.first().map(String::as_str));
    for line in stamped.iter().skip(1) {
        let _ = crate::journal::append_journal_record(&ctx.journalpath(), line);
    }
}

pub fn load_glossary(ctx: &CliCtx) -> Option<String> {
    std::fs::read_to_string(ctx.glossarypath()).ok()
}

pub fn eff_token(ctx: &CliCtx, kw: &[(String, String)]) -> String {
    effective_session_token(&ctx.root, kw_get(kw, "session"))
}

pub fn journal_session_token(ctx: &CliCtx, kw: &[(String, String)]) -> String {
    let t = eff_token(ctx, kw);
    let t = t.trim();
    if t.is_empty() {
        "none".to_string()
    } else {
        t.to_string()
    }
}

pub fn cmd_init(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    if ctx.lockpath().is_file() {
        return OpResult::fail(
            EXIT_ERR,
            &format!("lock already exists at {}", ctx.lockpath().display()),
        );
    }
    let _ = std::fs::create_dir_all(ctx.devdir());
    let mut st = State::default();
    if let Some(v) = kw_get(kw, "id-stride") {
        match v.parse::<i64>() {
            Ok(n) if n >= 1 => st.id_stride = n,
            Ok(_) => return OpResult::fail(EXIT_ERR, "--id-stride must be ≥ 1"),
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --id-stride (expected integer)"),
        }
    }
    if let Some(v) = kw_get(kw, "id-offset") {
        match v.parse::<i64>() {
            Ok(n) if n >= 1 => st.id_offset = n,
            Ok(_) => return OpResult::fail(EXIT_ERR, "--id-offset must be ≥ 1"),
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --id-offset (expected integer)"),
        }
    }
    if let Some(v) = kw_get(kw, "id-width") {
        match v.parse::<i64>() {
            Ok(n) if n >= 2 => st.id_pad_width = n,
            Ok(_) => return OpResult::fail(EXIT_ERR, "--id-width must be ≥ 2"),
            Err(_) => return OpResult::fail(EXIT_ERR, "bad --id-width (expected integer)"),
        }
    } else if st.id_stride != 1 || st.id_offset != 1 {
        st.id_pad_width = st.id_pad_width.max(3);
    }
    persist(ctx, &mut st, None);
    if !ctx.glossarypath().is_file() {
        let _ = std::fs::write(
            ctx.glossarypath(),
            "# Glossary\n\n| Term | Definition | Source |\n| --- | --- | --- |\n",
        );
    }
    let mut r = OpResult::ok();
    info(ctx, &mut r, &format!("initialised: {}", ctx.devdir().display()));
    r
}

pub const USAGE_ADD: &str = "usage: grove add <kind> --title=\"\u{2026}\" [...]";

pub fn cmd_add(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if kw_get(kw, "help").is_some() {
        let mut r = OpResult::ok();
        r.out.push_str(USAGE_ADD);
        r.out.push('\n');
        return r;
    }
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, USAGE_ADD);
    }
    if pos.len() > 1 {
        return OpResult::fail(
            EXIT_ERR,
            &format!("{USAGE_ADD} (unexpected positional argument: {})", pos[1]),
        );
    }
    if kw.is_empty() && pos[0].as_str() != "a" && pos[0].as_str() != "y" {
        return OpResult::fail(EXIT_ERR, USAGE_ADD);
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let r = op_add(&mut st, &pos[0], kw);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_set(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 2 {
        return OpResult::fail(EXIT_ERR, "usage: grove set <ID> <key>=<value>");
    }
    let id = &pos[0];
    let Some(eq) = pos[1].find('=') else {
        return OpResult::fail(EXIT_ERR, "usage: grove set <ID> <key>=<value>");
    };
    let (key, val) = (&pos[1][..eq], &pos[1][eq + 1..]);
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_set(&mut st, id, key, val, &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_field(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 3 {
        return OpResult::fail(
            EXIT_ERR,
            "usage: grove field <ID> <field> add|rm|clear [value]",
        );
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let value = pos.get(3).map(String::as_str);
    let r = op_field(&mut st, &pos[0], &pos[1], &pos[2], value, &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_link(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 3 {
        return OpResult::fail(EXIT_ERR, "usage: grove link <from> <label> <to>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_link(&mut st, &pos[0], &pos[1], &pos[2], &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_unlink(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 3 {
        return OpResult::fail(EXIT_ERR, "usage: grove unlink <from> <label> <to>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_unlink(&mut st, &pos[0], &pos[1], &pos[2], &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_evidence(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 2 {
        return OpResult::fail(EXIT_ERR, "usage: grove evidence <W-NN> \"…\"");
    }
    let args = vec![pos[0].clone(), "evidence".to_string(), "add".to_string(), pos[1].clone()];
    cmd_field(ctx, &args, kw)
}

pub fn cmd_fitness(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 3 {
        return OpResult::fail(EXIT_ERR, "usage: grove fitness <W-NN> <G-NN> <±delta>");
    }
    let delta: i64 = match pos[2].parse() {
        Ok(d) => d,
        Err(_) => return OpResult::fail(EXIT_ERR, "bad delta (expected integer)"),
    };
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_fitness(&mut st, &pos[0], &pos[1], delta, &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_renumber(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove renumber <ID> --to=<NEW-ID>");
    }
    let Some(new_id) = kw_get(kw, "to") else {
        return OpResult::fail(EXIT_ERR, "missing --to=<NEW-ID>");
    };
    let old_id = pos[0].trim();
    let new_id = new_id.trim();
    if new_id.is_empty() {
        return OpResult::fail(EXIT_ERR, "bad --to");
    }
    if old_id == new_id {
        return OpResult::ok();
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_renumber(&mut st, old_id, new_id, &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_resume(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove resume <W-NN>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_resume(&mut st, &pos[0], &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_handoff(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove handoff <W-NN> --to=<token>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_handoff(&mut st, &pos[0], kw_get(kw, "to"), &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_revert(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove revert <W-NN>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eff = eff_token(ctx, kw);
    let r = op_revert(&mut st, &pos[0], &eff);
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn cmd_undo(ctx: &CliCtx, _pos: &[String], kw: &[(String, String)]) -> OpResult {
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut glossary = load_glossary(ctx);
    let r = op_undo(
        &mut st,
        &ctx.journalpath(),
        glossary.as_mut(),
        kw_get(kw, "steps"),
        &journal_session_token(ctx, kw),
    );
    if r.code == EXIT_OK {
        if let Some(g) = &glossary {
            let _ = std::fs::write(ctx.glossarypath(), g);
        }
        persist(ctx, &mut st, None);
    }
    r
}

pub fn cmd_glossary(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if !pos.is_empty() && pos[0] == "rename" {
        return cmd_glossary_rename(ctx, &pos[1..], kw);
    }
    OpResult::fail(EXIT_ERR, "usage: grove glossary rename <old> <new>")
}

pub fn cmd_glossary_rename(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.len() < 2 {
        return OpResult::fail(EXIT_ERR, "usage: grove glossary rename <old> <new>");
    }
    let mut st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let mut glossary = load_glossary(ctx);
    let r = op_glossary_rename(&mut st, &mut glossary, &pos[0], &pos[1]);
    if r.code == EXIT_OK {
        if let Some(g) = &glossary {
            let _ = std::fs::write(ctx.glossarypath(), g);
        }
    }
    persist_result(ctx, &mut st, &r, &journal_session_token(ctx, kw));
    r
}

pub fn dispatch(ctx: &CliCtx, cmd: &str, pos: &[String], kw: &[(String, String)]) -> OpResult {
    match cmd {
        "init" => cmd_init(ctx, pos, kw),
        "add" => cmd_add(ctx, pos, kw),
        "set" => cmd_set(ctx, pos, kw),
        "field" => cmd_field(ctx, pos, kw),
        "link" => cmd_link(ctx, pos, kw),
        "unlink" => cmd_unlink(ctx, pos, kw),
        "evidence" => cmd_evidence(ctx, pos, kw),
        "fitness" => cmd_fitness(ctx, pos, kw),
        "renumber" => cmd_renumber(ctx, pos, kw),
        "resume" => cmd_resume(ctx, pos, kw),
        "handoff" => cmd_handoff(ctx, pos, kw),
        "revert" => cmd_revert(ctx, pos, kw),
        "undo" => cmd_undo(ctx, pos, kw),
        "glossary" => cmd_glossary(ctx, pos, kw),
        "archive" => crate::archive::cmd_archive(ctx, pos, kw),
        "distill" => crate::distill::cmd_distill(ctx, pos, kw),
        "revalidate" => crate::revalidate::cmd_revalidate(ctx, pos, kw),
        "render" => crate::render::cmd_render(ctx, pos, kw),
        "repair" => crate::check::cmd_repair(ctx, pos, kw),
        "check" => crate::check::cmd_check(ctx, pos, kw),
        "ready" => crate::packet::cmd_ready(ctx, pos, kw),
        "next" => crate::packet::cmd_next(ctx, pos, kw),
        "packet" => crate::packet::cmd_packet(ctx, pos, kw),
        "deps" => crate::report::cmd_deps(ctx, pos, kw),
        "impact" => crate::report::cmd_impact(ctx, pos, kw),
        "path" => crate::report::cmd_path(ctx, pos, kw),
        "triage" => crate::report::cmd_triage(ctx, pos, kw),
        "dor" => crate::report::cmd_dor(ctx, pos, kw),
        "show" => crate::report::cmd_show(ctx, pos, kw),
        "list" => crate::report::cmd_list(ctx, pos, kw),
        "graph" => crate::report::cmd_graph(ctx, pos, kw),
        "status" => crate::report::cmd_status(ctx, pos, kw),
        "stats" => crate::stats::cmd_stats(ctx, pos, kw),
        "diff" => crate::lockdiff::cmd_diff(ctx, pos, kw),
        "log" => crate::report::cmd_log(ctx, pos, kw),
        "gate" => crate::gate::cmd_gate(ctx, pos, kw),
        "projects" => crate::projects::cmd_projects(ctx, pos, kw),
        "promote" => crate::projects::cmd_promote(ctx, pos, kw),
        _ => unknown_command_result(cmd),
    }
}

pub const COMMAND_NAMES: [&str; 38] = [
    "init", "add", "set", "field", "link", "unlink", "evidence", "fitness", "archive", "distill",
    "render", "repair", "ready", "next", "packet", "deps", "impact", "path", "triage", "dor",
    "show", "list", "graph", "check", "status", "stats", "diff", "log", "renumber", "resume",
    "handoff", "revert", "undo", "gate", "revalidate", "glossary", "projects", "promote",
];

pub fn unknown_command_result(cmd: &str) -> OpResult {
    let mut r = OpResult::fail(EXIT_ERR, &format!("unknown command: {cmd}"));
    r.err.push_str(HELP);
    r
}

pub const SESSION_READ_COMMANDS: [&str; 18] = [
    "ready", "next", "packet", "deps", "impact", "path", "dor", "triage", "show", "list", "graph",
    "check", "status", "diff", "log", "stats", "projects", "promote",
];

pub const SESSION_MUTATE_COMMANDS: [&str; 20] = [
    "init", "add", "set", "field", "link", "unlink", "evidence", "fitness", "archive", "distill",
    "repair", "render", "undo", "renumber", "resume", "handoff", "revert", "gate", "revalidate",
    "glossary",
];

enum CliRequest {
    Immediate(OpResult),
    Dispatch {
        ctx: CliCtx,
        cmd: String,
        pos: Vec<String>,
        kw: Vec<(String, String)>,
        note: Option<String>,
    },
}

fn cli_prepare(args: &[String]) -> CliRequest {
    if args.is_empty() {
        let mut r = OpResult::ok();
        r.out.push_str(HELP);
        return CliRequest::Immediate(r);
    }
    if matches!(args[0].as_str(), "-h" | "--help" | "help") {
        let mut r = OpResult::ok();
        r.out.push_str(HELP);
        return CliRequest::Immediate(r);
    }
    let cmd = args[0].clone();
    if !COMMAND_NAMES.contains(&cmd.as_str()) {
        return CliRequest::Immediate(unknown_command_result(&cmd));
    }
    let (mut ctx, pos, kw) = parse_args(&args[1..]);
    let root_given = args[1..]
        .iter()
        .any(|a| a == "--root" || a.starts_with("--root="));
    match crate::projects::resolve_root(&ctx.root, &kw, root_given) {
        Ok(Some(resolved)) => ctx.root = resolved,
        Ok(None) => {
            return CliRequest::Immediate(OpResult {
                code: EXIT_NOTFOUND,
                out: String::new(),
                err: String::new(),
                journal: Vec::new(),
            })
        }
        Err(msg) => return CliRequest::Immediate(OpResult::fail(EXIT_NOTFOUND, &msg)),
    }
    let note = crate::projects::registry_note_open(&ctx.root, &cmd);
    CliRequest::Dispatch {
        ctx,
        cmd,
        pos,
        kw,
        note,
    }
}

pub fn run_cli(args: &[String]) -> OpResult {
    match cli_prepare(args) {
        CliRequest::Immediate(r) => r,
        CliRequest::Dispatch {
            ctx,
            cmd,
            pos,
            kw,
            note,
        } => {
            let mut r = dispatch(&ctx, &cmd, &pos, &kw);
            if let Some(w) = note {
                r.err = format!("{w}\n{}", r.err);
            }
            r
        }
    }
}

pub fn run_cli_session_locked(args: &[String]) -> OpResult {
    match cli_prepare(args) {
        CliRequest::Immediate(r) => r,
        CliRequest::Dispatch {
            ctx,
            cmd,
            pos,
            kw,
            note,
        } => crate::session_lock::with_session_locks_enabled(|| {
            let locked = if SESSION_READ_COMMANDS.contains(&cmd.as_str()) {
                crate::session_lock::with_session_shared(&ctx, || dispatch(&ctx, &cmd, &pos, &kw))
            } else if SESSION_MUTATE_COMMANDS.contains(&cmd.as_str()) {
                crate::session_lock::with_session_exclusive(&ctx, || {
                    dispatch(&ctx, &cmd, &pos, &kw)
                })
            } else {
                Ok((dispatch(&ctx, &cmd, &pos, &kw), Vec::new()))
            };
            let mut pre = String::new();
            if let Some(w) = &note {
                pre.push_str(w);
                pre.push('\n');
            }
            match locked {
                Ok((mut r, warnings)) => {
                    for w in &warnings {
                        pre.push_str(w);
                        pre.push('\n');
                    }
                    r.err = format!("{pre}{}", r.err);
                    r
                }
                Err(t) => {
                    for w in &t.warnings {
                        pre.push_str(w);
                        pre.push('\n');
                    }
                    pre.push_str(&t.msg);
                    pre.push('\n');
                    OpResult {
                        code: EXIT_GUARD,
                        out: String::new(),
                        err: pre,
                        journal: Vec::new(),
                    }
                }
            }
        }),
    }
}
