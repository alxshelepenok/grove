mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

fn conj_ok(st: &State, w: &Node, label: &str) -> bool {
    for (lb, ok, _) in dor_breakdown(st, w, false) {
        if lb == label {
            return ok;
        }
    }
    panic!("missing conjunct {label}");
}

fn wf_base(wid: &str, typ: &str) -> Node {
    let mut w = work(wid, typ, "proposed", "clear");
    reflist(&mut w, "goals", &["G-01"]);
    prose(&mut w, "ac", &["ac"]);
    prose(&mut w, "hypothesis", &["h"]);
    prose(&mut w, "evidence_strategy", &["e"]);
    fitness(&mut w, &[("G-01", 1)]);
    w
}

fn complex_feature_full(wid: &str, status: &str) -> Node {
    let mut w = work(wid, "feature", status, "complex");
    reflist(&mut w, "goals", &["G-01"]);
    prose(&mut w, "ac", &["a"]);
    prose(&mut w, "hypothesis", &["h"]);
    prose(&mut w, "evidence_strategy", &["e"]);
    fitness(&mut w, &[("G-01", 1)]);
    w
}

fn coverage_detail(st: &State, w: &Node) -> (bool, String) {
    for (lb, ok, detail) in dor_breakdown(st, w, false) {
        if lb == "coverage(w) ≥ θ" {
            return (ok, detail);
        }
    }
    panic!("missing coverage conjunct");
}

#[test]
fn dor_toggles_false_until_mandatory_fields_filled_then_true() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    let mut w = work("W-01", "feature", "proposed", "clear");
    put(&mut st, w.clone());
    assert!(!dor(&st, &w, false));
    reflist(&mut w, "goals", &["G-01"]);
    prose(&mut w, "ac", &["a"]);
    prose(&mut w, "hypothesis", &["h"]);
    prose(&mut w, "evidence_strategy", &["e"]);
    fitness(&mut w, &[("G-01", 1)]);
    put(&mut st, w.clone());
    assert!(dor(&st, &w, false));
}

#[test]
fn bug_spike_and_refactor_conjuncts_gate_readiness_independently() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));

    let mut wb = wf_base("W-B", "bug");
    put(&mut st, wb.clone());
    assert!(!dor(&st, &wb, false));
    prose(&mut wb, "repro", &["repro steps"]);
    put(&mut st, wb.clone());
    assert!(dor(&st, &wb, false));

    let mut ws = wf_base("W-S", "spike");
    ws.cynefin = Some("complex".to_string());
    put(&mut st, ws.clone());
    assert!(!dor(&st, &ws, false));
    prose(&mut ws, "exit", &["exit satisfied when D/Q/B recorded"]);
    put(&mut st, ws.clone());
    assert!(dor(&st, &ws, false));

    let mut wr = work("W-R", "refactor", "proposed", "clear");
    reflist(&mut wr, "goals", &["G-01"]);
    prose(&mut wr, "ac", &["a"]);
    prose(&mut wr, "evidence_strategy", &["e"]);
    fitness(&mut wr, &[("G-01", 1)]);
    put(&mut st, wr.clone());
    let mut t2 = plain(Kind::T, "T-02", "open");
    t2.archived = true;
    put(&mut st, t2);
    edge(&mut st, "T-02", "causes", "W-R");
    assert!(!dor(&st, &wr, false));
    put(&mut st, plain(Kind::T, "T-03", "open"));
    edge(&mut st, "T-03", "causes", "W-R");
    assert!(dor(&st, &wr, false));
}

