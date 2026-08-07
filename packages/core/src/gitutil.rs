use crate::cli::abspath;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn git_repository_root(root: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(abspath(root))
        .arg("rev-parse")
        .arg("--git-dir")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn git_show_path(root: &str, git_ref: &str, gitpath: &str) -> Result<String, String> {
    let spec = format!("{git_ref}:{gitpath}");
    let out = Command::new("git")
        .arg("-C")
        .arg(abspath(root))
        .arg("--no-pager")
        .arg("show")
        .arg(&spec)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output();
    match out {
        Err(_) => Err("git show failed".to_string()),
        Ok(o) if !o.status.success() => {
            let e = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if e.is_empty() {
                Err("git show failed".to_string())
            } else {
                Err(e)
            }
        }
        Ok(o) => match String::from_utf8(o.stdout) {
            Ok(s) => Ok(s.replace("\r\n", "\n")),
            Err(_) => Err("git show failed".to_string()),
        },
    }
}

pub fn read_worktree_lock_text(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path).map(|s| s.replace("\r\n", "\n"))
}
