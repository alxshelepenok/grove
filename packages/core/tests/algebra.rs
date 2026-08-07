mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::collections::BTreeSet;

fn feature_full(id: &str, status: &str) -> Node {
    let mut w = work(id, "feature", status, "clear");
    reflist(&mut w, "goals", &["G-01"]);
    fitness(&mut w, &[("G-01", 1)]);
    prose(&mut w, "ac", &["x"]);
    prose(&mut w, "hypothesis", &["x"]);
    prose(&mut w, "evidence_strategy", &["x"]);
    w
}

#[test]
fn bchain_collects_assumption_linked_by_targets_edge() {
    let mut st = State::default();
    put(&mut st, feature_full("W-01", "ready"));
    put(&mut st, plain(Kind::B, "B-01", "validated"));
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    edge(&mut st, "B-01", "targets", "W-01");
    let w = st.nodes.get("W-01").unwrap();
    assert!(bchain(&st, w).contains(&"B-01".to_string()));
}

#[test]
fn bchain_collects_assumption_via_tests_and_question_asks_work() {
    let mut st = State::default();
    put(&mut st, feature_full("W-01", "ready"));
    put(&mut st, plain(Kind::Q, "Q-01", "answered"));
    put(&mut st, plain(Kind::B, "B-01", "validated"));
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    edge(&mut st, "B-01", "tests", "Q-01");
    edge(&mut st, "Q-01", "asks", "W-01");
    let w = st.nodes.get("W-01").unwrap();
    assert_eq!(bchain(&st, w), vec!["B-01"]);
}

#[test]
fn refactor_conjunct_lists_materialised_artifacts_sorted_omitting_archived() {
    let mut st = State::default();
    let mut wr = work("W-01", "refactor", "ready", "clear");
    reflist(&mut wr, "goals", &["G-01"]);
    fitness(&mut wr, &[("G-01", 1)]);
    prose(&mut wr, "ac", &["x"]);
    prose(&mut wr, "evidence_strategy", &["e"]);
    put(&mut st, wr);
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    for (id, arch) in [("T-02", false), ("T-01", false), ("T-09", true)] {
        let mut a = plain(Kind::T, id, "open");
        a.archived = arch;
        put(&mut st, a);
        if !arch {
            edge(&mut st, id, "causes", "W-01");
        }
    }
    let wr = st.nodes.get("W-01").unwrap();
    let (ok, detail) = refactor_materialised_root_cause(&st, wr);
    assert!(ok);
    assert_eq!(detail, "T-01, T-02");
}

#[test]
fn rederive_opens_artifact_when_no_themed_work_remains() {
    let mut st = State::default();
    put(&mut st, plain(Kind::T, "T-01", "done"));
    put(&mut st, work("W-01", "feature", "done", "clear"));
    rederive_artifacts(&mut st);
    assert_eq!(st.nodes["T-01"].status, "open");
}

#[test]
fn rederive_closes_artifact_when_all_themed_work_terminal() {
    let mut st = State::default();
    put(&mut st, plain(Kind::T, "T-01", "open"));
    let mut w1 = work("W-01", "feature", "done", "clear");
    single(&mut w1, "theme", "T-01");
    put(&mut st, w1);
    let mut w2 = work("W-02", "feature", "ready", "clear");
    single(&mut w2, "theme", "T-01");
    put(&mut st, w2);
    rederive_artifacts(&mut st);
    assert_eq!(st.nodes["T-01"].status, "open");
    st.nodes.get_mut("W-02").unwrap().status = "done".to_string();
    rederive_artifacts(&mut st);
    assert_eq!(st.nodes["T-01"].status, "done");
}

#[test]
fn preds_clear_requires_verified_goals_on_blocks_edges() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "declined"));
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    edge(&mut st, "G-01", "blocks", "W-01");
    assert!(!preds_clear(&st, "W-01"));
    st.nodes.get_mut("G-01").unwrap().status = "verified".to_string();
    assert!(preds_clear(&st, "W-01"));
    put(&mut st, work("W-02", "feature", "progress", "clear"));
    put(&mut st, plain(Kind::G, "G-02", "declined"));
    edge(&mut st, "G-02", "blocks", "W-02");
    assert!(!i5_blocks_terminal(&st).is_empty());
    st.nodes.get_mut("G-02").unwrap().status = "verified".to_string();
    assert!(i5_blocks_terminal(&st).is_empty());
}

