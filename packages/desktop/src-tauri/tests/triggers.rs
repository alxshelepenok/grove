use grove_core::{parse_fixture, run_cli, serialize, EXIT_OK};
use grove_desktop_lib::bridge::{run_read, run_write};
use grove_desktop_lib::triggers::{self, Dismissals};
use grove_desktop_lib::views::load_state;
use std::collections::BTreeSet;
use std::path::PathBuf;

mod common;

const SESSION: &str = "desktop-test-token";

const FIXTURE: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"

g G-01 status=verified fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Verified goal"
  area: A-01
  fitness_current: true

g G-02 status=unverified t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Open goal"
  area: A-01

d D-01 status=accepted t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Accepted decision"
d D-02 status=proposed t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Proposed decision"
d D-03 status=accepted t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Spike output decision"

w W-01 type=feature status=done cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Implements accepted D"
  goals: G-02

w W-02 type=refactor status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Refactor done"
  goals: G-02

w W-03 type=spike status=done cynefin=complex t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Complex spike done"
  goals: G-02

w W-04 type=spike status=done cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Producing spike done"
  goals: G-02

w W-05 type=spike status=done cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Trivial spike done"
  goals: G-02

w W-06 type=feature status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Plain done"
  goals: G-02

w W-07 type=feature status=progress cynefin=clear session=other-token session_at=2031-01-01T00:00:00Z t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Claimed by another session"
  goals: G-02

w W-08 type=feature status=progress cynefin=clear session=desktop-test-token session_at=2020-01-01T00:00:00Z t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Claimed long ago"
  goals: G-02

w W-09 type=feature status=progress cynefin=clear session=desktop-test-token session_at=2031-01-01T00:00:00Z t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Fresh own claim"
  goals: G-02

q Q-01 status=open cynefin=chaotic t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Chaotic question"
q Q-02 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Plain open question"
q Q-03 status=answered cynefin=chaotic t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Answered chaotic question"

b B-01 status=invalidated_blocking cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Blocking bet"
b B-02 status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Proposed bet"
b B-03 status=validated cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Validated bet"

e W-01 implements D-01 t_created=2026-07-27T00:00:00Z
e W-04 produces D-03 t_created=2026-07-27T00:00:00Z
"#;

fn fixture_state() -> grove_core::State {
    parse_fixture(FIXTURE).expect("fixture parses")
}

fn detect_fixture() -> triggers::TriggerSet {
    let st = fixture_state();
    triggers::detect(&st, SESSION, &Dismissals::default(), 0)
}

