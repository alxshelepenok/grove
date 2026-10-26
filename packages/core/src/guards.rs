use crate::algebra::{bchain, preds_clear};
use crate::dor::dor;
use crate::invariants::{discovery_anchor_issues, WIP_LIMIT_DEFAULT};
use crate::message::{Code, Diagnostic};
use crate::model::{Kind, Node, State};
use crate::status::{is_terminal, listnodes, status_set};

pub enum GuardVerdict {
    Ok,
    Invalid(Vec<Diagnostic>),
    Reject(Vec<Diagnostic>),
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

pub fn accepted_decision_msg(id: &str) -> Diagnostic {
    Diagnostic::error(
        Code::Invariant,
        &format!("decision {id} is accepted, create a successor D with --supersedes"),
    )
}

pub fn terminal_freeze_msg(n: &Node) -> Option<Diagnostic> {
    if !is_terminal(n.kind, &n.status) {
        return None;
    }
    if n.kind == Kind::D && n.status == "accepted" {
        return Some(accepted_decision_msg(&n.id));
    }
    Some(Diagnostic::error(
        Code::Invariant,
        &format!(
            "{} is terminal ({}), create a successor node or undo the last mutation",
            n.id, n.status
        ),
    ))
}

pub fn guard_status_transition(st: &State, n: &Node, new: &str) -> GuardVerdict {
    if !status_set(n.kind).contains(&new) {
        return GuardVerdict::Invalid(vec![Diagnostic::error(
            Code::Invariant,
            &format!("invalid status `{}` for {}", new, n.kind.as_str()),
        )]);
    }
    if !(n.kind == Kind::D && n.status == "accepted" && new == "superseded") {
        if let Some(msg) = terminal_freeze_msg(n) {
            return GuardVerdict::Reject(vec![msg]);
        }
    }
    if n.kind == Kind::G
        && new == "verified"
        && crate::fitness::goal_structured_kind(n) != Some("manual")
    {
        return GuardVerdict::Reject(vec![Diagnostic::error(
            Code::Invariant,
            &format!("goal {} status is derived from fitness deltas", n.id),
        )]);
    }
    if n.kind == Kind::W && new == "ready" && n.wtype.as_deref() == Some("feature") {
        let bad: Vec<String> = bchain(st, n)
            .into_iter()
            .filter(|b| match st.nodes.get(b) {
                None => true,
                Some(bn) => !(bn.status == "validated" || bn.status == "invalidated_acceptable"),
            })
            .collect();
        if !bad.is_empty() {
            let detail = bad
                .iter()
                .map(|b| match st.nodes.get(b) {
                    None => format!("{b} (missing)"),
                    Some(bn) => format!("{} ({})", b, bn.status),
                })
                .collect::<Vec<_>>()
                .join(", ");
            return GuardVerdict::Reject(vec![Diagnostic::error_details(
                Code::Invariant,
                &format!(
                    "{} cannot enter ready, the assumption chain must be validated or invalidated_acceptable",
                    n.id
                ),
                vec![("chain".to_string(), detail)],
            )]);
        }
    }
    if n.kind == Kind::T {
        return GuardVerdict::Reject(vec![Diagnostic::error(
            Code::Invariant,
            "theme status is derived and cannot be set manually",
        )]);
    }
    if n.kind == Kind::A {
        return GuardVerdict::Reject(vec![Diagnostic::error(
            Code::Invariant,
            "area status is structural and cannot be set",
        )]);
    }
    if n.kind == Kind::W && new == "progress" {
        if !dor(st, n, false) {
            return GuardVerdict::Reject(vec![Diagnostic::error(
                Code::Invariant,
                &format!(
                    "DoR is not satisfied for {}, inspect readiness for the failing conjuncts",
                    n.id
                ),
            )]);
        }
        if !preds_clear(st, &n.id) {
            return GuardVerdict::Reject(vec![Diagnostic::error(
                Code::Invariant,
                "predecessors not cleared (goal blockers must be verified, not merely declined, partial, or unverified)",
            )]);
        }
        let wip = listnodes(st, Kind::W, false)
            .iter()
            .filter(|w| w.status == "progress")
            .count() as i64;
        if wip >= WIP_LIMIT_DEFAULT {
            return GuardVerdict::Reject(vec![Diagnostic::error_details(
                Code::Invariant,
                "WIP limit reached",
                vec![("limit".to_string(), WIP_LIMIT_DEFAULT.to_string())],
            )]);
        }
    }
    if n.kind == Kind::W && new == "done" {
        if !preds_clear(st, &n.id) {
            return GuardVerdict::Reject(vec![Diagnostic::error(
                Code::Invariant,
                "predecessors not cleared (goal blockers must be verified, not merely declined, partial, or unverified)",
            )]);
        }
        if n.wtype.as_deref() == Some("spike") {
            let has_produces = st
                .edges
                .iter()
                .any(|e| e.label == "produces" && e.from == n.id);
            if !has_produces {
                return GuardVerdict::Reject(vec![Diagnostic::error(
                    Code::Invariant,
                    &format!(
                        "{} is a done spike but produces is empty (no outgoing produces edges)",
                        n.id
                    ),
                )]);
            }
        }
        if n.lines("evidence").is_empty() {
            return GuardVerdict::Reject(vec![Diagnostic::error(
                Code::Invariant,
                &format!("{} has no evidence, record evidence for the item", n.id),
            )]);
        }
        let f = n.fitness();
        for g in n.lines("goals") {
            if !f.contains_key(&g) {
                return GuardVerdict::Reject(vec![Diagnostic::error_details(
                    Code::Invariant,
                    &format!("missing fitness delta for {g}, stage a delta for every linked goal"),
                    vec![
                        ("goal".to_string(), g.clone()),
                        ("work".to_string(), n.id.clone()),
                    ],
                )]);
            }
        }
    }
    if n.kind == Kind::Y {
        let cur = n.status.as_str();
        let ok = if new == "superseded" {
            cur != "superseded"
        } else if cur == "proposed" && new == "active" {
            let issues = discovery_anchor_issues(st, n);
            if !issues.is_empty() {
                let mut details = Vec::new();
                for (i, issue) in issues.iter().enumerate() {
                    details.push((format!("issue_{}", i + 1), issue.text.clone()));
                }
                return GuardVerdict::Reject(vec![Diagnostic::error_details(
                    Code::Invariant,
                    &format!(
                        "anchors not satisfied for {}, activation from proposed is refused",
                        n.id
                    ),
                    details,
                )]);
            }
            true
        } else if cur == "active" && new == "stale" {
            true
        } else {
            false
        };
        if !ok {
            return GuardVerdict::Reject(vec![Diagnostic::error_details(
                Code::Invariant,
                &format!("illegal discovery transition {} to {}", cur, new),
                vec![(
                    "allowed".to_string(),
                    "proposed→active, active→stale, non-terminal→superseded; stale→active only through revalidate".to_string(),
                )],
            )]);
        }
    }
    GuardVerdict::Ok
}
