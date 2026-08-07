mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::PathBuf;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-ptest-{}-{}-{}",
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

fn cmd_lock(st: &State) -> String {
    let mut st = st.clone();
    for n in st.nodes.values_mut() {
        for v in n.attrs.values_mut() {
            if v == "<ts>" {
                *v = "2026-01-01T00:00:00Z".to_string();
            }
        }
    }
    for e in st.edges.iter_mut() {
        if e.t_created.as_deref() == Some("<ts>") {
            e.t_created = Some("2026-01-01T00:00:00Z".to_string());
        }
    }
    serialize(&st)
}

fn ctx_with_state(tag: &str, st: &State) -> CliCtx {
    let dir = tmpdir(tag);
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    std::fs::create_dir_all(ctx.devdir()).unwrap();
    std::fs::write(ctx.lockpath(), cmd_lock(st)).unwrap();
    ctx
}

fn kw_of(args: &[String]) -> Vec<(String, String)> {
    args.iter()
        .filter_map(|a| a.strip_prefix("--"))
        .filter(|s| *s != "json")
        .map(|s| match s.find('=') {
            Some(eq) => (s[..eq].to_string(), s[eq + 1..].to_string()),
            None => (s.to_string(), "true".to_string()),
        })
        .collect()
}

#[test]
fn corpus_packet_cone_steps_byte_identical() {
    let sc = corpus_json("packet-cone");
    for (i, depth) in [(16usize, 4usize), (17, 4), (18, 1)] {
        let st = parse_fixture(&step_field(&sc, i - 1, "lock")).unwrap();
        let args = step_args(&sc, i);
        let w = st.nodes.get(&args[1]).unwrap();
        let got = format!("{}{}", packet(&st, w), packet_cone(&st, w, depth, 50));
        assert_eq!(got, step_field(&sc, i, "stdout"), "step {i} stdout");
        assert_eq!(step_exit(&sc, i), 0);
    }
}

#[test]
fn wave2b_next_log_cmd_steps_byte_identical() {
    let sc = corpus_json("next-log");
    for i in 31..=36usize {
        let st = parse_fixture(&step_field(&sc, i - 1, "lock")).unwrap();
        let args = step_args(&sc, i);
        let mut ctx = ctx_with_state("nextlog", &st);
        ctx.json = args.iter().any(|a| a == "--json");
        let pos: Vec<String> = args[1..]
            .iter()
            .filter(|a| !a.starts_with("--"))
            .cloned()
            .collect();
        let kw = kw_of(&args[1..]);
        let r = match args[0].as_str() {
            "ready" => cmd_ready(&ctx, &pos, &kw),
            "next" => cmd_next(&ctx, &pos, &kw),
            "packet" => cmd_packet(&ctx, &pos, &kw),
            other => panic!("unexpected command {other}"),
        };
        assert_eq!(r.code as i64, step_exit(&sc, i), "step {i} exit");
        assert_eq!(r.out, step_field(&sc, i, "stdout"), "step {i} stdout");
        assert_eq!(r.err, step_field(&sc, i, "stderr"), "step {i} stderr");
    }
}

#[test]
fn cmd_packet_error_paths() {
    let sc = corpus_json("packet-cone");
    let st = parse_fixture(&step_field(&sc, 15, "lock")).unwrap();
    let ctx = ctx_with_state("errs", &st);
    let pos = |s: &str| vec![s.to_string()];
    let kw = |k: &str, v: &str| vec![(k.to_string(), v.to_string())];

    let r = cmd_packet(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "usage: grove packet <W-NN>\n");

    let r = cmd_packet(&ctx, &pos("W-99"), &[]);
    assert_eq!(r.code, EXIT_NOTFOUND);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "not found\n");

    let r = cmd_packet(&ctx, &pos("G-01"), &[]);
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "not a work item\n");

    let r = cmd_packet(&ctx, &pos("W-03"), &kw("cone-depth", "x"));
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, "bad --cone-depth (expected integer)\n");

    let r = cmd_packet(&ctx, &pos("W-03"), &kw("cone-depth", "0"));
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, "--cone-depth must be ≥ 1\n");

    let r = cmd_packet(&ctx, &pos("W-03"), &kw("cone-max", "y"));
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, "bad --cone-max (expected integer)\n");

    let r = cmd_packet(&ctx, &pos("W-03"), &kw("cone-max", "0"));
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, "--cone-max must be ≥ 1\n");

    let r = cmd_packet(&ctx, &pos("W-99"), &kw("cone-depth", "0"));
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, "--cone-depth must be ≥ 1\n");
}

