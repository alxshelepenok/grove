mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::path::PathBuf;
use std::process::Command;

const TS_A: &str = "2026-01-01T00:00:00Z";
const TS_B: &str = "2026-01-01T01:00:00Z";

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-lockdiff-{}-{}-{}",
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

fn git(dir: &PathBuf, args: &[&str]) {
    let st = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "git {:?} failed", args);
}

fn git_init_commit(dir: &PathBuf, msg: &str) {
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.email=t@example.com",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            msg,
        ],
    );
}

fn fixture_states() -> (State, State) {
    let sc = corpus_json("diff-repair");
    let st_ref = parse_fixture(&step_field(&sc, 7, "lock")).unwrap();
    let mut st_wt = parse_fixture(&step_field(&sc, 10, "lock")).unwrap();
    st_wt
        .nodes
        .get_mut("W-01")
        .unwrap()
        .attrs
        .insert("t_updated".to_string(), TS_B.to_string());
    (st_ref, st_wt)
}

fn strictable_lock(step: usize, wt_t_updated: bool) -> String {
    let sc = corpus_json("diff-repair");
    let mut st = parse_fixture(&step_field(&sc, step, "lock")).unwrap();
    for n in st.nodes.values_mut() {
        for k in ["t_created", "t_updated"] {
            if let Some(v) = n.attrs.get_mut(k) {
                *v = TS_A.to_string();
            }
        }
    }
    for e in st.edges.iter_mut() {
        if e.t_created.is_some() {
            e.t_created = Some(TS_A.to_string());
        }
    }
    if wt_t_updated {
        st.nodes
            .get_mut("W-01")
            .unwrap()
            .attrs
            .insert("t_updated".to_string(), TS_B.to_string());
    }
    serialize(&st)
}

#[test]
fn fixture_diff_text_golden() {
    let sc = corpus_json("diff-repair");
    let (st_ref, st_wt) = fixture_states();
    assert_eq!(
        print_lock_structural_diff("HEAD", &st_ref, &st_wt),
        step_field(&sc, 11, "stdout")
    );
}

#[test]
fn fixture_diff_json_golden() {
    let sc = corpus_json("diff-repair");
    let (st_ref, st_wt) = fixture_states();
    let mut pl = lock_structural_diff_payload(&st_ref, &st_wt);
    pl.insert("command".to_string(), JVal::Str("diff".to_string()));
    pl.insert("since".to_string(), JVal::Str("HEAD".to_string()));
    assert_eq!(json_cli_out(pl), step_field(&sc, 12, "stdout"));
}