#[test]
fn blocked_by_deps_impact_critical_path_and_ready_helpers() {
    let mut st = State::default();
    for id in ["W-01", "W-02", "W-03", "W-04"] {
        put(&mut st, feature_full(id, "ready"));
    }
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    edge(&mut st, "W-01", "blocks", "W-02");
    edge(&mut st, "W-02", "blocks", "W-03");
    edge(&mut st, "W-01", "blocks", "W-04");
    assert!(blocked_by(&st, "W-02").contains(&"W-01".to_string()));
    assert_eq!(deps(&st, "W-03"), vec!["W-01", "W-02"]);
    let mut im = impact(&st, "W-01");
    im.sort();
    assert_eq!(im, vec!["W-02", "W-03", "W-04"]);
    assert_eq!(critical_path(&st), vec!["W-01", "W-02", "W-03"]);
    let rs = ready(&st);
    let ids: Vec<&str> = rs.iter().map(|w| w.id.as_str()).collect();
    assert!(ids.contains(&"W-01"));
    assert!(!ids.contains(&"W-02"));
}

#[test]
fn diamond_graph_deps_impact_closure_contraction_and_cones() {
    let mut st = State::default();
    for id in ["W-01", "W-02", "W-03", "W-04"] {
        put(&mut st, feature_full(id, "ready"));
    }
    edge(&mut st, "W-01", "blocks", "W-02");
    edge(&mut st, "W-01", "blocks", "W-03");
    edge(&mut st, "W-02", "blocks", "W-04");
    edge(&mut st, "W-03", "blocks", "W-04");
    let mut d = deps(&st, "W-04");
    d.sort();
    assert_eq!(d, vec!["W-01", "W-02", "W-03"]);
    let mut im = impact(&st, "W-01");
    im.sort();
    assert_eq!(im, vec!["W-02", "W-03", "W-04"]);
    let all: Vec<String> = ["W-01", "W-02", "W-03", "W-04"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(contraction_order(&st, &all), all);
    let bc = backward_cone(&st, "W-04", 4, 50);
    assert!(!bc.truncated);
    let mut bc_ids = bc.ids.clone();
    bc_ids.sort();
    assert_eq!(bc_ids, vec!["W-01", "W-02", "W-03"]);
    let fc = forward_cone(&st, "W-01", 4, 50);
    assert!(!fc.truncated);
    let mut fc_ids = fc.ids.clone();
    fc_ids.sort();
    assert_eq!(fc_ids, vec!["W-02", "W-03", "W-04"]);
    let h = backward_cone(&st, "W-04", 1, 50);
    assert_eq!(h.ids, vec!["W-02", "W-03"]);
    assert!(h.truncated);
}

#[test]
fn treewidth_upper_cycle_tree_and_empty() {
    let empty = State::default();
    assert_eq!(treewidth_upper(&empty), 0);
    let mut cyc = State::default();
    for id in ["W-01", "W-02", "W-03", "W-04"] {
        put(&mut cyc, work(id, "feature", "ready", "clear"));
    }
    edge(&mut cyc, "W-01", "blocks", "W-02");
    edge(&mut cyc, "W-02", "blocks", "W-04");
    edge(&mut cyc, "W-04", "blocks", "W-03");
    edge(&mut cyc, "W-03", "blocks", "W-01");
    assert_eq!(treewidth_upper(&cyc), 2);
    let mut tree = State::default();
    for id in ["W-01", "W-02", "W-03", "W-04"] {
        put(&mut tree, work(id, "feature", "ready", "clear"));
    }
    edge(&mut tree, "W-01", "blocks", "W-02");
    edge(&mut tree, "W-02", "blocks", "W-03");
    edge(&mut tree, "W-03", "blocks", "W-04");
    assert_eq!(treewidth_upper(&tree), 1);
}

#[test]
fn node_connectivity_menger_chain_diamond_and_edge_cases() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    for id in ["W-01", "W-02"] {
        put(&mut st, work(id, "feature", "ready", "clear"));
    }
    edge(&mut st, "G-01", "blocks", "W-01");
    edge(&mut st, "W-01", "blocks", "W-02");
    assert_eq!(node_connectivity(&st, "G-01", "W-02"), 1);
    let mut d = State::default();
    put(&mut d, plain(Kind::G, "G-01", "unverified"));
    for id in ["W-01", "W-02", "W-03"] {
        put(&mut d, work(id, "feature", "ready", "clear"));
    }
    edge(&mut d, "G-01", "blocks", "W-01");
    edge(&mut d, "G-01", "blocks", "W-02");
    edge(&mut d, "W-01", "blocks", "W-03");
    edge(&mut d, "W-02", "blocks", "W-03");
    assert_eq!(node_connectivity(&d, "G-01", "W-03"), 2);
    assert_eq!(node_connectivity(&d, "W-01", "W-01"), 0);
    d.nodes.get_mut("W-03").unwrap().archived = true;
    assert_eq!(node_connectivity(&d, "G-01", "W-03"), 0);
}

