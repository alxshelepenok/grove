mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::PathBuf;

const T_UNIFORM: &str = "2026-01-01T00:00:00Z";
const SESSION_PH: &str = "tokplaceholder";
const LOG_TS: [&str; 16] = [
    "2026-01-01T00:00:00Z",
    "2026-01-01T00:00:01Z",
    "2026-01-01T00:00:02Z",
    "2026-01-01T00:00:03Z",
    "2026-01-01T00:00:04Z",
    "2026-01-01T00:00:05Z",
    "2026-01-01T00:00:06Z",
    "2026-01-01T00:00:07Z",
    "2026-01-01T00:00:08Z",
    "2026-01-01T00:00:09Z",
    "2026-01-01T00:00:10Z",
    "2026-01-01T00:00:11Z",
    "2026-01-01T00:00:12Z",
    "2026-01-01T00:00:13Z",
    "2026-01-01T00:00:14Z",
    "2026-01-01T00:00:15Z",
];

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-report-test-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn normalize_placeholders(st: &mut State) {
    for n in st.nodes.values_mut() {
        for k in ["t_created", "t_updated"] {
            if n.attrs.get(k).map(|v| v.as_str()) == Some("<ts>") {
                n.attrs.insert(k.to_string(), T_UNIFORM.to_string());
            }
        }
        if n.attrs.get("session_at").map(|v| v.as_str()) == Some("<ts>") {
            n.attrs
                .insert("session_at".to_string(), utc_stamp_second());
        }
        if n.attrs.get("session").map(|v| v.as_str()) == Some("<session>") {
            n.attrs
                .insert("session".to_string(), SESSION_PH.to_string());
        }
    }
    for e in &mut st.edges {
        if e.t_created.as_deref() == Some("<ts>") {
            e.t_created = Some(T_UNIFORM.to_string());
        }
    }
}

fn normalize_out(s: &str, subs: &[(&str, &str)]) -> String {
    let mut out = s.to_string();
    for (from, to) in subs {
        out = out.replace(from, to);
    }
    out
}

fn dispatch_report(ctx: &CliCtx, cmd: &str, pos: &[String], kw: &[(String, String)]) -> OpResult {
    match cmd {
        "deps" => cmd_deps(ctx, pos, kw),
        "impact" => cmd_impact(ctx, pos, kw),
        "path" => cmd_path(ctx, pos, kw),
        "triage" => cmd_triage(ctx, pos, kw),
        "dor" => cmd_dor(ctx, pos, kw),
        "show" => cmd_show(ctx, pos, kw),
        "list" => cmd_list(ctx, pos, kw),
        "graph" => cmd_graph(ctx, pos, kw),
        "status" => cmd_status(ctx, pos, kw),
        "log" => cmd_log(ctx, pos, kw),
        other => panic!("unexpected command {other}"),
    }
}

fn check_step(r: OpResult, sc: &serde_json::Value, i: usize, subs: &[(&str, &str)]) {
    assert_eq!(r.code as i64, step_exit(sc, i), "exit code step {i}");
    assert_eq!(
        normalize_out(&r.out, subs),
        step_field(sc, i, "stdout"),
        "stdout step {i}"
    );
    assert_eq!(
        normalize_out(&r.err, subs),
        step_field(sc, i, "stderr"),
        "stderr step {i}"
    );
}

fn run_golden(sc: &serde_json::Value, i: usize, tag: &str, extra_kw: &[(&str, &str)]) {
    let args = step_args(sc, i);
    let (mut ctx, pos, mut kw) = parse_args(&args[1..]);
    let d = tmpdir(tag);
    ctx.root = d.to_string_lossy().into_owned();
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    let mut st = parse_fixture(&step_field(sc, i - 1, "lock")).unwrap();
    normalize_placeholders(&mut st);
    std::fs::write(ctx.lockpath(), serialize(&st)).unwrap();
    let journal = step_field(sc, i - 1, "journal");
    if !journal.is_empty() {
        std::fs::write(ctx.journalpath(), journal).unwrap();
    }
    for (k, v) in extra_kw {
        kw.push((k.to_string(), v.to_string()));
    }
    let r = dispatch_report(&ctx, &args[0], &pos, &kw);
    check_step(
        r,
        sc,
        i,
        &[(T_UNIFORM, "<ts>"), (SESSION_PH, "<session>")],
    );
}

