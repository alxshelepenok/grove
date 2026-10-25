use crate::cli::{abspath, journal_session_token, json_cli_out, load, persist, CliCtx};
use crate::ids::next_id;
use crate::journal::{jinv_rm_node, stamp_journal_session, wrap_journal_record};
use crate::json::{JVal, JuliaDict};
use crate::model::{Kind, Node};
use crate::ops::{kw_get, OpResult, EXIT_ERR, EXIT_GUARD, EXIT_NOTFOUND};
use crate::renumber::glossary_terms;
use crate::status::listnodes;
use crate::times::{stamp_new_node, utc_stamp_second};
use crate::toml::{parse_line, Line, TomlVal, REGISTRY_LINE_POLICY};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ProjectEntry {
    pub name: String,
    pub path: String,
    pub created: String,
    pub last_opened: String,
}

pub fn user_grove_dir() -> PathBuf {
    if let Some(v) = std::env::var_os("GROVE_HOME") {
        return PathBuf::from(v);
    }
    let home = if cfg!(windows) {
        std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
    } else {
        std::env::var_os("HOME")
    };
    match home {
        Some(h) => PathBuf::from(h).join(".grove"),
        None => PathBuf::from(".grove"),
    }
}

pub fn registry_path() -> PathBuf {
    user_grove_dir().join("projects.toml")
}

#[derive(Clone, Copy, PartialEq)]
enum ProjectsKind {
    Absent,
    Array,
    Table,
    Value,
}

enum Section {
    Top,
    Other,
    Projects(usize),
}

fn entry_from_pairs(pairs: &[(String, TomlVal)]) -> Option<ProjectEntry> {
    let get = |k: &str| {
        pairs
            .iter()
            .find(|(ek, _)| ek == k)
            .and_then(|(_, v)| match v {
                TomlVal::Str(s) => Some(s.clone()),
                _ => None,
            })
    };
    Some(ProjectEntry {
        name: get("name")?,
        path: get("path")?,
        created: get("created")?,
        last_opened: get("last_opened")?,
    })
}

fn parse_registry_toml(text: &str) -> Option<Vec<ProjectEntry>> {
    let mut kind = ProjectsKind::Absent;
    let mut arr: Vec<Vec<(String, TomlVal)>> = Vec::new();
    let mut value: Option<TomlVal> = None;
    let mut seen_tables: Vec<String> = Vec::new();
    let mut seen_arrays: Vec<String> = Vec::new();
    let mut top_keys: Vec<String> = Vec::new();
    let mut other_keys: Vec<String> = Vec::new();
    let mut section = Section::Top;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        match parse_line(line, REGISTRY_LINE_POLICY).ok()? {
            Line::Blank => {}
            Line::ArrayHeader(k) => {
                other_keys.clear();
                if k == "projects" {
                    match kind {
                        ProjectsKind::Absent => kind = ProjectsKind::Array,
                        ProjectsKind::Array => {}
                        _ => return None,
                    }
                    arr.push(Vec::new());
                    section = Section::Projects(arr.len() - 1);
                } else {
                    if seen_tables.contains(&k) {
                        return None;
                    }
                    if !seen_arrays.contains(&k) {
                        seen_arrays.push(k);
                    }
                    section = Section::Other;
                }
            }
            Line::Header(k) => {
                other_keys.clear();
                if k == "projects" {
                    if kind != ProjectsKind::Absent {
                        return None;
                    }
                    kind = ProjectsKind::Table;
                } else {
                    if seen_tables.contains(&k) || seen_arrays.contains(&k) {
                        return None;
                    }
                    seen_tables.push(k);
                }
                section = Section::Other;
            }
            Line::Assign(k, v) => match section {
                Section::Projects(i) => {
                    if arr[i].iter().any(|(ek, _)| *ek == k) {
                        return None;
                    }
                    arr[i].push((k, v));
                }
                Section::Top => {
                    if top_keys.contains(&k) {
                        return None;
                    }
                    top_keys.push(k.clone());
                    if k == "projects" {
                        kind = ProjectsKind::Value;
                        value = Some(v);
                    }
                }
                Section::Other => {
                    if other_keys.contains(&k) {
                        return None;
                    }
                    other_keys.push(k);
                }
            },
        }
    }
    match kind {
        ProjectsKind::Absent => Some(Vec::new()),
        ProjectsKind::Table => None,
        ProjectsKind::Value => match value {
            Some(TomlVal::Arr(items)) => Some(
                items
                    .iter()
                    .filter_map(|it| match it {
                        TomlVal::Table(pairs) => entry_from_pairs(pairs),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        },
        ProjectsKind::Array => Some(arr.iter().filter_map(|e| entry_from_pairs(e)).collect()),
    }
}

pub fn registry_load(path: &Path) -> Option<Vec<ProjectEntry>> {
    if !path.is_file() {
        return Some(Vec::new());
    }
    let text = std::fs::read_to_string(path).ok()?;
    parse_registry_toml(&text)
}

fn toml_basic_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn registry_save(entries: &[ProjectEntry], path: &Path) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        if !d.as_os_str().is_empty() && !d.is_dir() {
            std::fs::create_dir_all(d)?;
        }
    }
    let mut buf = String::new();
    for e in entries {
        buf.push_str("[[projects]]\n");
        buf.push_str("name = ");
        buf.push_str(&toml_basic_string(&e.name));
        buf.push('\n');
        buf.push_str("path = ");
        buf.push_str(&toml_basic_string(&e.path));
        buf.push('\n');
        buf.push_str("created = ");
        buf.push_str(&toml_basic_string(&e.created));
        buf.push('\n');
        buf.push_str("last_opened = ");
        buf.push_str(&toml_basic_string(&e.last_opened));
        buf.push('\n');
        buf.push('\n');
    }
    std::fs::write(path, buf)
}

