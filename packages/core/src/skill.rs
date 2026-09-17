// Paths mirror docs/skills/grove of the source tree; the build fails if one drifts.

pub fn skill_page(path: &str) -> Option<&'static str> {
    let path = path.trim_start_matches('/');
    Some(match path {
        "SKILL.md" => include_str!("../../../docs/skills/grove/SKILL.md"),
        "references/model.md" => include_str!("../../../docs/skills/grove/references/model.md"),
        "references/protocol.md" => include_str!("../../../docs/skills/grove/references/protocol.md"),
        "references/planning.md" => include_str!("../../../docs/skills/grove/references/planning.md"),
        "references/cli.md" => include_str!("../../../docs/skills/grove/references/cli.md"),
        "references/evidence.md" => include_str!("../../../docs/skills/grove/references/evidence.md"),
        "references/rules.md" => include_str!("../../../docs/skills/grove/references/rules.md"),
        "references/lockfile.md" => include_str!("../../../docs/skills/grove/references/lockfile.md"),
        "references/typography.md" => include_str!("../../../docs/skills/grove/references/typography.md"),
        "references/checklist.md" => include_str!("../../../docs/skills/grove/references/checklist.md"),
        "diagrams/dual-track.md" => include_str!("../../../docs/skills/grove/diagrams/dual-track.md"),
        "diagrams/graph-template.md" => include_str!("../../../docs/skills/grove/diagrams/graph-template.md"),
        "diagrams/workflow.md" => include_str!("../../../docs/skills/grove/diagrams/workflow.md"),
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

pub const PAGES: [&str; 13] = [
    "SKILL.md",
    "references/model.md",
    "references/protocol.md",
    "references/planning.md",
    "references/cli.md",
    "references/evidence.md",
    "references/rules.md",
    "references/lockfile.md",
    "references/typography.md",
    "references/checklist.md",
    "diagrams/dual-track.md",
    "diagrams/graph-template.md",
    "diagrams/workflow.md",
];

pub fn install_into(dir: &str) -> Result<(usize, String), String> {
    let target = std::path::Path::new(dir).join("grove");
    for sub in ["references", "diagrams"] {
        std::fs::create_dir_all(target.join(sub)).map_err(|e| e.to_string())?;
    }
    let mut n = 0usize;
    for p in PAGES {
        let text = if p == "SKILL.md" {
            stamped_skill_md()
        } else {
            skill_page(p)
                .ok_or_else(|| format!("missing embedded skill page: {p}"))?
                .to_string()
        };
        std::fs::write(target.join(p), text).map_err(|e| format!("write {p}: {e}"))?;
        n += 1;
    }
    Ok((n, target.display().to_string()))
}
