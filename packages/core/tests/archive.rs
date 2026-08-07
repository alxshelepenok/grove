mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::collections::BTreeSet;
use std::path::PathBuf;

const TS: &str = "2031-01-01T00:00:00Z";
const REAL_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const GLOSS_AUTH: &str =
    "# Glossary\n\n| Term | Definition | Source |\n| --- | --- | --- |\n| auth | a term | test |\n";

fn pin() {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(TS).unwrap()));
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-atest-{}-{}-{}",
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

fn mask_sha(text: &str) -> String {
    const PREFIX: &str = "# checksum: sha256:";
    match text.find(PREFIX) {
        Some(i) => {
            let start = i + PREFIX.len();
            if text.len() >= start + 64 {
                format!("{}<sha>{}", &text[..start], &text[start + 64..])
            } else {
                text.to_string()
            }
        }
        None => text.to_string(),
    }
}

fn normalize_lock(text: &str) -> String {
    mask_sha(&text.replace(TS, "<ts>"))
}

fn normalize_journal(text: &str, token: &str) -> String {
    text.replace(TS, "<ts>").replace(token, "<session>")
}

fn materialize(sc: &serde_json::Value, prev: usize, tag: &str) -> PathBuf {
    let dir = tmpdir(tag);
    let dev = dir.join(".grove");
    std::fs::create_dir_all(&dev).unwrap();
    let lock = step_field(sc, prev, "lock").replace("<ts>", TS);
    let st = parse_fixture(&lock).unwrap_or_else(|e| panic!("materialize step {prev}: {e}"));
    std::fs::write(dev.join("state.lock"), serialize(&st)).unwrap();
    if let Some(j) = sc["steps"][prev]["journal"].as_str() {
        std::fs::write(dev.join("journal.log"), j).unwrap();
    }
    dir
}

fn project_with_state(tag: &str, st: &State) -> (PathBuf, CliCtx) {
    let dir = tmpdir(tag);
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    std::fs::write(ctx.lockpath(), serialize(st)).unwrap();
    (dir, ctx)
}

fn run_step(dir: &PathBuf, args: &[String]) -> OpResult {
    let (mut ctx, pos, kw) = parse_args(&args[1..]);
    ctx.root = dir.to_string_lossy().into_owned();
    match args[0].as_str() {
        "distill" => cmd_distill(&ctx, &pos, &kw),
        "archive" => cmd_archive(&ctx, &pos, &kw),
        "check" => cmd_check(&ctx, &pos, &kw),
        "repair" => cmd_repair(&ctx, &pos, &kw),
        "revalidate" => cmd_revalidate(&ctx, &pos, &kw),
        other => panic!("unsupported step command {other}"),
    }
}

fn assert_step(sc: &serde_json::Value, i: usize, dir: &PathBuf) {
    pin();
    let name = sc["name"].as_str().unwrap_or("?");
    let args = step_args(sc, i);
    let r = run_step(dir, &args);
    let root = dir.to_string_lossy().into_owned();
    assert_eq!(r.code as i64, step_exit(sc, i), "{name} step {i} {args:?} exit");
    assert_eq!(
        r.out,
        step_field(sc, i, "stdout").replace("<root>", &root),
        "{name} step {i} {args:?} stdout"
    );
    assert_eq!(
        r.err,
        step_field(sc, i, "stderr").replace("<root>", &root),
        "{name} step {i} {args:?} stderr"
    );
    let want_lock = step_field(sc, i, "lock");
    if !want_lock.is_empty() {
        let got = std::fs::read_to_string(dir.join(".grove/state.lock")).unwrap();
        assert_eq!(normalize_lock(&got), want_lock, "{name} step {i} {args:?} lock");
    }
    let got_j = std::fs::read_to_string(dir.join(".grove/journal.log")).unwrap_or_default();
    match sc["steps"][i]["journal"].as_str() {
        Some(want_j) => {
            let (mut tctx, _, tkw) = parse_args(&args[1..]);
            tctx.root = root.clone();
            let token = journal_session_token(&tctx, &tkw);
            assert_eq!(
                normalize_journal(&got_j, &token),
                want_j,
                "{name} step {i} {args:?} journal"
            )
        }
        None => assert!(got_j.is_empty(), "{name} step {i} {args:?} journal should be absent"),
    }
}

