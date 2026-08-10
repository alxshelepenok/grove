use grove_core::{run_cli, EXIT_OK};
use grove_desktop_lib::bridge::{run_read, run_write, WRITE_COMMANDS};
use std::path::PathBuf;

mod common;

const SESSION: &str = "desktop-test-token";

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-desktop-wtest-{}-{}-{}",
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

fn init_root(root: &PathBuf) -> String {
    let r = run_cli(&["init".to_string(), format!("--root={}", root.display())]);
    assert_eq!(r.code, EXIT_OK, "init failed: {}", r.err);
    root.to_string_lossy().into_owned()
}

fn ok(root: &str, cmd: &str, args: &[&str]) -> String {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_write(root, SESSION, cmd, &argv).unwrap_or_else(|e| panic!("{cmd} {args:?} failed: {e}"))
}

#[test]
fn write_bridge_rejects_non_whitelist_verbs() {
    let (_guard, _home) = common::isolated_grove_home("whitelist-reject");
    for cmd in [
        "show", "status", "init", "undo", "repair", "renumber", "handoff", "gate", "revalidate",
        "glossary", "promote", "render",
    ] {
        let err = run_write("x", SESSION, cmd, &[]).unwrap_err();
        assert!(
            err.contains("write bridge only allows"),
            "{cmd} should be refused by the whitelist, got: {err}"
        );
    }
}

#[test]
fn write_bridge_accepts_every_whitelisted_verb() {
    let (_guard, _home) = common::isolated_grove_home("whitelist-accept");
    let root = init_root(&tmpdir("whitelist"));
    for cmd in WRITE_COMMANDS {
        let args = match cmd {
            "add" => vec!["q".to_string(), "--title=wl".to_string()],
            "distill" | "archive" => vec!["G-99".to_string()],
            _ => vec!["W-99".to_string()],
        };
        match run_write(&root, SESSION, cmd, &args) {
            Ok(_) => {}
            Err(e) => assert!(
                !e.contains("write bridge only allows"),
                "{cmd} must pass the whitelist, got: {e}"
            ),
        }
    }
}

#[test]
fn write_bridge_rejects_blocked_flags() {
    let (_guard, _home) = common::isolated_grove_home("blocked-flags");
    let root = init_root(&tmpdir("flags"));
    for bad in [
        "--root",
        "--root=elsewhere",
        "--session",
        "--session=other",
        "--project",
        "--project=grove",
    ] {
        let args = vec!["q".to_string(), "--title=x".to_string(), bad.to_string()];
        let err = run_write(&root, SESSION, "add", &args).unwrap_err();
        assert!(
            err.contains("flag not allowed on the write bridge"),
            "{bad} should be refused, got: {err}"
        );
    }
}

#[test]
fn write_bridge_distill_archive_report_core_errors() {
    let (_guard, _home) = common::isolated_grove_home("da-error-report");
    let root = init_root(&tmpdir("da-errors"));
    let err = run_write(&root, SESSION, "distill", &["G-99".to_string()]).unwrap_err();
    assert!(err.contains("exited"), "exit code surfaced: {err}");
    assert!(err.contains("G-99"), "stderr surfaced verbatim: {err}");
    let err = run_write(&root, SESSION, "archive", &["G-99".to_string()]).unwrap_err();
    assert!(err.contains("exited"), "exit code surfaced: {err}");
}

#[test]
fn write_round_trip_per_verb() {
    let (_guard, _home) = common::isolated_grove_home("round-trip-home");
    let root = init_root(&tmpdir("round-trip"));

    let a = ok(&root, "add", &["a", "--title=Area"]);
    assert_eq!(a.trim(), "A-01");
    let g = ok(&root, "add", &["g", "--title=Goal", "--area=A-01", "--fitness-kind=manual"]);
    assert_eq!(g.trim(), "G-01");
    let w = ok(&root, "add", &["w", "--title=Work", "--goals=G-01"]);
    assert_eq!(w.trim(), "W-01");
    let q = ok(&root, "add", &["q", "--title=Question"]);
    assert_eq!(q.trim(), "Q-01");

    let shown = run_read(&root, "show", &["W-01".to_string()]).unwrap();
    assert!(shown.contains("W-01"), "add read back: {shown}");
    assert!(shown.contains("G-01"), "goals flag read back: {shown}");

    ok(&root, "set", &["W-01", "status=ready"]);
    let shown = run_read(&root, "show", &["W-01".to_string()]).unwrap();
    assert!(shown.contains("ready"), "set read back: {shown}");

    ok(&root, "field", &["W-01", "tags", "add", "desktop"]);
    let shown = run_read(&root, "show", &["W-01".to_string()]).unwrap();
    assert!(shown.contains("desktop"), "field tags read back: {shown}");

    ok(&root, "evidence", &["W-01", "round trip evidence line"]);
    let shown = run_read(&root, "show", &["W-01".to_string()]).unwrap();
    assert!(
        shown.contains("round trip evidence line"),
        "evidence read back: {shown}"
    );

    ok(&root, "fitness", &["W-01", "G-01", "+1"]);
    let shown = run_read(&root, "show", &["W-01".to_string()]).unwrap();
    assert!(shown.contains("G-01"), "fitness read back: {shown}");

    let lock_text = || {
        std::fs::read_to_string(PathBuf::from(&root).join(".grove").join("state.lock")).unwrap()
    };

    ok(&root, "link", &["Q-01", "asks", "G-01"]);
    assert!(
        lock_text().contains("e Q-01 asks G-01"),
        "link read back in lock"
    );

    ok(&root, "unlink", &["Q-01", "asks", "G-01"]);
    assert!(
        !lock_text().contains("e Q-01 asks G-01"),
        "unlink read back in lock"
    );

    let journal = std::fs::read_to_string(
        PathBuf::from(&root).join(".grove").join("journal.log"),
    )
    .unwrap();
    assert!(
        journal.contains(SESSION),
        "journal records carry the injected session token"
    );
}

