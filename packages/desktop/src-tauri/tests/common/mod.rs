use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

static GROVE_HOME_LOCK: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
pub fn grove_home_guard() -> MutexGuard<'static, ()> {
    GROVE_HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[allow(dead_code)]
pub fn isolated_grove_home(tag: &str) -> (MutexGuard<'static, ()>, PathBuf) {
    let guard = grove_home_guard();
    let dir = std::env::temp_dir().join(format!(
        "grove-desktop-home-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("GROVE_HOME", &dir);
    (guard, dir)
}