#[test]
fn corpus_distill_archive() {
    pin();
    let sc = corpus_json("distill-archive");
    for (i, prev) in [
        (21usize, 20usize),
        (22, 21),
        (23, 22),
        (24, 23),
        (25, 24),
    ] {
        let dir = materialize(&sc, prev, &format!("da{i}"));
        assert_step(&sc, i, &dir);
    }
}

#[test]
fn corpus_discovery_lifecycle() {
    pin();
    let sc = corpus_json("discovery-lifecycle");
    let with_fs = |dir: &PathBuf, a: bool| {
        std::fs::write(dir.join(".grove/glossary.md"), GLOSS_AUTH).unwrap();
        let src = dir.join("src");
        std::fs::create_dir_all(&src).unwrap();
        if a {
            std::fs::write(src.join("a.jl"), "a\n").unwrap();
        }
        std::fs::write(src.join("b.jl"), "b\n").unwrap();
    };
    let dir = materialize(&sc, 10, "dl11");
    with_fs(&dir, true);
    assert_step(&sc, 11, &dir);
    let dir = materialize(&sc, 12, "dl13");
    with_fs(&dir, false);
    assert_step(&sc, 13, &dir);
    let dir = materialize(&sc, 14, "dl15");
    with_fs(&dir, false);
    assert_step(&sc, 15, &dir);
    let dir = materialize(&sc, 15, "dl16");
    with_fs(&dir, false);
    assert_step(&sc, 16, &dir);
    let dir = materialize(&sc, 19, "dl20");
    with_fs(&dir, false);
    assert_step(&sc, 20, &dir);
}

#[test]
fn corpus_check_foreign_status() {
    pin();
    let sc = corpus_json("check-foreign-status");
    let dir = materialize(&sc, 1, "cfs");
    let lp = dir.join(".grove/state.lock");
    let mut t = std::fs::read_to_string(&lp).unwrap();
    t.push_str("a A-02 status=archived \"Corrupt\"\n");
    std::fs::write(&lp, t).unwrap();
    assert_step(&sc, 3, &dir);
}

#[test]
fn corpus_check_y_archive() {
    pin();
    let sc = corpus_json("check-y-archive");
    let dir = materialize(&sc, 0, "cya");
    let lp = dir.join(".grove/state.lock");
    let mut t = std::fs::read_to_string(&lp).unwrap();
    t.push_str(":archive\ny Y-99 status=active \"Corrupt\"\n");
    std::fs::write(&lp, t).unwrap();
    assert_step(&sc, 3, &dir);
}

#[test]
fn corpus_diff_repair() {
    pin();
    let sc = corpus_json("diff-repair");
    let dir = tmpdir("dr");
    let dev = dir.join(".grove");
    std::fs::create_dir_all(&dev).unwrap();
    let corrupt = step_field(&sc, 13, "lock")
        .replace("<sha>", REAL_SHA)
        .replace("<ts>", TS);
    std::fs::write(dev.join("state.lock"), &corrupt).unwrap();
    std::fs::write(dev.join("journal.log"), step_field(&sc, 13, "journal")).unwrap();
    assert_step(&sc, 14, &dir);
    assert_step(&sc, 15, &dir);
    assert_step(&sc, 16, &dir);
}

