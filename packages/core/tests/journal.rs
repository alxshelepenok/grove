mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::collections::BTreeMap;
use std::path::PathBuf;

const TS: &str = "2031-01-01T00:00:00Z";
const EFF: &str = "testsession";
const INIT_GLOSSARY: &str = "# Glossary\n\n| Term | Definition | Source |\n| --- | --- | --- |\n";

fn pin() {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(TS).unwrap()));
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-jtest-{}-{}-{}",
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

fn w01_with_goals(status: &str) -> State {
    let mut st = State::default();
    put(&mut st, plain(Kind::G, "G-01", "unverified"));
    put(&mut st, plain(Kind::G, "G-02", "unverified"));
    let mut w = work("W-01", "feature", status, "clear");
    reflist(&mut w, "goals", &["G-01", "G-02"]);
    put(&mut st, w);
    st
}

#[test]
fn add_record_line() {
    pin();
    assert_eq!(
        wrap_journal_record("add", jinv_rm_node("A-01")),
        r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"A-01","op":"rm_node"}}"#
    );
}

#[test]
fn field_record_lines() {
    pin();
    assert_eq!(
        wrap_journal_record("field", jinv_field_pop_last("W-01", "ac")),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","op":"field_pop_last"}}"#
    );
    assert_eq!(
        wrap_journal_record("field", jinv_field_insert_line("W-01", "ac", 1, "first ac")),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"line":"first ac","id":"W-01","field":"ac","op":"field_insert_line","index":1}}"#
    );
    assert_eq!(
        wrap_journal_record(
            "field",
            jinv_field_restore_lines("W-01", "ac", &["second ac".to_string()])
        ),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","lines":["second ac"],"op":"field_restore_lines"}}"#
    );
    assert_eq!(
        wrap_journal_record("field", jinv_field_restore_single("W-01", "theme", "T-01")),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"theme","value":"T-01","op":"field_restore_single"}}"#
    );
    assert_eq!(
        wrap_journal_record("field", jinv_field_restore_single("W-01", "theme", "")),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"theme","value":"","op":"field_restore_single"}}"#
    );
    assert_eq!(
        wrap_journal_record(
            "field",
            jinv_field_restore_fitness("W-01", "fitness", &BTreeMap::new())
        ),
        r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"map":{},"id":"W-01","field":"fitness","op":"field_restore_fitness"}}"#
    );
}

#[test]
fn fitness_map_jval_order_matches_julia() {
    let mut m = BTreeMap::new();
    m.insert("G-01".to_string(), 1);
    m.insert("G-02".to_string(), 2);
    assert_eq!(emit_jval(&fitness_map_jval(&m)), r#"{"G-01":1,"G-02":2}"#);
    let mut m3 = BTreeMap::new();
    m3.insert("G-01".to_string(), 1);
    m3.insert("G-02".to_string(), 2);
    m3.insert("G-03".to_string(), 3);
    assert_eq!(
        emit_jval(&fitness_map_jval(&m3)),
        r#"{"G-01":1,"G-03":3,"G-02":2}"#
    );
}

#[test]
fn fitness_record_lines() {
    pin();
    assert_eq!(
        wrap_journal_record("fitness", jinv_restore_fitness_key("W-01", "G-01", false, None)),
        r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":false,"wid":"W-01","gid":"G-01","previous":null,"op":"restore_fitness_key"}}"#
    );
    assert_eq!(
        wrap_journal_record("fitness", jinv_restore_fitness_key("W-01", "G-01", true, Some(2))),
        r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":true,"wid":"W-01","gid":"G-01","previous":2,"op":"restore_fitness_key"}}"#
    );
}

#[test]
fn set_simple_record_lines() {
    pin();
    assert_eq!(
        wrap_journal_record("set", jinv_set_simple_old("set_cynefin", "W-01", "clear")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"clear","op":"set_cynefin"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_simple_old("set_type", "W-01", "feature")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"feature","op":"set_type"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_simple_old("set_title", "W-01", "Work one")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"Work one","op":"set_title"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_simple_old("set_g_attr_fitness", "G-02", "")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","old":"","op":"set_g_attr_fitness"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_g_attr_fitness_kind("G-02", false, "", "count")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"new":"count","id":"G-02","had_before":false,"old":"","op":"set_g_attr_fitness_kind"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_g_attr_fitness_kind("G-02", true, "count", "ratio")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"new":"ratio","id":"G-02","had_before":true,"old":"count","op":"set_g_attr_fitness_kind"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_g_area("G-02", true, "A-01")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","had_before":true,"old":"A-01","op":"set_g_area"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_requires_coverage("G-02", false, "")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","had_before":false,"old":"","op":"set_requires_coverage"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_status_plain("Q-01", "open")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"old_status":"open","id":"Q-01","op":"set_status_plain"}}"#
    );
    assert_eq!(
        wrap_journal_record("set", jinv_set_status_plain("B-01", "proposed")),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"old_status":"proposed","id":"B-01","op":"set_status_plain"}}"#
    );
}

