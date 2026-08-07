use grove_core::{run_cli, EXIT_OK};

pub const READ_COMMANDS: [&str; 8] = [
    "status", "stats", "list", "show", "packet", "triage", "check", "next",
];

pub const WRITE_COMMANDS: [&str; 11] = [
    "add", "set", "field", "link", "unlink", "evidence", "fitness", "distill", "archive", "resume",
    "revert",
];

const BLOCKED_FLAG_PREFIXES: [&str; 3] = ["--root", "--session", "--project"];

pub fn run_read(root: &str, cmd: &str, args: &[String]) -> Result<String, String> {
    if !READ_COMMANDS.contains(&cmd) {
        return Err(format!("read bridge only allows: {}", READ_COMMANDS.join(", ")));
    }
    for a in args {
        if BLOCKED_FLAG_PREFIXES
            .iter()
            .any(|p| a == p || a.starts_with(&format!("{p}=")))
        {
            return Err(format!("flag not allowed on the read bridge: {a}"));
        }
    }
    let mut argv = Vec::with_capacity(args.len() + 3);
    argv.push(cmd.to_string());
    argv.extend(args.iter().cloned());
    argv.push("--json".to_string());
    argv.push(format!("--root={root}"));
    let r = run_cli(&argv);
    if r.code == EXIT_OK {
        Ok(r.out)
    } else {
        let msg = if r.err.trim().is_empty() {
            r.out
        } else {
            r.err
        };
        Err(format!("grove {cmd} exited {}: {}", r.code, msg.trim()))
    }
}

pub fn run_write(root: &str, session: &str, cmd: &str, args: &[String]) -> Result<String, String> {
    if !WRITE_COMMANDS.contains(&cmd) {
        return Err(format!(
            "write bridge only allows: {}",
            WRITE_COMMANDS.join(", ")
        ));
    }
    for a in args {
        if BLOCKED_FLAG_PREFIXES
            .iter()
            .any(|p| a == p || a.starts_with(&format!("{p}=")))
        {
            return Err(format!("flag not allowed on the write bridge: {a}"));
        }
    }
    let mut argv = Vec::with_capacity(args.len() + 3);
    argv.push(cmd.to_string());
    argv.extend(args.iter().cloned());
    argv.push(format!("--root={root}"));
    argv.push(format!("--session={session}"));
    let r = run_cli(&argv);
    if r.code == EXIT_OK {
        Ok(r.out)
    } else {
        Err(format!(
            "grove {cmd} exited {}\nstdout:\n{}\nstderr:\n{}",
            r.code, r.out, r.err
        ))
    }
}
