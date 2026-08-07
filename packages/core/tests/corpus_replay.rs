mod common;

use grove_core::{
    days_from_civil, derive_default_session_token, format_unix_utc, run_cli,
    set_clock_unix_override,
};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static REPLAY_LOCK: Mutex<()> = Mutex::new(());

fn clock_base() -> i64 {
    days_from_civil(2031, 1, 1) * 86400
}

fn retime_base() -> i64 {
    days_from_civil(2026, 1, 1) * 86400
}

struct TempDirs {
    base: PathBuf,
    home: PathBuf,
}

impl TempDirs {
    fn new() -> TempDirs {
        let uniq = format!(
            "grove-replay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let base = std::env::temp_dir().join(format!("{uniq}-base"));
        let home = std::env::temp_dir().join(format!("{uniq}-home"));
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        TempDirs { base, home }
    }
}

impl Drop for TempDirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set_home(home: &Path) -> EnvGuard {
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
        for (k, v) in &self.saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

struct ClockGuard;

impl Drop for ClockGuard {
    fn drop(&mut self) {
        set_clock_unix_override(None);
    }
}

fn is_ts_window(c: &[u8]) -> bool {
    let d = |x: u8| x.is_ascii_digit();
    c.len() == 20
        && d(c[0])
        && d(c[1])
        && d(c[2])
        && d(c[3])
        && c[4] == b'-'
        && d(c[5])
        && d(c[6])
        && c[7] == b'-'
        && d(c[8])
        && d(c[9])
        && c[10] == b'T'
        && d(c[11])
        && d(c[12])
        && c[13] == b':'
        && d(c[14])
        && d(c[15])
        && c[16] == b':'
        && d(c[17])
        && d(c[18])
        && c[19] == b'Z'
}

fn replace_ts(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i + 20 <= b.len() {
        if is_ts_window(&b[i..i + 20]) {
            out.push_str(&s[last..i]);
            out.push_str("<ts>");
            i += 20;
            last = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[last..]);
    out
}

fn is_lower_hex(c: u8) -> bool {
    c.is_ascii_digit() || (b'a'..=b'f').contains(&c)
}

fn is_hex(c: u8) -> bool {
    is_lower_hex(c) || (b'A'..=b'F').contains(&c)
}

fn replace_sha256_prefixed(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i + 71 <= b.len() {
        if &b[i..i + 7] == b"sha256:" && b[i + 7..i + 71].iter().all(|&c| is_lower_hex(c)) {
            out.push_str(&s[last..i]);
            out.push_str("sha256:<sha>");
            i += 71;
            last = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[last..]);
    out
}

fn replace_bare_sha(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        if is_lower_hex(b[i]) {
            let mut j = i;
            while j < b.len() && is_lower_hex(b[j]) {
                j += 1;
            }
            let run_ok = j - i == 64
                && (i == 0 || !is_hex(b[i - 1]))
                && (j == b.len() || !is_hex(b[j]));
            if run_ok {
                out.push_str(&s[last..i]);
                out.push_str("<sha>");
                last = j;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[last..]);
    out
}

fn strip_trailing_ws(s: &str) -> String {
    s.split('\n')
        .map(|ln| ln.trim_end_matches([' ', '\t']))
        .collect::<Vec<&str>>()
        .join("\n")
}

fn normalize_path_suffixes(s: &str, ph: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(idx) = rest.find(ph) {
        let start = idx + ph.len();
        let end = rest[start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || "_.:/\\-".contains(c)))
            .map(|d| start + d)
            .unwrap_or(rest.len());
        out.push_str(&rest[..start]);
        out.push_str(&rest[start..end].replace("\\\\", "/").replace('\\', "/"));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn normalize(s: &str, paths: &[(String, String)], tokens: &[String]) -> String {
    let mut s = s.replace("\r\n", "\n").replace('\r', "\n");
    for (p, ph) in paths {
        let doubled = p.replace('\\', "\\\\");
        s = s.replace(&doubled, ph);
        s = s.replace(p.as_str(), ph);
        let fwd = p.replace('\\', "/");
        if fwd != *p {
            s = s.replace(&fwd, ph);
        }
    }
    for (_, ph) in paths {
        s = normalize_path_suffixes(&s, ph);
    }
    s = replace_ts(&s);
    s = replace_sha256_prefixed(&s);
    s = replace_bare_sha(&s);
    for t in tokens {
        s = s.replace(t.as_str(), "<session>");
        let prefix: String = t.chars().take(24).collect();
        if prefix != *t {
            s = s.replace(prefix.as_str(), "<session>");
        }
    }
    strip_trailing_ws(&s)
}

fn replace_first_json_ts(line: &str, ts: &str) -> String {
    let pat = "\"ts\":\"";
    let Some(start) = line.find(pat) else {
        return line.to_string();
    };
    let val_start = start + pat.len();
    let Some(end_off) = line[val_start..].find('"') else {
        return line.to_string();
    };
    format!("{}{}{}", &line[..val_start], ts, &line[val_start + end_off..])
}

fn retime_journal(target: &Path) {
    let jp = target.join(".grove").join("journal.log");
    assert!(jp.is_file(), "retime-journal: no journal at {}", jp.display());
    let text = std::fs::read_to_string(&jp).unwrap();
    let mut out = String::new();
    for (i, ln) in text.split('\n').filter(|l| !l.is_empty()).enumerate() {
        let ts = format_unix_utc(retime_base() + (i as i64) * 3600);
        out.push_str(&replace_first_json_ts(ln, &ts));
        out.push('\n');
    }
    std::fs::write(&jp, out).unwrap();
}

fn run_pseudo(step: &[String], target: &Path) -> (i32, String, String) {
    match step[0].as_str() {
        "!write" => {
            let p = target.join(&step[1]);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, step[2..].join(" ")).unwrap();
            (0, String::new(), String::new())
        }
        "!append" => {
            let p = target.join(&step[1]);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            let mut c = std::fs::read_to_string(&p).unwrap_or_default();
            c.push_str(&step[2..].join(" "));
            c.push('\n');
            std::fs::write(&p, c).unwrap();
            (0, String::new(), String::new())
        }
        "!rm" => {
            let _ = std::fs::remove_file(target.join(&step[1]));
            (0, String::new(), String::new())
        }
        "!cat" => {
            let p = target.join(&step[1]);
            assert!(p.is_file(), "!cat: no file at {}", p.display());
            (0, std::fs::read_to_string(&p).unwrap(), String::new())
        }
        "!git" => {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(target)
                .args(&step[1..])
                .output()
                .expect("git spawn failed");
            assert!(
                out.status.success(),
                "git step failed: {}\n{}",
                step.join(" "),
                String::from_utf8_lossy(&out.stderr)
            );
            (0, String::new(), String::new())
        }
        "!retime-journal" => {
            retime_journal(target);
            (0, String::new(), String::new())
        }
        other => panic!("unknown pseudo-step: {other}"),
    }
}

fn read_norm(path: &Path, paths: &[(String, String)], tokens: &[String]) -> String {
    match std::fs::read_to_string(path) {
        Ok(t) => normalize(&t, paths, tokens),
        Err(_) => String::new(),
    }
}

fn print_drift(name: &str, i: usize, args: &[String], field: &str, want: &str, got: &str) {
    eprintln!("DRIFT scenario={name} step={i} field={field} args={}", args.join(" "));
    let wl: Vec<&str> = want.split('\n').collect();
    let al: Vec<&str> = got.split('\n').collect();
    let mut shown = 0;
    for k in 0..wl.len().max(al.len()) {
        let w = wl.get(k);
        let a = al.get(k);
        if w == a {
            continue;
        }
        shown += 1;
        if shown > 20 {
            eprintln!("  ...");
            break;
        }
        if let Some(w) = w {
            eprintln!("  - {w}");
        }
        if let Some(a) = a {
            eprintln!("  + {a}");
        }
    }
}

fn replay(name: &str) {
    let _g = REPLAY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _clock_guard = ClockGuard;
    let sc = common::corpus_json(name);
    let n = common::scenario_len(&sc);
    let two_roots = (0..n).any(|i| {
        let a = common::step_args(&sc, i);
        !a.is_empty() && a[0] == "@2"
    });
    let dirs = TempDirs::new();
    let root = dirs.base.join("main");
    let root2 = dirs.base.join("other");
    std::fs::create_dir_all(&root).unwrap();
    if two_roots {
        std::fs::create_dir_all(&root2).unwrap();
    }
    let _env = EnvGuard::set_home(&dirs.home);
    let root_s = root.to_string_lossy().into_owned();
    let root2_s = root2.to_string_lossy().into_owned();
    let home_s = dirs.home.to_string_lossy().into_owned();
    let mut paths: Vec<(String, String)> = vec![
        (root_s.clone(), "<root>".to_string()),
        (home_s, "<home>".to_string()),
    ];
    if two_roots {
        paths.push((root2_s.clone(), "<root2>".to_string()));
    }
    let mut tokens = vec![derive_default_session_token(&root_s)];
    if two_roots {
        tokens.push(derive_default_session_token(&root2_s));
    }
    let mut clock = clock_base();
    for i in 0..n {
        let raw = common::step_args(&sc, i);
        assert!(!raw.is_empty(), "scenario {name} step {i} is empty");
        let mut step = raw.clone();
        let mut target = root.clone();
        let mut target_s = root_s.clone();
        if step[0] == "@2" {
            assert!(two_roots, "scenario {name} step {i} uses @2 without two_roots");
            step.remove(0);
            target = root2.clone();
            target_s = root2_s.clone();
        }
        let step: Vec<String> = step
            .iter()
            .map(|a| a.replace("{root2}", &root2_s).replace("{root}", &root_s))
            .collect();
        let (code, out, err) = if step[0].starts_with('!') {
            if step[0] == "!sleep" {
                clock += step[1].parse::<f64>().unwrap() as i64;
                (0, String::new(), String::new())
            } else {
                run_pseudo(&step, &target)
            }
        } else {
            set_clock_unix_override(Some(clock));
            clock += 3600;
            let mut args = step.clone();
            args.push(format!("--root={target_s}"));
            let r = run_cli(&args);
            (r.code, r.out, r.err)
        };
        let norm_want = |s: String| -> String {
            let mut s = s;
            for (_, ph) in &paths {
                s = normalize_path_suffixes(&s, ph);
            }
            s
        };
        let want_exit = common::step_exit(&sc, i);
        assert_eq!(
            code as i64, want_exit,
            "scenario {name} step {i} exit: expected {want_exit} got {code} args={}",
            raw.join(" ")
        );
        let want_out = norm_want(common::step_field(&sc, i, "stdout"));
        let got_out = normalize(&out, &paths, &tokens);
        if got_out != want_out {
            print_drift(name, i, &raw, "stdout", &want_out, &got_out);
            panic!("drift in scenario {name} step {i} stdout");
        }
        let want_err = norm_want(common::step_field(&sc, i, "stderr"));
        let got_err = normalize(&err, &paths, &tokens);
        if got_err != want_err {
            print_drift(name, i, &raw, "stderr", &want_err, &got_err);
            panic!("drift in scenario {name} step {i} stderr");
        }
        let want_lock = norm_want(common::step_field(&sc, i, "lock"));
        let got_lock = read_norm(&target.join(".grove").join("state.lock"), &paths, &tokens);
        if got_lock != want_lock {
            print_drift(name, i, &raw, "lock", &want_lock, &got_lock);
            panic!("drift in scenario {name} step {i} lock");
        }
        let want_journal = norm_want(common::step_field(&sc, i, "journal"));
        let got_journal = read_norm(&target.join(".grove").join("journal.log"), &paths, &tokens);
        if got_journal != want_journal {
            print_drift(name, i, &raw, "journal", &want_journal, &got_journal);
            panic!("drift in scenario {name} step {i} journal");
        }
    }
}

macro_rules! scenario {
    ($fnname:ident, $json:literal) => {
        #[test]
        fn $fnname() {
            replay($json);
        }
    };
}

scenario!(init, "init");
scenario!(add_kinds, "add-kinds");
scenario!(field_ops, "field-ops");
scenario!(link_cycle, "link-cycle");
scenario!(w_lifecycle, "w-lifecycle");
scenario!(dor_refusal, "dor-refusal");
scenario!(wip_i4, "wip-i4");
scenario!(i5_blocks, "i5-blocks");
scenario!(sessions, "sessions");
scenario!(staging_overwrite, "staging-overwrite");
scenario!(packet_cone, "packet-cone");
scenario!(render_golden, "render-golden");
scenario!(discovery_lifecycle, "discovery-lifecycle");
scenario!(distill_archive, "distill-archive");
scenario!(glossary_undo, "glossary-undo");
scenario!(undo_sequence, "undo-sequence");
scenario!(stats_scripted, "stats-scripted");
scenario!(promote, "promote");
scenario!(renumber, "renumber");
scenario!(triage, "triage");
scenario!(check_foreign_status, "check-foreign-status");
scenario!(check_y_archive, "check-y-archive");
scenario!(help, "help");
scenario!(gate, "gate");
scenario!(next_log, "next-log");
scenario!(decay_render, "decay-render");
scenario!(diff_repair, "diff-repair");
scenario!(projects, "projects");
