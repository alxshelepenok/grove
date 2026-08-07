mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::PathBuf;

const NOW: &str = "2031-01-01T00:00:00Z";

fn pin() {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(NOW).unwrap()));
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-statstest-{}-{}-{}",
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

fn recs(lines: &[&str]) -> Vec<Json> {
    lines.iter().map(|l| parse_json(l).expect("journal line")).collect()
}

fn ts(s: &str) -> i64 {
    parse_rfc3339_utc_second(s).expect("ts parses")
}

fn pget<'a>(d: &'a JuliaDict, key: &str) -> &'a JVal {
    d.iter_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("missing key {key}"))
}

fn mask_ts(s: &str) -> String {
    fn is_ts(w: &[u8]) -> bool {
        w.len() == 20
            && w[4] == b'-'
            && w[7] == b'-'
            && w[10] == b'T'
            && w[13] == b':'
            && w[16] == b':'
            && w[19] == b'Z'
            && w[0..4].iter().all(|c| c.is_ascii_digit())
            && w[5..7].iter().all(|c| c.is_ascii_digit())
            && w[8..10].iter().all(|c| c.is_ascii_digit())
            && w[11..13].iter().all(|c| c.is_ascii_digit())
            && w[14..16].iter().all(|c| c.is_ascii_digit())
            && w[17..19].iter().all(|c| c.is_ascii_digit())
    }
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() && i + 20 <= b.len() && is_ts(&b[i..i + 20]) {
            out.push_str("<ts>");
            i += 20;
        } else {
            let ch = s[i..].chars().next().expect("char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[test]
fn corpus_stats_scripted_text_and_json() {
    let sc = corpus_json("stats-scripted");
    let st = parse_fixture(&step_field(&sc, 13, "lock")).expect("fixture lock parses");
    let journal = step_field(&sc, 13, "journal");
    let mut recs = Vec::new();
    for (i, line) in journal.lines().enumerate() {
        let retimed = line.replacen(
            "\"ts\":\"<ts>\"",
            &format!("\"ts\":\"2026-01-01T{i:02}:00:00Z\""),
            1,
        );
        recs.push(parse_json(&retimed).expect("retimed line parses"));
    }
    let out = compute_stats(&st, &recs, NOW);
    assert_eq!(mask_ts(&out.text), step_field(&sc, 15, "stdout"));
    assert_eq!(
        mask_ts(&json_cli_out(out.payload)),
        step_field(&sc, 16, "stdout")
    );
}

#[test]
fn median_odd_and_even() {
    assert_eq!(stats_median(&mut vec![3.0, 1.0, 2.0]), 2.0);
    assert_eq!(stats_median(&mut vec![4.0, 1.0, 3.0, 2.0]), 2.5);
    assert_eq!(stats_median(&mut vec![7.5]), 7.5);
}

#[test]
fn cycle_time_skips_done_before_ready() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let recs = recs(&[
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T00:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"proposed","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T01:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"done","goal_statuses":{}}}"#,
    ]);
    let ivals = stats_intervals(&st, &recs, ts(NOW));
    let (classes, seconds, rows) = stats_cycle_time(&st, &ivals);
    assert_eq!(classes.len(), 0);
    assert!(seconds.is_empty());
    assert!(rows.is_empty());
    let out = compute_stats(&st, &recs, NOW);
    assert!(out.text.contains("  (no W with ready and done intervals)\n"));
}

#[test]
fn dor_first_pass_counts_rejects_before_progress_start() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "done", "clear"));
    put(&mut st, work("W-02", "feature", "done", "clear"));
    let recs = recs(&[
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T00:00:00Z","inv":{"op":"dor_reject","id":"W-01","missing":["ac"]}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T01:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T02:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"progress","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T03:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-02","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T04:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-02","old_w_status":"progress","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T05:00:00Z","inv":{"op":"dor_reject","id":"W-02","missing":["ac"]}}"#,
    ]);
    let ivals = stats_intervals(&st, &recs, ts(NOW));
    let (total, per_node, sorted, entries, first_pass, rate) = stats_dor(&st, &recs, &ivals);
    assert_eq!(total, 2);
    assert_eq!(sorted, vec![
        ("W-01".to_string(), 1),
        ("W-02".to_string(), 1),
    ]);
    assert_eq!(*pget(&per_node, "W-01"), JVal::Int(1));
    assert_eq!(*pget(&per_node, "W-02"), JVal::Int(1));
    assert_eq!(entries, 2);
    assert_eq!(first_pass, 1);
    assert_eq!(rate, JVal::Float(0.5));
    assert_eq!(stats_fmt_num(&rate), "0.5");
}