#[test]
fn link_record_lines() {
    pin();
    assert_eq!(
        wrap_journal_record("link", jinv_unlink_edge("T-01", "causes", "W-01")),
        r#"{"v":1,"cmd":"link","ts":"2031-01-01T00:00:00Z","inv":{"label":"causes","to":"W-01","op":"unlink_edge","from":"T-01"}}"#
    );
    assert_eq!(
        wrap_journal_record(
            "unlink",
            jinv_restore_edge("T-01", "causes", "W-01", Some("2031-01-01T00:00:00Z"))
        ),
        r#"{"v":1,"cmd":"unlink","ts":"2031-01-01T00:00:00Z","inv":{"label":"causes","t_created":"2031-01-01T00:00:00Z","to":"W-01","op":"restore_edge","from":"T-01"}}"#
    );
}

#[test]
fn w_status_record_lines() {
    pin();
    let st = w01_with_goals("proposed");
    let w = &st.nodes["W-01"];
    let gs = goal_statuses_jdict(&st, w);
    assert_eq!(
        wrap_journal_record("set", jinv_set_w_status_with_goals("W-01", "proposed", gs, w)),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"proposed","had_session_before":false,"id":"W-01","op":"set_w_status_with_goals","goal_statuses":{"G-01":"unverified","G-02":"unverified"},"old_session_at":"","old_session":""}}"#
    );

    let mut st2 = w01_with_goals("progress");
    {
        let w = st2.nodes.get_mut("W-01").unwrap();
        attr(w, "session", "testsession");
        attr(w, "session_at", "2031-01-01T00:00:00Z");
    }
    let w = &st2.nodes["W-01"];
    let gs = goal_statuses_jdict(&st2, w);
    assert_eq!(
        wrap_journal_record("set", jinv_set_w_status_with_goals("W-01", "progress", gs, w)),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":true,"old_w_status":"progress","had_session_before":true,"id":"W-01","op":"set_w_status_with_goals","goal_statuses":{"G-01":"unverified","G-02":"unverified"},"old_session_at":"2031-01-01T00:00:00Z","old_session":"testsession"}}"#
    );
}

#[test]
fn renumber_record_line() {
    pin();
    assert_eq!(
        wrap_journal_record("renumber", jinv_renumber_swap("W-05", "W-02")),
        r#"{"v":1,"cmd":"renumber","ts":"2031-01-01T00:00:00Z","inv":{"to":"W-02","op":"renumber_swap","from":"W-05"}}"#
    );
}

#[test]
fn glossary_record_lines() {
    pin();
    let mut snap = JuliaDict::new();
    snap.insert(
        "Y-01".to_string(),
        JVal::Arr(vec![JVal::Str("term-one".to_string())]),
    );
    assert_eq!(
        wrap_journal_record(
            "glossary",
            jinv_glossary_rename_restore(snap, "term-one", "term-two", false)
        ),
        r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-two","glossary_changed":false,"old":"term-one","tags":{"Y-01":["term-one"]},"op":"glossary_rename_restore"}}"#
    );

    let mut snap2 = JuliaDict::new();
    snap2.insert(
        "Y-02".to_string(),
        JVal::Arr(vec![
            JVal::Str("term-three".to_string()),
            JVal::Str("term-four".to_string()),
        ]),
    );
    assert_eq!(
        wrap_journal_record(
            "glossary",
            jinv_glossary_rename_restore(snap2, "term-three", "term-five", true)
        ),
        r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-five","glossary_changed":true,"old":"term-three","tags":{"Y-02":["term-three","term-four"]},"op":"glossary_rename_restore"}}"#
    );
}

#[test]
fn dor_reject_record_line() {
    pin();
    let missing: Vec<String> = [
        "goals(w) ≠ ∅",
        "AC(w) ≠ ∅",
        "fitness deltas set ∀ g",
        "evidence_strategy ≠ ∅",
        "hypothesis ≠ ⊥",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        wrap_journal_record("set", jinv_dor_reject("W-02", &missing)),
        r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"missing":["goals(w) ≠ ∅","AC(w) ≠ ∅","fitness deltas set ∀ g","evidence_strategy ≠ ∅","hypothesis ≠ ⊥"],"id":"W-02","op":"dor_reject"}}"#
    );
}