#[test]
fn each_conjunct_fails_independently_on_synthetic_feature_refactor_shapes() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "verified"));
    put(&mut st, plain(Kind::G, "G-07", "verified"));

    let mut w1 = wf_base("W-X1", "feature");
    reflist(&mut w1, "goals", &[]);
    put(&mut st, w1.clone());
    assert!(!conj_ok(&st, &w1, "goals(w) ≠ ∅"));

    let mut w2 = wf_base("W-X2", "feature");
    prose(&mut w2, "ac", &[]);
    put(&mut st, w2.clone());
    assert!(!conj_ok(&st, &w2, "AC(w) ≠ ∅"));

    let w3 = wf_base("W-X3", "feature");
    put(&mut st, w3.clone());
    put(&mut st, plain(Kind::Q, "Q-99", "open"));
    edge(&mut st, "Q-99", "asks", "W-X3");
    assert!(!conj_ok(&st, &w3, "∀ q ∈ asks(w), q terminal"));
    st.nodes.get_mut("Q-99").unwrap().status = "answered".to_string();
    assert!(conj_ok(&st, &w3, "∀ q ∈ asks(w), q terminal"));

    let w4 = wf_base("W-X4", "feature");
    put(&mut st, w4.clone());
    put(&mut st, plain(Kind::B, "B-98", "invalidated_blocking"));
    edge(&mut st, "B-98", "targets", "W-X4");
    assert!(!conj_ok(&st, &w4, "BChain validated"));

    let mut w5 = wf_base("W-X5", "feature");
    fitness(&mut w5, &[]);
    put(&mut st, w5.clone());
    assert!(!conj_ok(&st, &w5, "fitness deltas set ∀ g"));

    let mut w6 = wf_base("W-X6", "feature");
    prose(&mut w6, "evidence_strategy", &[]);
    put(&mut st, w6.clone());
    assert!(!conj_ok(&st, &w6, "evidence_strategy ≠ ∅"));

    let mut w7 = wf_base("W-X7", "feature");
    prose(&mut w7, "hypothesis", &[]);
    put(&mut st, w7.clone());
    assert!(!conj_ok(&st, &w7, "hypothesis ≠ ⊥"));

    let mut w8 = wf_base("W-X8", "feature");
    w8.cynefin = Some("chaotic".to_string());
    put(&mut st, w8.clone());
    assert!(!conj_ok(&st, &w8, "cynefin ≠ chaotic"));

    let mut wb = wf_base("W-XB", "bug");
    prose(&mut wb, "repro", &[]);
    put(&mut st, wb.clone());
    assert!(!conj_ok(&st, &wb, "repro(w) ≠ ∅"));

    let mut ws = wf_base("W-XS", "spike");
    ws.cynefin = Some("complex".to_string());
    prose(&mut ws, "exit", &[]);
    put(&mut st, ws.clone());
    assert!(!conj_ok(&st, &ws, "exit(w) ≠ ∅"));

    let mut wr = work("W-XR", "refactor", "proposed", "clear");
    reflist(&mut wr, "goals", &["G-01"]);
    prose(&mut wr, "ac", &["a"]);
    prose(&mut wr, "evidence_strategy", &["e"]);
    fitness(&mut wr, &[("G-01", 1)]);
    put(&mut st, wr.clone());
    assert!(!conj_ok(&st, &wr, "(A, causes, w) via materialised A"));

    let mut wg = wf_base("W-XM", "feature");
    reflist(&mut wg, "goals", &["G-01", "G-07"]);
    fitness(&mut wg, &[("G-01", 1)]);
    put(&mut st, wg.clone());
    assert!(!conj_ok(&st, &wg, "fitness deltas set ∀ g"));
}

#[test]
fn active_discovery_surfaces_unions_active_surfaces_only() {
    let mut st = State::default();
    let mut xa = plain(Kind::Y, "Y-01", "active");
    reflist(&mut xa, "surface", &["src/a.jl", "src/b.jl"]);
    put(&mut st, xa);
    let mut xs = plain(Kind::Y, "Y-02", "stale");
    reflist(&mut xs, "surface", &["src/stale.jl"]);
    put(&mut st, xs);
    let mut xp = plain(Kind::Y, "Y-03", "proposed");
    reflist(&mut xp, "surface", &["src/proposed.jl"]);
    put(&mut st, xp);
    let mut xd = plain(Kind::Y, "Y-04", "superseded");
    reflist(&mut xd, "surface", &["src/dead.jl"]);
    put(&mut st, xd);
    put(&mut st, plain(Kind::Y, "Y-05", "active"));
    let s = active_discovery_surfaces(&st);
    let v: Vec<&str> = s.iter().map(|x| x.as_str()).collect();
    assert_eq!(v, vec!["src/a.jl", "src/b.jl"]);
}

