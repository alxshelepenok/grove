use grove_core::{registry_load, registry_path, registry_save, ProjectEntry};
use grove_desktop_lib::bridge::run_write;
use grove_desktop_lib::{desktop_session, projects, resolve_startup_root, ProjectState};
use std::path::{Path, PathBuf};

mod common;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-desktop-ptest-{}-{}-{}",
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

fn slashed(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

fn abs(p: &Path) -> String {
    grove_core::abspath(&slashed(p))
}

fn write_lock(root: &Path) {
    let g = root.join(".grove");
    std::fs::create_dir_all(&g).unwrap();
    std::fs::write(g.join("state.lock"), "@grove 1\n").unwrap();
}

fn entry(name: &str, path: &Path, last_opened: &str) -> ProjectEntry {
    ProjectEntry {
        name: name.to_string(),
        path: abs(path),
        created: last_opened.to_string(),
        last_opened: last_opened.to_string(),
    }
}

fn registry_names() -> Vec<String> {
    registry_load(&registry_path())
        .unwrap()
        .iter()
        .map(|e| e.name.clone())
        .collect()
}

#[test]
fn recents_sorted_capped_at_five_and_skip_missing_locks() {
    let (_guard, _home) = common::isolated_grove_home("recents");
    let base = tmpdir("recents");
    let mut reg = Vec::new();
    for i in 0..7 {
        let dir = base.join(format!("p{i}"));
        if i != 6 {
            write_lock(&dir);
        }
        let stamp = format!("2026-08-01T00:00:0{i}Z");
        reg.push(entry(&format!("p{i}"), &dir, &stamp));
    }
    registry_save(&reg, &registry_path()).unwrap();

    let ps = ProjectState::default();
    let v = projects::list(&ps);
    assert_eq!(v["current"], serde_json::Value::Null);
    let recents = v["recents"].as_array().unwrap();
    assert_eq!(recents.len(), 5, "capped at 5: {recents:?}");
    let names: Vec<&str> = recents
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["p5", "p4", "p3", "p2", "p1"], "newest first, p6 has no lock");
    for r in recents {
        assert!(Path::new(r["path"].as_str().unwrap()).join(".grove").join("state.lock").is_file());
    }
}

#[test]
fn open_switches_root_and_rejects_missing_lock() {
    let (_guard, _home) = common::isolated_grove_home("open");
    let dir = tmpdir("open");
    write_lock(&dir);
    let ps = ProjectState::default();

    let err = ps.current_root().unwrap_err();
    assert!(err.starts_with("no_project:"), "prefix contract: {err}");

    let missing = tmpdir("no-lock");
    let err = projects::open(&ps, &slashed(&missing)).unwrap_err();
    assert_eq!(
        err,
        format!("No lock at {}; create the project instead.", slashed(&missing))
    );
    assert!(ps.current_root().is_err(), "failed open leaves root unset");

    let v = projects::open(&ps, &slashed(&dir)).unwrap();
    assert_eq!(v["path"].as_str().unwrap(), abs(&dir));
    assert_eq!(
        v["name"].as_str().unwrap(),
        dir.file_name().unwrap().to_string_lossy()
    );
    assert_eq!(ps.current_root().unwrap(), abs(&dir));
    assert_eq!(ps.session(), desktop_session(&abs(&dir)));

    let reg = registry_load(&registry_path()).unwrap();
    let e = reg.iter().find(|e| e.path == abs(&dir)).expect("open registers");
    assert_eq!(e.name, dir.file_name().unwrap().to_string_lossy());
}

#[test]
fn create_makes_parseable_lock_and_unique_names_on_collision() {
    let (_guard, _home) = common::isolated_grove_home("create");
    let base = tmpdir("create");
    let a = base.join("x").join("alpha");
    let b = base.join("y").join("alpha");
    let ps = ProjectState::default();

    let va = projects::create(&ps, &slashed(&a), "").unwrap();
    assert_eq!(va["name"].as_str().unwrap(), "alpha");
    let lock_a = std::fs::read_to_string(a.join(".grove").join("state.lock")).unwrap();
    grove_core::parse_strict(&lock_a).unwrap_or_else(|e| panic!("lock a parses: {e}"));

    let vb = projects::create(&ps, &slashed(&b), "").unwrap();
    assert_eq!(vb["name"].as_str().unwrap(), "alpha-2");
    let lock_b = std::fs::read_to_string(b.join(".grove").join("state.lock")).unwrap();
    grove_core::parse_strict(&lock_b).unwrap_or_else(|e| panic!("lock b parses: {e}"));

    assert_eq!(registry_names(), ["alpha", "alpha-2"]);
    assert_eq!(ps.current_root().unwrap(), abs(&b));

    let err = projects::create(&ps, &slashed(&a), "").unwrap_err();
    assert!(err.contains("lock already exists"), "init refusal surfaced: {err}");
}