pub fn registry_unique_name(reg: &[ProjectEntry], base: &str) -> String {
    let taken: Vec<&str> = reg.iter().map(|e| e.name.as_str()).collect();
    if !taken.contains(&base) {
        return base.to_string();
    }
    let mut n = 2;
    while taken.contains(&format!("{base}-{n}").as_str()) {
        n += 1;
    }
    format!("{base}-{n}")
}

pub fn registry_name_for_path(reg: &[ProjectEntry], p: &str) -> Option<String> {
    let ap = abspath(p);
    reg.iter().find(|e| e.path == ap).map(|e| e.name.clone())
}

pub fn registry_path_for_name(reg: &[ProjectEntry], name: &str) -> Option<String> {
    reg.iter()
        .find(|e| e.name == name)
        .map(|e| abspath(&e.path))
}

fn basename_normpath(p: &str) -> String {
    let trimmed = p.trim_end_matches(['/', '\\']);
    let base = if trimmed.is_empty() { p } else { trimmed };
    match base.rsplit(['/', '\\']).next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => base.to_string(),
    }
}

pub fn registry_note_open(root: &str, cmd: &str) -> Option<String> {
    if cmd != "init" && !Path::new(root).join(".grove").join("state.lock").is_file() {
        return None;
    }
    let rp = registry_path();
    let mut reg = match registry_load(&rp) {
        Some(r) => r,
        None => {
            return Some(format!(
                "warning: malformed registry {}; registry features disabled",
                rp.display()
            ))
        }
    };
    let p = abspath(root);
    let now = utc_stamp_second();
    match reg.iter().position(|e| e.path == p) {
        Some(i) => reg[i].last_opened = now,
        None => {
            let name = registry_unique_name(&reg, &basename_normpath(&p));
            reg.push(ProjectEntry {
                name,
                path: p,
                created: now.clone(),
                last_opened: now,
            });
        }
    }
    if registry_save(&reg, &rp).is_err() {
        return Some(format!(
            "warning: could not write registry {}",
            rp.display()
        ));
    }
    None
}

