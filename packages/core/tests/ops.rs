mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::PathBuf;

const TS: &str = "2031-01-01T00:00:00Z";
const TS_OLD: &str = "2020-06-01T00:00:00Z";
const EFF: &str = "testsession";
const INIT_GLOSSARY: &str = "# Glossary\n\n| Term | Definition | Source |\n| --- | --- | --- |\n";

const LOCK_ADDS: &str = include_str!("fixtures/lock_after_adds.lock");
const LOCK_DONE: &str = include_str!("fixtures/lock_after_done.lock");
const LOCK_RENUMBER: &str = include_str!("fixtures/lock_after_renumber.lock");
const LOCK_BEFORE_UNDO: &str = include_str!("fixtures/lock_before_undo.lock");
const LOCK_AFTER_UNDO3: &str = include_str!("fixtures/lock_after_undo3.lock");
const LOCK_AFTER_UNDO1: &str = include_str!("fixtures/lock_after_undo1.lock");

const DISTILL_REC: &str = r#"{"v":1,"cmd":"distill","ts":"2031-01-01T00:00:00Z","inv":{"goal":"G-01","op":"distill","empty":true}}"#;
const GATE_REC: &str = r#"{"v":1,"cmd":"gate","ts":"2031-01-01T00:00:00Z","inv":{"dones":1,"overflows":[],"invalidated":[],"op":"gate","tw":2,"empty":true,"overflow_counts":{}}}"#;

fn pin(ts: &str) {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(ts).unwrap()));
}