fn ids(refs: &[triggers::NodeRef]) -> Vec<&str> {
    refs.iter().map(|r| r.id.as_str()).collect()
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-desktop-trig-{}-{}-{}",
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

#[test]
fn chaotic_q_branch_matches_protocol() {
    let ts = detect_fixture();
    assert_eq!(ids(&ts.chaotic_q), ["Q-01", "Q-03"]);

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
q Q-01 status=open cynefin=complex t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Not chaotic"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(ts.chaotic_q.is_empty());
}

#[test]
fn blocked_b_branch_matches_protocol() {
    let ts = detect_fixture();
    assert_eq!(ids(&ts.blocked_b), ["B-01"]);

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
b B-01 status=invalidated_acceptable cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Acceptable"
b B-02 status=testing cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Testing"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(ts.blocked_b.is_empty());
}

#[test]
fn significant_done_w_branches() {
    let ts = detect_fixture();
    let significant: Vec<&str> = ts
        .done_w
        .iter()
        .filter(|d| d.significant)
        .map(|d| d.id.as_str())
        .collect();
    assert_eq!(significant, ["W-01", "W-02", "W-03", "W-04"]);
    let plain: Vec<&str> = ts
        .done_w
        .iter()
        .filter(|d| !d.significant)
        .map(|d| d.id.as_str())
        .collect();
    assert_eq!(plain, ["W-05", "W-06"]);
}

#[test]
fn significant_done_w_critical_path_clause() {
    let st = fixture_state();
    let w = st.nodes.get("W-06").unwrap();
    let cp: BTreeSet<String> = BTreeSet::from(["W-06".to_string()]);
    assert!(triggers::significant_done(&st, &cp, w));
    let empty: BTreeSet<String> = BTreeSet::new();
    assert!(!triggers::significant_done(&st, &empty, w));
}

#[test]
fn verified_g_branch() {
    let ts = detect_fixture();
    assert_eq!(ids(&ts.verified_g), ["G-01"]);
}

#[test]
fn idle_branch_requires_ready_empty_and_open_gap() {
    let ts = detect_fixture();
    assert!(ts.idle, "fixture: no ready W, open Q and proposed B exist");
    assert!(ts.ready.is_empty());

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
g G-01 status=unverified t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal"
  area: A-01
w W-01 type=feature status=ready cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Ready work"
  goals: G-01
  fitness: G-01=+1
  ac:
    | works
  hypothesis:
    | h
  evidence_strategy:
    | e
q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Open question"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(!ts.ready.is_empty(), "W-01 passes DoR and is ready");
    assert!(!ts.idle, "ready non-empty suppresses the idle trigger");

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
b B-01 status=testing cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Testing bet"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(ts.idle, "testing B with empty ready set fires idle");

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
q Q-01 status=answered cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Closed question"
b B-01 status=validated cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Validated bet"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(!ts.idle, "no open gap: idle stays off even with ready empty");
}

#[test]
fn stale_claim_branches() {
    let ts = detect_fixture();
    assert_eq!(ts.stale_claims.len(), 2);
    let w07 = ts.stale_claims.iter().find(|c| c.id == "W-07").unwrap();
    assert_eq!(w07.reason, "different session");
    assert_eq!(w07.session, "other-token");
    let w08 = ts.stale_claims.iter().find(|c| c.id == "W-08").unwrap();
    assert_eq!(w08.reason, "claimed >24h ago");
    assert!(!ts.stale_claims.iter().any(|c| c.id == "W-09"));

    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
w W-01 type=feature status=progress cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "No session record"
"#,
    )
    .unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert_eq!(ts.stale_claims.len(), 1);
    assert_eq!(ts.stale_claims[0].reason, "no session on record");
}

#[test]
fn trigger_and_badge_counts() {
    let ts = detect_fixture();
    assert_eq!(ts.trigger_count(), 2 + 1 + 4 + 1 + 1);
    assert_eq!(ts.badge_count(), ts.trigger_count() + 2);
}

#[test]
fn trigger_count_matches_core_alignment_triggers() {
    let st = fixture_state();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert_eq!(
        grove_core::alignment_triggers(&st).len(),
        ts.trigger_count(),
        "with no dismissals the detector and grove status agree"
    );
}

#[test]
fn dismissal_hides_significant_w_until_journal_moves() {
    let st = fixture_state();
    let mut dismissals = Dismissals::default();
    dismissals.dismiss("W-02", 5);

    let ts = triggers::detect(&st, SESSION, &dismissals, 5);
    let w02 = ts.done_w.iter().find(|d| d.id == "W-02").unwrap();
    assert!(w02.significant && w02.dismissed);
    assert_eq!(ts.live_significant().len(), 3);
    assert_eq!(ts.trigger_count(), 8);

    let ts = triggers::detect(&st, SESSION, &dismissals, 6);
    let w02 = ts.done_w.iter().find(|d| d.id == "W-02").unwrap();
    assert!(w02.significant && !w02.dismissed, "journal moved: dismissal void");
    assert_eq!(ts.trigger_count(), 9);
}

#[test]
fn dismissals_save_load_round_trip() {
    let dir = tmpdir("dismissals");
    let path = dir.join("nested").join("checkpoint-dismissals.json");
    let mut d = Dismissals::default();
    d.dismiss("W-10", 42);
    d.save(&path).unwrap();
    let loaded = Dismissals::load(&path);
    assert!(loaded.is_dismissed("W-10", 42));
    assert!(!loaded.is_dismissed("W-10", 43));
    assert!(!loaded.is_dismissed("W-11", 42));

    let missing = Dismissals::load(&dir.join("nope.json"));
    assert!(missing.entries.is_empty());

    std::fs::write(&path, "{not json").unwrap();
    let broken = Dismissals::load(&path);
    assert!(broken.entries.is_empty());
}

