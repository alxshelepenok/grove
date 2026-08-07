use crate::model::{Edge, Kind, Node, State};

pub const EDGE_LABELS: [&str; 9] = [
    "blocks",
    "causes",
    "implements",
    "asks",
    "tests",
    "supersedes",
    "targets",
    "produces",
    "distills",
];

pub const W_TYPES: [&str; 4] = ["feature", "refactor", "bug", "spike"];

pub const CYNEFIN: [&str; 4] = ["clear", "complicated", "complex", "chaotic"];

pub fn status_set(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::G => &["unverified", "partial", "verified", "declined"],
        Kind::W => &["proposed", "ready", "progress", "done", "rejected", "archived"],
        Kind::D => &["proposed", "accepted", "rejected", "superseded"],
        Kind::Q => &["open", "deferred", "answered", "dropped"],
        Kind::B => &[
            "proposed",
            "testing",
            "validated",
            "invalidated_acceptable",
            "invalidated_blocking",
        ],
        Kind::T => &["open", "done"],
        Kind::Y => &["proposed", "active", "stale", "superseded"],
        Kind::A => &["present"],
    }
}

pub fn is_terminal(kind: Kind, status: &str) -> bool {
    match kind {
        Kind::W => matches!(status, "done" | "rejected" | "archived"),
        Kind::G => matches!(status, "verified" | "declined"),
        Kind::D => matches!(status, "accepted" | "rejected" | "superseded"),
        Kind::Q => matches!(status, "answered" | "deferred" | "dropped"),
        Kind::B => matches!(
            status,
            "validated" | "invalidated_acceptable" | "invalidated_blocking"
        ),
        Kind::T => status == "done",
        Kind::Y => status == "superseded",
        Kind::A => false,
    }
}

pub fn clears_blocks_predecessor(n: &Node) -> bool {
    if n.kind == Kind::G {
        n.status == "verified"
    } else {
        is_terminal(n.kind, &n.status)
    }
}

pub fn idfamily(id: &str) -> char {
    id.chars().next().unwrap_or('\0')
}

pub fn getnode<'a>(st: &'a State, id: &str) -> Option<&'a Node> {
    st.nodes.get(id)
}

pub fn listnodes(st: &State, kind: Kind, include_archived: bool) -> Vec<&Node> {
    st.nodes
        .values()
        .filter(|n| n.kind == kind && (include_archived || !n.archived))
        .collect()
}

pub fn out_edges<'a>(st: &'a State, id: &'a str, label: &'a str) -> impl Iterator<Item = &'a Edge> {
    st.edges.iter().filter(move |e| e.from == id && e.label == label)
}

pub fn in_edges<'a>(st: &'a State, id: &'a str, label: &'a str) -> impl Iterator<Item = &'a Edge> {
    st.edges.iter().filter(move |e| e.to == id && e.label == label)
}

pub fn prose_field_nonempty(lines: &[String]) -> bool {
    lines.iter().any(|s| !s.trim().is_empty())
}

pub fn work_significant_when_done(st: &State, w: &Node) -> bool {
    if w.kind != Kind::W {
        return false;
    }
    if w.status != "done" {
        return false;
    }
    for did in crate::algebra::implements_of(st, w) {
        if let Some(d) = st.nodes.get(&did) {
            if d.kind == Kind::D && d.status == "accepted" {
                return true;
            }
        }
    }
    if w.wtype.as_deref() == Some("refactor") {
        return true;
    }
    if w.wtype.as_deref() == Some("spike") {
        if w.cynefin.as_deref() == Some("complex") {
            return true;
        }
        return st.edges.iter().any(|e| e.label == "produces" && e.from == w.id);
    }
    false
}

pub fn alignment_triggers(st: &State) -> Vec<String> {
    let mut out = Vec::new();
    for q in listnodes(st, Kind::Q, false) {
        if q.cynefin.as_deref() != Some("chaotic") {
            continue;
        }
        out.push(format!(
            "chaotic cynefin: {} ({}) status={}",
            q.id, q.title, q.status
        ));
    }
    for b in listnodes(st, Kind::B, false) {
        if b.status != "invalidated_blocking" {
            continue;
        }
        out.push(format!(
            "blocked assumption: {} ({}) invalidated_blocking",
            b.id, b.title
        ));
    }
    for w in listnodes(st, Kind::W, false) {
        if w.status != "done" {
            continue;
        }
        if !work_significant_when_done(st, w) {
            continue;
        }
        out.push(format!(
            "significant done work: {} ({}) type={} cynefin={}",
            w.id,
            w.title,
            w.wtype.as_deref().unwrap_or("nothing"),
            w.cynefin.as_deref().unwrap_or("nothing")
        ));
    }
    for g in listnodes(st, Kind::G, false) {
        if g.status != "verified" {
            continue;
        }
        out.push(format!("verified goal: {} ({})", g.id, g.title));
    }
    let rs = crate::dor::ready(st);
    if rs.is_empty() {
        let mut has_open_gap = false;
        for q in listnodes(st, Kind::Q, false) {
            if q.status == "open" {
                has_open_gap = true;
            }
        }
        for b in listnodes(st, Kind::B, false) {
            if b.status == "proposed" || b.status == "testing" {
                has_open_gap = true;
            }
        }
        if has_open_gap {
            out.push(
                "idle: no ready work but open question(s) or active assumption benchmarking exist"
                    .to_string(),
            );
        }
    }
    out.sort();
    out
}