#[test]
fn create_with_given_name_registers_it_uniquely() {
    let (_guard, _home) = common::isolated_grove_home("named");
    let base = tmpdir("named");
    let ps = ProjectState::default();
    let va = projects::create(&ps, &slashed(&base.join("one")), "My Project").unwrap();
    assert_eq!(va["name"].as_str().unwrap(), "My Project");
    let vb = projects::create(&ps, &slashed(&base.join("two")), "My Project").unwrap();
    assert_eq!(vb["name"].as_str().unwrap(), "My Project-2");
    assert_eq!(registry_names(), ["My Project", "My Project-2"]);
}

#[test]
fn close_clears_current_project() {
    let (_guard, _home) = common::isolated_grove_home("close");
    let dir = tmpdir("close");
    write_lock(&dir);
    let ps = ProjectState::default();
    projects::open(&ps, &slashed(&dir)).unwrap();
    assert!(projects::current(&ps).is_some());

    projects::close(&ps);
    assert!(projects::current(&ps).is_none());
    let err = ps.current_root().unwrap_err();
    assert!(err.starts_with("no_project:"), "prefix contract: {err}");
    assert_eq!(ps.session(), "");
}

#[test]
fn startup_arg_root_wins_over_all() {
    let (_guard, _home) = common::isolated_grove_home("sr-arg");
    let argdir = tmpdir("sr-arg");
    let regdir = tmpdir("sr-arg-reg");
    write_lock(&regdir);
    let reg = vec![entry("reg", &regdir, "2026-08-01T00:00:00Z")];
    let args = vec![format!("--root={}", slashed(&argdir))];
    let got = resolve_startup_root(&args, Some("reg"), &reg, &slashed(&regdir));
    assert_eq!(got, Some(abs(&argdir)));
}

#[test]
fn startup_env_project_resolves_dir_and_registry_name() {
    let (_guard, _home) = common::isolated_grove_home("sr-env");
    let regdir = tmpdir("sr-env-reg");
    write_lock(&regdir);
    let reg = vec![entry("envproj", &regdir, "2026-08-01T00:00:00Z")];
    registry_save(&reg, &registry_path()).unwrap();
    let cwd = tmpdir("sr-env-cwd");

    let got = resolve_startup_root(&[], Some("envproj"), &[], &slashed(&cwd));
    assert_eq!(got, Some(abs(&regdir)), "registry name via on-disk registry");
    let got = resolve_startup_root(&[], Some(&slashed(&regdir)), &[], &slashed(&cwd));
    assert_eq!(got, Some(abs(&regdir)), "plain directory path");
}

#[test]
fn startup_registry_picks_most_recent_with_lock() {
    let (_guard, _home) = common::isolated_grove_home("sr-reg");
    let older = tmpdir("sr-older");
    let newer = tmpdir("sr-newer");
    let nolock = tmpdir("sr-nolock");
    write_lock(&older);
    write_lock(&newer);
    let reg = vec![
        entry("older", &older, "2026-08-01T00:00:01Z"),
        entry("ghost", &nolock, "2026-08-03T00:00:00Z"),
        entry("newer", &newer, "2026-08-02T00:00:00Z"),
    ];
    let cwd = tmpdir("sr-reg-cwd");
    let got = resolve_startup_root(&[], None, &reg, &slashed(&cwd));
    assert_eq!(got, Some(abs(&newer)), "ghost has newest stamp but no lock");
}

#[test]
fn startup_falls_back_to_cwd_walk_up() {
    let (_guard, _home) = common::isolated_grove_home("sr-cwd");
    let root = tmpdir("sr-cwd-root");
    write_lock(&root);
    let nested = root.join("deep").join("deeper");
    std::fs::create_dir_all(&nested).unwrap();
    let got = resolve_startup_root(&[], None, &[], &slashed(&nested));
    assert_eq!(got, Some(abs(&root)));
}