#[test]
fn journal_len_counts_lines() {
    let dir = tmpdir("jlen");
    assert_eq!(triggers::journal_len(&dir.to_string_lossy()), 0);
    let dev = dir.join(".grove");
    std::fs::create_dir_all(&dev).unwrap();
    std::fs::write(dev.join("journal.log"), "one\ntwo\nthree\n").unwrap();
    assert_eq!(triggers::journal_len(&dir.to_string_lossy()), 3);
}

#[test]
fn detect_on_temp_root_matches_status_output() {
    let (_guard, _home) = common::isolated_grove_home("detect-root-home");
    let dir = tmpdir("detect-root");
    let r = run_cli(&["init".to_string(), format!("--root={}", dir.display())]);
    assert_eq!(r.code, EXIT_OK, "init failed: {}", r.err);
    let root = dir.to_string_lossy().into_owned();
    run_write(
        &root,
        SESSION,
        "add",
        &["q".to_string(), "--title=Chaos".to_string(), "--cynefin=chaotic".to_string()],
    )
    .unwrap();
    run_write(
        &root,
        SESSION,
        "add",
        &["a".to_string(), "--title=Area".to_string()],
    )
    .unwrap();
    run_write(
        &root,
        SESSION,
        "add",
        &["g".to_string(), "--title=Goal".to_string(), "--area=A-01".to_string()],
    )
    .unwrap();

    let st = load_state(&root).unwrap();
    let ts = triggers::detect(
        &st,
        SESSION,
        &Dismissals::default(),
        triggers::journal_len(&root),
    );
    assert_eq!(ids(&ts.chaotic_q), ["Q-01"]);

    let status: serde_json::Value =
        serde_json::from_str(&run_read(&root, "status", &[]).unwrap()).unwrap();
    let core_count = status["alignment_triggers"].as_array().unwrap().len();
    assert_eq!(core_count, ts.trigger_count());
    let progress = status["progress"].as_array().unwrap();
    assert_eq!(progress.len(), ts.stale_claims.len());
}

#[test]
fn resume_revert_round_trip_on_fixture_root() {
    let (_guard, _home) = common::isolated_grove_home("resume-revert-home");
    let dir = tmpdir("resume-revert");
    std::fs::create_dir_all(dir.join(".grove")).unwrap();
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
g G-01 status=unverified t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal"
  area: A-01
w W-01 type=feature status=progress cynefin=clear session=other-token session_at=2020-01-01T00:00:00Z t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Claimed work"
  goals: G-01
"#,
    )
    .unwrap();
    std::fs::write(dir.join(".grove/state.lock"), serialize(&st)).unwrap();
    let root = dir.to_string_lossy().into_owned();
    let lock_text = || std::fs::read_to_string(dir.join(".grove/state.lock")).unwrap();

    let st = load_state(&root).unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert_eq!(ts.stale_claims.len(), 1);
    assert_eq!(ts.stale_claims[0].id, "W-01");

    run_write(&root, SESSION, "resume", &["W-01".to_string()]).unwrap();
    assert!(
        lock_text().contains(&format!("session={SESSION}")),
        "resume adopts the desktop session token"
    );
    let st = load_state(&root).unwrap();
    let ts = triggers::detect(&st, SESSION, &Dismissals::default(), 0);
    assert!(ts.stale_claims.is_empty(), "fresh own claim is not stale");

    run_write(&root, SESSION, "revert", &["W-01".to_string()]).unwrap();
    let lock = lock_text();
    assert!(lock.contains("status=ready"), "revert returns W-01 to ready");
    assert!(!lock.contains("session="), "revert clears the session claim");
}
