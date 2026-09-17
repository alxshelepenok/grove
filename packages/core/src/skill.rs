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
