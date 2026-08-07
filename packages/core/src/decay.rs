use crate::cli::CliCtx;
use crate::model::{Kind, State};
use crate::status::listnodes;
use std::path::{Path, PathBuf};

pub fn discovery_decay_errors(st: &State, root: &str, gpath: &Path) -> Vec<String> {
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
                    out.push(format!("decay: {} dead surface: {p}", x.id));
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
                out.push(format!("decay: {} rotted origin: {} (superseded)", x.id, dst.id));
            } else if dst.kind == Kind::B
                && (dst.status == "invalidated_acceptable" || dst.status == "invalidated_blocking")
            {
                out.push(format!("decay: {} rotted origin: {} ({})", x.id, dst.id, dst.status));
            }
        }
        for t in x.lines("tags") {
            if !terms.contains(&t) {
                out.push(format!("decay: {} lost glossary term: {t}", x.id));
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
        let toks: Vec<&str> = e.split(' ').filter(|s| !s.is_empty()).collect();
        if toks.len() >= 2 {
            ids.insert(toks[1].to_string());
        }
    }
    ids.len() as i64
}
