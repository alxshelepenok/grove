mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::{Path, PathBuf};

const TS: &str = "2031-01-01T00:00:00Z";

fn pin(ts: &str) {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(ts).unwrap()));
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-gtest-{}-{}-{}",
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

fn kw(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn rec(s: &str) -> Json {
    parse_json(s).unwrap()
}

fn write_project(dir: &Path, lock: &str, journal: &str) {
    let grove = dir.join(".grove");
    std::fs::create_dir_all(&grove).unwrap();
    std::fs::write(grove.join("state.lock"), lock).unwrap();
    std::fs::write(grove.join("journal.log"), journal).unwrap();
}

fn journal_lines(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join(".grove/journal.log"))
        .unwrap()
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn git(dir: &Path, args: &[&str], date: Option<&str>) {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C").arg(dir).args(args).env("GIT_TERMINAL_PROMPT", "0");
    if let Some(d) = date {
        cmd.env("GIT_AUTHOR_DATE", d).env("GIT_COMMITTER_DATE", d);
    }
    let status = cmd.status().expect("spawn git");
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn stats_scripted_gate_step_matches_fixture() {
    pin(TS);
    let sc = corpus_json("stats-scripted");
    let dir = tmpdir("stats_scripted");
    let lock = step_field(&sc, 12, "lock").replace("<ts>", TS);
    let st = parse_fixture(&lock).expect("parse step 12 lock");
    let journal = step_field(&sc, 12, "journal")
        .replace("<ts>", TS)
        .replace("<session>", "testsession");
    write_project(&dir, &serialize(&st), &journal);
    let mut ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    ctx.quiet = true;
    let r = cmd_gate(&ctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    assert_eq!(r.out, step_field(&sc, 13, "stdout"));
    let lines = journal_lines(&dir);
    assert_eq!(lines.len(), 13);
    let want_last = step_field(&sc, 13, "journal");
    let tok = journal_session_token(&ctx, &[]);
    assert_eq!(
        lines
            .last()
            .unwrap()
            .replace(TS, "<ts>")
            .replace(&tok, "<session>"),
        want_last.lines().last().unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gate_scenario_two_runs_match_fixture() {
    let sc = corpus_json("gate");
    let dir = tmpdir("gate_scenario");
    git(&dir, &["init", "-q"], None);
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    git(&dir, &["add", "a.txt"], None);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "scaffold",
        ],
        Some(TS),
    );
    let lock = step_field(&sc, 16, "lock").replace("<ts>", TS);
    let st = parse_fixture(&lock).expect("parse step 16 lock");
    let journal = step_field(&sc, 16, "journal")
        .replace("<ts>", TS)
        .replace("<session>", "testsession");
    write_project(&dir, &serialize(&st), &journal);
    std::fs::write(dir.join("a.txt"), "a2\n").unwrap();
    std::fs::write(dir.join("b.txt"), "b\n").unwrap();
    git(&dir, &["add", "a.txt", "b.txt"], None);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "touch b for W-01",
        ],
        Some(TS),
    );
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    pin("2031-01-02T00:00:00Z");
    let r1 = cmd_gate(&ctx, &[], &kw(&[("theta", "0")]));
    assert_eq!(r1.code, 0);
    assert_eq!(r1.err, "");
    assert_eq!(r1.out, step_field(&sc, 22, "stdout"));
    let want_last1 = step_field(&sc, 22, "journal");
    let tok = journal_session_token(&ctx, &kw(&[("theta", "0")]));
    assert_eq!(
        journal_lines(&dir)
            .last()
            .unwrap()
            .replace("2031-01-02T00:00:00Z", "<ts>")
            .replace(&tok, "<session>"),
        want_last1.lines().last().unwrap()
    );
    pin("2031-01-03T00:00:00Z");
    let r2 = cmd_gate(&ctx, &[], &kw(&[("theta", "0")]));
    assert_eq!(r2.code, 0);
    assert_eq!(r2.err, "");
    assert_eq!(
        r2.out,
        step_field(&sc, 23, "stdout").replace("<ts>", "2031-01-02T00:00:00Z")
    );
    let got = journal_lines(&dir)
        .join("\n")
        .replace(TS, "<ts>")
        .replace("2031-01-02T00:00:00Z", "<ts>")
        .replace("2031-01-03T00:00:00Z", "<ts>")
        .replace(&tok, "<session>")
        .replace("testsession", "<session>");
    assert_eq!(got, step_field(&sc, 23, "journal").trim_end());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn baseline_last_gate_record_wins() {
    let recs = vec![
        rec(r#"{"v":1,"cmd":"gate","ts":"2030-01-01T00:00:00Z","inv":{"op":"gate","tw":1,"dones":2}}"#),
        rec(r#"{"v":1,"cmd":"set","ts":"2030-06-01T00:00:00Z","inv":{"op":"set_status_plain","id":"W-01"}}"#),
        rec(r#"{"v":1,"cmd":"gate","ts":"2030-12-31T00:00:00Z","inv":{"op":"gate","tw":3,"dones":4}}"#),
    ];
    let b = gate_baseline(&recs).expect("baseline");
    assert_eq!(b.ts, "2030-12-31T00:00:00Z");
    assert_eq!(b.tw, 3);
    assert_eq!(b.dones, 4);
}

#[test]
fn baseline_skips_non_gate_and_empty_ts() {
    let recs = vec![
        rec(r#"{"v":1,"cmd":"gate","ts":"2030-01-01T00:00:00Z","inv":{"op":"gate","tw":1,"dones":2}}"#),
        rec(r#"{"v":1,"cmd":"gate","ts":"   ","inv":{"op":"gate","tw":9,"dones":9}}"#),
        rec(r#"{"v":1,"cmd":"distill","ts":"2031-01-01T00:00:00Z","inv":{"op":"distill","empty":true}}"#),
        rec(r#"{"v":1,"cmd":"gate","ts":"2031-06-01T00:00:00Z"}"#),
        rec(r#"{}"#),
        Json::Null,
    ];
    let b = gate_baseline(&recs).expect("baseline");
    assert_eq!(b.ts, "2030-01-01T00:00:00Z");
    assert_eq!(b.tw, 1);
    assert_eq!(b.dones, 2);
    assert!(gate_baseline(&[]).is_none());
    let stripped = vec![rec(
        r#"{"v":1,"cmd":"gate","ts":"  2030-05-05T00:00:00Z  ","inv":{"op":"gate"}}"#,
    )];
    let b2 = gate_baseline(&stripped).expect("baseline");
    assert_eq!(b2.ts, "2030-05-05T00:00:00Z");
    assert_eq!(b2.tw, 0);
    assert_eq!(b2.dones, 0);
}

#[test]
fn scan_log_id_boundaries_and_accumulation() {
    let txt = "preamble noise\n\u{1}fix W-01,W-02 and XW-03 W-044 W-05x\nsrc/a.jl\nsrc/b.jl\n\n\u{1}wip W-01 again\nsrc/c.jl\nsrc/a.jl\n";
    let wids: Vec<String> = ["W-01", "W-02", "W-044", "W-05"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let out = gate_git_scan_log(txt, &wids);
    assert_eq!(out["W-01"], vec!["src/a.jl", "src/b.jl", "src/c.jl"]);
    assert_eq!(out["W-02"], vec!["src/a.jl", "src/b.jl"]);
    assert_eq!(out["W-044"], vec!["src/a.jl", "src/b.jl"]);
    assert!(out["W-05"].is_empty());
}

#[test]
fn scan_log_greedy_digits_and_dedup_within_commit() {
    let txt = "\u{1}land W-044 and W-04\nx.jl\n\u{1}again W-044 W-044\ny.jl\n";
    let wids: Vec<String> = ["W-04", "W-044"].iter().map(|s| s.to_string()).collect();
    let out = gate_git_scan_log(txt, &wids);
    assert_eq!(out["W-044"], vec!["x.jl", "y.jl"]);
    assert_eq!(out["W-04"], vec!["x.jl"]);
}

#[test]
fn git_files_by_w_since_cut() {
    let dir = tmpdir("gate_since");
    git(&dir, &["init", "-q"], None);
    std::fs::write(dir.join("a.txt"), "a\n").unwrap();
    git(&dir, &["add", "a.txt"], None);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "touch a for W-01",
        ],
        Some("2031-01-01T00:00:00Z"),
    );
    std::fs::write(dir.join("c.txt"), "c\n").unwrap();
    git(&dir, &["add", "c.txt"], None);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "touch c for W-01",
        ],
        Some("2031-01-03T00:00:00Z"),
    );
    let root = dir.to_string_lossy().into_owned();
    let wids: Vec<String> = ["W-01", "W-02"].iter().map(|s| s.to_string()).collect();
    let all = gate_git_files_by_w(&root, &wids, "");
    assert_eq!(all["W-01"], vec!["a.txt", "c.txt"]);
    assert!(all["W-02"].is_empty());
    let cut = gate_git_files_by_w(&root, &wids, "2031-01-02T00:00:00Z");
    assert_eq!(cut["W-01"], vec!["c.txt"]);
    assert!(cut["W-02"].is_empty());
    let empty = gate_git_files_by_w(&root, &[], "");
    assert!(empty.is_empty());
    let nonrepo = tmpdir("gate_nonrepo");
    let nr = gate_git_files_by_w(&nonrepo.to_string_lossy(), &wids, "");
    assert!(nr["W-01"].is_empty());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&nonrepo);
}

#[test]
fn cmd_gate_arg_errors() {
    let dir = tmpdir("gate_args");
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    let cases: [(&[(&str, &str)], &str); 4] = [
        (&[("theta", "abc")], "bad --theta (expected integer)\n"),
        (&[("theta", "-1")], "--theta must be ≥ 0\n"),
        (&[("n", "x")], "bad --n (expected integer)\n"),
        (&[("n", "0")], "--n must be ≥ 1\n"),
    ];
    for (pairs, want) in cases {
        let r = cmd_gate(&ctx, &[], &kw(pairs));
        assert_eq!(r.code, 1);
        assert_eq!(r.out, "");
        assert_eq!(r.err, want);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn json_payload_key_order_matches_julia() {
    let mut b = plain(Kind::B, "B-01", "invalidated_blocking");
    b.title = "Bet".to_string();
    let mut d = plain(Kind::D, "D-01", "accepted");
    d.title = "Dec".to_string();
    let rep = GateReport {
        baseline: Some(GateBaseline {
            ts: "2031-01-02T00:00:00Z".to_string(),
            tw: 0,
            dones: 1,
        }),
        tw_now: 0,
        tw_delta: 0,
        dones: 1,
        due: false,
        overflows: vec![("W-01".to_string(), vec!["b.txt".to_string()])],
        invalidated: vec![b],
        accepted: vec![d],
        empty: false,
        theta: 0,
        n: 5,
    };
    assert_eq!(
        json_cli_out(gate_json_payload(&rep)),
        "{\"dones\":1,\"due\":false,\"overflows\":[{\"w\":\"W-01\",\"paths\":[\"b.txt\"]}],\"command\":\"gate\",\"tw_now\":0,\"empty\":false,\"theta\":0,\"invalidated\":[{\"status\":\"invalidated_blocking\",\"id\":\"B-01\",\"title\":\"Bet\"}],\"tw_delta\":0,\"accepted\":[{\"id\":\"D-01\",\"title\":\"Dec\"}],\"baseline\":{\"dones\":1,\"tw\":0,\"ts\":\"2031-01-02T00:00:00Z\"},\"n\":5}\n"
    );
    let rep2 = GateReport {
        baseline: None,
        tw_now: 0,
        tw_delta: 0,
        dones: 1,
        due: false,
        overflows: Vec::new(),
        invalidated: Vec::new(),
        accepted: Vec::new(),
        empty: false,
        theta: 0,
        n: 5,
    };
    assert_eq!(
        json_cli_out(gate_json_payload(&rep2)),
        "{\"dones\":1,\"due\":false,\"overflows\":[],\"command\":\"gate\",\"tw_now\":0,\"empty\":false,\"theta\":0,\"invalidated\":[],\"tw_delta\":0,\"accepted\":[],\"baseline\":null,\"n\":5}\n"
    );
}

#[test]
fn gate_report_invalidated_accepted_and_cut() {
    let mut st = State::default();
    let mut b1 = plain(Kind::B, "B-01", "invalidated_acceptable");
    attr(&mut b1, "t_updated", TS);
    let mut b2 = plain(Kind::B, "B-02", "invalidated_blocking");
    attr(&mut b2, "t_updated", TS);
    let mut b3 = plain(Kind::B, "B-03", "invalidated_blocking");
    attr(&mut b3, "t_updated", TS);
    b3.archived = true;
    let mut b4 = plain(Kind::B, "B-04", "invalidated_blocking");
    attr(&mut b4, "t_updated", "2030-01-01T00:00:00Z");
    let mut d1 = plain(Kind::D, "D-01", "accepted");
    attr(&mut d1, "t_updated", TS);
    let mut d2 = plain(Kind::D, "D-02", "proposed");
    attr(&mut d2, "t_updated", TS);
    for n in [b1, b2, b3, b4, d1, d2] {
        put(&mut st, n);
    }
    let rep = gate_report(&st, &[], "", 0, 5);
    let inv_ids: Vec<&str> = rep.invalidated.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(inv_ids, vec!["B-01", "B-02", "B-04"]);
    let acc_ids: Vec<&str> = rep.accepted.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(acc_ids, vec!["D-01"]);
    assert!(rep.baseline.is_none());
    assert_eq!(rep.tw_now, 0);
    assert_eq!(rep.tw_delta, 0);
    assert_eq!(rep.dones, 0);
    assert!(!rep.due);
    assert!(!rep.empty);
    let recs = vec![rec(
        r#"{"v":1,"cmd":"gate","ts":"2030-06-01T00:00:00Z","inv":{"op":"gate","tw":2,"dones":0}}"#,
    )];
    let rep2 = gate_report(&st, &recs, "", 0, 5);
    let inv2: Vec<&str> = rep2.invalidated.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(inv2, vec!["B-01", "B-02"]);
    assert_eq!(rep2.tw_delta, -2);
    assert!(!rep2.empty);
}

#[test]
fn cmd_gate_text_baseline_and_negative_delta() {
    pin(TS);
    let dir = tmpdir("gate_text");
    let st = State::default();
    let journal = "{\"v\":1,\"cmd\":\"gate\",\"ts\":\"2030-12-31T00:00:00Z\",\"inv\":{\"op\":\"gate\",\"tw\":3,\"dones\":7,\"empty\":true,\"overflows\":[],\"overflow_counts\":{},\"invalidated\":[]}}\n";
    write_project(&dir, &serialize(&st), journal);
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    let r = cmd_gate(&ctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    assert_eq!(
        r.out,
        "baseline: 2030-12-31T00:00:00Z\ntreewidth: 0 (Δ -3)\ndone since baseline: 0\ndue: false\nwould distill: none\n"
    );
    let lines = journal_lines(&dir);
    assert_eq!(lines.len(), 2);
    assert!(lines[1].contains("\"cmd\":\"gate\""));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_gate_due_true_and_quiet_still_prints() {
    pin(TS);
    let dir = tmpdir("gate_due");
    let mut st = State::default();
    let mut w = work("W-01", "feature", "done", "clear");
    attr(&mut w, "t_updated", TS);
    put(&mut st, w);
    write_project(&dir, &serialize(&st), "");
    let mut ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    ctx.quiet = true;
    let r = cmd_gate(&ctx, &[], &kw(&[("n", "1")]));
    assert_eq!(r.code, 0);
    assert_eq!(r.err, "");
    assert_eq!(
        r.out,
        "baseline: none\ntreewidth: 0 (Δ +0)\ndone since baseline: 1\ndue: true\nwould distill: none\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
