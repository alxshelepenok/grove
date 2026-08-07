use crate::algebra::{ac_of, asks_of, bchain, coverage, goals_of};
use crate::json::{JVal, JuliaDict};
use crate::model::{Kind, Node, State};
use crate::status::{is_terminal, listnodes, prose_field_nonempty};

pub fn refactor_materialised_root_cause(st: &State, w: &Node) -> (bool, String) {
    let mut parts = Vec::new();
    for e in &st.edges {
        if e.label != "causes" || e.to != w.id {
            continue;
        }
        let a = match st.nodes.get(&e.from) {
            None => continue,
            Some(a) => a,
        };
        if a.kind != Kind::T || a.archived {
            continue;
        }
        parts.push(e.from.clone());
    }
    if parts.is_empty() {
        return (false, String::new());
    }
    parts.sort();
    parts.dedup();
    (true, parts.join(", "))
}

pub fn parse_requires_coverage(v: Option<&str>) -> Option<f64> {
    let v = v?;
    let s = v.trim();
    if s == "true" {
        return Some(0.5);
    }
    let x: f64 = s.parse().ok()?;
    if x > 0.0 && x <= 1.0 {
        Some(x)
    } else {
        None
    }
}

pub fn coverage_requirement(st: &State, w: &Node) -> Option<f64> {
    let mut theta: Option<f64> = None;
    for gid in goals_of(w) {
        let g = match st.nodes.get(&gid) {
            None => continue,
            Some(g) => g,
        };
        let v = parse_requires_coverage(g.attrs.get("requires_coverage").map(|s| s.as_str()));
        if let Some(v) = v {
            theta = Some(match theta {
                None => v,
                Some(t) => t.max(v),
            });
        }
    }
    let tid = w.single("theme");
    if !tid.is_empty() {
        if let Some(a) = st.nodes.get(&tid) {
            let v = parse_requires_coverage(a.attrs.get("requires_coverage").map(|s| s.as_str()));
            if let Some(v) = v {
                theta = Some(match theta {
                    None => v,
                    Some(t) => t.max(v),
                });
            }
        }
    }
    theta
}