#[test]
fn undo_ratio_null_without_mutations() {
    let recs0 = recs(&[
        r#"{"v":1,"cmd":"undo","ts":"2026-01-01T00:00:00Z","inv":{"op":"undo","steps":2}}"#,
    ]);
    let (events, steps, mutations, ratio) = stats_undo(&recs0);
    assert_eq!((events, steps, mutations), (1, 2, 0));
    assert_eq!(ratio, JVal::Null);
    assert_eq!(stats_fmt_num(&ratio), "\u{2013}");
    let recs1 = recs(&[
        r#"{"v":1,"cmd":"add","ts":"2026-01-01T00:00:00Z","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"cmd":"undo","ts":"2026-01-01T01:00:00Z","inv":{"op":"undo","steps":3}}"#,
    ]);
    let (events, steps, mutations, ratio) = stats_undo(&recs1);
    assert_eq!((events, steps, mutations), (1, 3, 1));
    assert_eq!(ratio, JVal::Float(100.0));
    assert_eq!(stats_fmt_num(&ratio), "100.0");
}

#[test]
fn cv_replay_counts_failures_and_skips_points() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "proposed", "clear"));
    let recs = recs(&[
        r#"{"v":1,"cmd":"add","ts":"2026-01-01T00:00:00Z","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"cmd":"unlink","ts":"2026-01-01T01:00:00Z","inv":{"op":"unlink_edge","from":"W-01","label":"blocks","to":"W-02"}}"#,
    ]);
    let (series, failures) = stats_cv_series(&st, &recs, NOW);
    assert_eq!(failures, 1);
    assert_eq!(series.len(), 2);
    assert_eq!(series[0], ("2026-01-01T00:00:00Z".to_string(), 0, 0));
    assert_eq!(series[1], (NOW.to_string(), 0, 1));
}

#[test]
fn gates_overflow_paths_null_vs_int() {
    let st = State::default();
    let recs = recs(&[
        r#"{"v":1,"cmd":"gate","ts":"2026-01-01T00:00:00Z","inv":{"op":"gate","tw":1,"dones":2,"empty":false,"overflows":["x"],"invalidated":["a","b"],"overflow_counts":{"p":2,"q":3}}}"#,
        r#"{"v":1,"cmd":"gate","ts":"2026-01-01T01:00:00Z","inv":{"op":"gate","empty":true}}"#,
    ]);
    let ivals = stats_intervals(&st, &recs, ts(NOW));
    let (stale, revalidations, runs, empty, overflow, invalidated, gates) =
        stats_discovery(&st, &recs, &ivals);
    assert_eq!((stale, revalidations, runs, empty, overflow, invalidated), (0, 0, 2, 1, 1, 2));
    assert_eq!(gates.len(), 2);
    let JVal::Obj(g1) = &gates[0] else {
        panic!("gate record is an object");
    };
    assert_eq!(*pget(g1, "ts"), JVal::Str("2026-01-01T00:00:00Z".to_string()));
    assert_eq!(*pget(g1, "tw"), JVal::Int(1));
    assert_eq!(*pget(g1, "dones"), JVal::Int(2));
    assert_eq!(*pget(g1, "empty"), JVal::Bool(false));
    assert_eq!(*pget(g1, "overflow_events"), JVal::Int(1));
    assert_eq!(*pget(g1, "overflow_paths"), JVal::Int(5));
    assert_eq!(*pget(g1, "invalidated_events"), JVal::Int(2));
    let JVal::Obj(g2) = &gates[1] else {
        panic!("gate record is an object");
    };
    assert_eq!(*pget(g2, "tw"), JVal::Int(0));
    assert_eq!(*pget(g2, "dones"), JVal::Int(0));
    assert_eq!(*pget(g2, "empty"), JVal::Bool(true));
    assert_eq!(*pget(g2, "overflow_events"), JVal::Int(0));
    assert_eq!(*pget(g2, "overflow_paths"), JVal::Null);
    assert_eq!(*pget(g2, "invalidated_events"), JVal::Int(0));
}

