mod common;
#[allow(unused_imports)]
use common::*;
use grove_core::*;
use std::io::Write;
use std::path::{Path, PathBuf};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const TS: &str = "2031-01-01T00:00:00Z";

fn pin() {
    set_clock_unix_override(Some(parse_rfc3339_utc_second(TS).unwrap()));
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-ptest-{}-{}-{}",
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

struct EnvGuard {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvGuard {
    fn new(home: &Path) -> EnvGuard {
        let keys = ["GROVE_HOME", "GROVE_PROJECT", "GROVE_SESSION"];
        let saved = keys.iter().map(|k| (*k, std::env::var_os(k))).collect();
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
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

fn is_ts_window(w: &[u8]) -> bool {
    w.len() == 20
        && w[4] == b'-'
        && w[7] == b'-'
        && w[10] == b'T'
        && w[13] == b':'
        && w[16] == b':'
        && w[19] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|&k| w[k].is_ascii_digit())
}

fn mask_ts(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() && i + 20 <= b.len() && is_ts_window(&b[i..i + 20]) {
            out.push_str("<ts>");
            i += 20;
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn hex_border(c: u8) -> bool {
    c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c)
}

fn mask_sha(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"sha256:")
            && i + 71 <= b.len()
            && b[i + 7..i + 71]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
        {
            out.push_str("sha256:<sha>");
            i += 71;
            continue;
        }
        if i + 64 <= b.len()
            && b[i..i + 64]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
            && (i == 0 || !hex_border(b[i - 1]))
            && (i + 64 == b.len() || !hex_border(b[i + 64]))
        {
            out.push_str("<sha>");
            i += 64;
            continue;
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_trailing_ws(s: &str) -> String {
    s.split('\n')
        .map(|l| l.trim_end_matches([' ', '\t']))
        .collect::<Vec<_>>()
        .join("\n")
}

fn norm(s: &str, paths: &[(String, &str)]) -> String {
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
    strip_trailing_ws(&mask_sha(&mask_ts(&s)))
}

struct Roots {
    base: PathBuf,
    root: PathBuf,
    root2: PathBuf,
    home: PathBuf,
}

impl Roots {
    fn path_pairs(&self) -> Vec<(String, &'static str)> {
        vec![
            (self.root.to_string_lossy().into_owned(), "<root>"),
            (self.home.to_string_lossy().into_owned(), "<home>"),
            (self.root2.to_string_lossy().into_owned(), "<root2>"),
        ]
    }
}

fn setup_roots(tag: &str) -> Roots {
    let base = tmpdir(tag);
    let root = base.join("main");
    let root2 = base.join("other");
    let home = base.join("home");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&root2).unwrap();
    std::fs::create_dir_all(&home).unwrap();
    Roots {
        base,
        root,
        root2,
        home,
    }
}

fn run_pseudo(step: &[String], target: &Path) -> (i32, String, String) {
    match step[0].as_str() {
        "!write" => {
            let p = target.join(&step[1]);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d).unwrap();
            }
            std::fs::write(&p, step[2..].join(" ")).unwrap();
            (0, String::new(), String::new())
        }
        "!append" => {
            let p = target.join(&step[1]);
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d).unwrap();
            }
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&p)
                .unwrap();
            writeln!(f, "{}", step[2..].join(" ")).unwrap();
            (0, String::new(), String::new())
        }
        "!cat" => {
            let p = target.join(&step[1]);
            (0, std::fs::read_to_string(&p).unwrap(), String::new())
        }
        op => panic!("unknown pseudo-step: {op}"),
    }
}

fn run_step(roots: &Roots, raw: &[String]) -> (i32, String, String) {
    let mut step: Vec<String> = raw.to_vec();
    let mut target = roots.root.clone();
    if step.first().is_some_and(|s| s == "@2") {
        target = roots.root2.clone();
        step.remove(0);
    }
    let root_s = roots.root.to_string_lossy().into_owned();
    let root2_s = roots.root2.to_string_lossy().into_owned();
    let step: Vec<String> = step
        .iter()
        .map(|s| s.replace("{root2}", &root2_s).replace("{root}", &root_s))
        .collect();
    if step[0].starts_with('!') {
        return run_pseudo(&step, &target);
    }
    let mut args = step;
    args.push(format!("--root={}", target.to_string_lossy()));
    let r = run_cli(&args);
    (r.code, r.out, r.err)
}

fn step_cmd(raw: &[String]) -> &str {
    let i = if raw.first().is_some_and(|s| s == "@2") {
        1
    } else {
        0
    };
    raw.get(i).map(String::as_str).unwrap_or("")
}

