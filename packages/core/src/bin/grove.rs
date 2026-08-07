use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = grove_core::run_cli_session_locked(&args);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(r.out.as_bytes());
    let _ = out.flush();
    let stderr = std::io::stderr();
    let mut err = stderr.lock();
    let _ = err.write_all(r.err.as_bytes());
    let _ = err.flush();
    std::process::exit(r.code);
}