#[test]
fn cmd_diff_live_git_repo_matches_goldens() {
    let sc = corpus_json("diff-repair");
    let dir = tmpdir("live");
    let grove_dir = dir.join(".grove");
    std::fs::create_dir_all(&grove_dir).unwrap();
    std::fs::write(grove_dir.join("state.lock"), strictable_lock(7, false)).unwrap();
    git_init_commit(&dir, "grove state");
    std::fs::write(grove_dir.join("state.lock"), strictable_lock(10, true)).unwrap();
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root.clone());
    let r = cmd_diff(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_OK);
    assert_eq!(r.out, step_field(&sc, 11, "stdout"));
    assert_eq!(r.err, "");
    let mut ctxj = CliCtx::new(root);
    ctxj.json = true;
    let rj = cmd_diff(&ctxj, &[], &[]);
    assert_eq!(rj.code, EXIT_OK);
    assert_eq!(rj.out, step_field(&sc, 12, "stdout"));
    assert_eq!(rj.err, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_diff_not_a_repo() {
    let dir = tmpdir("norepo");
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root.clone());
    let r = cmd_diff(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_ERR);
    let rp = abspath(&root);
    assert_eq!(
        r.err,
        format!(
            "grove diff: not a git repository (--root=`{rp}`): cannot resolve `HEAD:.grove/state.lock` via git\n"
        )
    );
    assert_eq!(r.out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_diff_missing_worktree_lock() {
    let dir = tmpdir("nolock");
    git(&dir, &["init", "-q"]);
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root);
    let r = cmd_diff(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_ERR);
    assert_eq!(r.err, format!("lock not found: {}\n", ctx.lockpath().display()));
    assert_eq!(r.out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_diff_bad_ref_git_stderr_prefix() {
    let dir = tmpdir("badref");
    let grove_dir = dir.join(".grove");
    std::fs::create_dir_all(&grove_dir).unwrap();
    std::fs::write(grove_dir.join("state.lock"), strictable_lock(7, false)).unwrap();
    git(&dir, &["init", "-q"]);
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root);
    let kw = vec![("since".to_string(), "HEAD~9".to_string())];
    let r = cmd_diff(&ctx, &[], &kw);
    assert_eq!(r.code, EXIT_ERR);
    assert!(r.err.starts_with("grove diff: "), "err was: {}", r.err);
    assert!(r.err.ends_with('\n'), "err was: {}", r.err);
    assert_eq!(r.out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_diff_worktree_parse_error() {
    let dir = tmpdir("wtparse");
    git(&dir, &["init", "-q"]);
    let grove_dir = dir.join(".grove");
    std::fs::create_dir_all(&grove_dir).unwrap();
    std::fs::write(grove_dir.join("state.lock"), "nonsense\n").unwrap();
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root);
    let r = cmd_diff(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_ERR);
    let want = parse_strict("nonsense\n").unwrap_err().to_string();
    assert_eq!(r.err, format!("{want}\n"));
    assert_eq!(r.out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cmd_diff_blob_parse_error_suffix() {
    let dir = tmpdir("blobparse");
    let grove_dir = dir.join(".grove");
    std::fs::create_dir_all(&grove_dir).unwrap();
    std::fs::write(grove_dir.join("state.lock"), "nonsense\n").unwrap();
    git_init_commit(&dir, "bad lock");
    std::fs::write(grove_dir.join("state.lock"), strictable_lock(7, false)).unwrap();
    let root = dir.to_string_lossy().into_owned();
    let ctx = CliCtx::new(root);
    let r = cmd_diff(&ctx, &[], &[]);
    assert_eq!(r.code, EXIT_ERR);
    let want = parse_strict("nonsense\n").unwrap_err().to_string();
    assert_eq!(
        r.err,
        format!("{want}\n (while parsing `HEAD:.grove/state.lock`)\n")
    );
    assert_eq!(r.out, "");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sections_added_removed_changed() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut g1 = plain(Kind::G, "G-01", "unverified");
    g1.title = "goal one".to_string();
    let g2 = plain(Kind::G, "G-02", "unverified");
    let g3 = plain(Kind::G, "G-03", "partial");
    put(&mut ref_st, g1.clone());
    put(&mut ref_st, g2);
    put(&mut ref_st, g3);
    put(&mut wt_st, g1);
    let mut g3w = plain(Kind::G, "G-03", "verified");
    g3w.title = String::new();
    put(&mut wt_st, g3w);
    let mut g4 = plain(Kind::G, "G-04", "unverified");
    g4.title = "goal four".to_string();
    put(&mut wt_st, g4);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## G\n### added (+)\n+ g G-04 unverified  goal four\n### removed (-)\n- g G-02 unverified  (no title)\n### changed (~)\n~ G-03\n  header: g G-03 partial -> g G-03 verified\n\n";
    assert_eq!(got, want);
}

#[test]
fn header_forms_per_kind() {
    let ref_st = State::default();
    let mut wt_st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    g.title = "t".to_string();
    put(&mut wt_st, g);
    let mut w = work("W-01", "feature", "ready", "clear");
    w.title = "t".to_string();
    put(&mut wt_st, w);
    let mut w2 = node(Kind::W, "W-02");
    w2.status = "proposed".to_string();
    w2.title = "t".to_string();
    put(&mut wt_st, w2);
    let mut d = plain(Kind::D, "D-01", "accepted");
    d.title = "t".to_string();
    put(&mut wt_st, d);
    let mut q = plain(Kind::Q, "Q-01", "open");
    q.cynefin = Some("clear".to_string());
    q.title = "t".to_string();
    put(&mut wt_st, q);
    let mut b = plain(Kind::B, "B-01", "testing");
    b.title = "t".to_string();
    put(&mut wt_st, b);
    let mut t = plain(Kind::T, "T-01", "open");
    t.title = "t".to_string();
    put(&mut wt_st, t);
    let mut y = plain(Kind::Y, "Y-01", "active");
    y.title = "t".to_string();
    put(&mut wt_st, y);
    let mut a = plain(Kind::A, "A-01", "present");
    a.title = "t".to_string();
    put(&mut wt_st, a);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## G\n### added (+)\n+ g G-01 unverified  t\n\n## W\n### added (+)\n+ w W-01 ready feature  t\n+ w W-02 proposed  t\n\n## D\n### added (+)\n+ d D-01 accepted  t\n\n## Q\n### added (+)\n+ q Q-01 open clear  t\n\n## B\n### added (+)\n+ b B-01 testing  t\n\n## T\n### added (+)\n+ t T-01 open  t\n\n## Y\n### added (+)\n+ y Y-01 active  t\n\n## A\n### added (+)\n+ a A-01  t\n\n";
    assert_eq!(got, want);
}

#[test]
fn attrs_change_line() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut d1 = plain(Kind::D, "D-01", "proposed");
    attr(&mut d1, "t_created", "x");
    let mut d2 = d1.clone();
    attr(&mut d2, "t_created", "y");
    put(&mut ref_st, d1);
    put(&mut wt_st, d2);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## D\n### changed (~)\n~ D-01\n  attrs: changed\n\n";
    assert_eq!(got, want);
}

#[test]
fn fitness_field_snap() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut wa = work("W-01", "bug", "proposed", "complicated");
    fitness(&mut wa, &[("G-01", 2), ("G-02", -1)]);
    let mut wb = wa.clone();
    fitness(&mut wb, &[("G-01", 3)]);
    put(&mut ref_st, wa);
    put(&mut wt_st, wb);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## W\n### changed (~)\n~ W-01\n  fitness: \"G-01=+2, G-02=-1\" -> \"G-01=+3\"\n\n";
    assert_eq!(got, want);
}

#[test]
fn prose_line_count_snap() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut q1 = plain(Kind::Q, "Q-01", "open");
    q1.cynefin = Some("clear".to_string());
    prose(&mut q1, "hypothesis", &["l1", "l2", "l3"]);
    let mut q2 = q1.clone();
    prose(&mut q2, "hypothesis", &["l1"]);
    put(&mut ref_st, q1);
    put(&mut wt_st, q2);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## Q\n### changed (~)\n~ Q-01\n  hypothesis: \"3 prose lines\" -> \"1 prose lines\"\n\n";
    assert_eq!(got, want);
}

#[test]
fn reflist_snap_sorted_and_empty() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut y1 = plain(Kind::Y, "Y-01", "active");
    reflist(&mut y1, "tags", &["beta", "alpha"]);
    let mut y2 = y1.clone();
    y2.fields.remove("tags");
    put(&mut ref_st, y1);
    put(&mut wt_st, y2);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## Y\n### changed (~)\n~ Y-01\n  tags: \"alpha,beta\" -> \"(empty)\"\n\n";
    assert_eq!(got, want);
}

#[test]
fn prose_order_is_not_a_change() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut d1 = plain(Kind::D, "D-01", "proposed");
    prose(&mut d1, "context", &["one", "two"]);
    let mut d2 = d1.clone();
    prose(&mut d2, "context", &["two", "one"]);
    put(&mut ref_st, d1);
    put(&mut wt_st, d2);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n(no semantic changes)\n\n";
    assert_eq!(got, want);
}