fn retime_next_log(st: &mut State) {
    let cases = [
        ("A-01", LOG_TS[1], LOG_TS[1]),
        ("G-01", LOG_TS[2], LOG_TS[2]),
        ("W-01", LOG_TS[3], LOG_TS[15]),
        ("W-02", LOG_TS[8], LOG_TS[14]),
    ];
    for (id, tc, tu) in cases {
        let n = st.nodes.get_mut(id).unwrap();
        n.attrs.insert("t_created".to_string(), tc.to_string());
        n.attrs.insert("t_updated".to_string(), tu.to_string());
    }
    for e in &mut st.edges {
        e.t_created = Some(LOG_TS[13].to_string());
    }
}

fn retime_next_log_journal(journal: &str) -> String {
    let mut out = String::new();
    for (idx, line) in journal.lines().enumerate() {
        let t = LOG_TS[idx + 1];
        out.push_str(&line.replace(
            "\"ts\":\"<ts>\"",
            &format!("\"ts\":\"{}\"", t),
        ));
        out.push('\n');
    }
    out
}

fn run_next_log_golden(sc: &serde_json::Value, i: usize, tag: &str) {
    let args = step_args(sc, i);
    let (mut ctx, pos, kw) = parse_args(&args[1..]);
    let d = tmpdir(tag);
    ctx.root = d.to_string_lossy().into_owned();
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    let mut st = parse_fixture(&step_field(sc, i - 1, "lock")).unwrap();
    normalize_placeholders(&mut st);
    retime_next_log(&mut st);
    std::fs::write(ctx.lockpath(), serialize(&st)).unwrap();
    let journal = step_field(sc, i - 1, "journal");
    if !journal.is_empty() {
        std::fs::write(ctx.journalpath(), retime_next_log_journal(&journal)).unwrap();
    }
    let r = dispatch_report(&ctx, &args[0], &pos, &kw);
    let mut subs: Vec<(&str, &str)> = LOG_TS.iter().rev().map(|t| (*t, "<ts>")).collect();
    subs.push((SESSION_PH, "<session>"));
    check_step(r, sc, i, &subs);
}

#[test]
fn golden_triage_text_and_json() {
    let sc = corpus_json("triage");
    let n = scenario_len(&sc);
    run_golden(&sc, n - 2, "triage-text", &[]);
    run_golden(&sc, n - 1, "triage-json", &[]);
}

#[test]
fn golden_status_sessions_tokens() {
    let sc = corpus_json("sessions");
    run_golden(&sc, 10, "status-alice", &[]);
    run_golden(&sc, 11, "status-bob", &[]);
    run_golden(&sc, 16, "status-final", &[]);
}

#[test]
fn golden_status_wip_i4_session_placeholder() {
    let sc = corpus_json("wip-i4");
    let n = scenario_len(&sc);
    run_golden(&sc, n - 1, "wip-i4-status", &[("session", SESSION_PH)]);
}

#[test]
fn golden_show_field_ops() {
    let sc = corpus_json("field-ops");
    run_golden(&sc, 10, "field-ops-show", &[]);
}

#[test]
fn golden_show_staging_overwrite() {
    let sc = corpus_json("staging-overwrite");
    run_golden(&sc, 14, "staging-show-g", &[]);
    run_golden(&sc, 15, "staging-show-w", &[]);
}

#[test]
fn golden_show_w_lifecycle() {
    let sc = corpus_json("w-lifecycle");
    run_golden(&sc, 13, "wlc-show-g13", &[]);
    run_golden(&sc, 15, "wlc-show-g15", &[]);
}

#[test]
fn golden_wave2b_next_log_readonly_steps() {
    let sc = corpus_json("next-log");
    for i in 37..45 {
        run_golden(&sc, i, &format!("next-log-{i}"), &[]);
    }
}

#[test]
fn golden_wave2b_next_log_log_steps() {
    let sc = corpus_json("next-log");
    for i in 45..scenario_len(&sc) {
        run_next_log_golden(&sc, i, &format!("next-log-{i}"));
    }
}

const GT_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:0f6fe917b6515f635b083a86b0779c0ec24314299af163dc85e091a1ae3589da
g G-01 status=unverified fitness=1/1 t_created=2026-07-19T18:12:13Z t_updated=2026-07-19T18:12:13Z "Goal"
  area: A-01

w W-01 type=feature status=progress cynefin=clear session=tok-abc session_at=2026-07-19T18:12:23Z t_created=2026-07-19T18:12:14Z t_updated=2026-07-19T18:12:23Z "Alpha"
  goals: G-01
  theme: T-01
  fitness: G-01=+1
  surface: src/a.jl, src/b.jl
  ac:
    | first ac
    | second ac
  hypothesis:
    | h
  evidence_strategy:
    | e

