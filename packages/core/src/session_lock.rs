use crate::cli::{abspath, CliCtx};
use crate::ops::OpResult;
use crate::times::utc_stamp_second;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const SESSION_POLL_SEC: f64 = 0.05;
pub const SESSION_STALE_SEC: f64 = 60.0;
pub const SESSION_HOLD_GRACE_SEC: f64 = 2.0;
pub const SESSION_LOCK_MAX_SPIN: u32 = 6000;

static SLOT_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static SESSION_LOCKS_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn session_locks_enabled() -> bool {
    SESSION_LOCKS_ENABLED.with(|c| c.get())
}

pub fn with_session_locks_enabled<T, F: FnOnce() -> T>(f: F) -> T {
    struct Reset(Option<bool>);
    impl Drop for Reset {
        fn drop(&mut self) {
            if let Some(prev) = self.0 {
                SESSION_LOCKS_ENABLED.with(|c| c.set(prev));
            }
        }
    }
    let prev = SESSION_LOCKS_ENABLED.with(|c| c.replace(true));
    let _guard = Reset(Some(prev));
    f()
}

#[derive(Debug)]
pub struct SessionExclusiveHold {
    dir: PathBuf,
}

#[derive(Debug)]
pub struct SessionSharedHold {
    dir: PathBuf,
}

impl Drop for SessionExclusiveHold {
    fn drop(&mut self) {
        if self.dir.is_dir() {
            rm_slot(&self.dir);
        }
    }
}

impl Drop for SessionSharedHold {
    fn drop(&mut self) {
        if self.dir.is_dir() {
            rm_slot(&self.dir);
        }
    }
}

#[derive(Debug)]
pub struct SessionLockTimeout {
    pub msg: String,
    pub warnings: Vec<String>,
}

fn time_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn time_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn mtime_sec(p: &Path) -> Option<f64> {
    let m = std::fs::metadata(p).ok()?.modified().ok()?;
    m.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs_f64())
}

pub fn session_locks_root(ctx: &CliCtx) -> PathBuf {
    Path::new(&abspath(&ctx.root)).join(".grove").join("locks")
}

pub fn maybe_migrate_legacy_session_locks(ctx: &CliCtx) {
    let root_abs = abspath(&ctx.root);
    let legacy = Path::new(&root_abs).join(".grove.locks");
    let fresh = session_locks_root(ctx);
    if fresh.exists() {
        return;
    }
    if !legacy.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(Path::new(&root_abs).join(".grove"));
    let _ = std::fs::rename(&legacy, &fresh);
}

fn session_exclusive_slot(ctx: &CliCtx) -> PathBuf {
    session_locks_root(ctx).join("exclusive")
}

fn session_readers_parent(ctx: &CliCtx) -> PathBuf {
    session_locks_root(ctx).join("readers")
}

fn session_holder(dir: &Path) -> PathBuf {
    dir.join("holder")
}

fn holder_age_sec(dir: &Path) -> Option<f64> {
    let p = session_holder(dir);
    if !p.exists() {
        return None;
    }
    mtime_sec(&p).map(|m| time_now() - m)
}

fn holder_fresh(dir: &Path) -> bool {
    match holder_age_sec(dir) {
        Some(age) => age <= SESSION_STALE_SEC,
        None => false,
    }
}

fn exclusive_fresh_present(ctx: &CliCtx) -> bool {
    let slot = session_exclusive_slot(ctx);
    slot.is_dir() && holder_fresh(&slot)
}

fn holder_racing_grace(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    match mtime_sec(dir) {
        Some(m) => time_now() - m < SESSION_HOLD_GRACE_SEC && !session_holder(dir).exists(),
        None => false,
    }
}

fn session_write_holder(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(
        session_holder(dir),
        format!("{}\n{}\n", std::process::id(), utc_stamp_second()),
    );
}

fn session_try_mkdir(path: &Path) -> bool {
    std::fs::create_dir(path).is_ok()
}

fn rm_slot(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn push_info(ctx: &CliCtx, warn: &mut Vec<String>, msg: String) {
    if !ctx.quiet {
        warn.push(msg);
    }
}

fn prune_stale_exclusive(ctx: &CliCtx, warn: &mut Vec<String>) {
    let slot = session_exclusive_slot(ctx);
    if !slot.is_dir() {
        return;
    }
    if holder_fresh(&slot) {
        return;
    }
    if holder_racing_grace(&slot) {
        return;
    }
    if holder_age_sec(&slot).is_none() {
        let young = mtime_sec(&slot)
            .map(|m| time_now() - m < SESSION_HOLD_GRACE_SEC)
            .unwrap_or(false);
        if young {
            return;
        }
    }
    rm_slot(&slot);
    push_info(
        ctx,
        warn,
        format!(
            "warning: broke stale grove exclusive session lock (> {} s or abandoned): {}",
            SESSION_STALE_SEC as i64,
            slot.display()
        ),
    );
}

fn prune_stale_readers(ctx: &CliCtx, warn: &mut Vec<String>) {
    let r = session_readers_parent(ctx);
    if !r.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(&r) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let d = entry.path();
        if !d.is_dir() {
            continue;
        }
        if holder_fresh(&d) {
            continue;
        }
        if holder_racing_grace(&d) {
            continue;
        }
        if holder_age_sec(&d).is_none() {
            let young = mtime_sec(&d)
                .map(|m| time_now() - m < SESSION_HOLD_GRACE_SEC)
                .unwrap_or(false);
            if young {
                continue;
            }
        }
        rm_slot(&d);
        push_info(
            ctx,
            warn,
            format!("warning: cleaned stale grove shared session lock: {}", d.display()),
        );
    }
}

