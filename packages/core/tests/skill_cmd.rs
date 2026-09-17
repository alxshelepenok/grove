use grove_core::{cmd_skill, CliCtx};

#[test]
fn skill_print_and_install() {
    let dir = std::env::temp_dir().join(format!("grove-skill-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());

    let r = cmd_skill(&ctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("---\n"));
    assert!(
        r.out.lines()
            .take(3)
            .any(|l| l.starts_with("version: ")),
        "printed skill must carry the version stamp"
    );

    let install = dir.to_string_lossy().into_owned();
    let r = cmd_skill(
        &ctx,
        &[],
        &[("install".to_string(), install)],
    );
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("installed 13 skill files to"), "{}", r.out);
    assert!(dir.join("grove").join("SKILL.md").is_file());
    assert!(dir.join("grove").join("references").join("rules.md").is_file());
    assert!(dir.join("grove").join("diagrams").join("workflow.md").is_file());

    let _ = std::fs::remove_dir_all(&dir);
}
