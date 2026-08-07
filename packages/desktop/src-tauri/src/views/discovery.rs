use super::{load_state, status_variant};
use crate::templates::Templates;
use grove_core::{content_health, listnodes, CliCtx, Kind, Node, State};
use serde_json::{json, Value};

pub const C_RULE: &str = "C(term) = non-archived accepted D + answered Q + validated B + active discovery whose tags contain the term; qualifying records matching no term are unattributed";

#[derive(Debug, PartialEq, Eq)]
pub struct GlossaryTerm {
    pub term: String,
    pub definition: String,
    pub source: String,
}

pub fn parse_glossary(text: &str) -> Vec<GlossaryTerm> {
    let mut out = Vec::new();
    for line in text.lines() {
        let s = line.trim();
        if !s.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = s.split('|').collect();
        if cells.len() < 4 {
            continue;
        }
        let term = cells[1].trim();
        if term.is_empty() || term == "Term" || term.chars().all(|c| c == '-') {
            continue;
        }
        out.push(GlossaryTerm {
            term: term.to_string(),
            definition: cells[2].trim().to_string(),
            source: cells[3].trim().to_string(),
        });
    }
    out
}

fn is_content(n: &Node) -> bool {
    match n.kind {
        Kind::D => n.status == "accepted",
        Kind::Q => n.status == "answered",
        Kind::B => n.status == "validated",
        Kind::Y => n.status == "active",
        _ => false,
    }
}

fn discovery_view(y: &Node) -> Value {
    let surface = y.lines("surface");
    json!({
        "id": y.id,
        "title": y.title,
        "status": y.status,
        "status_variant": status_variant(Kind::Y, &y.status),
        "tags": y.lines("tags"),
        "surface": surface,
        "has_surface": !surface.is_empty(),
        "invariant": y.lines("invariant").join(" "),
    })
}

pub fn model(st: &State, glossary: &str) -> Value {
    let glossary_terms = parse_glossary(glossary);
    let mut term_ids: Vec<Vec<String>> = glossary_terms.iter().map(|_| Vec::new()).collect();
    let mut unattributed: Vec<String> = Vec::new();
    for n in st.nodes.values() {
        if n.archived || !is_content(n) {
            continue;
        }
        let tags = n.lines("tags");
        let mut matched = false;
        for (i, t) in glossary_terms.iter().enumerate() {
            if tags.iter().any(|tag| tag == &t.term) {
                term_ids[i].push(n.id.clone());
                matched = true;
            }
        }
        if !matched {
            unattributed.push(n.id.clone());
        }
    }
    let terms: Vec<Value> = glossary_terms
        .iter()
        .zip(term_ids)
        .map(|(t, ids)| {
            json!({
                "term": t.term,
                "definition": t.definition,
                "source": t.source,
                "c": ids.len(),
                "c_ids": ids.join(", "),
            })
        })
        .collect();
    let (c, v) = content_health(st);
    let c_total: i64 = c.values().sum();
    let v_total: i64 = v.values().sum();
    let mut active: Vec<Value> = Vec::new();
    let mut inactive: Vec<Value> = Vec::new();
    for y in listnodes(st, Kind::Y, false) {
        if y.status == "active" {
            active.push(discovery_view(y));
        } else {
            inactive.push(discovery_view(y));
        }
    }
    json!({
        "terms": terms,
        "terms_empty": terms.is_empty(),
        "c_rule": C_RULE,
        "c_total": c_total,
        "v_total": v_total,
        "unattributed": unattributed.len(),
        "unattributed_ids": unattributed.join(", "),
        "discoveries": active,
        "discoveries_empty": active.is_empty(),
        "inactive_discoveries": inactive,
        "has_inactive": !inactive.is_empty(),
    })
}

pub fn render(tpl: &Templates, root: &str) -> Result<String, String> {
    let st = load_state(root)?;
    let ctx = CliCtx::new(root.to_string());
    let glossary = grove_core::load_glossary(&ctx).unwrap_or_default();
    tpl.render("discovery", &model(&st, &glossary))
}
