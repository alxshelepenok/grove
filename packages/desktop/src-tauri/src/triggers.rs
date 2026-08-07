use grove_core::{listnodes, Kind, Node, State};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct NodeRef {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct DoneWork {
    pub id: String,
    pub title: String,
    pub significant: bool,
    pub dismissed: bool,
}

#[derive(Clone, Debug)]
pub struct StaleClaim {
    pub id: String,
    pub title: String,
    pub session: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct TriggerSet {
    pub chaotic_q: Vec<NodeRef>,
    pub blocked_b: Vec<NodeRef>,
    pub verified_g: Vec<NodeRef>,
    pub done_w: Vec<DoneWork>,
    pub open_q: Vec<NodeRef>,
    pub open_b: Vec<NodeRef>,
    pub proposed_d: Vec<NodeRef>,
    pub ready: Vec<NodeRef>,
    pub critical_path: Vec<String>,
    pub idle: bool,
    pub stale_claims: Vec<StaleClaim>,
}

impl TriggerSet {
    pub fn live_significant(&self) -> Vec<&DoneWork> {
        self.done_w
            .iter()
            .filter(|d| d.significant && !d.dismissed)
            .collect()
    }

    pub fn trigger_count(&self) -> usize {
        self.chaotic_q.len()
            + self.blocked_b.len()
            + self.live_significant().len()
            + self.verified_g.len()
            + usize::from(self.idle)
    }

    pub fn badge_count(&self) -> usize {
        self.trigger_count() + self.stale_claims.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Dismissals {
    pub entries: BTreeMap<String, u64>,
}

impl Dismissals {
    pub fn load(path: &Path) -> Dismissals {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Dismissals::default();
        };
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            return Dismissals::default();
        };
        let mut entries = BTreeMap::new();
        if let Some(obj) = v.get("dismissed").and_then(|d| d.as_object()) {
            for (k, val) in obj {
                if let Some(n) = val.as_u64() {
                    entries.insert(k.clone(), n);
                }
            }
        }
        Dismissals { entries }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let body = serde_json::to_string_pretty(&json!({ "dismissed": self.entries }))
            .unwrap_or_default();
        std::fs::write(path, body).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    pub fn is_dismissed(&self, id: &str, journal_len: u64) -> bool {
        self.entries.get(id).copied() == Some(journal_len)
    }

    pub fn dismiss(&mut self, id: &str, journal_len: u64) {
        self.entries.insert(id.to_string(), journal_len);
    }
}

pub fn journal_len(root: &str) -> u64 {
    let path = Path::new(root).join(".grove").join("journal.log");
    std::fs::read_to_string(path)
        .map(|s| s.lines().count() as u64)
        .unwrap_or(0)
}

fn node_ref(n: &Node) -> NodeRef {
    NodeRef {
        id: n.id.clone(),
        title: n.title.clone(),
    }
}

pub fn significant_done(st: &State, critical_path: &BTreeSet<String>, w: &Node) -> bool {
    grove_core::work_significant_when_done(st, w) || critical_path.contains(&w.id)
}

fn stale_reason(w: &Node, session: &str) -> Option<String> {
    if w.status != "progress" {
        return None;
    }
    if !grove_core::progress_has_session_record(w) {
        return Some("no session on record".to_string());
    }
    let mut reasons = Vec::new();
    if !grove_core::session_token_matches(w, session) {
        reasons.push("different session".to_string());
    }
    if grove_core::session_claim_age_stale(w) {
        reasons.push(format!(
            "claimed >{}h ago",
            grove_core::SESSION_DISPLAY_STALE_AFTER_HOURS
        ));
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join(", "))
    }
}

pub fn detect(st: &State, session: &str, dismissals: &Dismissals, journal_len: u64) -> TriggerSet {
    let cp: BTreeSet<String> = grove_core::critical_path(st).into_iter().collect();
    let mut ts = TriggerSet {
        critical_path: cp.iter().cloned().collect(),
        ..TriggerSet::default()
    };
    for q in listnodes(st, Kind::Q, false) {
        if q.cynefin.as_deref() == Some("chaotic") {
            ts.chaotic_q.push(node_ref(q));
        }
        if q.status == "open" {
            ts.open_q.push(node_ref(q));
        }
    }
    for b in listnodes(st, Kind::B, false) {
        if b.status == "invalidated_blocking" {
            ts.blocked_b.push(node_ref(b));
        }
        if b.status == "proposed" || b.status == "testing" {
            ts.open_b.push(node_ref(b));
        }
    }
    for g in listnodes(st, Kind::G, false) {
        if g.status == "verified" {
            ts.verified_g.push(node_ref(g));
        }
    }
    for d in listnodes(st, Kind::D, false) {
        if d.status == "proposed" {
            ts.proposed_d.push(node_ref(d));
        }
    }
    for w in listnodes(st, Kind::W, false) {
        if w.status == "done" {
            let significant = significant_done(st, &cp, w);
            ts.done_w.push(DoneWork {
                id: w.id.clone(),
                title: w.title.clone(),
                significant,
                dismissed: significant && dismissals.is_dismissed(&w.id, journal_len),
            });
        }
        if let Some(reason) = stale_reason(w, session) {
            ts.stale_claims.push(StaleClaim {
                id: w.id.clone(),
                title: w.title.clone(),
                session: w.attr("session"),
                reason,
            });
        }
    }
    let mut ready = grove_core::ready(st);
    ready.sort_by_cached_key(|w| {
        (
            if cp.contains(&w.id) { 0 } else { 1 },
            -(grove_core::impact(st, &w.id).len() as i64),
            w.id.clone(),
        )
    });
    ts.ready = ready.into_iter().map(node_ref).collect();
    ts.idle = ts.ready.is_empty() && (!ts.open_q.is_empty() || !ts.open_b.is_empty());
    let by_id = |a: &NodeRef, b: &NodeRef| a.id.cmp(&b.id);
    ts.chaotic_q.sort_by(by_id);
    ts.blocked_b.sort_by(by_id);
    ts.verified_g.sort_by(by_id);
    ts.open_q.sort_by(by_id);
    ts.open_b.sort_by(by_id);
    ts.proposed_d.sort_by(by_id);
    ts.done_w.sort_by(|a, b| a.id.cmp(&b.id));
    ts.stale_claims.sort_by(|a, b| a.id.cmp(&b.id));
    ts
}