t T-01 status=open t_created=2026-07-19T18:12:13Z t_updated=2026-07-19T18:12:13Z "Topic"

a A-01 status=present t_created=2026-07-19T18:12:12Z t_updated=2026-07-19T18:12:12Z "Area"
"#;

fn gt_ctx(tag: &str) -> CliCtx {
    let d = tmpdir(tag);
    let ctx = CliCtx::new(d.to_string_lossy().into_owned());
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    let st = parse_fixture(GT_LOCK).unwrap();
    std::fs::write(ctx.lockpath(), serialize(&st)).unwrap();
    ctx
}

#[test]
fn show_json_snapshot_matches_julia_ground_truth() {
    let mut ctx = gt_ctx("show-json-gt");
    ctx.json = true;
    let r = cmd_show(&ctx, &["W-01".to_string()], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(
        r.out,
        "{\"record\":{\"kind\":\"w\",\"fields\":{\"theme\":\"T-01\",\"evidence_strategy\":[\"e\"],\"fitness\":{\"G-01\":1},\"ac\":[\"first ac\",\"second ac\"],\"hypothesis\":[\"h\"],\"goals\":[\"G-01\"],\"surface\":[\"src/a.jl\",\"src/b.jl\"]},\"status\":\"progress\",\"archived\":false,\"attrs\":{\"session_at\":\"2026-07-19T18:12:23Z\",\"t_created\":\"2026-07-19T18:12:14Z\",\"t_updated\":\"2026-07-19T18:12:23Z\",\"session\":\"tok-abc\"},\"id\":\"W-01\",\"cynefin\":\"clear\",\"title\":\"Alpha\",\"type\":\"feature\"},\"command\":\"show\"}\n"
    );
}

#[test]
fn status_json_match_and_stale_match_julia_ground_truth() {
    let mut ctx = gt_ctx("status-json-gt");
    let mut st = parse_fixture(&std::fs::read_to_string(ctx.lockpath()).unwrap()).unwrap();
    let fresh = format_unix_utc(unix_now() - 3600);
    if let Some(n) = st.nodes.get_mut("W-01") {
        n.attrs.insert("session_at".to_string(), fresh);
    }
    std::fs::write(ctx.lockpath(), serialize(&st)).unwrap();
    ctx.json = true;
    let r = cmd_status(&ctx, &[], &[("session".to_string(), "tok-abc".to_string())]);
    assert_eq!(
        r.out,
        "{\"invariants\":{\"messages\":[],\"ok\":true},\"progress\":[{\"stale_for_agent\":false,\"id\":\"W-01\",\"title\":\"Alpha\",\"options_hint\":\"\",\"session_detail\":\"  session=tok-abc\",\"session\":\"tok-abc\"}],\"command\":\"status\",\"alignment_triggers\":[]}\n"
    );
    let r = cmd_status(&ctx, &[], &[("session".to_string(), "someone-else".to_string())]);
    assert_eq!(
        r.out,
        "{\"invariants\":{\"messages\":[],\"ok\":true},\"progress\":[{\"stale_for_agent\":true,\"id\":\"W-01\",\"title\":\"Alpha\",\"options_hint\":\"grove resume W-01 | grove revert W-01 | grove handoff W-01 --to=<token>\",\"session_detail\":\"  session=tok-abc  [!= this session]\",\"session\":\"tok-abc\"}],\"command\":\"status\",\"alignment_triggers\":[]}\n"
    );
}

#[test]
fn graph_text_and_json_match_julia_ground_truth() {
    let ctx = gt_ctx("graph-gt");
    let text = "## Dependency graph\n\n```mermaid\ngraph TD\n  G_01[\"G-01: Goal\"]:::goal\n  W_01[\"W-01: Alpha\"]:::progress,critical\n  T_01[\"T-01: Topic\"]:::theme\n  A_01[\"A-01: Area\"]:::area\n  class W_01 critical\nclassDef area fill:#5a1e4a,color:#fff\nclassDef goal fill:#1e3a5f,color:#fff\nclassDef theme fill:#2a4a3a,color:#fff\nclassDef decision fill:#5a4a1e,color:#fff\nclassDef question fill:#5a3a1e,color:#fff\nclassDef assumption fill:#4a2d5a,color:#fff\nclassDef spike fill:#3a3a5a,color:#fff\nclassDef feature fill:#1e4a4a,color:#fff\nclassDef ready fill:#2d5a27,color:#fff\nclassDef progress fill:#3a4a6a,color:#fff,stroke:#fff,stroke-width:2px\nclassDef done fill:#2d5a27,color:#fff,stroke:#fff,stroke-width:2px\nclassDef rejected fill:#5a5a5a,color:#fff\nclassDef discovery fill:#1f4e5f,color:#fff\nclassDef critical stroke:#ff0,stroke-width:3px\n```\n\n";
    let r = cmd_graph(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, text);
    let mut ctxj = ctx.clone();
    ctxj.json = true;
    let r = cmd_graph(&ctxj, &[], &[]);
    let expected = format!(
        "{{\"command\":\"graph\",\"mermaid\":{}}}\n",
        emit_jval(&JVal::Str(text.to_string()))
    );
    assert_eq!(r.out, expected);
}

fn empty_ctx(tag: &str) -> CliCtx {
    let d = tmpdir(tag);
    let ctx = CliCtx::new(d.to_string_lossy().into_owned());
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    std::fs::write(ctx.lockpath(), serialize(&State::default())).unwrap();
    ctx
}

#[test]
fn silent_and_usage_error_paths() {
    let ctx = empty_ctx("errors");
    let r = cmd_deps(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (1, "", ""));
    let r = cmd_impact(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (1, "", ""));
    let r = cmd_show(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (1, "", ""));
    let r = cmd_show(&ctx, &["W-99".to_string()], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (5, "", ""));
    let r = cmd_dor(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (1, "", ""));
    let r = cmd_dor(&ctx, &["W-99".to_string()], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (5, "", ""));
    let r = cmd_list(&ctx, &[], &[]);
    assert_eq!((r.code, r.err.as_str()), (1, "usage: grove list <kind>\n"));
    let r = cmd_log(&ctx, &[], &[("limit".to_string(), "abc".to_string())]);
    assert_eq!(
        (r.code, r.err.as_str()),
        (1, "bad --limit (expected integer)\n")
    );
    let r = cmd_log(&ctx, &["W-99".to_string()], &[]);
    assert_eq!((r.code, r.err.as_str()), (5, "not found: W-99\n"));
}

#[test]
fn empty_state_outputs_match_julia_ground_truth() {
    let ctx = empty_ctx("empty");
    let r = cmd_triage(&ctx, &[], &[]);
    assert_eq!(r.out, "triage: no open work\n");
    let r = cmd_list(&ctx, &["z".to_string()], &[]);
    assert_eq!((r.code, r.out.as_str()), (0, ""));
    let r = cmd_list(&ctx, &["k".to_string()], &[]);
    assert_eq!((r.code, r.out.as_str()), (0, ""));
    let mut ctxj = ctx.clone();
    ctxj.json = true;
    let r = cmd_list(&ctxj, &["z".to_string()], &[]);
    assert_eq!(r.out, "{\"kind\":\"z\",\"rows\":[],\"command\":\"list\"}\n");
    let r = cmd_triage(&ctxj, &[], &[]);
    assert_eq!(r.out, "{\"rows\":[],\"command\":\"triage\"}\n");
    let r = cmd_status(&ctxj, &[], &[]);
    assert_eq!(
        r.out,
        "{\"invariants\":{\"messages\":[],\"ok\":true},\"progress\":[],\"command\":\"status\",\"alignment_triggers\":[]}\n"
    );
    let r = cmd_log(&ctxj, &[], &[]);
    assert_eq!(r.out, "{\"rows\":[],\"command\":\"log\",\"limit\":200}\n");
    let r = cmd_log(&ctxj, &[], &[("limit".to_string(), "-1".to_string())]);
    assert_eq!(r.out, "{\"rows\":[],\"command\":\"log\",\"limit\":-1}\n");
}

#[test]
fn fmt2_rounding_matches_julia_sprintf() {
    assert_eq!(format!("{:.2}", 0.5), "0.50");
    assert_eq!(format!("{:.2}", 1.0), "1.00");
    assert_eq!(format!("{:.2}", 0.25), "0.25");
    assert_eq!(format!("{:.2}", 0.125), "0.12");
    assert_eq!(format!("{:.2}", 0.375), "0.38");
    assert_eq!(format!("{:.2}", 2.675), "2.67");
    assert_eq!(format!("{:.2}", 1.005), "1.00");
}