#[test]
fn session_claim_record_lines() {
    pin();
    let mut w = work("W-06", "feature", "progress", "clear");
    attr(&mut w, "session", "testsession");
    attr(&mut w, "session_at", "2031-01-01T00:00:00Z");
    assert_eq!(
        wrap_journal_record("handoff", jinv_session_restore_claim("W-06", &w)),
        r#"{"v":1,"cmd":"handoff","ts":"2031-01-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"testsession","had_session_at_before":true}}"#
    );

    let mut w2 = work("W-06", "feature", "progress", "clear");
    attr(&mut w2, "session", "newtok");
    attr(&mut w2, "session_at", "2031-01-01T00:00:00Z");
    assert_eq!(
        wrap_journal_record("resume", jinv_session_restore_claim("W-06", &w2)),
        r#"{"v":1,"cmd":"resume","ts":"2031-01-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"newtok","had_session_at_before":true}}"#
    );
}

#[test]
fn undo_record_line() {
    pin();
    assert_eq!(
        wrap_journal_record("undo", jinv_undo(3)),
        r#"{"v":1,"cmd":"undo","ts":"2031-01-01T00:00:00Z","inv":{"steps":3,"op":"undo"}}"#
    );
}

#[test]
fn mutation_classification_and_tail_view() {
    pin();
    let add = parse_json(&wrap_journal_record("add", jinv_rm_node("A-01"))).unwrap();
    let gate = parse_json(r#"{"v":1,"cmd":"gate","ts":"2031-01-01T00:00:00Z","inv":{"dones":1,"overflows":[],"invalidated":[],"op":"gate","tw":2,"empty":true,"overflow_counts":{}}}"#).unwrap();
    let distill = parse_json(r#"{"v":1,"cmd":"distill","ts":"2031-01-01T00:00:00Z","inv":{"goal":"G-01","op":"distill","empty":true}}"#).unwrap();
    let dor = parse_json(&wrap_journal_record(
        "set",
        jinv_dor_reject("W-02", &["x".to_string()]),
    ))
    .unwrap();
    let undo = parse_json(&wrap_journal_record("undo", jinv_undo(3))).unwrap();
    let field = parse_json(&wrap_journal_record("field", jinv_field_pop_last("W-01", "ac"))).unwrap();
    assert!(journal_record_mutation(&add));
    assert!(!journal_record_mutation(&gate));
    assert!(!journal_record_mutation(&distill));
    assert!(!journal_record_mutation(&dor));
    assert!(!journal_record_mutation(&undo));
    assert!(journal_record_mutation(&field));
    assert!(journal_record_mutation(&Json::Null));

    let recs = vec![add.clone(), gate, field.clone(), distill, dor, undo];
    assert_eq!(journal_tail_mutation_view(&recs, 1), Some(vec![2]));
    assert_eq!(journal_tail_mutation_view(&recs, 2), Some(vec![0, 2]));
    assert_eq!(journal_tail_mutation_view(&recs, 3), None);
    assert_eq!(journal_tail_mutation_view(&recs, 0), None);
}

#[test]
fn undo_restores_state_and_truncates_journal() {
    pin();
    let dir = tmpdir("undo_seq");
    let jp = dir.join("journal.log");
    let mut st = State::default();
    let mut snaps = vec![serialize(&st)];
    macro_rules! step {
        ($e:expr) => {{
            let r = $e;
            assert_eq!(r.code, 0, "setup op failed: {}", r.err);
            for l in &r.journal {
                append_journal_record(&jp, l).unwrap();
            }
            snaps.push(serialize(&st));
        }};
    }
    step!(op_add(&mut st, "a", &kw(&[("title", "Area One")])));
    step!(op_add(
        &mut st,
        "g",
        &kw(&[
            ("title", "Goal one"),
            ("area", "A-01"),
            ("fitness-kind", "count"),
            ("fitness-target", "1"),
        ]),
    ));
    step!(op_add(
        &mut st,
        "w",
        &kw(&[
            ("title", "Work one"),
            ("goals", "G-01"),
            ("type", "feature"),
            ("cynefin", "clear"),
        ]),
    ));
    step!(op_field(&mut st, "W-01", "ac", "add", Some("first ac"), EFF));
    step!(op_field(
        &mut st,
        "W-01",
        "evidence_strategy",
        "add",
        Some("run tests"),
        EFF
    ));
    step!(op_field(
        &mut st,
        "W-01",
        "hypothesis",
        "add",
        Some("it works"),
        EFF
    ));
    step!(op_fitness(&mut st, "W-01", "G-01", 2, EFF));
    step!(op_set(&mut st, "W-01", "status", "ready", EFF));
    step!(op_set(&mut st, "W-01", "status", "progress", EFF));
    assert_eq!(journal_read_nonempty_pairs(&jp).0.len(), 9);

    let r = op_undo(&mut st, &jp, None, None, "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(serialize(&st), snaps[8]);
    assert_eq!(
        r.journal,
        vec![r#"{"v":1,"session":"none","cmd":"undo","ts":"2031-01-01T00:00:00Z","inv":{"steps":1,"op":"undo"}}"#.to_string()]
    );

    let r = op_undo(&mut st, &jp, None, Some("2"), "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(serialize(&st), snaps[6]);

    let r = op_undo(&mut st, &jp, None, Some("3"), "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(serialize(&st), snaps[3]);

    let (raw, _) = journal_read_nonempty_pairs(&jp);
    assert_eq!(raw.len(), 6);
    assert!(raw[0].contains(r#""id":"A-01""#), "{}", raw[0]);
    assert!(raw[1].contains(r#""id":"G-01""#), "{}", raw[1]);
    assert!(raw[2].contains(r#""id":"W-01""#), "{}", raw[2]);
    for (i, steps) in [(3, 1), (4, 2), (5, 3)] {
        assert!(
            raw[i].contains(&format!(r#"{{"steps":{steps},"op":"undo"}}"#)),
            "{}",
            raw[i]
        );
    }

    let r = op_undo(&mut st, &jp, None, Some("4"), "none");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        "grove undo: journal has fewer than 4 mutation entries\n"
    );

    let r = op_undo(&mut st, &jp, None, Some("3"), "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(serialize(&st), snaps[0]);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn undo_skips_gate_and_distill_records() {
    pin();
    let dir = tmpdir("undo_skip");
    let jp = dir.join("journal.log");
    let mut st = State::default();
    let r1 = op_add(&mut st, "a", &kw(&[("title", "Area One")]));
    for l in &r1.journal {
        append_journal_record(&jp, l).unwrap();
    }
    append_journal_record(&jp, r#"{"v":1,"cmd":"gate","ts":"2031-01-01T00:00:00Z","inv":{"dones":1,"overflows":[],"invalidated":[],"op":"gate","tw":2,"empty":true,"overflow_counts":{}}}"#).unwrap();
    append_journal_record(&jp, r#"{"v":1,"cmd":"distill","ts":"2031-01-01T00:00:00Z","inv":{"goal":"G-01","op":"distill","empty":true}}"#).unwrap();
    let snap_before_t = serialize(&st);
    let r2 = op_add(&mut st, "t", &kw(&[("title", "Theme one")]));
    for l in &r2.journal {
        append_journal_record(&jp, l).unwrap();
    }

    let r = op_undo(&mut st, &jp, None, Some("1"), "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(!st.nodes.contains_key("T-01"));
    assert!(st.nodes.contains_key("A-01"));
    assert_eq!(serialize(&st), snap_before_t);

    let (raw, _) = journal_read_nonempty_pairs(&jp);
    assert_eq!(raw.len(), 4);
    assert!(raw[0].contains(r#""cmd":"add""#), "{}", raw[0]);
    assert!(raw[1].contains(r#""cmd":"gate""#), "{}", raw[1]);
    assert!(raw[2].contains(r#""cmd":"distill""#), "{}", raw[2]);
    assert!(raw[3].contains(r#""op":"undo""#), "{}", raw[3]);

    let r = op_undo(&mut st, &jp, None, Some("3"), "none");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        "grove undo: journal has fewer than 3 mutation entries\n"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn undo_glossary_rename_restores_tags_and_text() {
    pin();
    let dir = tmpdir("undo_gloss");
    let jp = dir.join("journal.log");
    let mut st = State::default();
    let mut g = Some(INIT_GLOSSARY.to_string());
    let r = op_add(&mut st, "d", &kw(&[("title", "Decision one")]));
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(
        &mut st,
        "y",
        &kw(&[
            ("title", "Discovery one"),
            ("tags", "term-one"),
            ("surface", "src/x.jl"),
            ("from", "D-01"),
        ]),
    );
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(
        &mut st,
        "y",
        &kw(&[
            ("title", "Discovery two"),
            ("tags", "term-three,term-four"),
            ("why", "because reasons"),
            ("from", "D-01"),
        ]),
    );
    assert_eq!(r.code, 0, "{}", r.err);
    let snap_before = serialize(&st);

    let r1 = op_glossary_rename(&mut st, &mut g, "term-one", "term-two");
    assert_eq!(r1.code, 0, "{}", r1.err);
    assert_eq!(
        r1.journal,
        vec![r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-two","glossary_changed":false,"old":"term-one","tags":{"Y-01":["term-one"]},"op":"glossary_rename_restore"}}"#.to_string()]
    );
    assert_eq!(g.as_deref(), Some(INIT_GLOSSARY));
    assert_eq!(st.nodes["Y-01"].lines("tags"), vec!["term-two".to_string()]);

    let with_row = format!("{INIT_GLOSSARY}| term-three | a term | W-01 |\n");
    g = Some(with_row.clone());
    let r2 = op_glossary_rename(&mut st, &mut g, "term-three", "term-five");
    assert_eq!(r2.code, 0, "{}", r2.err);
    assert_eq!(
        r2.journal,
        vec![r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-five","glossary_changed":true,"old":"term-three","tags":{"Y-02":["term-three","term-four"]},"op":"glossary_rename_restore"}}"#.to_string()]
    );
    assert_eq!(
        g.as_deref(),
        Some(format!("{INIT_GLOSSARY}| term-five | a term | W-01 |\n").as_str())
    );
    assert_eq!(
        st.nodes["Y-02"].lines("tags"),
        vec!["term-five".to_string(), "term-four".to_string()]
    );

    for l in r1.journal.iter().chain(r2.journal.iter()) {
        append_journal_record(&jp, l).unwrap();
    }
    let r = op_undo(&mut st, &jp, g.as_mut(), Some("2"), "none");
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(g.as_deref(), Some(with_row.as_str()));
    assert_eq!(st.nodes["Y-01"].lines("tags"), vec!["term-one".to_string()]);
    assert_eq!(
        st.nodes["Y-02"].lines("tags"),
        vec!["term-three".to_string(), "term-four".to_string()]
    );
    assert_eq!(serialize(&st), snap_before);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn undo_error_paths() {
    pin();
    let dir = tmpdir("undo_err");
    let jp = dir.join("journal.log");
    let mut st = State::default();

    let r = op_undo(&mut st, &jp, None, None, "none");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        format!("grove undo: no journal at {}\n", jp.display())
    );

    std::fs::write(&jp, "").unwrap();
    let r = op_undo(&mut st, &jp, None, None, "none");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        format!("grove undo: no journal at {}\n", jp.display())
    );

    append_journal_record(
        &jp,
        r#"{"v":1,"cmd":"gate","ts":"2031-01-01T00:00:00Z","inv":{"dones":0,"overflows":[],"invalidated":[],"op":"gate","tw":0,"empty":true,"overflow_counts":{}}}"#,
    )
    .unwrap();
    let r = op_undo(&mut st, &jp, None, None, "none");
    assert_eq!(r.code, 1);
    assert_eq!(
        r.err,
        "grove undo: journal has fewer than 1 mutation entry\n"
    );

    let r = op_undo(&mut st, &jp, None, Some("abc"), "none");
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "grove undo: bad --steps\n");

    let r = op_undo(&mut st, &jp, None, Some("0"), "none");
    assert_eq!(r.code, 0);
    assert!(r.journal.is_empty());
    assert_eq!(journal_read_nonempty_pairs(&jp).0.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn apply_inverse_reports_malformed_records() {
    pin();
    let mut st = State::default();
    let inv = parse_json(r#"{"id":"A-01"}"#).unwrap();
    assert_eq!(
        journal_apply_inverse(&mut st, &inv).as_deref(),
        Some("journal undo: unknown inverse op ``")
    );
    let inv = parse_json(r#"{"op":"rm_node"}"#).unwrap();
    assert_eq!(
        journal_apply_inverse(&mut st, &inv).as_deref(),
        Some("journal undo: malformed record: missing `id`")
    );
    let inv = parse_json(r#"{"op":"rm_node","id":"A-99"}"#).unwrap();
    assert_eq!(journal_apply_inverse(&mut st, &inv), None);
    let inv = parse_json(r#"{"op":"unlink_edge","from":"A-01","label":"blocks","to":"A-02"}"#).unwrap();
    assert_eq!(
        journal_apply_inverse(&mut st, &inv).as_deref(),
        Some("journal undo: unlink_edge: missing edge A-01 blocks A-02")
    );
}
