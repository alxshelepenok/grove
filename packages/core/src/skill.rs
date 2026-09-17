// Paths mirror docs/skills of the source tree; the build fails if one drifts.

pub fn skill_page(path: &str) -> Option<&'static str> {
    let path = path.trim_start_matches('/');
    Some(match path {
        "SKILL.md" => include_str!("../../../docs/skills/SKILL.md"),
        "references/model.md" => include_str!("../../../docs/skills/references/model.md"),
        "references/protocol.md" => include_str!("../../../docs/skills/references/protocol.md"),
        "references/planning.md" => include_str!("../../../docs/skills/references/planning.md"),
        "references/cli.md" => include_str!("../../../docs/skills/references/cli.md"),
        "references/evidence.md" => include_str!("../../../docs/skills/references/evidence.md"),
        "references/rules.md" => include_str!("../../../docs/skills/references/rules.md"),
        "references/lockfile.md" => include_str!("../../../docs/skills/references/lockfile.md"),
        "references/typography.md" => include_str!("../../../docs/skills/references/typography.md"),
        "references/checklist.md" => include_str!("../../../docs/skills/references/checklist.md"),
        "diagrams/dual-track.md" => include_str!("../../../docs/skills/diagrams/dual-track.md"),
        "diagrams/graph-template.md" => include_str!("../../../docs/skills/diagrams/graph-template.md"),
        "diagrams/workflow.md" => include_str!("../../../docs/skills/diagrams/workflow.md"),
        _ => return None,
    })
}

pub fn skill_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn stamped_skill_md() -> String {
    let md = skill_page("SKILL.md").unwrap_or_default();
    if md.lines().take(3).any(|l| l.starts_with("version:")) {
        return md.to_string();
    }
    let mut out = String::new();
    for (i, line) in md.lines().enumerate() {
        out.push_str(line);
        out.push('\n');
        if i == 0 && line.trim() == "---" {
            out.push_str(&format!("version: {}\n", skill_version()));
        }
    }
    out
}

pub fn pointer_line() -> String {
    format!(
        "skill: grove://skill (skill v{}, binary v{})",
        skill_version(),
        skill_version()
    )
}