#[test]
fn ratio_splits_declared_surface_into_covered_and_uncovered() {
    let mut st = State::default();
    let mut xa = plain(Kind::Y, "Y-01", "active");
    reflist(&mut xa, "surface", &["src/a.jl"]);
    put(&mut st, xa);
    let mut w = work("W-01", "feature", "proposed", "complex");
    put(&mut st, w.clone());
    let (ratio, covered, uncovered) = coverage(&st, &w);
    assert!(ratio == 0.0);
    assert!(covered.is_empty());
    assert!(uncovered.is_empty());
    reflist(&mut w, "surface", &["src/b.jl", "src/a.jl", "src/c.jl"]);
    put(&mut st, w.clone());
    let (ratio, covered, uncovered) = coverage(&st, &w);
    assert!((ratio - 1.0 / 3.0).abs() < 1e-12);
    assert_eq!(covered, vec!["src/a.jl"]);
    assert_eq!(uncovered, vec!["src/b.jl", "src/c.jl"]);
    let xa = st.nodes.get_mut("Y-01").unwrap();
    reflist(xa, "surface", &["src/a.jl", "src/b.jl", "src/c.jl"]);
    let (ratio, _, _) = coverage(&st, &w);
    assert!(ratio == 1.0);
}

#[test]
fn conjunct_inactive_by_default_and_dor_unaffected() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    let mut w = complex_feature_full("W-01", "proposed");
    reflist(&mut w, "surface", &["src/a.jl"]);
    put(&mut st, w.clone());
    assert!(dor(&st, &w, false));
    let (ok, detail) = coverage_detail(&st, &w);
    assert!(ok);
    assert_eq!(detail, "(coverage not required)");
}

#[test]
fn goal_attr_activates_conjunct_and_active_discovery_lifts_it() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "true");
    put(&mut st, g);
    let mut w = complex_feature_full("W-01", "proposed");
    reflist(&mut w, "surface", &["src/a.jl", "src/b.jl", "src/c.jl"]);
    put(&mut st, w.clone());
    let mut xa = plain(Kind::Y, "Y-01", "active");
    reflist(&mut xa, "surface", &["src/a.jl"]);
    put(&mut st, xa);
    assert!(!dor(&st, &w, false));
    let (ok, det) = coverage_detail(&st, &w);
    assert!(!ok);
    assert_eq!(det, "0.33 < 0.50; uncovered: src/b.jl, src/c.jl");
    let xa = st.nodes.get_mut("Y-01").unwrap();
    reflist(xa, "surface", &["src/a.jl", "src/b.jl", "src/c.jl"]);
    assert!(dor(&st, &w, false));
    let (ok, det) = coverage_detail(&st, &w);
    assert!(ok);
    assert_eq!(det, "1.00 ≥ 0.50");
    st.nodes.get_mut("Y-01").unwrap().status = "stale".to_string();
    assert!(!dor(&st, &w, false));
}

#[test]
fn uncovered_detail_caps_at_five_entries() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "0.9");
    put(&mut st, g);
    let mut w = work("W-01", "feature", "proposed", "complex");
    reflist(&mut w, "goals", &["G-01"]);
    reflist(
        &mut w,
        "surface",
        &[
            "src/a.jl", "src/b.jl", "src/c.jl", "src/d.jl", "src/e.jl", "src/f.jl", "src/g.jl",
            "src/h.jl",
        ],
    );
    put(&mut st, w.clone());
    let mut xa = plain(Kind::Y, "Y-01", "active");
    reflist(&mut xa, "surface", &["src/a.jl"]);
    put(&mut st, xa);
    let (ok, det) = coverage_detail(&st, &w);
    assert!(!ok);
    assert_eq!(
        det,
        "0.12 < 0.90; uncovered: src/b.jl, src/c.jl, src/d.jl, src/e.jl, src/f.jl … (+2 more)"
    );
}