#[test]
fn cmd_packet_cone_text_matches_corpus() {
    let sc = corpus_json("packet-cone");
    let st = parse_fixture(&step_field(&sc, 15, "lock")).unwrap();
    let ctx = ctx_with_state("conetext", &st);
    let r = cmd_packet(
        &ctx,
        &["W-03".to_string()],
        &[("cone".to_string(), "true".to_string())],
    );
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, step_field(&sc, 16, "stdout"));
    assert_eq!(r.err, "");
}

#[test]
fn cmd_next_no_ready_items_reports_on_stderr() {
    let st = State::default();
    let ctx = ctx_with_state("empty", &st);
    let r = cmd_next(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, "");
    assert_eq!(r.err, "no ready work items\n");
}

#[test]
fn packet_header_prints_nothing_for_absent_type_and_cynefin() {
    let mut st = State::default();
    let mut w = node(Kind::W, "W-01");
    w.title = "Solo".to_string();
    put(&mut st, w);
    let pkt = packet(&st, st.nodes.get("W-01").unwrap());
    assert!(pkt.contains("type=nothing  status=proposed  cynefin=nothing\n"));
}

#[test]
fn packet_fitness_line_follows_julia_dict_slot_order() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    let mut w = work("W-01", "feature", "proposed", "clear");
    w.title = "Fit".to_string();
    reflist(&mut w, "goals", &["G-01", "G-02"]);
    fitness(&mut w, &[("G-01", 2), ("G-02", -1)]);
    put(&mut st, w);
    let pkt = packet(&st, st.nodes.get("W-01").unwrap());
    let mut d = JuliaDict::new();
    d.insert("G-01".to_string(), JVal::Int(2));
    d.insert("G-02".to_string(), JVal::Int(-1));
    let parts: Vec<String> = d
        .iter_pairs()
        .map(|(k, v)| {
            let v = match v {
                JVal::Int(i) => *i,
                _ => unreachable!(),
            };
            if v >= 0 {
                format!("{k}=+{v}")
            } else {
                format!("{k}={v}")
            }
        })
        .collect();
    let want = format!("**Fitness contribution:** {}\n", parts.join(", "));
    assert!(pkt.contains(&want), "packet missing {want:?}:\n{pkt}");
    assert!(pkt.contains("**Goals:** G-01, G-02\n"));
}

#[test]
fn packet_decision_assumption_question_sections_byte_layout() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    let mut w = work("W-01", "feature", "proposed", "clear");
    w.title = "Work".to_string();
    reflist(&mut w, "goals", &["G-01"]);
    put(&mut st, w);
    let mut d = plain(Kind::D, "D-01", "accepted");
    d.title = "Decide".to_string();
    prose(&mut d, "context", &["c1"]);
    prose(&mut d, "validation", &["v1"]);
    put(&mut st, d);
    let mut b = plain(Kind::B, "B-01", "validated");
    b.title = "Bet".to_string();
    b.cynefin = Some("clear".to_string());
    prose(&mut b, "vm", &["m1"]);
    put(&mut st, b);
    let mut q = plain(Kind::Q, "Q-01", "answered");
    q.title = "Quest".to_string();
    put(&mut st, q);
    edge(&mut st, "W-01", "implements", "D-01");
    edge(&mut st, "B-01", "targets", "W-01");
    edge(&mut st, "Q-01", "asks", "W-01");
    let got = packet(&st, st.nodes.get("W-01").unwrap());
    let want = "# Execution packet: W-01 (Work)\n\ntype=feature  status=proposed  cynefin=clear\n\n**Goals:** G-01\n\n## Decision D-01: Decide  (accepted)\n\n**context:**\n- c1\n\n**validation:**\n- v1\n\n## Assumption B-01: Bet  (validated, clear)\n**vm:**\n- m1\n\n## Question Q-01: Quest  (answered, nothing)\n\n## Definition of Ready\n\n- ⊤  goals(w) ≠ ∅ (G-01).\n- ⊥  AC(w) ≠ ∅ (0 entries).\n- ⊤  ∀ q ∈ asks(w), q terminal (Q-01).\n- ⊤  BChain validated (B-01).\n- ⊥  fitness deltas set ∀ g.\n- ⊥  evidence_strategy ≠ ∅ (0 entries).\n- ⊥  hypothesis ≠ ⊥.\n- ⊤  repro(w) ≠ ∅ ((non-bug)).\n- ⊤  exit(w) ≠ ∅ ((non-spike)).\n- ⊤  (A, causes, w) via materialised A ((non-refactor)).\n- ⊤  cynefin ≠ chaotic (clear).\n- ⊤  coverage(w) ≥ θ ((coverage not required)).\n\n**result: ⊥**\n";
    assert_eq!(got, want);
}