#[test]
fn goal_reference_sets_propagates_all_edge_kinds_and_theme() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    let mut w1 = work("W-01", "feature", "done", "clear");
    reflist(&mut w1, "goals", &["G-01"]);
    put(&mut st, w1);
    let mut w2 = work("W-02", "feature", "proposed", "clear");
    reflist(&mut w2, "goals", &["G-02"]);
    single(&mut w2, "theme", "T-02");
    put(&mut st, w2);
    put(&mut st, plain(Kind::T, "T-02", "open"));
    for id in ["D-01", "D-02", "D-03"] {
        put(&mut st, plain(Kind::D, id, "proposed"));
    }
    for id in ["Q-01", "Q-02", "Q-03"] {
        put(&mut st, plain(Kind::Q, id, "open"));
    }
    for id in ["B-01", "B-02", "B-03", "B-04"] {
        put(&mut st, plain(Kind::B, id, "proposed"));
    }
    put(&mut st, plain(Kind::T, "T-01", "open"));
    put(&mut st, plain(Kind::Y, "Y-01", "active"));
    edge(&mut st, "W-01", "implements", "D-01");
    edge(&mut st, "W-01", "produces", "D-02");
    edge(&mut st, "W-01", "produces", "Q-02");
    edge(&mut st, "W-01", "produces", "B-02");
    edge(&mut st, "W-01", "produces", "Y-01");
    edge(&mut st, "Q-01", "asks", "W-01");
    edge(&mut st, "B-01", "tests", "Q-01");
    edge(&mut st, "B-03", "targets", "W-01");
    edge(&mut st, "T-01", "causes", "W-01");
    edge(&mut st, "D-03", "supersedes", "D-01");
    edge(&mut st, "B-04", "tests", "Q-03");
    edge(&mut st, "Q-03", "asks", "W-01");
    let refs = goal_reference_sets(&st);
    let only = |id: &str| refs.get(id).cloned().unwrap_or_default();
    let g1: BTreeSet<String> = ["G-01".to_string()].into_iter().collect();
    for id in [
        "G-01", "W-01", "D-01", "D-02", "D-03", "Q-01", "Q-02", "Q-03", "B-01", "B-02", "B-03",
        "B-04", "T-01",
    ] {
        assert_eq!(only(id), g1, "refs[{id}]");
    }
    let g2: BTreeSet<String> = ["G-02".to_string()].into_iter().collect();
    for id in ["G-02", "W-02", "T-02"] {
        assert_eq!(only(id), g2, "refs[{id}]");
    }
    assert!(only("Y-01").is_empty(), "produces to y must not propagate");
}

#[test]
fn exclusive_archive_ids_respects_connectivity_and_exclusivity() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    let mut w1 = work("W-01", "feature", "done", "clear");
    reflist(&mut w1, "goals", &["G-01"]);
    single(&mut w1, "theme", "T-09");
    put(&mut st, w1);
    let mut w2 = work("W-02", "feature", "done", "clear");
    reflist(&mut w2, "goals", &["G-01", "G-02"]);
    put(&mut st, w2);
    let mut w99 = work("W-99", "feature", "done", "clear");
    reflist(&mut w99, "goals", &["G-01"]);
    w99.archived = true;
    put(&mut st, w99);
    put(&mut st, plain(Kind::T, "T-09", "open"));
    put(&mut st, plain(Kind::D, "D-01", "accepted"));
    put(&mut st, plain(Kind::D, "D-02", "accepted"));
    put(&mut st, plain(Kind::D, "D-99", "accepted"));
    put(&mut st, plain(Kind::Q, "Q-01", "answered"));
    put(&mut st, plain(Kind::Q, "Q-02", "answered"));
    put(&mut st, plain(Kind::B, "B-01", "validated"));
    put(&mut st, plain(Kind::T, "T-01", "open"));
    edge(&mut st, "W-01", "implements", "D-01");
    edge(&mut st, "D-02", "supersedes", "D-01");
    edge(&mut st, "Q-01", "asks", "W-01");
    edge(&mut st, "B-01", "targets", "W-01");
    edge(&mut st, "T-01", "causes", "W-01");
    edge(&mut st, "W-01", "produces", "Q-02");
    edge(&mut st, "W-02", "produces", "Q-02");
    edge(&mut st, "W-99", "implements", "D-99");
    let ids = exclusive_archive_ids(&st, "G-01");
    let want: BTreeSet<String> = ["G-01", "W-01", "D-01", "D-02", "Q-01", "B-01", "T-01"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(ids, want);
    assert!(exclusive_archive_ids(&st, "G-99").is_empty());
}

