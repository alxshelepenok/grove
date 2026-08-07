mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

#[test]
fn i1_dor_on_progress_flags_incomplete_dor() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "progress", "clear"));
    assert_eq!(
        i1_dor_on_progress(&st),
        vec!["I1: W-01 is `progress` but DoR ≢ ⊤".to_string()]
    );
}

#[test]
fn i1_dor_on_progress_passes_with_full_dor() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, dor_ready_feature("W-01", "progress"));
    assert!(i1_dor_on_progress(&st).is_empty());
}

#[test]
fn i2_spike_outputs_requires_produces_edge() {
    let mut st = State::default();
    put(&mut st, work("W-01", "spike", "done", "clear"));
    assert_eq!(
        i2_spike_outputs(&st),
        vec!["I2: W-01 is a done spike but `produces` is empty (no outgoing `produces` edges)"
            .to_string()]
    );
    put(&mut st, plain(Kind::Q, "Q-01", "open"));
    edge(&mut st, "W-01", "produces", "Q-01");
    assert!(i2_spike_outputs(&st).is_empty());
}

#[test]
fn i2_spike_outputs_ignores_non_spike_done_and_proposed_spike() {
    let mut st = State::default();
    put(&mut st, work("W-02", "feature", "done", "clear"));
    assert!(i2_spike_outputs(&st).is_empty());
    let mut st2 = State::default();
    put(&mut st2, work("W-03", "spike", "proposed", "clear"));
    assert!(i2_spike_outputs(&st2).is_empty());
}

#[test]
fn i3_done_has_evidence_flags_empty_evidence() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-09", "verified"));
    let mut wd = work("W-D", "feature", "done", "clear");
    reflist(&mut wd, "goals", &["G-09"]);
    fitness(&mut wd, &[("G-09", 1)]);
    put(&mut st, wd);
    assert_eq!(
        i3_done_has_evidence(&st),
        vec!["I3: W-D is `done` but `evidence` is empty".to_string()]
    );
    let wd = st.nodes.get_mut("W-D").expect("W-D present");
    prose(wd, "evidence", &["x"]);
    assert!(i3_done_has_evidence(&st).is_empty());
}

#[test]
fn i4_wip_limit_counts_progress_work() {
    let mut st = State::default();
    for (i, id) in ["W-W1", "W-W2", "W-W3"].iter().enumerate() {
        put(&mut st, work(id, "feature", "progress", "clear"));
        let r = i4_wip_limit(&st);
        if i < 2 {
            assert!(r.is_empty());
        } else {
            assert_eq!(r.len(), 1);
            assert_eq!(r, vec!["I4: WIP 3 exceeds limit 2".to_string()]);
            assert_eq!(
                i4_wip_limit_with(&st, 2),
                vec!["I4: WIP 3 exceeds limit 2".to_string()]
            );
        }
    }
    let empty_st = State::default();
    assert!(i4_wip_limit(&empty_st).is_empty());
}

#[test]
fn i5_blocks_terminal_flags_missing_blocker() {
    let mut st = State::default();
    put(&mut st, work("W-P", "feature", "progress", "clear"));
    edge(&mut st, "G-XX", "blocks", "W-P");
    assert_eq!(
        i5_blocks_terminal(&st),
        vec!["I5: W-P blocked by missing G-XX".to_string()]
    );
}

#[test]
fn i5_blocks_terminal_flags_unverified_goal_blocker() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-V", "unverified"));
    put(&mut st, work("W-P", "feature", "progress", "clear"));
    edge(&mut st, "G-V", "blocks", "W-P");
    assert_eq!(
        i5_blocks_terminal(&st),
        vec!["I5: W-P is `progress` but blocker G-V (unverified) does not satisfy blocks clearance (goals must be verified)".to_string()]
    );
    st.nodes.get_mut("G-V").expect("G-V present").status = "verified".to_string();
    assert!(i5_blocks_terminal(&st).is_empty());
}

#[test]
fn i7_blocks_dag_detects_cycle() {
    let mut st = State::default();
    for id in ["W-01", "W-02", "W-03"] {
        put(&mut st, work(id, "feature", "ready", "clear"));
    }
    edge(&mut st, "W-01", "blocks", "W-02");
    edge(&mut st, "W-02", "blocks", "W-03");
    assert!(i7_blocks_dag(&st).is_empty());
    edge(&mut st, "W-03", "blocks", "W-01");
    assert_eq!(
        i7_blocks_dag(&st),
        vec!["I7: blocks graph contains a cycle".to_string()]
    );
}

#[test]
fn i9_feature_bchain_requires_validated_bet() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-R", "verified"));
    let mut wf9 = work("W-I9", "feature", "ready", "clear");
    reflist(&mut wf9, "goals", &["G-R"]);
    put(&mut st, wf9);
    put(&mut st, plain(Kind::B, "B-77", "proposed"));
    edge(&mut st, "B-77", "targets", "W-I9");
    assert_eq!(
        i9_feature_bchain(&st),
        vec!["I9: W-I9 is `ready` but B-77 is `proposed`".to_string()]
    );
    st.nodes.get_mut("B-77").expect("B-77 present").status = "validated".to_string();
    assert!(i9_feature_bchain(&st).is_empty());
}

