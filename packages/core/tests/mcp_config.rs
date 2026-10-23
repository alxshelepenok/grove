use grove_core::{load_tool_config, mcp_config_path, registry_path};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn home_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "grove-core-mcp-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct HomeGuard {
    saved: Option<std::ffi::OsString>,
}

impl HomeGuard {
    fn set(home: &Path) -> HomeGuard {
        let saved = std::env::var_os("GROVE_HOME");
        std::env::set_var("GROVE_HOME", home);
        std::env::remove_var("GROVE_PROJECT");
        HomeGuard { saved }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        match self.saved.take() {
            Some(v) => std::env::set_var("GROVE_HOME", v),
            None => std::env::remove_var("GROVE_HOME"),
        }
    }
}

fn write_cfg(text: &str) {
    std::fs::write(mcp_config_path(), text).unwrap();
}

#[test]
fn missing_file_yields_default_config() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("missing");
    let _h = HomeGuard::set(&home);
    let got = load_tool_config().unwrap();
    assert!(got.disabled.is_empty());
    assert!(got.roles.is_empty());
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn present_file_parses_disabled_roles_and_instructions() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("present");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("disabled = [\"undo\", \"repair\"]\n# comment\n\r\n[roles.observer]\r\ntools = [\"ready\", \"check\"]\r\n\r\n[roles.worker]\ntools = [\"next\", \"packet\"]\n");
        let got = load_tool_config().unwrap();
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
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn empty_file_parses_as_empty_config() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("empty");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("");
        let got = load_tool_config().unwrap();
        assert!(got.disabled.is_empty());
        assert!(got.roles.is_empty());
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn extra_keys_and_sections_are_tolerated() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("extra");
    {
        let _h = HomeGuard::set(&home);
        write_cfg(
            "disabled = [\"gate\"]\nfuture_key = \"ignored\"\n\n[unrelated]\nfoo = 5\n\n[roles.worker]\ntools = [\"next\"]\n",
        );
        let got = load_tool_config().unwrap();
        assert_eq!(got.disabled, vec!["gate".to_string()]);
        assert_eq!(got.roles.len(), 1);
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn malformed_shapes_fail_open_to_none() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("malformed");
    {
        let _h = HomeGuard::set(&home);
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
            write_cfg(shape);
            assert!(
                load_tool_config().is_none(),
                "shape {i} must be malformed: {shape}"
            );
        }
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn non_role_headers_define_no_roles() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("non-role");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("[roles]\ntools = [\"next\"]\n\n[roles.multi.dot]\ntools = [\"ready\"]\n\n[roles.worker]\ntools = [\"packet\"]\n");
        let got = load_tool_config().unwrap();
        assert_eq!(got.roles.len(), 1);
        assert_eq!(got.roles.get("worker").unwrap().tools, vec!["packet".to_string()]);
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn role_without_tools_key_is_empty_allowlist() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("allow-empty");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("disabled = [\"undo\"]\n\n[roles.worker]\n");
        let got = load_tool_config().unwrap();
        assert_eq!(got.disabled, vec!["undo".to_string()]);
        assert!(got.roles.get("worker").unwrap().tools.is_empty());
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn unknown_commands_are_kept_and_flagged() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("unknown");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("disabled = [\"undo\"]\n\n[roles.worker]\ntools = [\"packet\", \"renumberx\"]\n");
        let got = load_tool_config().unwrap();
        assert_eq!(got.disabled, vec!["undo".to_string()]);
        assert_eq!(
            got.roles.get("worker").unwrap().tools,
            vec!["packet".to_string(), "renumberx".to_string()]
        );
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn per_role_instructions_parse_on_accepts_and_dups_reject() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("instructions");
    {
        let _h = HomeGuard::set(&home);
        write_cfg(
            "[roles.keeper]\ntools = [\"check\"]\ninstructions = \"You only run invariant checks; start each session with the check tool.\"\n",
        );
        let got = load_tool_config().unwrap();
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
            write_cfg(shape);
            assert!(
                load_tool_config().is_none(),
                "shape {i} must be malformed: {shape}"
            );
        }
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn multiline_tool_arrays_parse() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("multiline");
    {
        let _h = HomeGuard::set(&home);
        write_cfg(
            "disabled = [\r\n  \"undo\",\r\n# why not\r\n  \"repair\",\r\n]\r\n\r\n[roles.keeper]\r\ntools = [\r\n  \"check\",  # invariants\r\n  \"status\",\r\n]\r\ninstructions = \"Keep the invariants.\"\r\n",
        );
        let got = load_tool_config().unwrap();
        assert_eq!(got.disabled, vec!["undo".to_string(), "repair".to_string()]);
        assert_eq!(got.roles.len(), 1);
        let scope = got.roles.get("keeper").unwrap();
        assert_eq!(
            scope.tools,
            vec!["check".to_string(), "status".to_string()]
        );
        assert_eq!(scope.instructions.as_deref(), Some("Keep the invariants."));
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn bracket_inside_string_does_not_open_multiline() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("bracket-string");
    {
        let _h = HomeGuard::set(&home);
        write_cfg(
            "[roles.ghost]\ntools = [\"check\"]\ninstructions = \"mention a left [ bracket and stay on one line\"\n\n[roles.keeper]\ntools = [\"status\"]\n",
        );
        let got = load_tool_config().unwrap();
        assert_eq!(got.roles.len(), 2);
        assert_eq!(
            got.roles.get("ghost").unwrap().instructions.as_deref(),
            Some("mention a left [ bracket and stay on one line")
        );
        assert_eq!(
            got.roles.get("keeper").unwrap().tools,
            vec!["status".to_string()]
        );
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn unterminated_multiline_array_is_malformed() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("unterminated");
    {
        let _h = HomeGuard::set(&home);
        write_cfg("[roles.keeper]\ntools = [\"check\",\n  \"status\"\n");
        assert!(load_tool_config().is_none());
    }
    std::fs::remove_dir_all(&home).ok();
}

#[test]
fn config_path_sits_next_to_projects_registry() {
    let _g = ENV_LOCK.lock().unwrap();
    let home = home_dir("path");
    {
        let _h = HomeGuard::set(&home);
        let p = mcp_config_path();
        assert_eq!(p, home.join("mcp.toml"));
        let reg = registry_path();
        assert_eq!(p.parent(), reg.parent());
    }
    std::fs::remove_dir_all(&home).ok();
}
