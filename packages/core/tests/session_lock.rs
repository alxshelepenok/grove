use grove_core::{
    acquire_exclusive, acquire_shared, release_exclusive, release_shared, run_cli,
    run_cli_session_locked, CliCtx, COMMAND_NAMES, SESSION_MUTATE_COMMANDS, SESSION_READ_COMMANDS,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_root(tag: &str) -> PathBuf {
    static CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let uniq = format!(
        "grove-locktest-{tag}-{}-{n}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let dir = std::env::temp_dir().join(uniq);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ctx_of(root: &Path) -> CliCtx {
    CliCtx::new(root.to_string_lossy().into_owned())
}

fn locks_dir(root: &Path) -> PathBuf {
    root.join(".grove").join("locks")
}

fn make_stale_exclusive(root: &Path, age_sec: u64) {
    let slot = locks_dir(root).join("exclusive");
    std::fs::create_dir_all(&slot).unwrap();
    let holder = slot.join("holder");
    std::fs::write(&holder, "99999\n2020-01-01T00:00:00Z\n").unwrap();
    let f = std::fs::OpenOptions::new().write(true).open(&holder).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(age_sec))
        .unwrap();
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(home: &Path) -> EnvGuard {
        let saved = vec![
            ("GROVE_HOME", std::env::var("GROVE_HOME").ok()),
            ("GROVE_PROJECT", std::env::var("GROVE_PROJECT").ok()),
            ("GROVE_SESSION", std::env::var("GROVE_SESSION").ok()),
        ];
        std::env::set_var("GROVE_HOME", home);
        std::env::remove_var("GROVE_PROJECT");
        std::env::remove_var("GROVE_SESSION");
        EnvGuard { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in self.saved.drain(..) {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

fn args(cmd: &str, root: &Path) -> Vec<String> {
    vec![
        cmd.to_string(),
        format!("--root={}", root.to_string_lossy()),
    ]
}

#[test]
fn classification_covers_every_command_exactly_once() {
    for c in COMMAND_NAMES {
        let read = SESSION_READ_COMMANDS.contains(&c);
        let mutate = SESSION_MUTATE_COMMANDS.contains(&c);
        assert!(read ^ mutate, "command {c} must be in exactly one class");
    }
}

#[test]
fn run_cli_stays_lock_free() {
    let _g = ENV_LOCK.lock().unwrap();
    let root = temp_root("nolock");
    let home = temp_root("nolock-home");
    let _env = EnvGuard::set(&home);
    let r = run_cli(&args("init", &root));
    assert_eq!(r.code, 0, "init failed: {}", r.err);
    let r = run_cli(&args("status", &root));
    assert_eq!(r.code, 0, "status failed: {}", r.err);
    assert!(
        !locks_dir(&root).exists(),
        "in-process run_cli must not create lock dirs"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn locked_cli_acquires_and_releases() {
    let _g = ENV_LOCK.lock().unwrap();
    let root = temp_root("locked");
    let home = temp_root("locked-home");
    let _env = EnvGuard::set(&home);
    let r = run_cli_session_locked(&args("init", &root));
    assert_eq!(r.code, 0, "init failed: {}", r.err);
    assert!(locks_dir(&root).is_dir(), "locks root should exist");
    assert!(
        locks_dir(&root).join("readers").is_dir(),
        "readers parent should exist"
    );
    assert!(
        !locks_dir(&root).join("exclusive").exists(),
        "exclusive slot must be released after init"
    );
    let r = run_cli_session_locked(&args("status", &root));
    assert_eq!(r.code, 0, "status failed: {}", r.err);
    let leftover: Vec<_> = std::fs::read_dir(locks_dir(&root).join("readers"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    assert!(leftover.is_empty(), "reader slots must be released");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn exclusive_blocks_shared_until_release() {
    let root = temp_root("excl");
    let ctx = ctx_of(&root);
    let mut warn = Vec::new();
    let h = acquire_exclusive(&ctx, &mut warn).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let root2 = root.clone();
    let th = std::thread::spawn(move || {
        let ctx2 = ctx_of(&root2);
        let mut w2 = Vec::new();
        let hold = acquire_shared(&ctx2, &mut w2).unwrap();
        tx.send(()).unwrap();
        hold
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        rx.try_recv().is_err(),
        "shared must wait while exclusive held"
    );
    release_exclusive(h);
    let _shared = th.join().unwrap();
    rx.recv().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn shared_blocks_exclusive_until_release() {
    let root = temp_root("shrd");
    let ctx = ctx_of(&root);
    let mut warn = Vec::new();
    let h = acquire_shared(&ctx, &mut warn).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let root2 = root.clone();
    let th = std::thread::spawn(move || {
        let ctx2 = ctx_of(&root2);
        let mut w2 = Vec::new();
        let hold = acquire_exclusive(&ctx2, &mut w2).unwrap();
        tx.send(()).unwrap();
        hold
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        rx.try_recv().is_err(),
        "exclusive must wait while shared held"
    );
    release_shared(h);
    let _excl = th.join().unwrap();
    rx.recv().unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn stale_exclusive_is_broken_with_warning() {
    let root = temp_root("stale");
    make_stale_exclusive(&root, 120);
    let ctx = ctx_of(&root);
    let mut warn = Vec::new();
    let h = acquire_shared(&ctx, &mut warn).unwrap();
    assert!(
        warn.iter()
            .any(|m| m.contains("broke stale grove exclusive session lock")),
        "expected stale-break warning, got {warn:?}"
    );
    release_shared(h);

    make_stale_exclusive(&root, 120);
    let mut quiet_ctx = ctx_of(&root);
    quiet_ctx.quiet = true;
    let mut warn2 = Vec::new();
    let h2 = acquire_shared(&quiet_ctx, &mut warn2).unwrap();
    assert!(warn2.is_empty(), "quiet must suppress warnings: {warn2:?}");
    release_shared(h2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn timeout_messages_match_julia() {
    let root = temp_root("tmo");
    let ctx = ctx_of(&root);
    let mut warn = Vec::new();
    let h = acquire_exclusive(&ctx, &mut warn).unwrap();
    let mut w2 = Vec::new();
    let err = grove_core::acquire_shared_with_limit(&ctx, &mut w2, 3).unwrap_err();
    assert_eq!(
        err.msg,
        "timeout waiting for grove shared session lock (~>0s); try later"
    );
    let mut w3 = Vec::new();
    let err2 = grove_core::acquire_exclusive_with_limit(&ctx, &mut w3, 3).unwrap_err();
    assert_eq!(
        err2.msg,
        "timeout waiting for grove exclusive session lock (held ~>0s); try later"
    );
    release_exclusive(h);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn legacy_locks_dir_is_migrated() {
    let root = temp_root("legacy");
    let legacy = root.join(".grove.locks");
    std::fs::create_dir_all(&legacy).unwrap();
    std::fs::write(legacy.join("marker"), "x").unwrap();
    let ctx = ctx_of(&root);
    let mut warn = Vec::new();
    let h = acquire_exclusive(&ctx, &mut warn).unwrap();
    assert!(
        locks_dir(&root).join("marker").is_file(),
        "legacy .grove.locks must move under .grove/locks"
    );
    assert!(!legacy.exists());
    release_exclusive(h);
    let _ = std::fs::remove_dir_all(&root);
}