#[test]
fn i10_done_fitness_requires_delta_per_goal() {
    let mut st = State::default();
    let mut w10 = work("W-10", "feature", "done", "clear");
    reflist(&mut w10, "goals", &["G-Q"]);
    fitness(&mut w10, &[]);
    put(&mut st, plain(Kind::G, "G-Q", "verified"));
    put(&mut st, w10);
    assert_eq!(
        i10_done_fitness(&st),
        vec!["I10: W-10 is `done` but no fitness delta for G-Q".to_string()]
    );
    let w10 = st.nodes.get_mut("W-10").expect("W-10 present");
    fitness(w10, &[("G-Q", 1)]);
    assert!(i10_done_fitness(&st).is_empty());
}

#[test]
fn i11_progress_flags_missing_session_claim() {
    let mut st = State::default();
    put(&mut st, work("W-I11", "feature", "progress", "clear"));
    assert_eq!(
        i11_progress_has_session_claim(&st),
        vec!["I11: W-I11 is `progress` but has no session token".to_string()]
    );
    let w = st.nodes.get_mut("W-I11").expect("W-I11 present");
    attr(w, "session", "tok");
    attr(w, "session_at", "2099-01-01T00:00:00Z");
    assert!(i11_progress_has_session_claim(&st).is_empty());
}

#[test]
fn i12_discovery_anchor_issues_reports_all_three() {
    let mut st = State::default();
    put(&mut st, plain(Kind::Y, "Y-01", "proposed"));
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec![
            "I12: Y-01 has no provenance edge (needs `produces` from a W or `distills` to a D/Q/B)"
                .to_string(),
            "I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string(),
            "I12: Y-01 has empty `tags` (≥1 glossary term required)".to_string(),
        ]
    );
}

#[test]
fn i12_produces_edge_from_w_clears_provenance() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, plain(Kind::Y, "Y-01", "proposed"));
    edge(&mut st, "W-01", "produces", "Y-01");
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec![
            "I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string(),
            "I12: Y-01 has empty `tags` (≥1 glossary term required)".to_string(),
        ]
    );
    let y = st.nodes.get_mut("Y-01").expect("Y-01 present");
    reflist(y, "tags", &["term"]);
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec!["I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string()]
    );
    let y = st.nodes.get_mut("Y-01").expect("Y-01 present");
    reflist(y, "surface", &["src/x.jl"]);
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert!(discovery_anchor_issues(&st, y).is_empty());
}

#[test]
fn i12_surface_only_clears_anchor_requirement() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let mut y = plain(Kind::Y, "Y-01", "proposed");
    reflist(&mut y, "tags", &["term"]);
    put(&mut st, y);
    edge(&mut st, "W-01", "produces", "Y-01");
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec!["I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string()]
    );
    let y = st.nodes.get_mut("Y-01").expect("Y-01 present");
    reflist(y, "surface", &["src/x.jl"]);
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert!(discovery_anchor_issues(&st, y).is_empty());
}

#[test]
fn i12_why_only_clears_anchor_requirement() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let mut y = plain(Kind::Y, "Y-01", "proposed");
    reflist(&mut y, "tags", &["term"]);
    put(&mut st, y);
    edge(&mut st, "W-01", "produces", "Y-01");
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec!["I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string()]
    );
    let y = st.nodes.get_mut("Y-01").expect("Y-01 present");
    prose(y, "why", &["because"]);
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert!(discovery_anchor_issues(&st, y).is_empty());
}

#[test]
fn i12_distills_to_d_satisfies_provenance() {
    let mut st = State::default();
    put(&mut st, plain(Kind::D, "D-01", "proposed"));
    put(&mut st, plain(Kind::Y, "Y-01", "proposed"));
    edge(&mut st, "Y-01", "distills", "D-01");
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert_eq!(
        discovery_anchor_issues(&st, y),
        vec![
            "I12: Y-01 has empty `surface` and empty `why` (≥1 anchor required)".to_string(),
            "I12: Y-01 has empty `tags` (≥1 glossary term required)".to_string(),
        ]
    );
}

#[test]
fn i12_archived_y_has_no_issues() {
    let mut st = State::default();
    let mut y = plain(Kind::Y, "Y-01", "proposed");
    y.archived = true;
    put(&mut st, y);
    let y = st.nodes.get("Y-01").expect("Y-01 present");
    assert!(discovery_anchor_issues(&st, y).is_empty());
}

#[test]
fn i13_goal_without_area_is_flagged() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    assert_eq!(
        check_area_membership(&st),
        vec!["I13: G-01 has no `area` field (every goal belongs to an area: `grove set G-01 area=A-NN`)".to_string()]
    );
}

#[test]
fn i13_area_referencing_missing_node_is_flagged() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    single(&mut g, "area", "A-99");
    put(&mut st, g);
    assert_eq!(
        check_area_membership(&st),
        vec!["I13: G-01 area A-99 does not reference an existing area (a) node".to_string()]
    );
}