#[test]
fn text_table_header_and_row_spacing() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "done", "clear"));
    let recs = recs(&[
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T00:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"proposed","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T01:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
    ]);
    let out = compute_stats(&st, &recs, NOW);
    assert!(out
        .text
        .contains("  class          n   mean h  median h    max h\n"));
    assert!(out
        .text
        .contains("  clear          1      1.0       1.0      1.0\n"));
}

#[test]
fn intervals_track_old_w_status_and_old_status() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "done", "clear"));
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    let recs = recs(&[
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T00:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T01:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"progress","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T02:00:00Z","inv":{"op":"set_status_plain","id":"G-01","old_status":"unverified"}}"#,
    ]);
    let now = ts(NOW);
    let ivals = stats_intervals(&st, &recs, now);
    let t0 = ts("2026-01-01T00:00:00Z");
    let t1 = ts("2026-01-01T01:00:00Z");
    let t2 = ts("2026-01-01T02:00:00Z");
    assert_eq!(
        ivals["W-01"],
        vec![
            (Some(t0), t0, "ready".to_string()),
            (Some(t0), t1, "progress".to_string()),
            (Some(t1), now, "done".to_string()),
        ]
    );
    assert_eq!(
        ivals["G-01"],
        vec![
            (Some(t0), t2, "unverified".to_string()),
            (Some(t2), now, "verified".to_string()),
        ]
    );
    let (_, seconds, rows) = stats_cycle_time(&st, &ivals);
    assert_eq!(seconds, vec![3600]);
    assert_eq!(rows, vec![("clear".to_string(), 1, 1.0, 1.0, 1.0)]);
}

#[test]
fn fmt_num_matches_julia_rounding() {
    assert_eq!(stats_fmt_num(&JVal::Null), "\u{2013}");
    assert_eq!(stats_fmt_num(&JVal::Int(7)), "7");
    assert_eq!(stats_fmt_num(&JVal::Float(3.0)), "3.0");
    assert_eq!(stats_fmt_num(&JVal::Float(0.125)), "0.12");
    assert_eq!(stats_fmt_num(&JVal::Float(2.5)), "2.5");
    assert_eq!(stats_fmt_num(&JVal::Float(1.0 / 3.0)), "0.33");
    assert_eq!(stats_fmt_num(&JVal::Float(0.0)), "0.0");
}

#[test]
fn cmd_stats_text_and_json_match_compute() {
    pin();
    let d = tmpdir("cmd");
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "done", "clear"));
    let grove = d.join(".grove");
    std::fs::create_dir_all(&grove).unwrap();
    std::fs::write(grove.join("state.lock"), serialize(&st)).unwrap();
    let lines = [
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T00:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"proposed","goal_statuses":{}}}"#,
        r#"{"v":1,"cmd":"set","ts":"2026-01-01T01:00:00Z","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
    ];
    std::fs::write(grove.join("journal.log"), lines.join("\n") + "\n").unwrap();
    let recs = recs(&lines);
    let ctx = CliCtx::new(d.to_string_lossy().into_owned());
    let r = cmd_stats(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, compute_stats(&st, &recs, NOW).text);
    let mut jctx = CliCtx::new(d.to_string_lossy().into_owned());
    jctx.json = true;
    let r2 = cmd_stats(&jctx, &[], &[]);
    assert_eq!(r2.code, EXIT_OK);
    assert_eq!(r2.out, json_cli_out(compute_stats(&st, &recs, NOW).payload));
}