#[test]
fn relevant_discoveries_anchor_ranking_and_stale_exclusion() {
    let mut st = State::default();
    let mut w = work("W-01", "feature", "ready", "clear");
    reflist(&mut w, "surface", &["src/a.jl"]);
    reflist(&mut w, "tags", &["auth"]);
    put(&mut st, w);
    let mut y1 = plain(Kind::Y, "Y-01", "active");
    reflist(&mut y1, "surface", &["src/a.jl"]);
    put(&mut st, y1);
    let mut y2 = plain(Kind::Y, "Y-02", "active");
    reflist(&mut y2, "surface", &["src/a.jl"]);
    reflist(&mut y2, "tags", &["auth"]);
    put(&mut st, y2);
    let mut y3 = plain(Kind::Y, "Y-03", "stale");
    reflist(&mut y3, "surface", &["src/a.jl"]);
    reflist(&mut y3, "tags", &["auth"]);
    put(&mut st, y3);
    let cone = backward_cone(&st, "W-01", 4, 50).ids;
    let w = st.nodes.get("W-01").unwrap();
    let ranked = relevant_discoveries(&st, w, &cone, 10);
    assert_eq!(ranked, vec!["Y-02", "Y-01"]);
    assert!(!ranked.contains(&"Y-03".to_string()));
}

#[test]
fn area_relevant_discoveries_active_and_stale() {
    let mut st = State::default();
    put(&mut st, plain(Kind::A, "A-01", "present"));
    let mut g = plain(Kind::G, "G-01", "unverified");
    single(&mut g, "area", "A-01");
    put(&mut st, g);
    let mut w = work("W-01", "feature", "ready", "clear");
    reflist(&mut w, "goals", &["G-01"]);
    reflist(&mut w, "surface", &["src/a.jl"]);
    reflist(&mut w, "tags", &["auth"]);
    put(&mut st, w);
    let mut y1 = plain(Kind::Y, "Y-01", "active");
    reflist(&mut y1, "surface", &["src/a.jl"]);
    put(&mut st, y1);
    let mut y2 = plain(Kind::Y, "Y-02", "active");
    reflist(&mut y2, "surface", &["src/a.jl"]);
    reflist(&mut y2, "tags", &["auth"]);
    put(&mut st, y2);
    let mut y3 = plain(Kind::Y, "Y-03", "stale");
    reflist(&mut y3, "surface", &["src/a.jl"]);
    reflist(&mut y3, "tags", &["auth"]);
    put(&mut st, y3);
    let a = st.nodes.get("A-01").unwrap();
    let ranked = area_relevant_discoveries(&st, a);
    assert_eq!(ranked, vec!["Y-02", "Y-01"]);
    assert!(!ranked.contains(&"Y-03".to_string()));
}

#[test]
fn discovery_anchor_count_cone_link() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, work("W-02", "feature", "ready", "clear"));
    edge(&mut st, "W-02", "blocks", "W-01");
    put(&mut st, plain(Kind::Y, "Y-01", "active"));
    edge(&mut st, "Y-01", "distills", "W-02");
    let cone: BTreeSet<String> = backward_cone(&st, "W-01", 4, 50)
        .ids
        .into_iter()
        .collect();
    let surfaces: BTreeSet<String> = BTreeSet::new();
    let tags: BTreeSet<String> = BTreeSet::new();
    let y = st.nodes.get("Y-01").unwrap();
    assert_eq!(discovery_anchor_count(&st, y, &surfaces, &tags, &cone), 1);
    assert!(discovery_anchor_matches(&st, y, &surfaces, &tags, &cone));
}

#[test]
fn goal_fragility_on_diamond() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    put(&mut st, work("W-02", "feature", "ready", "clear"));
    let mut w3 = work("W-03", "feature", "ready", "clear");
    reflist(&mut w3, "goals", &["G-01"]);
    put(&mut st, w3);
    edge(&mut st, "G-01", "blocks", "W-01");
    edge(&mut st, "G-01", "blocks", "W-02");
    edge(&mut st, "W-01", "blocks", "W-03");
    edge(&mut st, "W-02", "blocks", "W-03");
    let w3 = st.nodes.get("W-03").unwrap();
    assert_eq!(goal_fragility(&st, w3), vec![("G-01".to_string(), 2)]);
}

#[test]
fn critical_path_tie_break_picks_smallest_id() {
    let mut st = State::default();
    for id in ["W-01", "W-02", "W-03"] {
        put(&mut st, work(id, "feature", "ready", "clear"));
    }
    edge(&mut st, "W-01", "blocks", "W-02");
    edge(&mut st, "W-01", "blocks", "W-03");
    assert_eq!(critical_path(&st), vec!["W-01", "W-02"]);
}