#[test]
fn distill_candidates_filters_and_sorts() {
    let mut st = State::default();
    let mut b1 = plain(Kind::B, "B-01", "validated");
    b1.title = "Bench".to_string();
    put(&mut st, b1);
    let mut q1 = plain(Kind::Q, "Q-01", "answered");
    q1.title = "Quest".to_string();
    put(&mut st, q1);
    let mut d1 = plain(Kind::D, "D-01", "accepted");
    d1.title = "Dec".to_string();
    put(&mut st, d1);
    put(&mut st, plain(Kind::B, "B-02", "proposed"));
    put(&mut st, plain(Kind::Q, "Q-02", "open"));
    put(&mut st, plain(Kind::D, "D-02", "rejected"));
    let mut b3 = plain(Kind::B, "B-03", "validated");
    b3.archived = true;
    put(&mut st, b3);
    put(&mut st, plain(Kind::Y, "Y-01", "active"));
    let pool: BTreeSet<String> = [
        "B-01", "B-02", "B-03", "D-01", "D-02", "D-99", "Q-01", "Q-02", "Y-01",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let cands = distill_candidates(&st, &pool);
    assert_eq!(
        cands,
        vec![
            ("B-01".to_string(), "b".to_string(), "Bench".to_string()),
            ("D-01".to_string(), "d".to_string(), "Dec".to_string()),
            ("Q-01".to_string(), "q".to_string(), "Quest".to_string()),
        ]
    );
}

#[test]
fn distill_skeleton_is_byte_exact() {
    assert_eq!(
        distill_skeleton("B-01"),
        "grove add y --from=B-01 --title=\"…\" --tags=<glossary-term> --surface=<path>  # xor --why=\"…\""
    );
}

#[test]
fn cmd_distill_worksheet_variants() {
    pin();
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "verified");
    g.title = "Goal".to_string();
    put(&mut st, g);
    let (_dir, ctx) = project_with_state("dw1", &st);
    let pos = vec!["G-01".to_string()];
    let r = cmd_distill(&ctx, &pos, &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.err, "");
    assert_eq!(
        r.out,
        "distillation worksheet for G-01 (Goal)\narchive precondition: not met; `grove archive G-01` refuses until a Discovery is linked or a null-distill attestation exists\nno validated B / answered Q / accepted D in the goal's mass\nnothing worth distilling? `grove distill G-01 --null`\n"
    );
    append_journal_record(
        &ctx.journalpath(),
        "{\"v\":1,\"cmd\":\"distill\",\"ts\":\"<ts>\",\"inv\":{\"goal\":\"G-01\",\"op\":\"distill\",\"empty\":true}}",
    )
    .unwrap();
    let r = cmd_distill(&ctx, &pos, &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(
        r.out,
        "distillation worksheet for G-01 (Goal)\narchive precondition: met (null-distill attested)\nno validated B / answered Q / accepted D in the goal's mass\nnothing worth distilling? `grove distill G-01 --null`\n"
    );
    let mut st2 = State::default();
    let mut g2 = plain(Kind::G, "G-01", "verified");
    g2.title = "Goal".to_string();
    put(&mut st2, g2);
    let mut w = work("W-01", "feature", "done", "clear");
    reflist(&mut w, "goals", &["G-01"]);
    put(&mut st2, w);
    put(&mut st2, plain(Kind::Y, "Y-01", "active"));
    let mut e = Edge::new("W-01", "produces", "Y-01");
    e.t_created = Some(TS.to_string());
    st2.edges.push(e);
    let (_dir2, ctx2) = project_with_state("dw2", &st2);
    let r = cmd_distill(&ctx2, &pos, &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(
        r.out,
        "distillation worksheet for G-01 (Goal)\narchive precondition: met (linked Discovery: Y-01)\nno validated B / answered Q / accepted D in the goal's mass\nnothing worth distilling? `grove distill G-01 --null`\n"
    );
}

#[test]
fn cmd_distill_null_attests_and_json() {
    pin();
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    let (_dir, ctx) = project_with_state("dn", &st);
    let pos = vec!["G-01".to_string()];
    let kw = vec![("null".to_string(), "true".to_string())];
    let r = cmd_distill(&ctx, &pos, &kw);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "null-distill attested for G-01\n");
    let (raw, _) = journal_read_nonempty_pairs(&ctx.journalpath());
    let tok = journal_session_token(&ctx, &kw);
    assert_eq!(
        raw,
        vec![format!(
            "{{\"v\":1,\"session\":\"{tok}\",\"cmd\":\"distill\",\"ts\":\"2031-01-01T00:00:00Z\",\"inv\":{{\"goal\":\"G-01\",\"op\":\"distill\",\"empty\":true}}}}"
        )]
    );
    assert!(distill_null_attested(&ctx.journalpath(), "G-01"));
    assert!(!distill_null_attested(&ctx.journalpath(), "G-02"));
    let mut ctxj = CliCtx::new(ctx.root.clone());
    ctxj.json = true;
    let r = cmd_distill(&ctxj, &pos, &kw);
    assert_eq!(r.code, EXIT_OK);
    let v = parse_json(&r.out).unwrap();
    assert_eq!(v.get("command").and_then(|x| x.as_str()), Some("distill"));
    assert_eq!(v.get("goal").and_then(|x| x.as_str()), Some("G-01"));
    assert_eq!(v.get("null").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(v.get("empty").and_then(|x| x.as_bool()), Some(true));
}

#[test]
fn cmd_revalidate_from_edges_and_json() {
    pin();
    let mut st = State::default();
    put(&mut st, plain(Kind::Y, "Y-01", "stale"));
    put(&mut st, plain(Kind::Y, "Y-02", "stale"));
    put(&mut st, plain(Kind::D, "D-01", "accepted"));
    put(&mut st, work("W-01", "feature", "done", "clear"));
    let (_dir, ctx) = project_with_state("rv", &st);
    let kw = vec![("from".to_string(), "D-01,W-01".to_string())];
    let r = cmd_revalidate(&ctx, &["Y-01".to_string()], &kw);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "");
    let st2 = load(&ctx, true).ok().expect("reload");
    let y1 = &st2.nodes["Y-01"];
    assert_eq!(y1.status, "active");
    assert_eq!(
        y1.lines("revalidation"),
        vec!["2031-01-01T00:00:00Z from=D-01,W-01".to_string()]
    );
    assert!(st2
        .edges
        .iter()
        .any(|e| e.from == "Y-01" && e.label == "distills" && e.to == "D-01"));
    assert!(st2
        .edges
        .iter()
        .any(|e| e.from == "W-01" && e.label == "produces" && e.to == "Y-01"));
    let (raw, _) = journal_read_nonempty_pairs(&ctx.journalpath());
    assert_eq!(raw.len(), 1);
    let rec = parse_json(&raw[0]).unwrap();
    let inv = rec.get("inv").unwrap();
    assert_eq!(inv.get("op").and_then(|x| x.as_str()), Some("revalidate_restore"));
    assert_eq!(inv.get("id").and_then(|x| x.as_str()), Some("Y-01"));
    assert_eq!(inv.get("old_status").and_then(|x| x.as_str()), Some("stale"));
    assert_eq!(inv.get("had_surface").and_then(|x| x.as_bool()), Some(false));
    let added = inv.get("added_edges").and_then(|x| x.as_arr()).unwrap();
    assert_eq!(added.len(), 2);
    let mut ctxj = CliCtx::new(ctx.root.clone());
    ctxj.json = true;
    let kw2 = vec![("from".to_string(), "D-01".to_string())];
    let r = cmd_revalidate(&ctxj, &["Y-02".to_string()], &kw2);
    assert_eq!(r.code, EXIT_OK);
    let v = parse_json(&r.out).unwrap();
    assert_eq!(v.get("command").and_then(|x| x.as_str()), Some("revalidate"));
    assert_eq!(v.get("id").and_then(|x| x.as_str()), Some("Y-02"));
    assert_eq!(v.get("status").and_then(|x| x.as_str()), Some("active"));
    assert_eq!(
        v.get("line").and_then(|x| x.as_str()),
        Some("2031-01-01T00:00:00Z from=D-01")
    );
}

#[test]
fn cmd_error_paths_byte_exact() {
    pin();
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    put(&mut st, work("W-01", "feature", "proposed", "clear"));
    put(&mut st, plain(Kind::Y, "Y-01", "stale"));
    put(&mut st, plain(Kind::Y, "Y-02", "active"));
    put(&mut st, plain(Kind::D, "D-02", "superseded"));
    put(&mut st, plain(Kind::B, "B-01", "invalidated_blocking"));
    let (_dir, ctx) = project_with_state("err", &st);
    let p = |s: &str| vec![s.to_string()];
    let kw = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];
    let r = cmd_archive(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "usage: grove archive <G-NN>\n"));
    let r = cmd_archive(&ctx, &p("G-09"), &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_NOTFOUND, "", ""));
    let r = cmd_archive(&ctx, &p("G-02"), &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_GUARD, "", "goal must be verified\n"));
    let r = cmd_distill(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "usage: grove distill <G-NN> [--null]\n"));
    let r = cmd_distill(&ctx, &p("G-09"), &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_NOTFOUND, "", "not found: G-09\n"));
    let r = cmd_distill(&ctx, &p("W-01"), &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "distill: W-01 is not a goal\n"));
    let r = cmd_distill(&ctx, &p("G-02"), &[]);
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_GUARD, "", "distill: G-02 is `unverified`; distillation happens at `verified`\n")
    );
    let r = cmd_revalidate(&ctx, &[], &[]);
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_ERR, "", "usage: grove revalidate <Y-NN> [--surface=p1,p2] [--from=ID,...]\n")
    );
    let r = cmd_revalidate(&ctx, &p("Y-09"), &kw("from", "D-02"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_NOTFOUND, "", "not found: Y-09\n"));
    let r = cmd_revalidate(&ctx, &p("W-01"), &kw("from", "D-02"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "revalidate: W-01 is not a discovery\n"));
    let r = cmd_revalidate(&ctx, &p("Y-02"), &kw("from", "D-02"));
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_GUARD, "", "revalidate: Y-02 is `active`, not `stale`\n")
    );
    let r = cmd_revalidate(&ctx, &p("Y-01"), &[]);
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_GUARD, "", "revalidate: refusing without payment; pass --surface=<paths> and/or --from=<ID>\n")
    );
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("surface", " , "));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "revalidate: --surface given but empty\n"));
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("from", " ,"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "revalidate: --from given but empty\n"));
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("surface", "nope.txt"));
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_GUARD, "", "revalidate: surface path does not exist under root: nope.txt\n")
    );
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("from", "D-99"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_GUARD, "", "revalidate: unknown --from id: D-99\n"));
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("from", "G-01"));
    assert_eq!(
        (r.code, r.out.as_str(), r.err.as_str()),
        (EXIT_GUARD, "", "revalidate: --from G-01 must reference W or D/Q/B\n")
    );
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("from", "D-02"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_GUARD, "", "revalidate: --from D-02 is superseded\n"));
    let r = cmd_revalidate(&ctx, &p("Y-01"), &kw("from", "B-01"));
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_GUARD, "", "revalidate: --from B-01 is invalidated\n"));
    let r = cmd_repair(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_ERR, "", "refusing without --confirm\n"));
}

#[test]
fn cmd_check_ok_text_and_json() {
    pin();
    let (_dir, ctx) = project_with_state("chk", &State::default());
    let r = cmd_check(&ctx, &[], &[]);
    assert_eq!((r.code, r.out.as_str(), r.err.as_str()), (EXIT_OK, "", "ok\n"));
    let mut ctxj = CliCtx::new(ctx.root.clone());
    ctxj.json = true;
    let r = cmd_check(&ctxj, &[], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.err, "");
    let v = parse_json(&r.out).unwrap();
    assert_eq!(v.get("command").and_then(|x| x.as_str()), Some("check"));
    assert_eq!(v.get("ok").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(v.get("errors").and_then(|x| x.as_arr()).unwrap().len(), 0);
}
