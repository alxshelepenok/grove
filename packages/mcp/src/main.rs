use grove_core::{abspath, handle_message, McpServer};
use sha2::{Digest, Sha256};
use std::io::{BufRead, Write};
use std::path::Path;

fn discover_root(start: &str) -> Option<String> {
    let mut dir = abspath(start);
    loop {
        if Path::new(&dir).join(".grove").join("state.lock").is_file() {
            return Some(dir);
        }
        let parent = Path::new(&dir)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if parent == dir || parent.is_empty() {
            return None;
        }
        dir = parent;
    }
}

fn default_session(root: &str) -> String {
    let digest = Sha256::digest(root.as_bytes());
    let hex8: String = digest[..4].iter().map(|b| format!("{b:02x}")).collect();
    format!("mcp-{}-{hex8}", std::process::id())
}

fn usage() -> String {
    "usage: grove-mcp [--root=<dir>] [--session=<token>]".to_string()
}

fn main() {
    let mut root: Option<String> = None;
    let mut session: Option<String> = None;
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--root=") {
            root = Some(v.to_string());
        } else if let Some(v) = arg.strip_prefix("--session=") {
            session = Some(v.to_string());
        } else {
            eprintln!("grove-mcp: unknown argument {arg}\n{}", usage());
            std::process::exit(1);
        }
    }
    let root = match root {
        Some(r) => abspath(&r),
        None => {
            let cwd = std::env::current_dir()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default();
            match discover_root(&cwd) {
                Some(r) => r,
                None => {
                    eprintln!(
                        "grove-mcp: no .grove/state.lock found at {cwd} or any ancestor; pass --root=<dir>"
                    );
                    std::process::exit(1);
                }
            }
        }
    };
    let session = session
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default_session(&root));
    let mut server = McpServer::new(root, session);
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if let Some(resp) = handle_message(&mut server, &line) {
            if out.write_all(resp.as_bytes()).is_err() {
                break;
            }
            if out.write_all(b"\n").is_err() {
                break;
            }
            let _ = out.flush();
        }
    }
}