pub fn walk_up_root(start: &str) -> String {
    let start_abs = abspath(start);
    let mut dir = start_abs.clone();
    loop {
        if Path::new(&dir).join(".grove").join("state.lock").is_file() {
            return dir;
        }
        let parent = Path::new(&dir)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if parent == dir || parent.is_empty() {
            return start_abs;
        }
        dir = parent;
    }
}

pub fn resolve_project_target(v: &str) -> Result<String, String> {
    if Path::new(v).is_dir() {
        return Ok(abspath(v));
    }
    let reg = registry_load(&registry_path()).unwrap_or_default();
    if let Some(p) = registry_path_for_name(&reg, v) {
        return Ok(p);
    }
    Err(format!("unknown project: {v}"))
}

pub fn resolve_root(
    default_root: &str,
    kw: &[(String, String)],
    root_given: bool,
) -> Result<Option<String>, String> {
    if root_given {
        return Ok(Some(default_root.to_string()));
    }
    let proj = match kw_get(kw, "project") {
        Some(p) if !p.trim().is_empty() => Some(p.to_string()),
        _ => match std::env::var("GROVE_PROJECT") {
            Ok(e) if !e.trim().is_empty() => Some(e),
            _ => None,
        },
    };
    match proj {
        None => {
            let cwd = std::env::current_dir()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Some(walk_up_root(&cwd)))
        }
        Some(p) => resolve_project_target(&p).map(Some),
    }
}

pub fn cmd_projects(ctx: &CliCtx, _pos: &[String], _kw: &[(String, String)]) -> OpResult {
    let rp = registry_path();
    let mut r = OpResult::ok();
    let reg = match registry_load(&rp) {
        Some(reg) => reg,
        None => {
            r.err.push_str(&format!(
                "warning: malformed registry {}; registry features disabled\n",
                rp.display()
            ));
            Vec::new()
        }
    };
    if ctx.json {
        let entries: Vec<JVal> = reg
            .iter()
            .map(|e| {
                JVal::Obj(JuliaDict::from_pairs(vec![
                    ("name".to_string(), JVal::Str(e.name.clone())),
                    ("path".to_string(), JVal::Str(e.path.clone())),
                    ("created".to_string(), JVal::Str(e.created.clone())),
                    ("last_opened".to_string(), JVal::Str(e.last_opened.clone())),
                ]))
            })
            .collect();
        r.out.push_str(&json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("projects".to_string())),
            ("projects".to_string(), JVal::Arr(entries)),
        ])));
        return r;
    }
    for e in &reg {
        r.out
            .push_str(&format!("{}\t{}\t{}\n", e.name, e.path, e.last_opened));
    }
    r
}