pub fn dor_breakdown(st: &State, w: &Node, pin_coverage: bool) -> Vec<(String, bool, String)> {
    let mut out: Vec<(String, bool, String)> = Vec::new();
    let g = goals_of(w);
    out.push(("goals(w) ≠ ∅".to_string(), !g.is_empty(), g.join(", ")));
    let ac = ac_of(w);
    out.push((
        "AC(w) ≠ ∅".to_string(),
        !ac.is_empty(),
        format!("{} entries", ac.len()),
    ));
    let asks = asks_of(st, w);
    let asks_ok = asks.iter().all(|q| match st.nodes.get(q) {
        None => false,
        Some(n) => is_terminal(Kind::Q, &n.status),
    });
    out.push((
        "∀ q ∈ asks(w), q terminal".to_string(),
        asks_ok,
        asks.join(", "),
    ));
    let is_feature = w.wtype.as_deref() == Some("feature");
    if is_feature {
        let chain = bchain(st, w);
        let chain_ok = chain.iter().all(|b| match st.nodes.get(b) {
            None => false,
            Some(n) => n.status == "validated" || n.status == "invalidated_acceptable",
        });
        out.push(("BChain validated".to_string(), chain_ok, chain.join(", ")));
    } else {
        out.push((
            "BChain validated".to_string(),
            true,
            "(non-feature)".to_string(),
        ));
    }
    let fitness = w.fitness();
    let fit_ok = !g.is_empty() && g.iter().all(|gid| fitness.contains_key(gid));
    let mut fdict = JuliaDict::new();
    for (k, v) in fitness {
        fdict.insert(k.clone(), JVal::Int(v));
    }
    let fit_detail = fdict
        .iter_pairs()
        .map(|(k, v)| {
            let v = match v {
                JVal::Int(i) => *i,
                _ => 0,
            };
            if v >= 0 {
                format!("{k}=+{v}")
            } else {
                format!("{k}={v}")
            }
        })
        .collect::<Vec<String>>()
        .join(", ");
    out.push(("fitness deltas set ∀ g".to_string(), fit_ok, fit_detail));
    let es = w.lines("evidence_strategy");
    out.push((
        "evidence_strategy ≠ ∅".to_string(),
        !es.is_empty(),
        format!("{} entries", es.len()),
    ));
    if is_feature {
        let hyp = w.lines("hypothesis");
        out.push(("hypothesis ≠ ⊥".to_string(), !hyp.is_empty(), String::new()));
    } else {
        out.push((
            "hypothesis ≠ ⊥".to_string(),
            true,
            "(non-feature)".to_string(),
        ));
    }
    if w.wtype.as_deref() == Some("bug") {
        let rp = w.lines("repro");
        let r_ok = prose_field_nonempty(&rp);
        out.push((
            "repro(w) ≠ ∅".to_string(),
            r_ok,
            if r_ok {
                format!("{} entries", rp.len())
            } else {
                String::new()
            },
        ));
    } else {
        out.push(("repro(w) ≠ ∅".to_string(), true, "(non-bug)".to_string()));
    }
    if w.wtype.as_deref() == Some("spike") {
        let ex = w.lines("exit");
        let e_ok = prose_field_nonempty(&ex);
        out.push((
            "exit(w) ≠ ∅".to_string(),
            e_ok,
            if e_ok {
                format!("{} entries", ex.len())
            } else {
                String::new()
            },
        ));
    } else {
        out.push(("exit(w) ≠ ∅".to_string(), true, "(non-spike)".to_string()));
    }
    if w.wtype.as_deref() == Some("refactor") {
        let (rc_ok, rc_detail) = refactor_materialised_root_cause(st, w);
        out.push((
            "(A, causes, w) via materialised A".to_string(),
            rc_ok,
            rc_detail,
        ));
    } else {
        out.push((
            "(A, causes, w) via materialised A".to_string(),
            true,
            "(non-refactor)".to_string(),
        ));
    }
    out.push((
        "cynefin ≠ chaotic".to_string(),
        w.cynefin.as_deref() != Some("chaotic"),
        w.cynefin.clone().unwrap_or_default(),
    ));
    let theta = coverage_requirement(st, w);
    let label = "coverage(w) ≥ θ".to_string();
    let is_complex = w.cynefin.as_deref() == Some("complex");
    if theta.is_none() {
        out.push((label, true, "(coverage not required)".to_string()));
    } else if !(is_feature && is_complex) {
        out.push((label, true, "(non-complex-feature)".to_string()));
    } else if pin_coverage {
        out.push((label, true, "(pinned at transition)".to_string()));
    } else {
        let theta = theta.expect("checked above");
        let surface_w = w.lines("surface");
        let (ratio, _, uncovered) = coverage(st, w);
        let theta_s = format!("{:.2}", theta);
        if surface_w.is_empty() {
            out.push((
                label,
                false,
                format!("no declared surface; declare via field {} surface add …", w.id),
            ));
        } else if ratio < theta {
            let shown: Vec<String> = uncovered.iter().take(5).cloned().collect();
            let mut det = format!("{:.2} < {}; uncovered: {}", ratio, theta_s, shown.join(", "));
            if uncovered.len() > 5 {
                det.push_str(&format!(" … (+{} more)", uncovered.len() - 5));
            }
            out.push((label, false, det));
        } else {
            out.push((label, true, format!("{:.2} ≥ {}", ratio, theta_s)));
        }
    }
    out
}

pub fn dor(st: &State, w: &Node, pin_coverage: bool) -> bool {
    dor_breakdown(st, w, pin_coverage).iter().all(|t| t.1)
}

pub fn dor_id(st: &State, id: &str, pin_coverage: bool) -> Option<bool> {
    st.nodes.get(id).map(|n| dor(st, n, pin_coverage))
}

pub fn ready(st: &State) -> Vec<&Node> {
    let mut out = Vec::new();
    for w in listnodes(st, Kind::W, false) {
        if w.status != "ready" {
            continue;
        }
        if !crate::algebra::preds_clear(st, &w.id) {
            continue;
        }
        if !dor(st, w, false) {
            continue;
        }
        out.push(w);
    }
    out
}

pub fn format_dor_report(st: &State, id: &str) -> Option<String> {
    let n = st.nodes.get(id)?;
    let mut out = format!("{} DoR:\n", n.id);
    for (label, ok, detail) in dor_breakdown(st, n, false) {
        let sym = if ok { "⊤" } else { "⊥" };
        if detail.is_empty() {
            out.push_str(&format!("  {}  {}\n", sym, label));
        } else {
            out.push_str(&format!("  {}  {}  → {}\n", sym, label, detail));
        }
    }
    let overall = if dor(st, n, false) { "⊤" } else { "⊥" };
    out.push_str(&format!("result: {}\n", overall));
    Some(out)
}