fn replay(name: &str, skip: &dyn Fn(usize, &[String]) -> bool) {
    let _lock = env_lock();
    pin();
    let roots = setup_roots(name);
    let _env = EnvGuard::new(&roots.home);
    let sc = corpus_json(name);
    let paths = roots.path_pairs();
    for i in 0..scenario_len(&sc) {
        let raw = step_args(&sc, i);
        if skip(i, &raw) {
            continue;
        }
        let (code, out, err) = run_step(&roots, &raw);
        assert_eq!(
            norm(&out, &paths),
            step_field(&sc, i, "stdout"),
            "step {i} stdout args={raw:?}"
        );
        assert_eq!(
            norm(&err, &paths),
            step_field(&sc, i, "stderr"),
            "step {i} stderr args={raw:?}"
        );
        assert_eq!(code as i64, step_exit(&sc, i), "step {i} exit args={raw:?}");
        let target = if raw.first().is_some_and(|s| s == "@2") {
            &roots.root2
        } else {
            &roots.root
        };
        let lockp = target.join(".grove").join("state.lock");
        let got_lock = lockp
            .is_file()
            .then(|| norm(&std::fs::read_to_string(&lockp).unwrap(), &paths));
        assert_eq!(
            got_lock.as_deref(),
            sc["steps"][i]["lock"].as_str(),
            "step {i} lock args={raw:?}"
        );
        let jp = target.join(".grove").join("journal.log");
        let tok = derive_default_session_token(&target.to_string_lossy());
        let got_journal = jp.is_file().then(|| {
            norm(&std::fs::read_to_string(&jp).unwrap(), &paths).replace(&tok, "<session>")
        });
        assert_eq!(
            got_journal.as_deref(),
            sc["steps"][i]["journal"].as_str(),
            "step {i} journal args={raw:?}"
        );
    }
    std::fs::remove_dir_all(&roots.base).ok();
}

#[test]
fn wave2b_projects_fixture() {
    replay("projects", &|_i, raw| step_cmd(raw) == "show");
}

#[test]
fn corpus_promote_fixture() {
    replay("promote", &|_i, raw| {
        step_cmd(raw) == "show" || step_cmd(raw) == "check"
    });
}

fn entry(name: &str, path: &str) -> ProjectEntry {
    ProjectEntry {
        name: name.to_string(),
        path: path.to_string(),
        created: TS.to_string(),
        last_opened: TS.to_string(),
    }
}