#[test]
fn i13_valid_area_membership_passes() {
    let mut st = State::default();
    put(&mut st, plain(Kind::A, "A-01", "present"));
    let mut g = plain(Kind::G, "G-01", "unverified");
    single(&mut g, "area", "A-01");
    put(&mut st, g);
    assert!(check_area_membership(&st).is_empty());
}

#[test]
fn i13_archived_goal_without_area_is_still_flagged() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    g.archived = true;
    put(&mut st, g);
    assert_eq!(
        check_area_membership(&st),
        vec!["I13: G-01 has no `area` field (every goal belongs to an area: `grove set G-01 area=A-NN`)".to_string()]
    );
}

#[test]
fn check_orphan_edges_flags_missing_endpoint() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    edge(&mut st, "X-99", "blocks", "W-01");
    assert_eq!(
        check_orphan_edges(&st),
        vec!["edge endpoint missing: X-99".to_string()]
    );
}

#[test]
fn check_edge_types_accepts_valid_produces_targets_tests() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, plain(Kind::B, "B-01", "proposed"));
    put(&mut st, plain(Kind::Q, "Q-01", "open"));
    put(&mut st, plain(Kind::D, "D-01", "proposed"));
    edge(&mut st, "B-01", "targets", "W-01");
    edge(&mut st, "W-01", "produces", "Q-01");
    edge(&mut st, "W-01", "produces", "D-01");
    assert!(check_edge_types(&st).is_empty());
    edge(&mut st, "B-01", "tests", "Q-01");
    assert!(check_edge_types(&st).is_empty());
}

#[test]
fn check_edge_types_rejects_w_produces_w() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    edge(&mut st, "W-01", "produces", "W-01");
    assert_eq!(
        check_edge_types(&st),
        vec!["edge type mismatch: W-01 -produces-> W-01".to_string()]
    );
}

#[test]
fn check_edge_types_rejects_causes_from_w() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, work("W-02", "feature", "ready", "clear"));
    edge(&mut st, "W-01", "causes", "W-02");
    assert_eq!(
        check_edge_types(&st),
        vec!["edge type mismatch: W-01 -causes-> W-02".to_string()]
    );
}

#[test]
fn validate_and_push_edge_success_stamps_and_dedupes() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, work("W-02", "feature", "ready", "clear"));
    assert_eq!(validate_and_push_edge(&mut st, "W-01", "blocks", "W-02", true), None);
    assert_eq!(st.edges.len(), 1);
    let t = st.edges[0].t_created.clone().expect("t_created stamped");
    assert!(parse_rfc3339_utc_second(&t).is_some());
    assert_eq!(validate_and_push_edge(&mut st, "W-01", "blocks", "W-02", true), None);
    assert_eq!(st.edges.len(), 1);
}

#[test]
fn validate_and_push_edge_rejects_unknown_label() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, work("W-02", "feature", "ready", "clear"));
    assert_eq!(
        validate_and_push_edge(&mut st, "W-01", "frobnicate", "W-02", true),
        Some("unknown edge label: frobnicate".to_string())
    );
    assert!(st.edges.is_empty());
}

#[test]
fn validate_and_push_edge_rejects_missing_endpoint() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    assert_eq!(
        validate_and_push_edge(&mut st, "W-99", "blocks", "W-01", true),
        Some("missing node W-99".to_string())
    );
    assert!(st.edges.is_empty());
}

#[test]
fn validate_and_push_edge_rejects_blocks_cycle_and_rolls_back() {
    let mut st = State::default();
    for id in ["W-01", "W-02", "W-03"] {
        put(&mut st, work(id, "feature", "ready", "clear"));
    }
    assert_eq!(validate_and_push_edge(&mut st, "W-01", "blocks", "W-02", true), None);
    assert_eq!(validate_and_push_edge(&mut st, "W-02", "blocks", "W-03", true), None);
    let before = st.edges.len();
    assert_eq!(
        validate_and_push_edge(&mut st, "W-03", "blocks", "W-01", true),
        Some("I7: blocks introduces a cycle".to_string())
    );
    assert_eq!(st.edges.len(), before);
}

#[test]
fn validate_and_push_edge_rejects_type_violation_and_rolls_back() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let before = st.edges.len();
    assert_eq!(
        validate_and_push_edge(&mut st, "W-01", "produces", "W-01", true),
        Some("edge type mismatch: W-01 -produces-> W-01".to_string())
    );
    assert_eq!(st.edges.len(), before);
}

#[test]
fn check_all_concatenates_in_julia_order() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "progress", "clear"));
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    let msgs = check_all(&st);
    assert_eq!(
        msgs,
        vec![
            "I1: W-01 is `progress` but DoR ≢ ⊤".to_string(),
            "I11: W-01 is `progress` but has no session token".to_string(),
            "I13: G-01 has no `area` field (every goal belongs to an area: `grove set G-01 area=A-NN`)".to_string(),
        ]
    );
    assert!(msgs.first().expect("first").starts_with("I1:"));
    assert!(msgs.last().expect("last").starts_with("I13:"));
}
