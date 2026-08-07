use crate::ProjectState;
use grove_core::{
    abspath, cmd_init, registry_load, registry_name_for_path, registry_note_open, registry_path,
    registry_save, registry_unique_name, utc_stamp_second, CliCtx, ProjectEntry, EXIT_OK,
};
use serde_json::{json, Value};
use std::path::Path;

const MAX_RECENTS: usize = 5;

fn normalize_reg(reg: &mut Vec<ProjectEntry>) -> bool {
    let mut changed = false;
    let mut merged: Vec<ProjectEntry> = Vec::new();
    for e in reg.drain(..) {
        let p = abspath(&e.path);
        if p != e.path {
            changed = true;
        }
        match merged.iter_mut().find(|x| x.path == p) {
            Some(x) => {
                changed = true;
                if e.last_opened > x.last_opened {
                    x.last_opened = e.last_opened;
                }
                if e.created < x.created {
                    x.created = e.created;
                }
            }
            None => merged.push(ProjectEntry { path: p, ..e }),
        }
    }
    *reg = merged;
    changed
}

fn lock_exists(root: &str) -> bool {
    Path::new(root).join(".grove").join("state.lock").is_file()
}

fn basename(p: &str) -> String {
    let trimmed = p.trim_end_matches(['/', '\\']);
    let base = if trimmed.is_empty() { p } else { trimmed };
    match base.rsplit(['/', '\\']).next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => base.to_string(),
    }
}

fn payload(name: &str, path: &str) -> Value {
    json!({"name": name, "path": path})
}

pub fn display_name(path: &str) -> String {
    let reg = registry_load(&registry_path()).unwrap_or_default();
    registry_name_for_path(&reg, path).unwrap_or_else(|| basename(path))
}

pub fn current(project: &ProjectState) -> Option<Value> {
    let root = project.current_root().ok()?;
    Some(payload(&display_name(&root), &root))
}

pub fn list(project: &ProjectState) -> Value {
    let rp = registry_path();
    let mut reg = registry_load(&rp).unwrap_or_default();
    if normalize_reg(&mut reg) {
        let _ = registry_save(&reg, &rp);
    }
    reg.retain(|e| lock_exists(&e.path));
    reg.sort_by(|a, b| b.last_opened.cmp(&a.last_opened));
    let recents: Vec<Value> = reg
        .iter()
        .take(MAX_RECENTS)
        .map(|e| payload(&e.name, &e.path))
        .collect();
    json!({"current": current(project), "recents": recents})
}

pub fn open(project: &ProjectState, path: &str) -> Result<Value, String> {
    if !lock_exists(path) {
        return Err(format!("No lock at {path}; create the project instead."));
    }
    let root = abspath(path);
    project.open(root.clone());
    let _ = registry_note_open(&root, "open");
    Ok(payload(&display_name(&root), &root))
}

pub fn create(project: &ProjectState, path: &str, name: &str) -> Result<Value, String> {
    let root = abspath(path);
    let r = cmd_init(&CliCtx::new(root.clone()), &[], &[]);
    if r.code != EXIT_OK {
        let msg = if r.err.trim().is_empty() { r.out } else { r.err };
        return Err(msg.trim().to_string());
    }
    if !lock_exists(&root) {
        return Err(format!("init did not produce a lock at {root}"));
    }
    let given = name.trim();
    if !given.is_empty() {
        register_with_name(&root, given);
    }
    project.open(root.clone());
    let _ = registry_note_open(&root, "open");
    Ok(payload(&display_name(&root), &root))
}

fn register_with_name(root: &str, name: &str) {
    let rp = registry_path();
    let Some(mut reg) = registry_load(&rp) else {
        return;
    };
    let p = abspath(root);
    let others: Vec<ProjectEntry> = reg.iter().filter(|e| e.path != p).cloned().collect();
    let unique = registry_unique_name(&others, name);
    let now = utc_stamp_second();
    match reg.iter_mut().find(|e| e.path == p) {
        Some(e) => {
            e.name = unique;
            e.last_opened = now;
        }
        None => reg.push(ProjectEntry {
            name: unique,
            path: p,
            created: now.clone(),
            last_opened: now,
        }),
    }
    let _ = registry_save(&reg, &rp);
}

pub fn close(project: &ProjectState) {
    project.close();
}

pub fn remove(project: &ProjectState, path: &str) -> Value {
    let rp = registry_path();
    if let Some(mut reg) = registry_load(&rp) {
        let p = abspath(path);
        reg.retain(|e| abspath(&e.path) != p);
        let _ = registry_save(&reg, &rp);
    }
    list(project)
}