#[test]
fn toml_round_trip() {
    let dir = tmpdir("toml-round");
    let p = dir.join("projects.toml");
    let entries = vec![
        entry("main", "C:\\work\\main"),
        entry("quo\"te", "/tmp/back\\slash"),
    ];
    registry_save(&entries, &p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    assert_eq!(
        text,
        "[[projects]]\nname = \"main\"\npath = \"C:\\\\work\\\\main\"\ncreated = \"2031-01-01T00:00:00Z\"\nlast_opened = \"2031-01-01T00:00:00Z\"\n\n[[projects]]\nname = \"quo\\\"te\"\npath = \"/tmp/back\\\\slash\"\ncreated = \"2031-01-01T00:00:00Z\"\nlast_opened = \"2031-01-01T00:00:00Z\"\n\n"
    );
    let got = registry_load(&p).unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].name, "main");
    assert_eq!(got[0].path, "C:\\work\\main");
    assert_eq!(got[0].created, TS);
    assert_eq!(got[0].last_opened, TS);
    assert_eq!(got[1].name, "quo\"te");
    assert_eq!(got[1].path, "/tmp/back\\slash");
    registry_save(&[], &p).unwrap();
    assert_eq!(std::fs::read_to_string(&p).unwrap(), "");
    assert_eq!(registry_load(&p).unwrap().len(), 0);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn toml_parse_rules() {
    let dir = tmpdir("toml-rules");
    let p = dir.join("projects.toml");
    let ok_text = "# comment\r\n\r\n[[projects]] # trailing\r\nname = \"main\" # c\r\npath = \"C:\\\\x\\\\main\"\r\ncreated = \"2031-01-01T00:00:00Z\"\r\nlast_opened = \"2031-01-01T00:00:00Z\"\r\nextra_key = \"ignored\"\r\n\r\n[other]\r\nfoo = 5\r\n\r\n[[projects]]\r\nname = \"second\"\r\npath = \"/tmp/second\"\r\ncreated = \"2031-01-01T00:00:00Z\"\r\nlast_opened = \"2031-01-01T00:00:00Z\"\r\n\r\n[[projects]]\r\nname = \"incomplete\"\r\n";
    std::fs::write(&p, ok_text).unwrap();
    let got = registry_load(&p).unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].name, "main");
    assert_eq!(got[1].name, "second");
    std::fs::write(&p, "projects = 5\n").unwrap();
    assert!(registry_load(&p).is_none());
    std::fs::write(&p, "[projects]\nname = \"x\"\n").unwrap();
    assert!(registry_load(&p).is_none());
    std::fs::write(&p, "[[projects]]\nname = \"unterminated\n").unwrap();
    assert!(registry_load(&p).is_none());
    std::fs::write(&p, "[[projects]]\nname = \"a\"\nname = \"b\"\n").unwrap();
    assert!(registry_load(&p).is_none());
    std::fs::write(&p, "not toml at all\n").unwrap();
    assert!(registry_load(&p).is_none());
    std::fs::write(&p, "projects = [1, 2]\n").unwrap();
    assert_eq!(registry_load(&p).unwrap().len(), 0);
    std::fs::write(
        &p,
        "projects = [{name = \"a\", path = \"b\", created = \"c\", last_opened = \"d\"}, 5]\n",
    )
    .unwrap();
    let got = registry_load(&p).unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, "a");
    assert_eq!(got[0].last_opened, "d");
    std::fs::write(&p, "[[projects]]\nname = \"a\\u0041\"\npath = \"p\"\ncreated = \"c\"\nlast_opened = \"l\"\n").unwrap();
    let got = registry_load(&p).unwrap();
    assert_eq!(got[0].name, "aA");
    assert!(registry_load(&dir.join("missing.toml")).unwrap().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unique_name() {
    let reg = vec![entry("main", "/a"), entry("main-2", "/b")];
    assert_eq!(registry_unique_name(&reg, "main"), "main-3");
    assert_eq!(registry_unique_name(&reg, "other"), "other");
    assert_eq!(registry_unique_name(&[], "main"), "main");
}

#[test]
fn name_path_lookup() {
    let dir = tmpdir("lookup");
    let p = dir.join("main");
    std::fs::create_dir_all(&p).unwrap();
    let ps = p.to_string_lossy().into_owned();
    let reg = vec![entry("main", &abspath(&ps))];
    assert_eq!(registry_name_for_path(&reg, &ps), Some("main".to_string()));
    assert_eq!(registry_name_for_path(&reg, &dir.to_string_lossy()), None);
    assert_eq!(
        registry_path_for_name(&reg, "main"),
        Some(abspath(&ps))
    );
    assert_eq!(registry_path_for_name(&reg, "nope"), None);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resolve_project_target_cases() {
    let _lock = env_lock();
    let dir = tmpdir("resolve-target");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let _env = EnvGuard::new(&home);
    let err = resolve_project_target("definitely-not-a-grove-project-xyz").unwrap_err();
    assert_eq!(err, "unknown project: definitely-not-a-grove-project-xyz");
    let proj = dir.join("main");
    std::fs::create_dir_all(&proj).unwrap();
    let ps = proj.to_string_lossy().into_owned();
    assert_eq!(resolve_project_target(&ps).unwrap(), abspath(&ps));
    let reg = vec![entry("byname", &abspath(&ps))];
    registry_save(&reg, &registry_path()).unwrap();
    assert_eq!(resolve_project_target("byname").unwrap(), abspath(&ps));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn note_open_lifecycle() {
    let _lock = env_lock();
    pin();
    let dir = tmpdir("note-open");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let _env = EnvGuard::new(&home);
    let base1 = dir.join("one");
    let root1 = base1.join("main");
    let base2 = dir.join("two");
    let root2 = base2.join("main");
    std::fs::create_dir_all(&root1).unwrap();
    std::fs::create_dir_all(&root2).unwrap();
    let r1 = root1.to_string_lossy().into_owned();
    let r2 = root2.to_string_lossy().into_owned();
    assert_eq!(registry_note_open(&r1, "projects"), None);
    assert!(!registry_path().is_file());
    assert_eq!(registry_note_open(&r1, "init"), None);
    let reg = registry_load(&registry_path()).unwrap();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg[0].name, "main");
    assert_eq!(reg[0].path, abspath(&r1));
    assert_eq!(reg[0].created, TS);
    assert_eq!(reg[0].last_opened, TS);
    assert_eq!(registry_note_open(&r2, "init"), None);
    let reg = registry_load(&registry_path()).unwrap();
    assert_eq!(reg.len(), 2);
    assert_eq!(reg[1].name, "main-2");
    std::fs::create_dir_all(root1.join(".grove")).unwrap();
    std::fs::write(root1.join(".grove").join("state.lock"), "").unwrap();
    assert_eq!(registry_note_open(&r1, "projects"), None);
    let reg = registry_load(&registry_path()).unwrap();
    assert_eq!(reg.len(), 2);
    assert_eq!(reg[0].name, "main");
    assert_eq!(reg[0].last_opened, TS);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn malformed_registry_warning() {
    let _lock = env_lock();
    pin();
    let dir = tmpdir("malformed");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let _env = EnvGuard::new(&home);
    std::fs::write(registry_path(), "[[projects]]\nname = \"unterminated\n").unwrap();
    assert!(registry_load(&registry_path()).is_none());
    let want = format!(
        "warning: malformed registry {}; registry features disabled",
        registry_path().display()
    );
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());
    let r = cmd_projects(&ctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out, "");
    assert_eq!(r.err, format!("{want}\n"));
    let jctx = CliCtx {
        root: ctx.root.clone(),
        quiet: false,
        json: true,
        no_render: false,
    };
    let r = cmd_projects(&jctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert_eq!(r.out, "{\"command\":\"projects\",\"projects\":[]}\n");
    assert_eq!(r.err, format!("{want}\n"));
    let root = dir.join("main");
    std::fs::create_dir_all(root.join(".grove")).unwrap();
    std::fs::write(root.join(".grove").join("state.lock"), "").unwrap();
    assert_eq!(
        registry_note_open(&root.to_string_lossy(), "projects"),
        Some(want)
    );
    std::fs::remove_dir_all(&dir).ok();
}