fn kw(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-otest-{}-{}-{}",
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

struct Ctx {
    st: State,
    jp: PathBuf,
    mirror: Vec<String>,
}

impl Ctx {
    fn step(&mut self, label: &str, r: OpResult, code: i32, out: &str, err: &str, jl: &[&str]) {
        assert_eq!(r.code, code, "{label} rc");
        assert_eq!(r.out, out, "{label} out");
        assert_eq!(r.err, err, "{label} err");
        let got: Vec<&str> = r.journal.iter().map(|s| s.as_str()).collect();
        assert_eq!(got, jl, "{label} journal");
        for l in &r.journal {
            append_journal_record(&self.jp, l).unwrap();
            self.mirror.push(l.clone());
        }
    }

    fn file_lines(&self) -> Vec<String> {
        journal_read_nonempty_pairs(&self.jp).0
    }

    fn assert_file_mirror(&self) {
        assert_eq!(self.file_lines(), self.mirror, "journal file != mirror");
    }
}

macro_rules! step {
    ($ctx:ident, $label:expr, $op:expr, $code:expr, $out:expr, $err:expr, $jl:expr) => {{
        let r = $op;
        $ctx.step($label, r, $code, $out, $err, $jl);
    }};
}

#[test]
fn replay_truth_scenario() {
    pin(TS);
    let dir = tmpdir("replay");
    let mut ctx = Ctx {
        st: State::default(),
        jp: dir.join("journal.log"),
        mirror: Vec::new(),
    };

    step!(ctx, "add_a", op_add(&mut ctx.st, "a", &kw(&[("title", "Area One")])), 0, "A-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"A-01","op":"rm_node"}}"#]);
    step!(ctx, "add_g", op_add(&mut ctx.st, "g", &kw(&[("title", "Goal one"), ("area", "A-01"), ("fitness-kind", "count"), ("fitness-target", "1")])), 0, "G-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-01","op":"rm_node"}}"#]);
    step!(ctx, "add_g2", op_add(&mut ctx.st, "g", &kw(&[("title", "Goal two"), ("area", "A-01")])), 0, "G-02\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","op":"rm_node"}}"#]);
    step!(ctx, "add_t", op_add(&mut ctx.st, "t", &kw(&[("title", "Theme one")])), 0, "T-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"T-01","op":"rm_node"}}"#]);
    step!(ctx, "add_w", op_add(&mut ctx.st, "w", &kw(&[("title", "Work one"), ("goals", "G-01"), ("theme", "T-01"), ("type", "feature"), ("cynefin", "clear")])), 0, "W-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","op":"rm_node"}}"#]);
    step!(ctx, "add_w2", op_add(&mut ctx.st, "w", &kw(&[("title", "Work two")])), 0, "W-02\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-02","op":"rm_node"}}"#]);
    step!(ctx, "add_d1", op_add(&mut ctx.st, "d", &kw(&[("title", "Decision one")])), 0, "D-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"D-01","op":"rm_node"}}"#]);
    step!(ctx, "add_d2", op_add(&mut ctx.st, "d", &kw(&[("title", "Decision two"), ("supersedes", "D-01")])), 0, "D-02\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"D-02","op":"rm_node"}}"#]);
    step!(ctx, "add_q", op_add(&mut ctx.st, "q", &kw(&[("title", "Question one"), ("targets", "W-01")])), 0, "Q-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"Q-01","op":"rm_node"}}"#]);
    step!(ctx, "add_b", op_add(&mut ctx.st, "b", &kw(&[("title", "Bet one"), ("tests", "Q-01"), ("targets", "W-01")])), 0, "B-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"B-01","op":"rm_node"}}"#]);
    step!(ctx, "add_y_w", op_add(&mut ctx.st, "y", &kw(&[("title", "Discovery one"), ("tags", "term-one"), ("surface", "src/x.jl"), ("from", "W-01")])), 0, "Y-01\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"Y-01","op":"rm_node"}}"#]);
    step!(ctx, "add_y_d", op_add(&mut ctx.st, "y", &kw(&[("title", "Discovery two"), ("tags", "term-three,term-four"), ("why", "because reasons"), ("from", "D-01")])), 0, "Y-02\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"Y-02","op":"rm_node"}}"#]);

    step!(ctx, "add_g_noarea", op_add(&mut ctx.st, "g", &kw(&[("title", "Goal bad")])), 1, "",
        "add g: --area=A-NN is required (create one with grove add a --title=...)\n", &[]);
    step!(ctx, "add_g_badarea", op_add(&mut ctx.st, "g", &kw(&[("title", "Goal bad"), ("area", "A-99")])), 1, "",
        "add g: unknown --area id: A-99\n", &[]);
    step!(ctx, "add_y_notitle", op_add(&mut ctx.st, "y", &kw(&[("tags", "term-one"), ("from", "W-01")])), 1, "",
        "add y: --title is required\n", &[]);
    step!(ctx, "add_y_notags", op_add(&mut ctx.st, "y", &kw(&[("title", "X"), ("from", "W-01")])), 1, "",
        "add y: --tags=<t1,t2> is required (≥1 glossary term)\n", &[]);
    step!(ctx, "add_y_nowhy", op_add(&mut ctx.st, "y", &kw(&[("title", "X"), ("tags", "term-one"), ("from", "W-01")])), 1, "",
        "add y: --surface absent requires --why prose\n", &[]);
    step!(ctx, "add_y_nofrom", op_add(&mut ctx.st, "y", &kw(&[("title", "X"), ("tags", "term-one"), ("surface", "src/x.jl")])), 1, "",
        "add y: --from=<W-NN|D-NN|Q-NN|B-NN> is required (≥1 provenance record)\n", &[]);
    step!(ctx, "add_y_badfrom", op_add(&mut ctx.st, "y", &kw(&[("title", "X"), ("tags", "term-one"), ("surface", "src/x.jl"), ("from", "Z-99")])), 4, "",
        "add y: unknown --from id: Z-99\n", &[]);

    assert_eq!(serialize(&ctx.st), LOCK_ADDS, "lock after adds");
    ctx.assert_file_mirror();

    step!(ctx, "field_ac_add", op_field(&mut ctx.st, "W-01", "ac", "add", Some("first ac"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","op":"field_pop_last"}}"#]);
    step!(ctx, "field_ac_add2", op_field(&mut ctx.st, "W-01", "ac", "add", Some("second ac"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","op":"field_pop_last"}}"#]);
    step!(ctx, "field_ac_rm", op_field(&mut ctx.st, "W-01", "ac", "rm", Some("1"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"line":"first ac","id":"W-01","field":"ac","op":"field_insert_line","index":1}}"#]);
    step!(ctx, "field_ac_clear", op_field(&mut ctx.st, "W-01", "ac", "clear", None, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","lines":["second ac"],"op":"field_restore_lines"}}"#]);
    step!(ctx, "field_ac_add3", op_field(&mut ctx.st, "W-01", "ac", "add", Some("ac again"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"ac","op":"field_pop_last"}}"#]);
    step!(ctx, "field_goals_add", op_field(&mut ctx.st, "W-01", "goals", "add", Some("G-02"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"goals","op":"field_pop_last"}}"#]);
    step!(ctx, "field_theme_clear", op_field(&mut ctx.st, "W-01", "theme", "clear", None, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"theme","value":"T-01","op":"field_restore_single"}}"#]);
    step!(ctx, "field_theme_add", op_field(&mut ctx.st, "W-01", "theme", "add", Some("T-01"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"theme","value":"","op":"field_restore_single"}}"#]);
    step!(ctx, "field_es_add", op_field(&mut ctx.st, "W-01", "evidence_strategy", "add", Some("run tests"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"evidence_strategy","op":"field_pop_last"}}"#]);
    step!(ctx, "field_hyp_add", op_field(&mut ctx.st, "W-01", "hypothesis", "add", Some("it works"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"hypothesis","op":"field_pop_last"}}"#]);
    step!(ctx, "field_unknown", op_field(&mut ctx.st, "W-01", "bogus", "add", Some("x"), EFF), 1, "",
        "unknown field bogus on w\n", &[]);
    step!(ctx, "field_fitness_add", op_field(&mut ctx.st, "W-01", "fitness", "add", Some("G-01=+1"), EFF), 1, "",
        "field fitness not addable\n", &[]);
    step!(ctx, "field_fitness_clear", op_field(&mut ctx.st, "W-01", "fitness", "clear", None, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"map":{},"id":"W-01","field":"fitness","op":"field_restore_fitness"}}"#]);
    step!(ctx, "fitness_stage1", op_fitness(&mut ctx.st, "W-01", "G-01", 2, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":false,"wid":"W-01","gid":"G-01","previous":null,"op":"restore_fitness_key"}}"#]);
    step!(ctx, "fitness_stage2", op_fitness(&mut ctx.st, "W-01", "G-01", 1, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":true,"wid":"W-01","gid":"G-01","previous":2,"op":"restore_fitness_key"}}"#]);
    step!(ctx, "fitness_stage3", op_fitness(&mut ctx.st, "W-01", "G-02", 0, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":false,"wid":"W-01","gid":"G-02","previous":null,"op":"restore_fitness_key"}}"#]);
    step!(ctx, "set_cynefin", op_set(&mut ctx.st, "W-01", "cynefin", "complicated", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"clear","op":"set_cynefin"}}"#]);
    step!(ctx, "set_type", op_set(&mut ctx.st, "W-01", "type", "bug", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"feature","op":"set_type"}}"#]);
    step!(ctx, "set_type2", op_set(&mut ctx.st, "W-01", "type", "feature", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"bug","op":"set_type"}}"#]);
    step!(ctx, "set_title", op_set(&mut ctx.st, "W-01", "title", "Work One Renamed", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","old":"Work one","op":"set_title"}}"#]);
    step!(ctx, "set_g_fitness", op_set(&mut ctx.st, "G-02", "fitness", "1/2", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","old":"","op":"set_g_attr_fitness"}}"#]);
    step!(ctx, "set_g_fitness_kind", op_set(&mut ctx.st, "G-02", "fitness_kind", "count", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"new":"count","id":"G-02","had_before":false,"old":"","op":"set_g_attr_fitness_kind"}}"#]);
    step!(ctx, "set_g_fitness_kind2", op_set(&mut ctx.st, "G-02", "fitness_kind", "ratio", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"new":"ratio","id":"G-02","had_before":true,"old":"count","op":"set_g_attr_fitness_kind"}}"#]);
    step!(ctx, "set_g_fitness_kind_bad", op_set(&mut ctx.st, "G-02", "fitness_kind", "bogus", EFF), 1, "",
        "bad fitness_kind (expected one of: count, ratio, boolean, metric, manual)\n", &[]);
    step!(ctx, "set_g_area", op_set(&mut ctx.st, "G-02", "area", "A-01", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","had_before":true,"old":"A-01","op":"set_g_area"}}"#]);
    step!(ctx, "set_g_area_bad", op_set(&mut ctx.st, "G-02", "area", "A-99", EFF), 1, "",
        "set: unknown area: A-99 (expected an existing A-NN node)\n", &[]);
    step!(ctx, "set_g_rc", op_set(&mut ctx.st, "G-02", "requires_coverage", "true", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"id":"G-02","had_before":false,"old":"","op":"set_requires_coverage"}}"#]);
    step!(ctx, "set_g_rc_bad", op_set(&mut ctx.st, "G-02", "requires_coverage", "bogus", EFF), 1, "",
        "bad requires_coverage (expected `true` or a float in (0,1])\n", &[]);
    step!(ctx, "set_unsupported", op_set(&mut ctx.st, "W-01", "bogus", "1", EFF), 1, "",
        "unsupported key: bogus\n", &[]);
    step!(ctx, "set_q_status", op_set(&mut ctx.st, "Q-01", "status", "answered", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"old_status":"open","id":"Q-01","op":"set_status_plain"}}"#]);
    step!(ctx, "set_b_status", op_set(&mut ctx.st, "B-01", "status", "validated", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"old_status":"proposed","id":"B-01","op":"set_status_plain"}}"#]);
    step!(ctx, "link_causes", op_link(&mut ctx.st, "T-01", "causes", "W-01", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"link","ts":"2031-01-01T00:00:00Z","inv":{"label":"causes","to":"W-01","op":"unlink_edge","from":"T-01"}}"#]);
    step!(ctx, "unlink_causes", op_unlink(&mut ctx.st, "T-01", "causes", "W-01", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"unlink","ts":"2031-01-01T00:00:00Z","inv":{"label":"causes","t_created":"2031-01-01T00:00:00Z","to":"W-01","op":"restore_edge","from":"T-01"}}"#]);
    let before_cycle = serialize(&ctx.st);
    step!(ctx, "link_cycle", op_link(&mut ctx.st, "W-01", "blocks", "W-01", EFF), 4, "",
        "I7: blocks introduces a cycle\n", &[]);
    assert_eq!(serialize(&ctx.st), before_cycle, "cycle refusal must not mutate");
    step!(ctx, "link_missing", op_link(&mut ctx.st, "W-01", "blocks", "W-99", EFF), 4, "",
        "missing node W-99\n", &[]);
    step!(ctx, "unlink_nosuch", op_unlink(&mut ctx.st, "W-01", "blocks", "W-02", EFF), 5, "",
        "no such edge\n", &[]);
    step!(ctx, "set_w_ready", op_set(&mut ctx.st, "W-01", "status", "ready", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"proposed","had_session_before":false,"id":"W-01","op":"set_w_status_with_goals","goal_statuses":{"G-01":"unverified","G-02":"unverified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "set_w_progress", op_set(&mut ctx.st, "W-01", "status", "progress", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"ready","had_session_before":false,"id":"W-01","op":"set_w_status_with_goals","goal_statuses":{"G-01":"unverified","G-02":"unverified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "evidence_add", op_evidence(&mut ctx.st, "W-01", "did the thing", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"evidence","op":"field_pop_last"}}"#]);
    step!(ctx, "fitness_progress", op_fitness(&mut ctx.st, "W-01", "G-01", 3, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":true,"wid":"W-01","gid":"G-01","previous":1,"op":"restore_fitness_key"}}"#]);
    step!(ctx, "set_w_done", op_set(&mut ctx.st, "W-01", "status", "done", EFF), 0, "",
        "grove: goal G-01 (Goal one) verified, distill content: `grove distill G-01` (or `grove distill G-01 --null` when nothing is worth keeping; lazy distill, see rules.md). To skip: add a `notes` prose line containing `--distill-deferred`.\n",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":true,"old_w_status":"progress","had_session_before":true,"id":"W-01","op":"set_w_status_with_goals","goal_statuses":{"G-01":"unverified","G-02":"unverified"},"old_session_at":"2031-01-01T00:00:00Z","old_session":"testsession"}}"#]);

    assert_eq!(serialize(&ctx.st), LOCK_DONE, "lock after done");

    step!(ctx, "dor_reject", op_set(&mut ctx.st, "W-02", "status", "progress", EFF), 4, "",
        "DoR ≢ ⊤ for W-02; see `grove dor W-02`\n",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"missing":["goals(w) ≠ ∅","AC(w) ≠ ∅","fitness deltas set ∀ g","evidence_strategy ≠ ∅","hypothesis ≠ ⊥"],"id":"W-02","op":"dor_reject"}}"#]);
    step!(ctx, "renumber", op_renumber(&mut ctx.st, "W-02", "W-05", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"renumber","ts":"2031-01-01T00:00:00Z","inv":{"to":"W-02","op":"renumber_swap","from":"W-05"}}"#]);
    step!(ctx, "evidence_w01", op_evidence(&mut ctx.st, "W-01", "verified W-01 behavior", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-01","field":"evidence","op":"field_pop_last"}}"#]);
    step!(ctx, "renumber_refused", op_renumber(&mut ctx.st, "W-01", "W-09", EFF), 4, "",
        "grove renumber: refusing; id occurs in evidence on a done W\n", &[]);

    assert_eq!(serialize(&ctx.st), LOCK_RENUMBER, "lock after renumber");

    let mut g = Some(INIT_GLOSSARY.to_string());
    {
        let r = op_glossary_rename(&mut ctx.st, &mut g, "term-one", "term-two");
        ctx.step("gloss_rename_tags", r, 0, "", "",
            &[r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-two","glossary_changed":false,"old":"term-one","tags":{"Y-01":["term-one"]},"op":"glossary_rename_restore"}}"#]);
        assert_eq!(g.as_deref(), Some(INIT_GLOSSARY));
        let with_row = format!("{INIT_GLOSSARY}| term-three | a term | W-01 |\n");
        g = Some(with_row);
        let r = op_glossary_rename(&mut ctx.st, &mut g, "term-three", "term-five");
        ctx.step("gloss_rename_full", r, 0, "", "",
            &[r#"{"v":1,"cmd":"glossary","ts":"2031-01-01T00:00:00Z","inv":{"new":"term-five","glossary_changed":true,"old":"term-three","tags":{"Y-02":["term-three","term-four"]},"op":"glossary_rename_restore"}}"#]);
        assert_eq!(
            g.as_deref(),
            Some(format!("{INIT_GLOSSARY}| term-five | a term | W-01 |\n").as_str())
        );
        let r = op_glossary_rename(&mut ctx.st, &mut g, "nosuchterm", "x");
        ctx.step("gloss_rename_absent", r, 5, "",
            "glossary rename: `nosuchterm` is neither in glossary.md nor used by any discovery\n", &[]);
        let r = op_glossary_rename(&mut ctx.st, &mut g, "term-two", "term-five");
        ctx.step("gloss_rename_exists", r, 4, "",
            "glossary rename: `term-five` already present in glossary.md\n", &[]);
        let r = op_glossary_rename(&mut ctx.st, &mut g, "term-two", "term-two");
        ctx.step("gloss_rename_same", r, 1, "",
            "glossary rename: old and new are identical\n", &[]);
    }
    step!(ctx, "set_g2_declined", op_set(&mut ctx.st, "G-02", "status", "declined", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"old_status":"unverified","id":"G-02","op":"set_status_plain"}}"#]);
    step!(ctx, "add_w3", op_add(&mut ctx.st, "w", &kw(&[("title", "Work three"), ("goals", "G-01")])), 0, "W-06\n", "",
        &[r#"{"v":1,"cmd":"add","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-06","op":"rm_node"}}"#]);
    step!(ctx, "field_w3_ac", op_field(&mut ctx.st, "W-06", "ac", "add", Some("ac3"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-06","field":"ac","op":"field_pop_last"}}"#]);
    step!(ctx, "field_w3_es", op_field(&mut ctx.st, "W-06", "evidence_strategy", "add", Some("es3"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-06","field":"evidence_strategy","op":"field_pop_last"}}"#]);
    step!(ctx, "field_w3_hyp", op_field(&mut ctx.st, "W-06", "hypothesis", "add", Some("h3"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"field","ts":"2031-01-01T00:00:00Z","inv":{"id":"W-06","field":"hypothesis","op":"field_pop_last"}}"#]);
    step!(ctx, "fitness_w3", op_fitness(&mut ctx.st, "W-06", "G-01", 1, EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"fitness","ts":"2031-01-01T00:00:00Z","inv":{"had_key":false,"wid":"W-06","gid":"G-01","previous":null,"op":"restore_fitness_key"}}"#]);
    step!(ctx, "set_w3_ready", op_set(&mut ctx.st, "W-06", "status", "ready", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"proposed","had_session_before":false,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "set_w3_progress", op_set(&mut ctx.st, "W-06", "status", "progress", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"ready","had_session_before":false,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "handoff_nonholder", op_handoff(&mut ctx.st, "W-06", Some("newtok"), "intruder"), 4, "",
        "only the holding session can hand off; use `grove resume` first\n", &[]);
    step!(ctx, "handoff_ok", op_handoff(&mut ctx.st, "W-06", Some("newtok"), EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"handoff","ts":"2031-01-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"testsession","had_session_at_before":true}}"#]);
    step!(ctx, "resume_ok", op_resume(&mut ctx.st, "W-06", "newtok"), 0, "", "",
        &[r#"{"v":1,"cmd":"resume","ts":"2031-01-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"newtok","had_session_at_before":true}}"#]);
    step!(ctx, "revert_nonholder", op_revert(&mut ctx.st, "W-06", "intruder"), 4, "",
        "I11/session: cannot release W-06: token differs and claim is fresh (<24h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait\n", &[]);
    step!(ctx, "revert_ok", op_revert(&mut ctx.st, "W-06", "newtok"), 0, "", "",
        &[r#"{"v":1,"cmd":"revert","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":true,"old_w_status":"progress","had_session_before":true,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"2031-01-01T00:00:00Z","old_session":"newtok"}}"#]);

    append_journal_record(&ctx.jp, DISTILL_REC).unwrap();
    ctx.mirror.push(DISTILL_REC.to_string());
    append_journal_record(&ctx.jp, GATE_REC).unwrap();
    ctx.mirror.push(GATE_REC.to_string());

    assert_eq!(serialize(&ctx.st), LOCK_BEFORE_UNDO, "lock before undo");
    ctx.assert_file_mirror();

    let r = op_undo(&mut ctx.st, &ctx.jp, None, Some("3"), "none");
    assert_eq!(r.code, 0, "undo3 rc: {}", r.err);
    assert_eq!(
        r.journal,
        vec![r#"{"v":1,"session":"none","cmd":"undo","ts":"2031-01-01T00:00:00Z","inv":{"steps":3,"op":"undo"}}"#.to_string()]
    );
    {
        let n = ctx.mirror.len();
        let d = ctx.mirror[n - 2].clone();
        let grec = ctx.mirror[n - 1].clone();
        ctx.mirror.truncate(n - 5);
        ctx.mirror.push(d);
        ctx.mirror.push(grec);
        ctx.mirror.push(r.journal[0].clone());
    }
    assert_eq!(serialize(&ctx.st), LOCK_AFTER_UNDO3, "lock after undo3");
    ctx.assert_file_mirror();

    let r = op_undo(&mut ctx.st, &ctx.jp, None, None, "none");
    assert_eq!(r.code, 0, "undo1 rc: {}", r.err);
    assert_eq!(
        r.journal,
        vec![r#"{"v":1,"session":"none","cmd":"undo","ts":"2031-01-01T00:00:00Z","inv":{"steps":1,"op":"undo"}}"#.to_string()]
    );
    {
        let n = ctx.mirror.len();
        let tail: Vec<String> = ctx.mirror[n - 3..].to_vec();
        ctx.mirror.truncate(n - 4);
        ctx.mirror.extend(tail);
        ctx.mirror.push(r.journal[0].clone());
    }
    assert_eq!(serialize(&ctx.st), LOCK_AFTER_UNDO1, "lock after undo1");
    ctx.assert_file_mirror();

    step!(ctx, "set_w3_progress2", op_set(&mut ctx.st, "W-06", "status", "progress", "newtok"), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"ready","had_session_before":false,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "field_w3_denied", op_field(&mut ctx.st, "W-06", "ac", "add", Some("sneaky"), "intruder"), 4, "",
        "I11/session: W-06 is `progress` and owned by another session; try `grove resume W-06` after adopting, or coordinate a `grove handoff`\n", &[]);
    step!(ctx, "set_w3_denied", op_set(&mut ctx.st, "W-06", "title", "Sneaky", "intruder"), 4, "",
        "I11/session: W-06 is `progress` and owned by another session; try `grove resume W-06` after adopting, or coordinate a `grove handoff`\n", &[]);
    step!(ctx, "revert_w3_intruder", op_revert(&mut ctx.st, "W-06", "intruder"), 4, "",
        "I11/session: cannot release W-06: token differs and claim is fresh (<24h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait\n", &[]);
    pin(TS_OLD);
    step!(ctx, "resume_w3_staleclaim", op_resume(&mut ctx.st, "W-06", "newtok"), 0, "", "",
        &[r#"{"v":1,"cmd":"resume","ts":"2020-06-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"newtok","had_session_at_before":true}}"#]);
    pin(TS);
    step!(ctx, "revert_w3_stale", op_revert(&mut ctx.st, "W-06", "intruder"), 0, "", "",
        &[r#"{"v":1,"cmd":"revert","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":true,"old_w_status":"progress","had_session_before":true,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"2020-06-01T00:00:00Z","old_session":"newtok"}}"#]);
    step!(ctx, "set_w3_progress3", op_set(&mut ctx.st, "W-06", "status", "progress", EFF), 0, "", "",
        &[r#"{"v":1,"cmd":"set","ts":"2031-01-01T00:00:00Z","inv":{"had_session_at_before":false,"old_w_status":"ready","had_session_before":false,"id":"W-06","op":"set_w_status_with_goals","goal_statuses":{"G-01":"verified"},"old_session_at":"","old_session":""}}"#]);
    step!(ctx, "resume_w3_notprogress", op_resume(&mut ctx.st, "W-02", EFF), 5, "", "", &[]);
    pin(TS_OLD);
    step!(ctx, "resume_w3_crash", op_resume(&mut ctx.st, "W-06", "crashgrabber"), 0, "", "",
        &[r#"{"v":1,"cmd":"resume","ts":"2020-06-01T00:00:00Z","inv":{"had_session_before":true,"id":"W-06","op":"session_restore_claim","old_session_at":"2031-01-01T00:00:00Z","old_session":"testsession","had_session_at_before":true}}"#]);
    pin(TS);
    step!(ctx, "set_w3_done_noev", op_set(&mut ctx.st, "W-06", "status", "done", "crashgrabber"), 4, "",
        "I3: W-06 has no evidence; use `grove evidence W-06 \"…\"`\n", &[]);

    assert_eq!(ctx.mirror.len(), 65, "final journal line count");
    ctx.assert_file_mirror();

    let _ = std::fs::remove_dir_all(&dir);
}

fn small_state() -> State {
    let mut st = State::default();
    let r = op_add(&mut st, "a", &kw(&[("title", "Area One")]));
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(
        &mut st,
        "g",
        &kw(&[("title", "Goal one"), ("area", "A-01")]),
    );
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(&mut st, "t", &kw(&[("title", "Theme one")]));
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(
        &mut st,
        "w",
        &kw(&[
            ("title", "Work one"),
            ("goals", "G-01"),
            ("theme", "T-01"),
            ("type", "feature"),
            ("cynefin", "clear"),
        ]),
    );
    assert_eq!(r.code, 0, "{}", r.err);
    st
}

#[test]
fn renumber_rewires_goal_fitness_theme_refs() {
    pin(TS);
    let mut st = small_state();
    let r = op_fitness(&mut st, "W-01", "G-01", 2, EFF);
    assert_eq!(r.code, 0, "{}", r.err);

    let r = op_renumber(&mut st, "G-01", "G-09", EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(
        r.journal,
        vec![r#"{"v":1,"cmd":"renumber","ts":"2031-01-01T00:00:00Z","inv":{"to":"G-01","op":"renumber_swap","from":"G-09"}}"#.to_string()]
    );
    let w = &st.nodes["W-01"];
    assert_eq!(w.lines("goals"), vec!["G-09".to_string()]);
    match w.fields.get("fitness") {
        Some(FieldValue::Fitness(m)) => {
            assert!(m.contains_key("G-09"));
            assert!(!m.contains_key("G-01"));
        }
        other => panic!("fitness field: {other:?}"),
    }

    let r = op_renumber(&mut st, "T-01", "T-09", EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(st.nodes["W-01"].single("theme"), "T-09");

    let r = op_renumber(&mut st, "G-09", "G-01", EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(st.nodes["W-01"].lines("goals"), vec!["G-01".to_string()]);

    let r = op_renumber(&mut st, "W-01", "W-01", EFF);
    assert_eq!(r.code, 0);
    assert!(r.journal.is_empty(), "no-op renumber must not journal");

    let r = op_renumber(&mut st, "W-01", "", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "bad --to\n");

    let r = op_renumber(&mut st, "G-77", "G-78", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "rename: missing record G-77\n");

    let r = op_renumber(&mut st, "G-01", "T-09", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "rename: target already exists T-09\n");

    let r = op_renumber(&mut st, "G-01", "W-77", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "rename: family mismatch G-01 vs W-77\n");
}

#[test]
fn missing_node_and_bad_arg_messages() {
    pin(TS);
    let mut st = small_state();

    let r = op_set(&mut st, "W-99", "title", "x", EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "not found: W-99\n");

    let r = op_field(&mut st, "W-99", "ac", "add", Some("x"), EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "not found: W-99\n");

    let r = op_fitness(&mut st, "W-99", "G-01", 1, EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "missing: W-99\n");

    let r = op_fitness(&mut st, "W-01", "G-99", 1, EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "missing: G-99\n");

    let r = op_handoff(&mut st, "W-99", Some("tok"), EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "");

    let r = op_revert(&mut st, "W-99", EFF);
    assert_eq!(r.code, 5);
    assert_eq!(r.err, "");

    let r = op_add(&mut st, "z", &kw(&[("title", "x")]));
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "unknown kind: z\n");

    let r = op_add(&mut st, "g", &kw(&[("title", "x"), ("area", "A-01"), ("fitness-kind", "bogus")]));
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "bad --fitness-kind\n");

    let r = op_link(&mut st, "T-01", "bogus", "W-01", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "unknown label: bogus\n");

    let r = op_field(&mut st, "W-01", "ac", "bogus", Some("x"), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "unknown op: bogus\n");

    let r = op_field(&mut st, "W-01", "ac", "add", None, EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "missing value\n");

    let r = op_field(&mut st, "W-01", "ac", "rm", None, EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "missing index\n");

    let r = op_field(&mut st, "W-01", "theme", "rm", Some("1"), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "index out of range\n");

    let r = op_field(&mut st, "W-01", "ac", "rm", Some("7"), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "index out of range\n");

    let r = op_field(&mut st, "W-01", "ac", "rm", Some("notanumber"), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "", "non-numeric rm index: empty stderr (Julia crashes)");
}

#[test]
fn fitness_current_derived_guard() {
    pin(TS);
    let mut st = State::default();
    let r = op_add(&mut st, "a", &kw(&[("title", "Area One")]));
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_add(
        &mut st,
        "g",
        &kw(&[
            ("title", "Goal one"),
            ("area", "A-01"),
            ("fitness-kind", "count"),
            ("fitness-target", "1"),
        ]),
    );
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_field(&mut st, "G-01", "fitness_current", "add", Some("5"), EFF);
    assert_eq!(r.code, 4);
    assert_eq!(
        r.err,
        "grove field: `fitness_current` is derived for structured goals; use kind=manual to author it\n"
    );
}

#[test]
fn session_guards_on_mutations() {
    pin(TS);
    let mut st = small_state();
    let r = op_field(&mut st, "W-01", "ac", "add", Some("a1"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_field(&mut st, "W-01", "evidence_strategy", "add", Some("es"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_field(&mut st, "W-01", "hypothesis", "add", Some("h"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_fitness(&mut st, "W-01", "G-01", 1, EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_set(&mut st, "W-01", "status", "ready", EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_set(&mut st, "W-01", "status", "progress", EFF);
    assert_eq!(r.code, 0, "{}", r.err);

    let denial = "I11/session: W-01 is `progress` and owned by another session; try `grove resume W-01` after adopting, or coordinate a `grove handoff`\n";
    let before = serialize(&st);

    let r = op_fitness(&mut st, "W-01", "G-01", 9, "intruder");
    assert_eq!(r.code, 4);
    assert_eq!(r.err, denial);

    let r = op_link(&mut st, "T-01", "causes", "W-01", "intruder");
    assert_eq!(r.code, 4);
    assert_eq!(r.err, denial);

    let r = op_unlink(&mut st, "W-01", "blocks", "T-01", "intruder");
    assert_eq!(r.code, 5, "missing edge check precedes session guard");

    let r = op_evidence(&mut st, "W-01", "sneaky", "intruder");
    assert_eq!(r.code, 4);
    assert_eq!(r.err, denial);

    let r = op_set(&mut st, "W-01", "status", "done", "intruder");
    assert_eq!(r.code, 4);
    assert_eq!(
        r.err,
        "I11/session: cannot release W-01: token differs and claim is fresh (<24h); pass the owning GROVE_SESSION/--session, use `grove resume`, or wait\n"
    );

    assert_eq!(serialize(&st), before, "denied ops must not mutate");

    let r = op_resume(&mut st, "G-01", "intruder");
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "not a work item\n");

    let r = op_handoff(&mut st, "G-01", Some("tok"), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "not a work item\n");

    let r = op_revert(&mut st, "G-01", EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "not a work item\n");

    let r = op_add(&mut st, "w", &kw(&[("title", "Work two")]));
    assert_eq!(r.code, 0, "{}", r.err);

    let r = op_resume(&mut st, "W-02", EFF);
    assert_eq!(r.code, 4);
    assert_eq!(r.err, "W-02 is not in progress\n");

    let r = op_handoff(&mut st, "W-02", Some("tok"), EFF);
    assert_eq!(r.code, 4);
    assert_eq!(r.err, "W-02 is not in progress\n");

    let r = op_revert(&mut st, "W-02", EFF);
    assert_eq!(r.code, 4);
    assert_eq!(r.err, "W-02 is not in progress\n");

    let r = op_handoff(&mut st, "W-01", None, EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "missing --to=<session-token>\n");

    let r = op_handoff(&mut st, "W-01", Some("  "), EFF);
    assert_eq!(r.code, 1);
    assert_eq!(r.err, "empty --to\n");
}

#[test]
fn handoff_requires_existing_claim() {
    pin(TS);
    let mut st = small_state();
    let r = op_field(&mut st, "W-01", "ac", "add", Some("a1"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_field(&mut st, "W-01", "evidence_strategy", "add", Some("es"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_field(&mut st, "W-01", "hypothesis", "add", Some("h"), EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_fitness(&mut st, "W-01", "G-01", 1, EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_set(&mut st, "W-01", "status", "ready", EFF);
    assert_eq!(r.code, 0, "{}", r.err);
    let r = op_set(&mut st, "W-01", "status", "progress", EFF);
    assert_eq!(r.code, 0, "{}", r.err);

    {
        let w = st.nodes.get_mut("W-01").unwrap();
        w.attrs.remove("session");
        w.attrs.remove("session_at");
    }
    let r = op_handoff(&mut st, "W-01", Some("tok"), EFF);
    assert_eq!(r.code, 4);
    assert_eq!(r.err, "W-01 has no session claim; use `grove resume`\n");
}