#[test]
fn startup_none_when_nothing_resolves() {
    let (_guard, _home) = common::isolated_grove_home("sr-none");
    let cwd = tmpdir("sr-none-cwd");
    let got = resolve_startup_root(&[], None, &[], &slashed(&cwd));
    assert_eq!(got, None);
}

#[test]
fn startup_bad_env_project_falls_through_to_registry() {
    let (_guard, _home) = common::isolated_grove_home("sr-badenv");
    let regdir = tmpdir("sr-badenv-reg");
    write_lock(&regdir);
    let reg = vec![entry("reg", &regdir, "2026-08-01T00:00:00Z")];
    let cwd = tmpdir("sr-badenv-cwd");
    let got = resolve_startup_root(&[], Some("no-such-project"), &reg, &slashed(&cwd));
    assert_eq!(got, Some(abs(&regdir)));
}

#[test]
fn remove_drops_entry_from_recents() {
    let (_guard, _home) = common::isolated_grove_home("remove");
    let base = tmpdir("remove");
    let a = base.join("a");
    let b = base.join("b");
    write_lock(&a);
    write_lock(&b);
    let reg = vec![
        entry("a", &a, "2026-08-01T00:00:01Z"),
        entry("b", &b, "2026-08-01T00:00:02Z"),
    ];
    registry_save(&reg, &registry_path()).unwrap();

    let ps = ProjectState::default();
    let v = projects::remove(&ps, &slashed(&a));
    let recents = v["recents"].as_array().unwrap();
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0]["name"], "b");
    let reg = registry_load(&registry_path()).unwrap();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg[0].name, "b");
}

#[test]
fn list_normalizes_and_merges_duplicate_paths() {
    let (_guard, _home) = common::isolated_grove_home("dupes");
    let dir = tmpdir("dupes");
    write_lock(&dir);
    let trailing = ProjectEntry {
        name: "one".to_string(),
        path: format!("{}/", slashed(&dir)),
        created: "2026-08-01T00:00:01Z".to_string(),
        last_opened: "2026-08-01T00:00:01Z".to_string(),
    };
    let reg = vec![trailing, entry("two", &dir, "2026-08-01T00:00:02Z")];
    registry_save(&reg, &registry_path()).unwrap();

    let ps = ProjectState::default();
    let v = projects::list(&ps);
    let recents = v["recents"].as_array().unwrap();
    assert_eq!(recents.len(), 1, "duplicate paths merged: {recents:?}");
    assert_eq!(recents[0]["path"].as_str().unwrap(), abs(&dir));

    let reg = registry_load(&registry_path()).unwrap();
    assert_eq!(reg.len(), 1, "registry persisted normalized");
    assert_eq!(reg[0].path, abs(&dir));
    assert_eq!(reg[0].last_opened, "2026-08-01T00:00:02Z", "newest stamp wins");
}

#[test]
fn switching_projects_redirects_bridge_writes() {
    let (_guard, _home) = common::isolated_grove_home("switch");
    let base = tmpdir("switch");
    let a = base.join("alpha");
    let b = base.join("beta");
    let ps = ProjectState::default();
    projects::create(&ps, &slashed(&a), "").unwrap();
    projects::create(&ps, &slashed(&b), "").unwrap();
    assert_eq!(ps.current_root().unwrap(), abs(&b));

    let add = |ps: &ProjectState| {
        run_write(
            &ps.current_root().unwrap(),
            &ps.session(),
            "add",
            &["q".to_string(), "--title=switch".to_string()],
        )
        .unwrap()
    };
    assert_eq!(add(&ps).trim(), "Q-01");
    let lock_a = std::fs::read_to_string(a.join(".grove").join("state.lock")).unwrap();
    assert!(!lock_a.contains("Q-01"), "previous project untouched");

    projects::open(&ps, &slashed(&a)).unwrap();
    assert_eq!(add(&ps).trim(), "Q-01", "fresh id sequence in project a");
    let lock_a = std::fs::read_to_string(a.join(".grove").join("state.lock")).unwrap();
    assert!(lock_a.contains("Q-01"), "write followed the switched root");
}
