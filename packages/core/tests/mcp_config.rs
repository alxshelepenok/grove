use grove_core::mcp_config_path;
use grove_core::load_tool_config;
use std::path::PathBuf;

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-mcp-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(d.join(".grove")).unwrap();
    d
}

fn cfg_path(dir: &PathBuf) -> PathBuf {
    mcp_config_path(&dir.to_string_lossy())
}

fn write_cfg(dir: &PathBuf, text: &str) {
    std::fs::write(cfg_path(dir), text).unwrap();
}

#[test]
fn missing_file_yields_default_config() {
    let dir = tmpdir("missing");
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert!(got.disabled.is_empty());
    assert!(got.roles.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn present_file_parses_disabled_and_roles() {
    let dir = tmpdir("present");
    write_cfg(
        &dir,
        "# comment\n\r\ndisabled = [\"undo\", \"repair\"]\r\n\r\n[roles.worker]\ntools = [\"next\", \"packet\"]\n\r\n[roles.observer]\r\ntools = [\"ready\", \"check\"] # trailing
",
    );
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert_eq!(got.disabled, vec!["undo".to_string(), "repair".to_string()]);
    assert_eq!(got.roles.len(), 2);
    assert_eq!(
        got.roles.get("worker").unwrap().tools,
        vec!["next".to_string(), "packet".to_string()]
    );
    assert_eq!(
        got.roles.get("observer").unwrap().tools,
        vec!["ready".to_string(), "check".to_string()]
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn empty_file_parses_as_empty_config() {
    let dir = tmpdir("empty");
    write_cfg(&dir, "");
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert!(got.disabled.is_empty());
    assert!(got.roles.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn extra_keys_and_sections_are_tolerated() {
    let dir = tmpdir("extra");
    write_cfg(
        &dir,
        "disabled = [\"gate\"]\nfuture_key = \"ignored\"\n\n[unrelated]\nfoo = 5\n\n[roles.worker]\ntools = [\"next\"]\n",
    );
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert_eq!(got.disabled, vec!["gate".to_string()]);
    assert_eq!(got.roles.len(), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn malformed_shapes_fail_open_to_none() {
    let dir = tmpdir("malformed");
    let shapes = [
        "disabled = \"undo\"\n",
        "disabled = [1, 2]\n",
        "disabled = [\"undo\"\n",
        "roles.jpg = [\"x\"]\n",
        "[[roles.worker]]\ntools = [\"next\"]\n",
        "[roles.worker]\ntools = [\"next\"]\ntools = [\"ready\"]\n",
        "disabled = [\"a\"]\ndisabled = [\"b\"]\n",
        "[roles.worker]\n[roles.worker]\ntools = [\"next\"]\n",
        "not toml at all\n",
        "[roles.\"]\ntools = [\"next\"]\n",
    ];
    for (i, shape) in shapes.iter().enumerate() {
        write_cfg(&dir, shape);
        assert!(
            load_tool_config(&dir.to_string_lossy()).is_none(),
            "shape {i} must be malformed: {shape}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn non_role_headers_define_no_roles() {
    let dir = tmpdir("non-role");
    write_cfg(
        &dir,
        "[roles]\ntools = [\"next\"]\n\n[roles.multi.dot]\ntools = [\"ready\"]\n\n[roles.worker]\ntools = [\"packet\"]\n",
    );
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert_eq!(got.roles.len(), 1);
    assert_eq!(got.roles.get("worker").unwrap().tools, vec!["packet".to_string()]);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn role_without_tools_key_is_empty_allowlist() {
    let dir = tmpdir("allow-empty");
    write_cfg(&dir, "disabled = [\"undo\"]\n\n[roles.worker]\n");
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert_eq!(got.disabled, vec!["undo".to_string()]);
    assert!(got.roles.get("worker").unwrap().tools.is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unknown_commands_are_kept_and_flagged() {
    let dir = tmpdir("unknown");
    write_cfg(
        &dir,
        "disabled = [\"undo\"]\n\n[roles.worker]\ntools = [\"packet\", \"renumberx\"]\n",
    );
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    assert_eq!(got.disabled, vec!["undo".to_string()]);
    assert_eq!(
        got.roles.get("worker").unwrap().tools,
        vec!["packet".to_string(), "renumberx".to_string()]
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn per_role_instructions_parse_on_accepts_and_dups_reject() {
    let dir = tmpdir("instructions");
    write_cfg(
        &dir,
        "[roles.keeper]\ntools = [\"check\"]\ninstructions = \"You only run invariant checks; start each session with the check tool.\"\n",
    );
    let got = load_tool_config(&dir.to_string_lossy()).unwrap();
    let scope = got.roles.get("keeper").unwrap();
    assert_eq!(scope.tools, vec!["check".to_string()]);
    assert_eq!(
        scope.instructions.as_deref(),
        Some("You only run invariant checks; start each session with the check tool.")
    );
    let shapes = [
        "[roles.keeper]\ninstructions = 5\n",
        "[roles.keeper]\ninstructions = [\"check\"]\n",
        "[roles.keeper]\ninstructions = \"one\"\ninstructions = \"two\"\n",
    ];
    for (i, shape) in shapes.iter().enumerate() {
        write_cfg(&dir, shape);
        assert!(
            load_tool_config(&dir.to_string_lossy()).is_none(),
            "shape {i} must be malformed: {shape}"
        );
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn config_path_sits_next_to_state_lock() {
    let dir = tmpdir("path");
    let p = mcp_config_path(&dir.to_string_lossy());
    assert_eq!(
        p,
        dir.join(".grove").join("mcp.toml")
    );
    std::fs::remove_dir_all(&dir).ok();
}
