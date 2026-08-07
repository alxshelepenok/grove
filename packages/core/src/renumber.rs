use crate::ids::{parse_id_numeric, reconcile_counters};
use crate::model::{field_form, field_order, FieldValue, Form, Kind, Node, State};
use crate::status::listnodes;
use crate::times::stamp_touch_node;
use std::collections::BTreeSet;

fn is_idc(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

pub fn id_occurs_exactly_as_token(line: &str, id: &str) -> bool {
    let ib = id.as_bytes();
    if ib.is_empty() {
        return false;
    }
    let lb = line.as_bytes();
    let mut start = 0usize;
    while start <= lb.len() {
        let found = lb[start..]
            .windows(ib.len())
            .position(|w| w == ib)
            .map(|p| start + p);
        let Some(a) = found else {
            return false;
        };
        let b = a + ib.len();
        let left_ok = a == 0 || !is_idc(lb[a - 1]);
        let right_ok = b == lb.len() || !is_idc(lb[b]);
        if left_ok && right_ok {
            return true;
        }
        start = b + 1;
    }
    false
}

pub fn renumber_blocked_by_done_evidence(st: &State, old_id: &str) -> bool {
    for w in listnodes(st, Kind::W, false) {
        if w.status != "done" {
            continue;
        }
        for line in w.lines("evidence") {
            if id_occurs_exactly_as_token(&line, old_id) {
                return true;
            }
        }
    }
    false
}

fn rewrite_node_fields_after_renumber(n: &mut Node, old_id: &str, new_id: &str) {
    for key in field_order(n.kind) {
        let Some(form) = field_form(n.kind, key) else {
            continue;
        };
        let Some(field) = n.fields.get_mut(*key) else {
            continue;
        };
        match (form, field) {
            (Form::RefList, FieldValue::RefList(v)) => {
                for item in v.iter_mut() {
                    if item == old_id {
                        *item = new_id.to_string();
                    }
                }
            }
            (Form::Fitness, FieldValue::Fitness(m)) => {
                if let Some(val) = m.remove(old_id) {
                    m.insert(new_id.to_string(), val);
                }
            }
            (Form::Single, FieldValue::Single(s)) => {
                if s == old_id {
                    *s = new_id.to_string();
                }
            }
            _ => {}
        }
    }
}

pub fn apply_renumber(st: &mut State, old_id: &str, new_id: &str) -> Result<(), String> {
    let o = old_id;
    let nw = new_id;
    if o == nw {
        return Ok(());
    }
    if !st.nodes.contains_key(o) {
        return Err(format!("rename: missing record {o}"));
    }
    if st.nodes.contains_key(nw) {
        return Err(format!("rename: target already exists {nw}"));
    }
    let (p0, _) = parse_id_numeric(o)?;
    let (p1, _) = parse_id_numeric(nw)?;
    if p0 != p1 {
        return Err(format!("rename: family mismatch {o} vs {nw}"));
    }
    let mut n = st.nodes.remove(o).expect("checked above");
    n.id = nw.to_string();
    st.nodes.insert(nw.to_string(), n);
    for e in &mut st.edges {
        if e.from == o {
            e.from = nw.to_string();
        }
        if e.to == o {
            e.to = nw.to_string();
        }
    }
    for x in st.nodes.values_mut() {
        if x.attrs.get("goal").map(|g| g == o).unwrap_or(false) {
            x.attrs.insert("goal".to_string(), nw.to_string());
        }
        rewrite_node_fields_after_renumber(x, o, nw);
    }
    reconcile_counters(st);
    let n = st.nodes.get_mut(nw).expect("inserted above");
    stamp_touch_node(n);
    Ok(())
}

pub fn glossary_terms(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let s = line.trim();
        if !s.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = s.split('|').collect();
        if cells.len() < 3 {
            continue;
        }
        let term = cells[1].trim();
        if term.is_empty() || term == "Term" || term.chars().all(|c| c == '-') {
            continue;
        }
        out.insert(term.to_string());
    }
    out
}

pub fn glossary_rename_in_text(text: &str, old: &str, new: &str) -> (String, bool) {
    let mut changed = false;
    let mut lines: Vec<String> = text.split('\n').map(|s| s.to_string()).collect();
    for line in lines.iter_mut() {
        let s = line.trim();
        if !s.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.split('|').collect();
        if cells.len() < 3 {
            continue;
        }
        let cell = cells[1];
        let stripped = cell.trim();
        if stripped.is_empty() || stripped == "Term" || stripped.chars().all(|c| c == '-') {
            continue;
        }
        if stripped != old {
            continue;
        }
        let Some(byte_idx) = cell.find(old) else {
            continue;
        };
        let replaced = format!("{}{}{}", &cell[..byte_idx], new, &cell[byte_idx + old.len()..]);
        let mut owned: Vec<String> = cells.iter().map(|c| c.to_string()).collect();
        owned[1] = replaced;
        *line = owned.join("|");
        changed = true;
    }
    (lines.join("\n"), changed)
}