#[test]
fn edge_multiset_ordering() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    edge(&mut ref_st, "Q-01", "asks", "W-01");
    edge(&mut ref_st, "Q-01", "asks", "W-01");
    edge(&mut ref_st, "B-01", "tests", "Q-01");
    edge(&mut wt_st, "Q-01", "asks", "W-01");
    edge(&mut wt_st, "B-01", "targets", "W-02");
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## EDGES\n- e B-01 tests Q-01\n- e Q-01 asks W-01\n+ e B-01 targets W-02\n\n";
    assert_eq!(got, want);
}

#[test]
fn archived_node_counts_as_removed() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut g = plain(Kind::G, "G-01", "unverified");
    g.title = "t".to_string();
    put(&mut ref_st, g.clone());
    g.archived = true;
    put(&mut wt_st, g);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## G\n### removed (-)\n- g G-01 unverified  t\n\n";
    assert_eq!(got, want);
}

#[test]
fn repr_escapes_in_single_snap() {
    let mut ref_st = State::default();
    let mut wt_st = State::default();
    let mut wa = work("W-01", "spike", "proposed", "clear");
    single(&mut wa, "theme", "a\"b\\c");
    let mut wb = wa.clone();
    single(&mut wb, "theme", "c$d\ne");
    put(&mut ref_st, wa);
    put(&mut wt_st, wb);
    let got = print_lock_structural_diff("HEAD", &ref_st, &wt_st);
    let want = "# grove diff (ref -> worktree)\n\nbaseline: `HEAD`\n\n## W\n### changed (~)\n~ W-01\n  theme: \"a\\\"b\\\\c\" -> \"c\\$d\\ne\"\n\n";
    assert_eq!(got, want);
}