#[test]
fn theta_parsing_and_max_over_carriers() {
    assert_eq!(parse_requires_coverage(Some("true")), Some(0.5));
    assert_eq!(parse_requires_coverage(Some("0.3")), Some(0.3));
    assert_eq!(parse_requires_coverage(Some("1")), Some(1.0));
    assert_eq!(parse_requires_coverage(Some("abc")), None);
    assert_eq!(parse_requires_coverage(Some("2")), None);
    assert_eq!(parse_requires_coverage(Some("0")), None);
    assert_eq!(parse_requires_coverage(Some("-0.5")), None);
    assert_eq!(parse_requires_coverage(Some("")), None);
    assert_eq!(parse_requires_coverage(None), None);

    let mut st = State::default();
    let mut g1 = plain(Kind::G, "G-01", "unverified");
    attr(&mut g1, "requires_coverage", "0.3");
    put(&mut st, g1);
    let mut g2 = plain(Kind::G, "G-02", "unverified");
    attr(&mut g2, "requires_coverage", "true");
    put(&mut st, g2);
    let mut t = plain(Kind::T, "T-01", "open");
    attr(&mut t, "requires_coverage", "0.7");
    put(&mut st, t);
    let mut w = work("W-01", "feature", "proposed", "complex");
    reflist(&mut w, "goals", &["G-01", "G-02", "G-99"]);
    put(&mut st, w.clone());
    assert_eq!(coverage_requirement(&st, &w), Some(0.5));
    single(&mut w, "theme", "T-01");
    put(&mut st, w.clone());
    assert_eq!(coverage_requirement(&st, &w), Some(0.7));
    reflist(&mut w, "goals", &[]);
    put(&mut st, w.clone());
    assert_eq!(coverage_requirement(&st, &w), Some(0.7));
    single(&mut w, "theme", "T-99");
    put(&mut st, w.clone());
    assert_eq!(coverage_requirement(&st, &w), None);
}

#[test]
fn theme_carried_attr_activates_conjunct() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    let mut t = plain(Kind::T, "T-01", "open");
    attr(&mut t, "requires_coverage", "true");
    put(&mut st, t);
    let mut w = complex_feature_full("W-01", "proposed");
    single(&mut w, "theme", "T-01");
    reflist(&mut w, "surface", &["src/a.jl"]);
    put(&mut st, w.clone());
    assert!(!dor(&st, &w, false));
}

#[test]
fn none_form_discovery_never_counts() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "true");
    put(&mut st, g);
    let mut x = plain(Kind::Y, "Y-01", "active");
    prose(&mut x, "why", &["process knowledge"]);
    put(&mut st, x);
    let mut w = work("W-01", "feature", "proposed", "complex");
    reflist(&mut w, "goals", &["G-01"]);
    reflist(&mut w, "surface", &["src/a.jl"]);
    put(&mut st, w.clone());
    assert!(active_discovery_surfaces(&st).is_empty());
    let (ratio, _, _) = coverage(&st, &w);
    assert!(ratio == 0.0);
}

#[test]
fn empty_declared_surface_fails_with_guidance() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "true");
    put(&mut st, g);
    let w = complex_feature_full("W-01", "proposed");
    put(&mut st, w.clone());
    assert!(!dor(&st, &w, false));
    let (ok, det) = coverage_detail(&st, &w);
    assert!(!ok);
    assert_eq!(det, "no declared surface; declare via field W-01 surface add …");
}

#[test]
fn non_feature_and_non_complex_are_exempt() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "true");
    put(&mut st, g);

    let mut wb = work("W-B", "bug", "proposed", "complex");
    reflist(&mut wb, "goals", &["G-01"]);
    prose(&mut wb, "ac", &["a"]);
    prose(&mut wb, "evidence_strategy", &["e"]);
    fitness(&mut wb, &[("G-01", 1)]);
    prose(&mut wb, "repro", &["repro"]);
    put(&mut st, wb.clone());
    assert!(dor(&st, &wb, false));

    let mut wf = work("W-F", "feature", "proposed", "clear");
    reflist(&mut wf, "goals", &["G-01"]);
    prose(&mut wf, "ac", &["a"]);
    prose(&mut wf, "evidence_strategy", &["e"]);
    fitness(&mut wf, &[("G-01", 1)]);
    prose(&mut wf, "hypothesis", &["h"]);
    put(&mut st, wf.clone());
    assert!(dor(&st, &wf, false));

    let mut ws = work("W-S", "spike", "proposed", "complex");
    reflist(&mut ws, "goals", &["G-01"]);
    prose(&mut ws, "ac", &["a"]);
    prose(&mut ws, "evidence_strategy", &["e"]);
    fitness(&mut ws, &[("G-01", 1)]);
    prose(&mut ws, "exit", &["exit"]);
    put(&mut st, ws.clone());
    assert!(dor(&st, &ws, false));

    for w in [&wb, &wf, &ws] {
        let (ok, detail) = coverage_detail(&st, w);
        assert!(ok);
        assert_eq!(detail, "(non-complex-feature)");
    }
}