pub fn cmd_promote(ctx: &CliCtx, pos: &[String], kw: &[(String, String)]) -> OpResult {
    if pos.is_empty() {
        return OpResult::fail(EXIT_ERR, "usage: grove promote Y-NN --to=<project>");
    }
    let id = pos[0].clone();
    let to = kw_get(kw, "to");
    if to.map(|t| t.trim().is_empty()).unwrap_or(true) {
        return OpResult::fail(EXIT_ERR, "promote: --to=<project> is required");
    }
    let to = to.unwrap_or("");
    let st = match load(ctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let Some(src) = st.nodes.get(&id) else {
        return OpResult::fail(EXIT_NOTFOUND, &format!("not found: {id}"));
    };
    if src.kind != Kind::Y {
        return OpResult::fail(
            EXIT_ERR,
            &format!("promote: {id} is kind {}, not y", src.kind.as_str()),
        );
    }
    let target_root = match resolve_project_target(to) {
        Ok(p) => p,
        Err(msg) => return OpResult::fail(EXIT_NOTFOUND, &msg),
    };
    if abspath(&target_root) == abspath(&ctx.root) {
        return OpResult::fail(EXIT_ERR, "promote: target is the source project");
    }
    let tctx = CliCtx {
        root: target_root.clone(),
        quiet: ctx.quiet,
        json: ctx.json,
        no_render: ctx.no_render,
    };
    if !tctx.lockpath().is_file() {
        return OpResult::fail(
            EXIT_ERR,
            &format!(
                "promote: target lock not found: {} (run `grove init --root={target_root}`)",
                tctx.lockpath().display()
            ),
        );
    }
    if !crate::session_lock::session_locks_enabled() {
        return promote_into_target(ctx, src, &tctx);
    }
    match crate::session_lock::with_session_exclusive(&tctx, || {
        promote_into_target(ctx, src, &tctx)
    }) {
        Ok((mut r, warnings)) => {
            let mut pre = String::new();
            for w in &warnings {
                pre.push_str(w);
                pre.push('\n');
            }
            r.err = format!("{pre}{}", r.err);
            r
        }
        Err(t) => {
            let mut err = String::new();
            for w in &t.warnings {
                err.push_str(w);
                err.push('\n');
            }
            err.push_str(&t.msg);
            err.push('\n');
            OpResult {
                code: EXIT_GUARD,
                out: String::new(),
                err,
                journal: Vec::new(),
            }
        }
    }
}

fn promote_into_target(sctx: &CliCtx, src: &Node, tctx: &CliCtx) -> OpResult {
    let mut tst = match load(tctx, true) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let reg = registry_load(&registry_path()).unwrap_or_default();
    let origin_project = registry_name_for_path(&reg, &sctx.root)
        .unwrap_or_else(|| basename_normpath(&abspath(&sctx.root)));
    let origin_id = src.id.clone();
    for x in listnodes(&tst, Kind::Y, false) {
        if x.attr("origin_project") == origin_project && x.attr("origin_id") == origin_id {
            return OpResult::fail(
                EXIT_GUARD,
                &format!("promote: already promoted as {}", x.id),
            );
        }
    }
    let nid = next_id(&mut tst, Kind::Y);
    let mut n = Node::new(Kind::Y, nid.clone());
    n.title = src.title.clone();
    n.status = "proposed".to_string();
    for f in [
        "tags",
        "surface",
        "invariant",
        "why",
        "skill_updates",
        "glossary_updates",
    ] {
        if let Some(v) = src.fields.get(f) {
            n.fields.insert(f.to_string(), v.clone());
        }
    }
    n.attrs
        .insert("origin_project".to_string(), origin_project.clone());
    n.attrs.insert("origin_id".to_string(), origin_id.clone());
    n.attrs
        .insert("origin_version".to_string(), src.attr("t_updated"));
    stamp_new_node(&mut n);
    tst.nodes.insert(nid.clone(), n.clone());
    promote_glossary_terms(&tctx.glossarypath(), &n.lines("tags"), &origin_project);
    persist(
        tctx,
        &mut tst,
        Some(&stamp_journal_session(
            &wrap_journal_record("promote", jinv_rm_node(&nid)),
            &journal_session_token(tctx, &[]),
        )),
    );
    let mut r = OpResult::ok();
    if tctx.json {
        r.out.push_str(&json_cli_out(JuliaDict::from_pairs(vec![
            ("command".to_string(), JVal::Str("promote".to_string())),
            ("id".to_string(), JVal::Str(nid)),
            ("origin_project".to_string(), JVal::Str(origin_project)),
            ("origin_id".to_string(), JVal::Str(origin_id)),
        ])));
    }
    r
}

fn promote_glossary_terms(gpath: &Path, tags: &[String], origin_project: &str) {
    if tags.is_empty() {
        return;
    }
    let text = std::fs::read_to_string(gpath).unwrap_or_default();
    let terms = glossary_terms(&text);
    let missing: Vec<&String> = tags
        .iter()
        .filter(|t| !terms.contains(t.as_str()))
        .collect();
    if missing.is_empty() {
        return;
    }
    if !gpath.is_file() {
        if let Some(d) = gpath.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(gpath, "| Term | Meaning |\n| --- | --- |\n");
    }
    let mut lines = String::new();
    for t in missing {
        lines.push_str(&format!("| {t} | copied from {origin_project} |\n"));
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(gpath)
        .and_then(|mut f| f.write_all(lines.as_bytes()));
}
