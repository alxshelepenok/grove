use grove_core::{cmd_skill, cmd_specification, CliCtx};

#[test]
fn skill_print_and_install() {
    let dir = std::env::temp_dir().join(format!("grove-skill-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let ctx = CliCtx::new(dir.to_string_lossy().into_owned());

    let r = cmd_skill(&ctx, &[], &[]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("---\n"));
    assert!(
        r.out.lines().take(3).any(|l| l.starts_with("version: ")),
        "printed skill must carry the version stamp"
    );
    assert!(r.out.contains("](specification.md)"));
    assert!(!r.out.contains("../../specification.md"));

    let specification = cmd_specification(&ctx, &[], &[]);
    assert_eq!(specification.code, 0);
    assert!(specification.out.starts_with("# Grove\n"));
    let mut json_ctx = ctx.clone();
    json_ctx.json = true;
    let json = cmd_specification(&json_ctx, &[], &[]);
    let payload: serde_json::Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(payload["command"], "specification");
    assert_eq!(payload["mode"], "print");
    assert_eq!(payload["markdown"].as_str().unwrap(), specification.out);

    let install = dir.to_string_lossy().into_owned();
    let r = cmd_skill(&ctx, &[], &[("install".to_string(), install)]);
    assert_eq!(r.code, 0);
    assert!(r.out.starts_with("installed 2 skill files to"), "{}", r.out);
    let installed = dir.join("grove");
    assert!(installed.join("SKILL.md").is_file());
    assert_eq!(
        std::fs::read_to_string(installed.join("specification.md")).unwrap(),
        specification.out
    );
    assert_eq!(std::fs::read_dir(installed).unwrap().count(), 2);

    let _ = std::fs::remove_dir_all(&dir);
}