#[test]
fn write_surfaces_guard_refusal_verbatim() {
    let (_guard, _home) = common::isolated_grove_home("guard-refusal");
    let root = init_root(&tmpdir("guard"));
    ok(&root, "add", &["w", "--title=Unguarded"]);
    let err = run_write(
        &root,
        SESSION,
        "set",
        &["W-01".to_string(), "status=progress".to_string()],
    )
    .unwrap_err();
    assert!(err.contains("exited 4"), "guard exit code surfaced: {err}");
    assert!(err.contains("DoR"), "DoR refusal surfaced verbatim: {err}");
    assert!(err.contains("W-01"), "refusal names the node: {err}");

    let err = run_write(
        &root,
        SESSION,
        "evidence",
        &["W-99".to_string(), "nope".to_string()],
    )
    .unwrap_err();
    assert!(err.contains("exited"), "not-found exit code surfaced: {err}");
    assert!(err.contains("not found: W-99"), "stderr verbatim: {err}");
}

#[test]
fn write_distill_archive_round_trip() {
    let (_guard, _home) = common::isolated_grove_home("distill-archive-home");
    let root = init_root(&tmpdir("distill-archive"));
    ok(&root, "add", &["a", "--title=Area"]);
    ok(&root, "add", &["g", "--title=Goal", "--area=A-01", "--fitness-kind=manual"]);
    ok(&root, "set", &["G-01", "status=verified"]);
    ok(&root, "distill", &["G-01", "--null"]);
    ok(&root, "archive", &["G-01"]);
    let lock = std::fs::read_to_string(
        PathBuf::from(&root).join(".grove").join("state.lock"),
    )
    .unwrap();
    let archive_section = lock
        .split(":archive")
        .nth(1)
        .expect("lock has an archive section");
    assert!(
        archive_section.contains("g G-01"),
        "goal archived after distill --null + archive: {archive_section}"
    );
}

#[test]
fn write_bridge_leaves_seeded_user_registry_untouched() {
    let _guard = common::grove_home_guard();
    let fake_profile = tmpdir("fake-profile");
    let seeded_dir = fake_profile.join(".grove");
    std::fs::create_dir_all(&seeded_dir).unwrap();
    let seeded_reg = seeded_dir.join("projects.toml");
    let seeded = "[[projects]]\nname = \"seeded\"\npath = \"C:/seeded/root\"\ncreated = \"2026-01-01T00:00:00Z\"\nlast_opened = \"2026-01-01T00:00:00Z\"\n\n";
    std::fs::write(&seeded_reg, seeded).unwrap();

    let grove_home = tmpdir("override-home");
    std::env::set_var("USERPROFILE", &fake_profile);
    std::env::set_var("HOME", &fake_profile);
    std::env::set_var("GROVE_HOME", &grove_home);

    let root = init_root(&tmpdir("isolated-root"));
    ok(&root, "add", &["q", "--title=registry isolation"]);
    run_read(&root, "status", &[]).unwrap();

    let after = std::fs::read_to_string(&seeded_reg).unwrap();
    assert_eq!(
        after, seeded,
        "seeded user registry stays byte-identical after bridge writes"
    );

    let used = std::fs::read_to_string(grove_home.join("projects.toml")).unwrap();
    assert!(
        used.contains("isolated-root"),
        "GROVE_HOME override registry received the upsert: {used}"
    );
}