fn pobj<'a>(d: &'a JuliaDict, key: &str) -> &'a JuliaDict {
    match pget(d, key) {
        JVal::Obj(o) => o,
        v => panic!("key {key} is not an object: {v:?}"),
    }
}

fn parr<'a>(d: &'a JuliaDict, key: &str) -> &'a [JVal] {
    match pget(d, key) {
        JVal::Arr(a) => a,
        v => panic!("key {key} is not an array: {v:?}"),
    }
}

#[test]
fn journal_records_carry_effective_session_token() {
    let d = tmpdir("session");
    let ctx = CliCtx::new(d.to_string_lossy().into_owned());
    let r = cmd_init(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_add(
        &ctx,
        &["a".to_string()],
        &[
            ("title".to_string(), "Area".to_string()),
            ("session".to_string(), "tok-x".to_string()),
        ],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_add(
        &ctx,
        &["a".to_string()],
        &[("title".to_string(), "Other".to_string())],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let (raw, recs) = journal_read_nonempty_pairs(&ctx.journalpath());
    assert_eq!(recs.len(), 2);
    assert_eq!(
        recs[0].get("session").and_then(|v| v.as_str()),
        Some("tok-x")
    );
    let want = effective_session_token(&d.to_string_lossy(), None);
    assert_eq!(
        recs[1].get("session").and_then(|v| v.as_str()),
        Some(want.as_str())
    );
    assert!(
        raw[0].contains(r#"{"v":1,"session":"tok-x","cmd":"add""#),
        "{}",
        raw[0]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn audit_sessions_group_commands_with_unknown_bucket() {
    let st = State::default();
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","session":"tok-a","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","session":"tok-a","inv":{"op":"set_title","id":"W-01","old":"x"}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:00:00Z","cmd":"gate","inv":{"op":"gate","tw":1,"dones":0,"empty":true,"overflows":[],"invalidated":[]}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let sess = pobj(pobj(&out.payload, "audit"), "sessions");
    assert_eq!(*pget(sess, "count"), JVal::Int(2));
    let per = parr(sess, "per_session");
    assert_eq!(per.len(), 2);
    let JVal::Obj(e0) = &per[0] else { panic!("entry obj") };
    assert_eq!(*pget(e0, "session"), JVal::Str("tok-a".to_string()));
    assert_eq!(*pget(e0, "commands"), JVal::Int(2));
    let JVal::Obj(e1) = &per[1] else { panic!("entry obj") };
    assert_eq!(*pget(e1, "session"), JVal::Str("unknown".to_string()));
    assert_eq!(*pget(e1, "commands"), JVal::Int(1));
    assert_eq!(*pget(sess, "mean"), JVal::Float(1.5));
    assert_eq!(*pget(sess, "median"), JVal::Float(1.5));
    assert_eq!(*pget(sess, "max"), JVal::Int(2));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    let sess = pobj(pobj(&e.payload, "audit"), "sessions");
    assert_eq!(*pget(sess, "count"), JVal::Int(0));
    assert_eq!(*pget(sess, "mean"), JVal::Null);
    assert_eq!(*pget(sess, "median"), JVal::Null);
    assert_eq!(*pget(sess, "max"), JVal::Null);
    assert!(e.text.contains("  sessions: 0 mean \u{2013} median \u{2013} max \u{2013}\n"));
}

#[test]
fn audit_human_truncates_long_session_tokens() {
    let st = State::default();
    let long_tok = "t".repeat(30);
    let line = format!(
        r#"{{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","session":"{long_tok}","inv":{{"op":"rm_node","id":"W-01"}}}}"#
    );
    let recs = recs(&[&line]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    assert!(
        out.text.contains(&format!("    {} 1\n", "t".repeat(24))),
        "{}",
        out.text
    );
    assert!(!out.text.contains(&"t".repeat(25)), "{}", out.text);
}

#[test]
fn archive_record_is_audit_only_and_not_undoable() {
    let d = tmpdir("archive");
    let ctx = CliCtx::new(d.to_string_lossy().into_owned());
    let r = cmd_init(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_add(
        &ctx,
        &["a".to_string()],
        &[("title".to_string(), "Area".to_string())],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_add(
        &ctx,
        &["g".to_string()],
        &[
            ("title".to_string(), "G".to_string()),
            ("area".to_string(), "A-01".to_string()),
        ],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_set(&ctx, &["G-01".to_string(), "status=verified".to_string()], &[]);
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_distill(
        &ctx,
        &["G-01".to_string()],
        &[("null".to_string(), String::new())],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let r = cmd_archive(&ctx, &["G-01".to_string()], &[]);
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let (raw, recs) = journal_read_nonempty_pairs(&ctx.journalpath());
    let last = recs.last().expect("journal nonempty");
    assert_eq!(last.get("cmd").and_then(|v| v.as_str()), Some("archive"));
    let inv = last.get("inv").expect("archive inv");
    assert_eq!(inv.get("op").and_then(|v| v.as_str()), Some("archive"));
    assert_eq!(inv.get("id").and_then(|v| v.as_str()), Some("G-01"));
    assert_eq!(
        inv.get("ids").and_then(|v| v.as_arr()),
        Some(&vec![Json::Str("G-01".to_string())])
    );
    assert!(last.get("session").is_some());
    assert!(recs[recs.len() - 2].get("session").is_some());
    assert!(!journal_record_mutation(last));
    assert!(
        journal_apply_inverse(&mut State::default(), inv).is_some(),
        "archive inverse falls through to unknown-op message"
    );
    assert!(
        raw.last()
            .expect("raw line")
            .contains(r#""inv":{"ids":["G-01"],"id":"G-01","op":"archive"}"#),
        "{}",
        raw.last().expect("raw line")
    );
    let r = cmd_undo(
        &ctx,
        &[],
        &[("steps".to_string(), "1".to_string())],
    );
    assert_eq!(r.code, EXIT_OK, "{}", r.err);
    let st = match load(&ctx, true) {
        Ok(s) => s,
        Err(e) => panic!("lock loads: {}", e.err),
    };
    assert!(st.nodes["G-01"].archived);
    assert_eq!(st.nodes["G-01"].status, "unverified");
    let (_, recs2) = journal_read_nonempty_pairs(&ctx.journalpath());
    assert!(recs2
        .iter()
        .any(|r| r.get("cmd").and_then(|v| v.as_str()) == Some("archive")));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn checkpoint_latency_both_series_plus_empty_case() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "progress", "clear"));
    put(&mut st, plain(Kind::Y, "Y-01", "active"));
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:30:00Z","cmd":"add","inv":{"op":"rm_node","id":"Y-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-01","missing":["goals(w) ≠ ∅"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T03:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"ts":"2026-01-01T04:30:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"Y-01","old_status":"proposed"}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let cpl = pobj(pobj(&out.payload, "audit"), "checkpoint_latency");
    let dor = pobj(cpl, "dor");
    assert_eq!(*pget(dor, "n"), JVal::Int(1));
    assert_eq!(*pget(dor, "mean_hours"), JVal::Float(2.0));
    assert_eq!(*pget(dor, "median_hours"), JVal::Float(2.0));
    assert_eq!(*pget(dor, "max_hours"), JVal::Float(2.0));
    let disc = pobj(cpl, "discovery");
    assert_eq!(*pget(disc, "n"), JVal::Int(1));
    assert_eq!(*pget(disc, "mean_hours"), JVal::Float(4.0));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    let cpl = pobj(pobj(&e.payload, "audit"), "checkpoint_latency");
    let dor = pobj(cpl, "dor");
    assert_eq!(*pget(dor, "n"), JVal::Int(0));
    assert_eq!(*pget(dor, "mean_hours"), JVal::Null);
    assert_eq!(*pget(dor, "median_hours"), JVal::Null);
    assert_eq!(*pget(dor, "max_hours"), JVal::Null);
    let disc = pobj(cpl, "discovery");
    assert_eq!(*pget(disc, "n"), JVal::Int(0));
    assert_eq!(*pget(disc, "mean_hours"), JVal::Null);
    assert!(parr(&e.payload, "surprise_series").is_empty());
}

#[test]
fn post_approval_invalidation_rate() {
    let mut st = State::default();
    put(&mut st, plain(Kind::B, "B-01", "invalidated_blocking"));
    put(&mut st, plain(Kind::B, "B-02", "validated"));
    put(&mut st, plain(Kind::B, "B-03", "testing"));
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","inv":{"op":"rm_node","id":"B-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:10:00Z","cmd":"add","inv":{"op":"rm_node","id":"B-02"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:20:00Z","cmd":"add","inv":{"op":"rm_node","id":"B-03"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"B-01","old_status":"testing"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:30:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"B-02","old_status":"testing"}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:00:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"B-01","old_status":"validated"}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let pai = pobj(pobj(&out.payload, "audit"), "post_approval_invalidation");
    assert_eq!(*pget(pai, "invalidated"), JVal::Int(1));
    assert_eq!(*pget(pai, "ever_validated"), JVal::Int(2));
    assert_eq!(*pget(pai, "rate"), JVal::Float(0.5));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    let pai = pobj(pobj(&e.payload, "audit"), "post_approval_invalidation");
    assert_eq!(*pget(pai, "rate"), JVal::Null);
}

#[test]
fn rework_covered_uncovered_split_with_reject_counts() {
    let mut st = State::default();
    let mut w1 = work("W-01", "feature", "ready", "clear");
    reflist(&mut w1, "surface", &["src/a.jl"]);
    let mut w2 = work("W-02", "feature", "ready", "clear");
    reflist(&mut w2, "surface", &["src/b.jl"]);
    let w3 = work("W-03", "feature", "ready", "clear");
    let mut w4 = work("W-04", "feature", "done", "clear");
    w4.archived = true;
    reflist(&mut w4, "surface", &["src/a.jl"]);
    let mut y1 = plain(Kind::Y, "Y-01", "active");
    reflist(&mut y1, "surface", &["src/a.jl"]);
    let mut y2 = plain(Kind::Y, "Y-02", "stale");
    reflist(&mut y2, "surface", &["src/b.jl"]);
    for n in [w1, w2, w3, w4, y1, y2] {
        put(&mut st, n);
    }
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-01","missing":["x"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-01","missing":["x"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-03","missing":["x"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T03:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-04","missing":["x"]}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let cov = pobj(pobj(&out.payload, "rework"), "covered");
    assert_eq!(*pget(cov, "w"), JVal::Int(2));
    assert_eq!(*pget(cov, "rejects"), JVal::Int(3));
    assert_eq!(*pget(cov, "mean_rejects"), JVal::Float(1.5));
    let per = parr(cov, "per_w");
    assert_eq!(per.len(), 2);
    let JVal::Obj(r0) = &per[0] else { panic!("row obj") };
    assert_eq!(*pget(r0, "id"), JVal::Str("W-01".to_string()));
    assert_eq!(*pget(r0, "rejects"), JVal::Int(2));
    let JVal::Obj(r1) = &per[1] else { panic!("row obj") };
    assert_eq!(*pget(r1, "id"), JVal::Str("W-04".to_string()));
    assert_eq!(*pget(r1, "rejects"), JVal::Int(1));
    let unc = pobj(pobj(&out.payload, "rework"), "uncovered");
    assert_eq!(*pget(unc, "w"), JVal::Int(2));
    assert_eq!(*pget(unc, "rejects"), JVal::Int(1));
    assert_eq!(*pget(unc, "mean_rejects"), JVal::Float(0.5));
    let per = parr(unc, "per_w");
    assert_eq!(per.len(), 2);
    let JVal::Obj(r0) = &per[0] else { panic!("row obj") };
    assert_eq!(*pget(r0, "id"), JVal::Str("W-02".to_string()));
    assert_eq!(*pget(r0, "rejects"), JVal::Int(0));
    let JVal::Obj(r1) = &per[1] else { panic!("row obj") };
    assert_eq!(*pget(r1, "id"), JVal::Str("W-03".to_string()));
    assert_eq!(*pget(r1, "rejects"), JVal::Int(1));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    let cov = pobj(pobj(&e.payload, "rework"), "covered");
    assert_eq!(*pget(cov, "w"), JVal::Int(0));
    assert_eq!(*pget(cov, "mean_rejects"), JVal::Null);
    let unc = pobj(pobj(&e.payload, "rework"), "uncovered");
    assert!(parr(unc, "per_w").is_empty());
}

#[test]
fn distill_yield_real_null_none_per_archived_goal() {
    let mut st = State::default();
    let mut g1 = plain(Kind::G, "G-01", "verified");
    g1.archived = true;
    let mut g2 = plain(Kind::G, "G-02", "verified");
    g2.archived = true;
    let mut g3 = plain(Kind::G, "G-03", "verified");
    g3.archived = true;
    let g4 = plain(Kind::G, "G-04", "verified");
    let mut w1 = work("W-01", "feature", "done", "clear");
    w1.archived = true;
    reflist(&mut w1, "goals", &["G-01"]);
    let mut d1 = plain(Kind::D, "D-01", "accepted");
    d1.archived = true;
    let y1 = plain(Kind::Y, "Y-01", "active");
    for n in [g1, g2, g3, g4, w1, d1, y1] {
        put(&mut st, n);
    }
    edge(&mut st, "W-01", "implements", "D-01");
    edge(&mut st, "Y-01", "distills", "D-01");
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"distill","inv":{"op":"distill","goal":"G-02","empty":true}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let dy = pobj(&out.payload, "distill_yield");
    assert_eq!(*pget(dy, "goals_with_real"), JVal::Int(1));
    assert_eq!(*pget(dy, "goals_null_attested"), JVal::Int(1));
    assert_eq!(*pget(dy, "goals_without"), JVal::Int(1));
    let goals = parr(dy, "goals");
    assert_eq!(goals.len(), 3);
    let JVal::Obj(e0) = &goals[0] else { panic!("entry obj") };
    assert_eq!(*pget(e0, "goal"), JVal::Str("G-01".to_string()));
    assert_eq!(*pget(e0, "status"), JVal::Str("real".to_string()));
    assert_eq!(
        *pget(e0, "discoveries"),
        JVal::Arr(vec![JVal::Str("Y-01".to_string())])
    );
    let JVal::Obj(e1) = &goals[1] else { panic!("entry obj") };
    assert_eq!(*pget(e1, "goal"), JVal::Str("G-02".to_string()));
    assert_eq!(*pget(e1, "status"), JVal::Str("null".to_string()));
    assert_eq!(*pget(e1, "discoveries"), JVal::Arr(vec![]));
    let JVal::Obj(e2) = &goals[2] else { panic!("entry obj") };
    assert_eq!(*pget(e2, "goal"), JVal::Str("G-03".to_string()));
    assert_eq!(*pget(e2, "status"), JVal::Str("none".to_string()));
    assert_eq!(*pget(e2, "discoveries"), JVal::Arr(vec![]));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    assert!(parr(pobj(&e.payload, "distill_yield"), "goals").is_empty());
}

#[test]
fn dor_first_pass_split_three_categories() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "progress", "clear"));
    put(&mut st, work("W-02", "feature", "progress", "clear"));
    put(&mut st, work("W-03", "feature", "progress", "clear"));
    put(&mut st, plain(Kind::Q, "Q-01", "answered"));
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:01:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-02"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:02:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-03"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:03:00Z","cmd":"add","inv":{"op":"rm_node","id":"Q-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-02","missing":["x"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:30:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"Q-01","old_status":"open"}}"#,
        r#"{"v":1,"ts":"2026-01-01T03:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-02","old_w_status":"ready","goal_statuses":{}}}"#,
        r#"{"v":1,"ts":"2026-01-01T04:00:00Z","cmd":"set","inv":{"op":"dor_reject","id":"W-03","missing":["x"]}}"#,
        r#"{"v":1,"ts":"2026-01-01T04:30:00Z","cmd":"set","inv":{"op":"set_title","id":"W-01","old":"a"}}"#,
        r#"{"v":1,"ts":"2026-01-01T05:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-03","old_w_status":"ready","goal_statuses":{}}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let fps = pobj(pobj(&out.payload, "dor"), "first_pass_split");
    assert_eq!(*pget(fps, "no_reject"), JVal::Int(1));
    assert_eq!(*pget(fps, "reject_discovery"), JVal::Int(1));
    assert_eq!(*pget(fps, "reject_plain"), JVal::Int(1));
    assert_eq!(*pget(fps, "discovery_rate"), JVal::Float(0.5));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    let fps = pobj(pobj(&e.payload, "dor"), "first_pass_split");
    assert_eq!(*pget(fps, "discovery_rate"), JVal::Null);
}

#[test]
fn surprise_series_delta_and_c_assignment() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "done", "clear"));
    put(&mut st, work("W-02", "feature", "done", "clear"));
    put(&mut st, plain(Kind::Q, "Q-01", "answered"));
    put(&mut st, plain(Kind::B, "B-01", "validated"));
    let recs = recs(&[
        r#"{"v":1,"ts":"2026-01-01T00:00:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:05:00Z","cmd":"add","inv":{"op":"rm_node","id":"W-02"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:10:00Z","cmd":"add","inv":{"op":"rm_node","id":"Q-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T00:15:00Z","cmd":"add","inv":{"op":"rm_node","id":"B-01"}}"#,
        r#"{"v":1,"ts":"2026-01-01T01:00:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"Q-01","old_status":"open"}}"#,
        r#"{"v":1,"ts":"2026-01-01T02:00:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"B-01","old_status":"testing"}}"#,
        r#"{"v":1,"ts":"2026-01-01T03:00:00Z","cmd":"set","inv":{"op":"set_status_plain","id":"B-01","old_status":"invalidated_blocking"}}"#,
        r#"{"v":1,"ts":"2026-01-01T04:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-01","old_w_status":"progress","goal_statuses":{}}}"#,
        r#"{"v":1,"ts":"2026-01-01T05:00:00Z","cmd":"gate","inv":{"op":"gate","tw":1,"dones":1,"empty":false,"overflows":["W-01","W-02"],"invalidated":[]}}"#,
        r#"{"v":1,"ts":"2026-01-01T06:00:00Z","cmd":"set","inv":{"op":"set_w_status_with_goals","id":"W-02","old_w_status":"progress","goal_statuses":{}}}"#,
    ]);
    let out = compute_stats(&st, &recs, "2026-01-02T00:00:00Z");
    let ss = parr(&out.payload, "surprise_series");
    assert_eq!(ss.len(), 2);
    let JVal::Obj(e0) = &ss[0] else { panic!("entry obj") };
    assert_eq!(*pget(e0, "id"), JVal::Str("W-01".to_string()));
    assert_eq!(
        *pget(e0, "ts"),
        JVal::Str("2026-01-01T04:00:00Z".to_string())
    );
    assert_eq!(*pget(e0, "delta"), JVal::Int(1));
    assert_eq!(*pget(e0, "c"), JVal::Int(2));
    let JVal::Obj(e1) = &ss[1] else { panic!("entry obj") };
    assert_eq!(*pget(e1, "id"), JVal::Str("W-02".to_string()));
    assert_eq!(
        *pget(e1, "ts"),
        JVal::Str("2026-01-01T06:00:00Z".to_string())
    );
    assert_eq!(*pget(e1, "delta"), JVal::Int(2));
    assert_eq!(*pget(e1, "c"), JVal::Int(2));
    let e = compute_stats(&State::default(), &[], "2026-01-02T00:00:00Z");
    assert!(parr(&e.payload, "surprise_series").is_empty());
    assert!(e.text.contains("surprise series:\n  \u{2013}\n"));
}