fn count_fresh_readers(ctx: &CliCtx) -> usize {
    let r = session_readers_parent(ctx);
    if !r.is_dir() {
        return 0;
    }
    let mut n = 0;
    if let Ok(entries) = std::fs::read_dir(&r) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let d = entry.path();
            if !d.is_dir() {
                continue;
            }
            if holder_fresh(&d) {
                n += 1;
            }
        }
    }
    n
}

fn sleep_poll() {
    std::thread::sleep(Duration::from_secs_f64(SESSION_POLL_SEC));
}

fn timeout_seconds(max_spin: u32) -> i64 {
    (max_spin as f64 * SESSION_POLL_SEC).round() as i64
}

pub fn acquire_exclusive_with_limit(
    ctx: &CliCtx,
    warn: &mut Vec<String>,
    max_spin: u32,
) -> Result<SessionExclusiveHold, SessionLockTimeout> {
    maybe_migrate_legacy_session_locks(ctx);
    let root = session_locks_root(ctx);
    let exc = session_exclusive_slot(ctx);
    let rp = session_readers_parent(ctx);
    let _ = std::fs::create_dir_all(&root);
    let _ = std::fs::create_dir_all(&rp);

    let mut spins = 0u32;
    loop {
        spins += 1;
        if spins > max_spin {
            return Err(SessionLockTimeout {
                msg: format!(
                    "timeout waiting for grove exclusive session lock (held ~>{}s); try later",
                    timeout_seconds(max_spin)
                ),
                warnings: std::mem::take(warn),
            });
        }

        prune_stale_exclusive(ctx, warn);
        prune_stale_readers(ctx, warn);

        if exclusive_fresh_present(ctx) {
            sleep_poll();
            continue;
        }
        while count_fresh_readers(ctx) > 0 {
            sleep_poll();
        }
        for _ in 0..40 {
            if count_fresh_readers(ctx) != 0 {
                break;
            }
            std::thread::sleep(Duration::from_secs_f64(0.002));
        }
        if count_fresh_readers(ctx) > 0 {
            continue;
        }

        if !session_try_mkdir(&exc) {
            sleep_poll();
            continue;
        }
        session_write_holder(&exc);

        if count_fresh_readers(ctx) > 0 {
            rm_slot(&exc);
            sleep_poll();
            continue;
        }
        if !holder_fresh(&exc) {
            rm_slot(&exc);
            sleep_poll();
            continue;
        }
        return Ok(SessionExclusiveHold { dir: exc });
    }
}

pub fn acquire_exclusive(
    ctx: &CliCtx,
    warn: &mut Vec<String>,
) -> Result<SessionExclusiveHold, SessionLockTimeout> {
    acquire_exclusive_with_limit(ctx, warn, SESSION_LOCK_MAX_SPIN)
}

pub fn acquire_shared_with_limit(
    ctx: &CliCtx,
    warn: &mut Vec<String>,
    max_spin: u32,
) -> Result<SessionSharedHold, SessionLockTimeout> {
    maybe_migrate_legacy_session_locks(ctx);
    let root = session_locks_root(ctx);
    let rp = session_readers_parent(ctx);
    let _ = std::fs::create_dir_all(&root);
    let _ = std::fs::create_dir_all(&rp);

    let mut spins = 0u32;
    loop {
        spins += 1;
        if spins > max_spin {
            return Err(SessionLockTimeout {
                msg: format!(
                    "timeout waiting for grove shared session lock (~>{}s); try later",
                    timeout_seconds(max_spin)
                ),
                warnings: std::mem::take(warn),
            });
        }

        prune_stale_exclusive(ctx, warn);

        if exclusive_fresh_present(ctx) {
            sleep_poll();
            continue;
        }

        let sid = format!(
            "{}-{}-{}",
            std::process::id(),
            time_nanos(),
            SLOT_COUNTER.fetch_add(1, Ordering::Relaxed) % 100_007
        );
        let slot = rp.join(sid);
        if !session_try_mkdir(&slot) {
            continue;
        }
        session_write_holder(&slot);

        if exclusive_fresh_present(ctx) {
            rm_slot(&slot);
            sleep_poll();
            continue;
        }
        if !holder_fresh(&slot) {
            rm_slot(&slot);
            sleep_poll();
            continue;
        }
        return Ok(SessionSharedHold { dir: slot });
    }
}

pub fn acquire_shared(
    ctx: &CliCtx,
    warn: &mut Vec<String>,
) -> Result<SessionSharedHold, SessionLockTimeout> {
    acquire_shared_with_limit(ctx, warn, SESSION_LOCK_MAX_SPIN)
}

pub fn release_exclusive(h: SessionExclusiveHold) {
    drop(h);
}

pub fn release_shared(h: SessionSharedHold) {
    drop(h);
}

pub fn with_session_exclusive<F: FnOnce() -> OpResult>(
    ctx: &CliCtx,
    f: F,
) -> Result<(OpResult, Vec<String>), SessionLockTimeout> {
    let mut warn = Vec::new();
    let h = acquire_exclusive(ctx, &mut warn)?;
    let r = f();
    release_exclusive(h);
    Ok((r, warn))
}

pub fn with_session_shared<F: FnOnce() -> OpResult>(
    ctx: &CliCtx,
    f: F,
) -> Result<(OpResult, Vec<String>), SessionLockTimeout> {
    let mut warn = Vec::new();
    let h = acquire_shared(ctx, &mut warn)?;
    let r = f();
    release_shared(h);
    Ok((r, warn))
}
