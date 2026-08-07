mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;

fn expect_invalid(v: &GuardVerdict) -> &Vec<String> {
    match v {
        GuardVerdict::Invalid(m) => m,
        _ => panic!("expected Invalid, got exit code {}", v.exit_code()),
    }
}

fn expect_reject(v: &GuardVerdict) -> &Vec<String> {
    match v {
        GuardVerdict::Reject(m) => m,
        _ => panic!("expected Reject, got exit code {}", v.exit_code()),
    }
}

fn expect_ok(v: &GuardVerdict) {
    match v {
        GuardVerdict::Ok => {}
        _ => panic!("expected Ok, got exit code {}", v.exit_code()),
    }
}

#[test]
fn guard_rejects_invalid_status_for_kind() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "bogus");
    assert_eq!(expect_invalid(&v), &vec!["invalid status `bogus` for w".to_string()]);
    assert_eq!(v.exit_code(), 1);
}

#[test]
fn guard_theme_status_is_derived() {
    let mut st = State::default();
    put(&mut st, plain(Kind::T, "T-01", "open"));
    let n = st.nodes.get("T-01").expect("T-01 present");
    let v = guard_status_transition(&st, n, "done");
    assert_eq!(
        expect_reject(&v),
        &vec!["theme status is derived; cannot set manually".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_area_status_is_structural() {
    let mut st = State::default();
    put(&mut st, plain(Kind::A, "A-01", "present"));
    let n = st.nodes.get("A-01").expect("A-01 present");
    let v = guard_status_transition(&st, n, "present");
    assert_eq!(
        expect_reject(&v),
        &vec!["area status is structural; cannot set".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_w_progress_requires_dor() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "progress");
    assert_eq!(
        expect_reject(&v),
        &vec!["DoR ≢ ⊤ for W-01; see `grove dor W-01`".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_w_progress_requires_verified_goal_blockers() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, dor_ready_feature("W-01", "ready"));
    edge(&mut st, "G-01", "blocks", "W-01");
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "progress");
    assert_eq!(
        expect_reject(&v),
        &vec!["I5: predecessors not cleared (goal blockers must be verified, not merely declined/partial/unverified)".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_w_progress_enforces_wip_limit() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    for id in ["W-02", "W-03"] {
        let mut w = dor_ready_feature(id, "progress");
        attr(&mut w, "session", "tok");
        attr(&mut w, "session_at", "2099-01-01T00:00:00Z");
        put(&mut st, w);
    }
    put(&mut st, dor_ready_feature("W-01", "ready"));
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "progress");
    assert_eq!(
        expect_reject(&v),
        &vec!["I4: WIP limit (2) reached".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_w_progress_ok_when_fully_satisfied() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, dor_ready_feature("W-01", "ready"));
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "progress");
    expect_ok(&v);
    assert_eq!(v.exit_code(), 0);
}

#[test]
fn guard_w_done_requires_evidence() {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, dor_ready_feature("W-01", "progress"));
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "done");
    assert_eq!(
        expect_reject(&v),
        &vec!["I3: W-01 has no evidence; use `grove evidence W-01 \"…\"`".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_w_done_requires_fitness_delta_per_goal() {
    let mut st = State::default();
    let mut w = work("W-01", "feature", "progress", "clear");
    reflist(&mut w, "goals", &["G-01"]);
    prose(&mut w, "evidence", &["ran it"]);
    put(&mut st, w);
    let n = st.nodes.get("W-01").expect("W-01 present");
    let v = guard_status_transition(&st, n, "done");
    assert_eq!(
        expect_reject(&v),
        &vec!["I10: missing fitness delta for G-01; use `grove fitness W-01 G-01 <delta>`"
            .to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_decision_accepted_only_supersedes() {
    let mut st = State::default();
    put(&mut st, plain(Kind::D, "D-01", "accepted"));
    let n = st.nodes.get("D-01").expect("D-01 present");
    let v = guard_status_transition(&st, n, "rejected");
    assert_eq!(
        expect_reject(&v),
        &vec!["decision D-01 is accepted; create a new D with --supersedes".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
    let v = guard_status_transition(&st, n, "superseded");
    expect_ok(&v);
}

#[test]
fn guard_y_proposed_to_active_rejects_unmet_anchors() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let mut y = plain(Kind::Y, "Y-01", "proposed");
    reflist(&mut y, "surface", &["src/x.jl"]);
    put(&mut st, y);
    edge(&mut st, "W-01", "produces", "Y-01");
    let n = st.nodes.get("Y-01").expect("Y-01 present");
    let v = guard_status_transition(&st, n, "active");
    assert_eq!(
        expect_reject(&v),
        &vec![
            "y Y-01 anchors not satisfied (proposed → active refused):".to_string(),
            "  I12: Y-01 has empty `tags` (≥1 glossary term required)".to_string(),
        ]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_y_proposed_to_active_ok_with_anchors() {
    let mut st = State::default();
    put(&mut st, work("W-01", "feature", "ready", "clear"));
    let mut y = plain(Kind::Y, "Y-01", "proposed");
    reflist(&mut y, "surface", &["src/x.jl"]);
    reflist(&mut y, "tags", &["term"]);
    put(&mut st, y);
    edge(&mut st, "W-01", "produces", "Y-01");
    let n = st.nodes.get("Y-01").expect("Y-01 present");
    let v = guard_status_transition(&st, n, "active");
    expect_ok(&v);
}

#[test]
fn guard_y_active_to_proposed_is_illegal() {
    let mut st = State::default();
    put(&mut st, plain(Kind::Y, "Y-01", "active"));
    let n = st.nodes.get("Y-01").expect("Y-01 present");
    let v = guard_status_transition(&st, n, "proposed");
    assert_eq!(
        expect_reject(&v),
        &vec!["illegal y transition active → proposed (allowed: proposed→active, active→stale, non-terminal→superseded; stale→active only via `grove revalidate`)".to_string()]
    );
    assert_eq!(v.exit_code(), 4);
}

#[test]
fn guard_y_stale_to_superseded_ok() {
    let mut st = State::default();
    put(&mut st, plain(Kind::Y, "Y-01", "stale"));
    let n = st.nodes.get("Y-01").expect("Y-01 present");
    let v = guard_status_transition(&st, n, "superseded");
    expect_ok(&v);
}

#[test]
fn guard_y_superseded_to_superseded_is_illegal() {
    let mut st = State::default();
    put(&mut st, plain(Kind::Y, "Y-01", "superseded"));
    let n = st.nodes.get("Y-01").expect("Y-01 present");
    let v = guard_status_transition(&st, n, "superseded");
    assert_eq!(
        expect_reject(&v),
        &vec!["illegal y transition superseded → superseded (allowed: proposed→active, active→stale, non-terminal→superseded; stale→active only via `grove revalidate`)".to_string()]
    );
}

#[test]
fn progress_has_session_record_false_and_true() {
    let mut w = work("W-01", "feature", "progress", "clear");
    assert!(!progress_has_session_record(&w));
    attr(&mut w, "session", "   ");
    assert!(!progress_has_session_record(&w));
    attr(&mut w, "session", "tok");
    assert!(progress_has_session_record(&w));
}

#[test]
fn session_token_matches_compares_trimmed() {
    let mut w = work("W-01", "feature", "progress", "clear");
    assert!(!session_token_matches(&w, "tok"));
    attr(&mut w, "session", "  tok  ");
    assert!(session_token_matches(&w, "tok"));
    assert!(session_token_matches(&w, "  tok "));
    assert!(!session_token_matches(&w, "other"));
}

#[test]
fn session_claim_age_stale_at_uses_explicit_now() {
    let mut w = work("W-01", "feature", "progress", "clear");
    attr(&mut w, "session", "tok");
    attr(&mut w, "session_at", "2026-01-01T00:00:00Z");
    let stale_now = parse_rfc3339_utc_second("2026-01-02T00:00:01Z").expect("parses");
    assert!(session_claim_age_stale_at(&w, stale_now));
    let fresh_now = parse_rfc3339_utc_second("2026-01-01T12:00:00Z").expect("parses");
    assert!(!session_claim_age_stale_at(&w, fresh_now));
}

#[test]
fn session_denial_progress_mutate_blocks_other_sessions() {
    let mut w = work("W-01", "feature", "progress", "clear");
    attr(&mut w, "session", "owner");
    attr(&mut w, "session_at", "2099-01-01T00:00:00Z");
    assert_eq!(
        session_denial_progress_mutate(&w, "other"),
        Some("I11/session: W-01 is `progress` and owned by another session; try `grove resume W-01` after adopting, or coordinate a `grove handoff`".to_string())
    );
    assert_eq!(session_denial_progress_mutate(&w, "owner"), None);
}

#[test]
fn session_denial_progress_release_blocks_fresh_foreign_claim() {
    let mut w = work("W-01", "feature", "progress", "clear");
    attr(&mut w, "session", "owner");
    attr(&mut w, "session_at", "2099-01-01T00:00:00Z");
    assert_eq!(
        session_denial_progress_release(&w, "other"),
        Some("I11/session: cannot release W-01: token differs and claim is fresh (<24h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait".to_string())
    );
    assert_eq!(session_denial_progress_release(&w, "owner"), None);
}

#[test]
fn session_denial_progress_release_allows_stale_claim() {
    let mut w = work("W-01", "feature", "progress", "clear");
    attr(&mut w, "session", "owner");
    attr(&mut w, "session_at", "2020-01-01T00:00:00Z");
    assert_eq!(session_denial_progress_release(&w, "other"), None);
}

#[test]
fn session_denials_ignore_non_progress_work() {
    let mut w = work("W-01", "feature", "ready", "clear");
    attr(&mut w, "session", "owner");
    attr(&mut w, "session_at", "2099-01-01T00:00:00Z");
    assert_eq!(session_denial_progress_mutate(&w, "other"), None);
    assert_eq!(session_denial_progress_release(&w, "other"), None);
    assert_eq!(session_denial_progress_mutate(&w, "owner"), None);
    assert_eq!(session_denial_progress_release(&w, "owner"), None);
}

#[test]
fn assign_and_clear_w_session_attrs_roundtrip() {
    let mut w = work("W-01", "feature", "progress", "clear");
    assign_w_claim_session(&mut w, "  tok  ");
    assert_eq!(w.attr("session"), "tok");
    let sa = w.attr("session_at");
    assert!(parse_rfc3339_utc_second(&sa).is_some());
    clear_w_session_attrs(&mut w);
    assert_eq!(w.attr("session"), "");
    assert_eq!(w.attr("session_at"), "");
    assert!(!progress_has_session_record(&w));
}
