use crate::algebra::preds_clear;
use crate::dor::dor;
use crate::invariants::{discovery_anchor_issues, WIP_LIMIT_DEFAULT};
use crate::model::{Kind, Node, State};
use crate::status::{listnodes, status_set};

pub enum GuardVerdict {
    Ok,
    Invalid(Vec<String>),
    Reject(Vec<String>),
}

impl GuardVerdict {
    pub fn exit_code(&self) -> i32 {
        match self {
            GuardVerdict::Ok => 0,
            GuardVerdict::Invalid(_) => 1,
            GuardVerdict::Reject(_) => 4,
        }
    }
}

pub fn guard_status_transition(st: &State, n: &Node, new: &str) -> GuardVerdict {
    if !status_set(n.kind).contains(&new) {
        return GuardVerdict::Invalid(vec![format!(
            "invalid status `{}` for {}",
            new,
            n.kind.as_str()
        )]);
    }
    if n.kind == Kind::T {
        return GuardVerdict::Reject(vec![
            "theme status is derived; cannot set manually".to_string(),
        ]);
    }
    if n.kind == Kind::A {
        return GuardVerdict::Reject(vec!["area status is structural; cannot set".to_string()]);
    }
    if n.kind == Kind::W && new == "progress" {
        if !dor(st, n, false) {
            return GuardVerdict::Reject(vec![format!(
                "DoR ≢ ⊤ for {}; see `grove dor {}`",
                n.id, n.id
            )]);
        }
        if !preds_clear(st, &n.id) {
            return GuardVerdict::Reject(vec![
                "I5: predecessors not cleared (goal blockers must be verified, not merely declined/partial/unverified)"
                    .to_string(),
            ]);
        }
        let wip = listnodes(st, Kind::W, false)
            .iter()
            .filter(|w| w.status == "progress")
            .count() as i64;
        if wip >= WIP_LIMIT_DEFAULT {
            return GuardVerdict::Reject(vec![format!(
                "I4: WIP limit ({}) reached",
                WIP_LIMIT_DEFAULT
            )]);
        }
    }
    if n.kind == Kind::W && new == "done" {
        if n.lines("evidence").is_empty() {
            return GuardVerdict::Reject(vec![format!(
                "I3: {} has no evidence; use `grove evidence {} \"…\"`",
                n.id, n.id
            )]);
        }
        let f = n.fitness();
        for g in n.lines("goals") {
            if !f.contains_key(&g) {
                return GuardVerdict::Reject(vec![format!(
                    "I10: missing fitness delta for {}; use `grove fitness {} {} <delta>`",
                    g, n.id, g
                )]);
            }
        }
    }
    if n.kind == Kind::D && n.status == "accepted" && new != "superseded" {
        return GuardVerdict::Reject(vec![format!(
            "decision {} is accepted; create a new D with --supersedes",
            n.id
        )]);
    }
    if n.kind == Kind::Y {
        let cur = n.status.as_str();
        let ok = if new == "superseded" {
            cur != "superseded"
        } else if cur == "proposed" && new == "active" {
            let issues = discovery_anchor_issues(st, n);
            if !issues.is_empty() {
                let mut msgs = vec![format!(
                    "y {} anchors not satisfied (proposed → active refused):",
                    n.id
                )];
                for i in issues {
                    msgs.push(format!("  {}", i));
                }
                return GuardVerdict::Reject(msgs);
            }
            true
        } else if cur == "active" && new == "stale" {
            true
        } else {
            false
        };
        if !ok {
            return GuardVerdict::Reject(vec![format!(
                "illegal y transition {} → {} (allowed: proposed→active, active→stale, non-terminal→superseded; stale→active only via `grove revalidate`)",
                cur, new
            )]);
        }
    }
    GuardVerdict::Ok
}
