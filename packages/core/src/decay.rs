use crate::cli::CliCtx;
use crate::message::{Code, Diagnostic};
use crate::model::{Kind, State};
use crate::status::listnodes;
use std::path::{Path, PathBuf};

pub fn discovery_decay_errors(st: &State, root: &str, gpath: &Path) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let xs: Vec<&crate::model::Node> = listnodes(st, Kind::Y, false)
        .into_iter()
        .filter(|x| x.status == "proposed" || x.status == "active")
        .collect();
    if xs.is_empty() {
        return out;
    }
    let gtext = std::fs::read_to_string(gpath).unwrap_or_default();
    let terms = crate::renumber::glossary_terms(&gtext);
    for x in xs {
        if x.fields.contains_key("surface") {
            for p in x.lines("surface") {
                let pth: PathBuf = if Path::new(&p).is_absolute() {
                    PathBuf::from(&p)
                } else {
                    Path::new(root).join(&p)
                };
                if !pth.exists() {
                    out.push(Diagnostic::error_details(
                        Code::Invariant,
                        &format!("{} has a dead surface path", x.id),
                        vec![("path".to_string(), p.clone())],
                    ));
                }
            }
        }
        for e in &st.edges {
            if !(e.label == "distills" && e.from == x.id) {
                continue;
            }
            let Some(dst) = st.nodes.get(&e.to) else {
                continue;
            };
            if dst.archived {
                continue;
            }
            if dst.kind == Kind::D && dst.status == "superseded" {
                out.push(Diagnostic::error_details(
                    Code::Invariant,
                    &format!("{} has a rotted origin ({})", x.id, dst.id),
                    vec![
                        ("origin".to_string(), dst.id.clone()),
                        ("status".to_string(), dst.status.clone()),
                    ],
                ));
            } else if dst.kind == Kind::B
                && (dst.status == "invalidated_acceptable" || dst.status == "invalidated_blocking")
            {
                out.push(Diagnostic::error_details(
                    Code::Invariant,
                    &format!("{} has a rotted origin ({})", x.id, dst.id),
                    vec![
                        ("origin".to_string(), dst.id.clone()),
                        ("status".to_string(), dst.status.clone()),
                    ],
                ));
            }
        }
        for t in x.lines("tags") {
            if !terms.contains(&t) {
                out.push(Diagnostic::error_details(
                    Code::Invariant,
                    &format!("{} lost a glossary term ({t})", x.id),
                    vec![("term".to_string(), t.clone())],
                ));
            }
        }
    }
    out
}

pub fn dashboard_decay_count(ctx: &CliCtx, st: &State) -> i64 {
    let any_y = listnodes(st, Kind::Y, false)
        .iter()
        .any(|x| x.status == "proposed" || x.status == "active");
    if !any_y {
        return 0;
    }
    let errs = discovery_decay_errors(st, &ctx.root, &ctx.glossarypath());
    let mut ids = std::collections::BTreeSet::new();
    for e in &errs {
        let toks: Vec<&str> = e.text.split(' ').filter(|s| !s.is_empty()).collect();
        if !toks.is_empty() {
            ids.insert(toks[0].to_string());
        }
    }
    ids.len() as i64
}