#[test]
fn packet_cone_fragility_lines_for_zero_one_two_paths() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, work("W-01", "feature", "proposed", "clear"));
    let mut w2 = work("W-02", "feature", "proposed", "clear");
    reflist(&mut w2, "goals", &["G-01"]);
    put(&mut st, w2);
    let cone = packet_cone(&st, st.nodes.get("W-02").unwrap(), 4, 50);
    assert!(cone.starts_with("\n## Contraction order\n\n"), "{cone}");
    assert!(cone.contains("- G-01: no blocks-path\n"), "{cone}");
    edge(&mut st, "G-01", "blocks", "W-01");
    edge(&mut st, "W-01", "blocks", "W-02");
    let cone = packet_cone(&st, st.nodes.get("W-02").unwrap(), 4, 50);
    assert!(cone.contains("- G-01: 1 (brittle)\n"), "{cone}");

    let mut d = State::default();
    put(&mut d, plain(Kind::G, "G-01", "unverified"));
    put(&mut d, work("W-01", "feature", "proposed", "clear"));
    put(&mut d, work("W-02", "feature", "proposed", "clear"));
    let mut w3 = work("W-03", "feature", "proposed", "clear");
    reflist(&mut w3, "goals", &["G-01"]);
    put(&mut d, w3);
    edge(&mut d, "G-01", "blocks", "W-01");
    edge(&mut d, "G-01", "blocks", "W-02");
    edge(&mut d, "W-01", "blocks", "W-03");
    edge(&mut d, "W-02", "blocks", "W-03");
    let cone = packet_cone(&d, d.nodes.get("W-03").unwrap(), 4, 50);
    assert!(cone.contains("- G-01: 2 disjoint blocks-paths\n"), "{cone}");

    let mut d2 = State::default();
    put(&mut d2, plain(Kind::G, "G-01", "unverified"));
    let mut w = work("W-01", "feature", "proposed", "clear");
    reflist(&mut w, "goals", &["G-01"]);
    put(&mut d2, w);
    edge(&mut d2, "G-01", "blocks", "W-01");
    let cone = packet_cone(&d2, d2.nodes.get("W-01").unwrap(), 4, 50);
    assert!(cone.contains("- G-01: 3 disjoint blocks-paths\n"), "{cone}");
}

#[test]
fn packet_cone_truncation_line_for_wide_cone() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "proposed", "clear"));
    for i in 0..60 {
        let id = format!("W-1{i:02}");
        put(&mut st, work(&id, "feature", "proposed", "clear"));
        edge(&mut st, &id, "blocks", "W-01");
    }
    let cone = packet_cone(&st, st.nodes.get("W-01").unwrap(), 4, 5);
    assert!(cone.ends_with("> cone truncated (depth=4, max=5)\n"), "{cone}");
    let numbered = cone
        .lines()
        .filter(|l| l.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
        .count();
    assert_eq!(numbered, 5, "{cone}");
    assert!(cone.contains("1. W-100  proposed"), "{cone}");
    assert!(cone.contains("5. W-104  proposed"), "{cone}");
}
