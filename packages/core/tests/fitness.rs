mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

fn done_work_with_delta(id: &str, gid: &str, delta: i64) -> Node {
    let mut w = work(id, "feature", "done", "clear");
    reflist(&mut w, "goals", &[gid]);
    fitness(&mut w, &[(gid, delta)]);
    w
}

#[test]
fn count_kind_partial_then_verified() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness_kind", "count");
    single(&mut g, "fitness_target", "2");
    put(&mut st, g);
    put(&mut st, done_work_with_delta("W-01", "G-01", 1));
    rederive_goals(&mut st, "W-01");
    assert_eq!(st.nodes["G-01"].status, "partial");
    assert_eq!(st.nodes["G-01"].single("fitness_current"), "1");
    put(&mut st, done_work_with_delta("W-02", "G-01", 1));
    rederive_goals(&mut st, "W-02");
    assert_eq!(st.nodes["G-01"].status, "verified");
    assert_eq!(st.nodes["G-01"].single("fitness_current"), "2");
}

#[test]
fn boolean_kind_verifies_on_delta() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness_kind", "boolean");
    put(&mut st, g);
    put(&mut st, done_work_with_delta("W-01", "G-01", 1));
    rederive_goals(&mut st, "W-01");
    assert_eq!(st.nodes["G-01"].status, "verified");
    assert_eq!(st.nodes["G-01"].single("fitness_current"), "true");
}

#[test]
fn boolean_kind_zero_deltas_stays_unverified_with_false_current() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness_kind", "boolean");
    put(&mut st, g);
    refresh_goal_structured_fitness(&mut st, "G-01");
    assert_eq!(st.nodes["G-01"].status, "unverified");
    assert_eq!(st.nodes["G-01"].single("fitness_current"), "false");
}

#[test]
fn legacy_fitness_fraction_partial_then_verified() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness", "1/2");
    put(&mut st, g);
    put(&mut st, done_work_with_delta("W-01", "G-01", 1));
    rederive_goals(&mut st, "W-01");
    assert_eq!(st.nodes["G-01"].status, "partial");
    put(&mut st, done_work_with_delta("W-02", "G-01", 1));
    rederive_goals(&mut st, "W-02");
    assert_eq!(st.nodes["G-01"].status, "verified");
}

#[test]
fn manual_kind_refresh_changes_nothing() {
    let mut st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness_kind", "manual");
    put(&mut st, g);
    put(&mut st, done_work_with_delta("W-01", "G-01", 99));
    refresh_goal_structured_fitness(&mut st, "G-01");
    assert_eq!(st.nodes["G-01"].status, "unverified");
    assert!(!st.nodes["G-01"].fields.contains_key("fitness_current"));
}

#[test]
fn parse_fitness_target_first_match_group_two() {
    assert_eq!(parse_fitness_target("1/2"), Some(2));
    assert_eq!(parse_fitness_target("3 of 4"), None);
    assert_eq!(parse_fitness_target("x 10/20"), Some(20));
    assert_eq!(parse_fitness_target("1/2/3"), Some(2));
    assert_eq!(parse_fitness_target("no digits"), None);
}

#[test]
fn aggregate_fitness_delta_sums_only_done_work() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, done_work_with_delta("W-01", "G-01", 2));
    let mut w2 = work("W-02", "feature", "ready", "clear");
    fitness(&mut w2, &[("G-01", 3)]);
    put(&mut st, w2);
    let mut w3 = work("W-03", "feature", "rejected", "clear");
    fitness(&mut w3, &[("G-01", 5)]);
    put(&mut st, w3);
    assert_eq!(aggregate_fitness_delta(&st, "G-01"), 2);
}

#[test]
fn goal_fitness_table_cell_legacy_and_structured() {
    let mut g = plain(Kind::G, "G-01", "unverified");
    attr(&mut g, "fitness", "1/2");
    assert_eq!(goal_fitness_table_cell(&g), "1/2");
    let mut g2 = plain(Kind::G, "G-02", "unverified");
    attr(&mut g2, "fitness_kind", "count");
    single(&mut g2, "fitness_current", "1");
    single(&mut g2, "fitness_target", "2");
    assert_eq!(goal_fitness_table_cell(&g2), "count; current=1 target=2");
}
