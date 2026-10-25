pub fn skill_page(path: &str) -> Option<&'static str> {
    let path = path.trim_start_matches('/');
    Some(match path {
        "SKILL.md" => include_str!("../../../docs/skills/grove/SKILL.md"),
        "specification.md" => include_str!("../../../docs/specification.md"),
        _ => return None,
    })
}

pub fn skill_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn specification_md() -> &'static str {
    skill_page("specification.md").unwrap_or_default()
}

pub fn stamped_skill_md() -> String {
    let md = skill_page("SKILL.md")
        .unwrap_or_default()
        .replace("../../specification.md", "specification.md");
    if md.lines().take(3).any(|l| l.starts_with("version:")) {
        return md;
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

pub const PAGES: [&str; 2] = ["SKILL.md", "specification.md"];

pub fn install_into(dir: &str) -> Result<(usize, String), String> {
    let target = std::path::Path::new(dir).join("grove");
    std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
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