#[test]
fn in_flight_progress_keeps_pinned_dor_when_discovery_goes_stale() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "requires_coverage", "true");
    put(&mut st, g);
    let mut w = complex_feature_full("W-01", "progress");
    reflist(&mut w, "surface", &["src/a.jl", "src/b.jl"]);
    put(&mut st, w.clone());
    let mut y = plain(Kind::Y, "Y-01", "active");
    reflist(&mut y, "surface", &["src/a.jl", "src/b.jl"]);
    put(&mut st, y);
    st.nodes.get_mut("Y-01").unwrap().status = "stale".to_string();
    assert!(dor(&st, &w, true));
    assert!(!dor(&st, &w, false));
    let mut found = false;
    for (lb, ok, detail) in dor_breakdown(&st, &w, true) {
        if lb == "coverage(w) ≥ θ" {
            assert!(ok);
            assert_eq!(detail, "(pinned at transition)");
            found = true;
        }
    }
    assert!(found);
}

#[test]
fn format_dor_report_ready_feature_exact_text() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, dor_ready_feature("W-01", "ready"));
    let report = format_dor_report(&st, "W-01").unwrap();
    assert!(report.starts_with("W-01 DoR:\n"));
    assert!(report.contains("  ⊤  goals(w) ≠ ∅  → G-01\n"));
    assert!(report.ends_with("result: ⊤\n"));
    let expected = "W-01 DoR:\n  ⊤  goals(w) ≠ ∅  → G-01\n  ⊤  AC(w) ≠ ∅  → 1 entries\n  ⊤  ∀ q ∈ asks(w), q terminal\n  ⊤  BChain validated\n  ⊤  fitness deltas set ∀ g  → G-01=+1\n  ⊤  evidence_strategy ≠ ∅  → 1 entries\n  ⊤  hypothesis ≠ ⊥\n  ⊤  repro(w) ≠ ∅  → (non-bug)\n  ⊤  exit(w) ≠ ∅  → (non-spike)\n  ⊤  (A, causes, w) via materialised A  → (non-refactor)\n  ⊤  cynefin ≠ chaotic  → clear\n  ⊤  coverage(w) ≥ θ  → (coverage not required)\nresult: ⊤\n";
    assert_eq!(report, expected);
}

#[test]
fn format_dor_report_empty_work_shows_failures() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "proposed", "clear"));
    let report = format_dor_report(&st, "W-01").unwrap();
    assert!(report.contains("result: ⊥"));
    assert!(report.contains("  ⊥  goals(w) ≠ ∅"));
    assert!(format_dor_report(&st, "W-99").is_none());
}

#[test]
fn dor_fitness_detail_julia_slot_order() {
    let mut st = State::default();
    for gid in ["G-01", "G-02", "G-03", "G-04"] {
        put(&mut st, plain(Kind::G, gid, "unverified"));
    }
    let mut w = work("W-01", "feature", "proposed", "clear");
    reflist(&mut w, "goals", &["G-01", "G-02", "G-03", "G-04"]);
    fitness(&mut w, &[("G-01", 2), ("G-02", -1), ("G-03", 5), ("G-04", -3)]);
    put(&mut st, w);
    let w = st.nodes.get("W-01").unwrap();
    let mut found = None;
    for (lb, _, detail) in dor_breakdown(&st, w, false) {
        if lb == "fitness deltas set ∀ g" {
            found = Some(detail);
        }
    }
    assert_eq!(found.unwrap(), "G-04=-3, G-01=+2, G-03=+5, G-02=-1");
}
