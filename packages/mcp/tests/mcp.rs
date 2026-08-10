use grove_core::{handle_message, run_cli, McpServer, COMMAND_NAMES};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(home: &Path) -> EnvGuard {
        let saved = vec![
            ("GROVE_HOME", std::env::var("GROVE_HOME").ok()),
            ("GROVE_PROJECT", std::env::var("GROVE_PROJECT").ok()),
            ("GROVE_SESSION", std::env::var("GROVE_SESSION").ok()),
        ];
        std::env::set_var("GROVE_HOME", home);
        std::env::remove_var("GROVE_PROJECT");
        std::env::remove_var("GROVE_SESSION");
        EnvGuard { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.saved {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    static CTR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let uniq = format!(
        "grove-mcptest-{tag}-{}-{n}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let dir = std::env::temp_dir().join(uniq);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Fixture {
    root: PathBuf,
    _home: PathBuf,
    _env: EnvGuard,
}

fn fixture(tag: &str) -> Fixture {
    let root = temp_dir(tag);
    let home = temp_dir(&format!("{tag}-home"));
    let env = EnvGuard::set(&home);
    Fixture {
        root,
        _home: home,
        _env: env,
    }
}

fn server(root: &Path) -> McpServer {
    McpServer::new(
        root.to_string_lossy().into_owned(),
        "mcp-test-token".to_string(),
    )
}

fn rpc(s: &mut McpServer, msg: &str) -> serde_json::Value {
    let resp = handle_message(s, msg).expect("expected a response");
    serde_json::from_str(&resp).expect("response must parse as JSON")
}

fn call(s: &mut McpServer, id: i64, name: &str, args: serde_json::Value) -> serde_json::Value {
    let msg = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"tools/call\",\"params\":{{\"name\":\"{name}\",\"arguments\":{args}}}}}"
    );
    rpc(s, &msg)
}

fn handshake(s: &mut McpServer) {
    let r = rpc(
        s,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"0\"}}}",
    );
    assert_eq!(r["result"]["serverInfo"]["name"], "grove-mcp");
    assert!(
        handle_message(s, "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}").is_none()
    );
}

fn text_of(v: &serde_json::Value) -> &str {
    v["result"]["content"][0]["text"].as_str().unwrap()
}

fn is_error(v: &serde_json::Value) -> bool {
    v["result"]["isError"].as_bool().unwrap()
}

#[test]
fn malformed_json_returns_parse_error() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("parse");
    let mut s = server(&f.root);
    let v = rpc(&mut s, "{not json");
    assert_eq!(v["jsonrpc"], "2.0");
    assert!(v["id"].is_null());
    assert_eq!(v["error"]["code"], -32700);
}

#[test]
fn batch_request_is_rejected() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("batch");
    let mut s = server(&f.root);
    let v = rpc(
        &mut s,
        "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]",
    );
    assert!(v["id"].is_null());
    assert_eq!(v["error"]["code"], -32600);
}

#[test]
fn unknown_method_returns_method_not_found() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("method");
    let mut s = server(&f.root);
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"bogus/thing\"}");
    assert_eq!(v["id"], 7);
    assert_eq!(v["error"]["code"], -32601);
}

#[test]
fn notifications_never_get_responses() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("notif");
    let mut s = server(&f.root);
    assert!(handle_message(&mut s, "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}").is_none());
    assert!(handle_message(&mut s, "{\"jsonrpc\":\"2.0\",\"method\":\"tools/list\"}").is_none());
    assert!(handle_message(&mut s, "{\"jsonrpc\":\"2.0\",\"method\":\"bogus/thing\"}").is_none());
    assert!(handle_message(&mut s, "   ").is_none());
}

#[test]
fn initialize_handshake_shape_and_negotiation() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("init-shake");
    let mut s = server(&f.root);
    let v = rpc(
        &mut s,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2024-11-05\"}}",
    );
    assert_eq!(v["id"], 1);
    assert_eq!(v["result"]["protocolVersion"], "2024-11-05");
    assert!(v["result"]["capabilities"]["tools"].is_object());
    assert!(v["result"]["capabilities"]["resources"].is_object());
    assert_eq!(v["result"]["serverInfo"]["name"], "grove-mcp");
    assert!(v["result"]["serverInfo"]["version"].is_string());
    let v2 = rpc(
        &mut s,
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"1999-01-01\"}}",
    );
    assert_eq!(v2["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn requests_before_initialized_are_refused() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("gate32002");
    let mut s = server(&f.root);
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}");
    assert_eq!(v["error"]["code"], -32002);
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}");
    assert!(v["result"].is_object());
    rpc(
        &mut s,
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\"}}",
    );
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/list\"}");
    assert_eq!(v["error"]["code"], -32002);
    assert!(
        handle_message(&mut s, "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}")
            .is_none()
    );
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/list\"}");
    assert!(v["result"]["tools"].is_array());
}

#[test]
fn tools_list_covers_all_commands_with_valid_schemas() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("toolslist");
    let mut s = server(&f.root);
    handshake(&mut s);
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/list\"}");
    let tools = v["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), COMMAND_NAMES.len());
    let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = COMMAND_NAMES.to_vec();
    expected.sort_unstable();
    assert_eq!(names, expected);
    for t in tools {
        assert!(t["description"].as_str().unwrap().len() > 3, "{}", t["name"]);
        assert_eq!(t["inputSchema"]["type"], "object");
        assert!(t["inputSchema"]["properties"].is_object(), "{}", t["name"]);
        assert!(t["inputSchema"]["required"].is_array(), "{}", t["name"]);
    }
    let find = |n: &str| tools.iter().find(|t| t["name"] == n).unwrap().clone();
    let add = find("add");
    let add_req: Vec<&str> = add["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(add_req, ["kind", "title"]);
    assert_eq!(
        add["inputSchema"]["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    assert!(add["inputSchema"]["properties"]["goals"].is_object());
    assert!(add["inputSchema"]["properties"]["surface"].is_object());
    let set = find("set");
    let set_req: Vec<&str> = set["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(set_req, ["id", "key", "value"]);
    let packet = find("packet");
    let packet_req: Vec<&str> = packet["inputSchema"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(packet_req, ["id"]);
    assert_eq!(packet["inputSchema"]["properties"]["cone"]["type"], "boolean");
    assert_eq!(
        packet["inputSchema"]["properties"]["cone_depth"]["type"],
        "integer"
    );
    assert_eq!(
        packet["inputSchema"]["properties"]["cone_max"]["type"],
        "integer"
    );
}

#[test]
fn tools_call_unknown_tool_and_argument_validation() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("callvalid");
    let mut s = server(&f.root);
    handshake(&mut s);
    let v = call(&mut s, 1, "bogus", serde_json::json!({}));
    assert_eq!(v["error"]["code"], -32602);
    let v = call(&mut s, 2, "add", serde_json::json!({"kind": "a", "title": "x", "bogus": 1}));
    assert!(v["error"].is_null());
    assert!(is_error(&v));
    let t = text_of(&v);
    assert!(t.contains("unknown argument(s): bogus"), "{t}");
    assert!(t.contains("kind"), "{t}");
    assert!(t.contains("title"), "{t}");
    let v = call(&mut s, 3, "add", serde_json::json!({"title": "x"}));
    assert!(is_error(&v));
    assert!(text_of(&v).contains("missing required argument(s): kind"));
    let v = rpc(
        &mut s,
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"add\",\"arguments\":[1]}}",
    );
    assert_eq!(v["error"]["code"], -32602);
}

fn init_project(s: &mut McpServer, id: i64) {
    let v = call(s, id, "init", serde_json::json!({}));
    assert!(!is_error(&v), "{}", text_of(&v));
}

#[test]
fn tools_call_maps_arguments_to_cli() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("argvmap");
    let mut s = server(&f.root);
    handshake(&mut s);
    init_project(&mut s, 10);
    let v = call(&mut s, 11, "add", serde_json::json!({"kind": "a", "title": "Area one"}));
    assert_eq!(text_of(&v), "A-01");
    let v = call(
        &mut s,
        12,
        "add",
        serde_json::json!({"kind": "g", "title": "Goal one", "area": "A-01", "fitness_kind": "manual"}),
    );
    assert_eq!(text_of(&v), "G-01");
    let v = call(
        &mut s,
        13,
        "add",
        serde_json::json!({
            "kind": "w",
            "title": "Map the argv",
            "type": "feature",
            "cynefin": "clear",
            "goals": "G-01",
            "surface": "src/a.rs,src/b.rs"
        }),
    );
    assert_eq!(text_of(&v), "W-01");
    let v = call(&mut s, 14, "show", serde_json::json!({"id": "W-01"}));
    let shown = text_of(&v).to_string();
    assert!(shown.contains("Map the argv"), "{shown}");
    assert!(shown.contains("G-01"), "{shown}");
    assert!(shown.contains("src/a.rs"), "{shown}");
    let v = call(
        &mut s,
        15,
        "field",
        serde_json::json!({"id": "W-01", "field": "ac", "op": "add", "value": "acceptance here"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = call(
        &mut s,
        16,
        "fitness",
        serde_json::json!({"id": "W-01", "goal": "G-01", "delta": 1}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = call(
        &mut s,
        17,
        "evidence",
        serde_json::json!({"id": "W-01", "text": "evidence line"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = call(
        &mut s,
        18,
        "set",
        serde_json::json!({"id": "W-01", "key": "status", "value": "ready"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = call(&mut s, 19, "show", serde_json::json!({"id": "W-01"}));
    let shown = text_of(&v).to_string();
    assert!(shown.contains("ready"), "{shown}");
    assert!(shown.contains("acceptance here"), "{shown}");
    assert!(shown.contains("evidence line"), "{shown}");
    let v = call(
        &mut s,
        20,
        "packet",
        serde_json::json!({"id": "W-01", "cone": true, "cone_depth": 2, "cone_max": 10}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    assert!(text_of(&v).contains("Contraction order"), "{}", text_of(&v));
    let v = call(&mut s, 21, "check", serde_json::json!({}));
    assert!(!is_error(&v), "{}", text_of(&v));
    assert!(text_of(&v).contains("ok"), "{}", text_of(&v));
    let v = call(&mut s, 22, "stats", serde_json::json!({}));
    assert!(!is_error(&v), "{}", text_of(&v));
    assert!(text_of(&v).contains("cycle time"), "{}", text_of(&v));
}

#[test]
fn guard_refusal_is_tool_error_not_protocol_error() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("guard");
    let mut s = server(&f.root);
    handshake(&mut s);
    init_project(&mut s, 10);
    let v = call(&mut s, 11, "add", serde_json::json!({"kind": "a", "title": "Area"}));
    assert_eq!(text_of(&v), "A-01");
    let v = call(
        &mut s,
        12,
        "add",
        serde_json::json!({"kind": "g", "title": "Goal", "area": "A-01", "fitness_kind": "manual"}),
    );
    assert_eq!(text_of(&v), "G-01");
    let v = call(
        &mut s,
        13,
        "add",
        serde_json::json!({"kind": "w", "title": "Not ready", "type": "feature", "cynefin": "clear", "goals": "G-01"}),
    );
    assert_eq!(text_of(&v), "W-01");
    let v = call(
        &mut s,
        14,
        "set",
        serde_json::json!({"id": "W-01", "key": "status", "value": "progress"}),
    );
    assert!(v["error"].is_null(), "{v}");
    assert!(is_error(&v));
    let t = text_of(&v);
    assert!(t.contains("DoR"), "{t}");
    assert!(t.contains("exit code: 4"), "{t}");
}

#[test]
fn scripted_session_drives_full_work_loop() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("fullloop");
    let mut s = server(&f.root);
    handshake(&mut s);
    let token = "mcp-loop-session";
    s.session = token.to_string();
    let mut id = 100;
    let mut step = |s: &mut McpServer, name: &str, args: serde_json::Value| -> serde_json::Value {
        id += 1;
        let v = call(s, id, name, args);
        assert_eq!(v["jsonrpc"], "2.0", "{v}");
        assert_eq!(v["id"], id, "{v}");
        assert!(v["result"].is_object(), "{v}");
        v
    };
    let v = step(&mut s, "init", serde_json::json!({}));
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(&mut s, "add", serde_json::json!({"kind": "a", "title": "Loop area"}));
    assert_eq!(text_of(&v), "A-01");
    let v = step(
        &mut s,
        "add",
        serde_json::json!({"kind": "g", "title": "Loop goal", "area": "A-01", "fitness_kind": "manual"}),
    );
    assert_eq!(text_of(&v), "G-01");
    let v = step(
        &mut s,
        "add",
        serde_json::json!({"kind": "w", "title": "Loop work", "type": "feature", "cynefin": "clear", "goals": "G-01"}),
    );
    assert_eq!(text_of(&v), "W-01");
    for field in ["ac", "hypothesis", "evidence_strategy"] {
        let v = step(
            &mut s,
            "field",
            serde_json::json!({"id": "W-01", "field": field, "op": "add", "value": "x"}),
        );
        assert!(!is_error(&v), "{field}: {}", text_of(&v));
    }
    let v = step(
        &mut s,
        "fitness",
        serde_json::json!({"id": "W-01", "goal": "G-01", "delta": 1}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(
        &mut s,
        "set",
        serde_json::json!({"id": "W-01", "key": "status", "value": "ready"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(
        &mut s,
        "set",
        serde_json::json!({"id": "W-01", "key": "status", "value": "progress"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let lock = std::fs::read_to_string(f.root.join(".grove").join("state.lock")).unwrap();
    assert!(lock.contains(token), "lock must hold the server session token:\n{lock}");
    let v = step(
        &mut s,
        "evidence",
        serde_json::json!({"id": "W-01", "text": "loop evidence"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(
        &mut s,
        "fitness",
        serde_json::json!({"id": "W-01", "goal": "G-01", "delta": 1}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(
        &mut s,
        "set",
        serde_json::json!({"id": "W-01", "key": "status", "value": "done"}),
    );
    assert!(!is_error(&v), "{}", text_of(&v));
    let v = step(&mut s, "check", serde_json::json!({}));
    assert!(!is_error(&v), "{}", text_of(&v));
    assert!(text_of(&v).contains("ok"), "{}", text_of(&v));
}

#[test]
fn resources_list_and_read_match_cli_output() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("resources");
    let mut s = server(&f.root);
    handshake(&mut s);
    init_project(&mut s, 10);
    call(&mut s, 11, "add", serde_json::json!({"kind": "a", "title": "Res area"}));
    call(
        &mut s,
        12,
        "add",
        serde_json::json!({"kind": "g", "title": "Res goal", "area": "A-01", "fitness_kind": "manual"}),
    );
    call(
        &mut s,
        13,
        "add",
        serde_json::json!({"kind": "w", "title": "Res work", "type": "feature", "cynefin": "clear", "goals": "G-01"}),
    );
    let v = rpc(&mut s, "{\"jsonrpc\":\"2.0\",\"id\":20,\"method\":\"resources/list\"}");
    let resources = v["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 7, "{resources:?}");
    let find_uri = |u: &str| {
        resources
            .iter()
            .find(|r| r["uri"] == u)
            .unwrap_or_else(|| panic!("missing {u}"))
            .clone()
    };
    let primer = find_uri("grove://skill");
    assert_eq!(primer["mimeType"], "text/markdown");
    let pw = find_uri("grove://packet/W-01");
    assert_eq!(pw["mimeType"], "text/markdown");
    assert_eq!(pw["name"], "W-01 Res work");
    let sw = find_uri("grove://show/W-01");
    assert_eq!(sw["mimeType"], "text/plain");
    find_uri("grove://packet/G-01");
    find_uri("grove://show/A-01");
    let root = f.root.to_string_lossy().into_owned();
    let read = |s: &mut McpServer, id: i64, uri: &str| -> serde_json::Value {
        let msg = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"resources/read\",\"params\":{{\"uri\":\"{uri}\"}}}}"
        );
        rpc(s, &msg)
    };
    let v = read(&mut s, 21, "grove://show/W-01");
    let cli = run_cli(&["show".to_string(), "W-01".to_string(), format!("--root={root}")]);
    assert_eq!(cli.code, 0);
    assert_eq!(v["result"]["contents"][0]["text"], cli.out);
    assert_eq!(v["result"]["contents"][0]["mimeType"], "text/plain");
    let v = read(&mut s, 22, "grove://packet/W-01");
    let cli = run_cli(&["packet".to_string(), "W-01".to_string(), format!("--root={root}")]);
    assert_eq!(cli.code, 0);
    assert_eq!(v["result"]["contents"][0]["text"], cli.out);
    assert_eq!(v["result"]["contents"][0]["mimeType"], "text/markdown");
    let v = read(&mut s, 23, "grove://bogus/W-01");
    assert_eq!(v["error"]["code"], -32602);
    let v = read(&mut s, 24, "grove://show/W-99");
    assert_eq!(v["error"]["code"], -32000);
    let v = read(&mut s, 25, "grove://skill");
    assert_eq!(v["result"]["contents"][0]["mimeType"], "text/markdown");
    let primer_text = v["result"]["contents"][0]["text"].as_str().unwrap();
    for keyword in [
        "dual-track",
        "Definition of Ready",
        "WIP_LIMIT",
        "chaotic",
        "distill",
        "evidence",
    ] {
        assert!(primer_text.contains(keyword), "primer misses {keyword}");
    }
}

#[test]
fn binary_smoke_ndjson_session() {
    let _g = ENV_LOCK.lock().unwrap();
    let f = fixture("binsmoke");
    let home = temp_dir("binsmoke-childhome");
    let root = f.root.to_string_lossy().into_owned();
    let r = run_cli(&["init".to_string(), format!("--root={root}")]);
    assert_eq!(r.code, 0, "{}", r.err);
    let script = [
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{},\"clientInfo\":{\"name\":\"smoke\",\"version\":\"0\"}}}",
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/list\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"add\",\"arguments\":{\"kind\":\"a\",\"title\":\"Smoke area\"}}}",
        "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"resources/list\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"tools/call\",\"params\":{\"name\":\"check\",\"arguments\":{}}}",
    ];
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_grove-mcp"))
        .arg(format!("--root={root}"))
        .arg("--session=smoke-token")
        .env("GROVE_HOME", &home)
        .env_remove("GROVE_PROJECT")
        .env_remove("GROVE_SESSION")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn grove-mcp");
    {
        let mut stdin = child.stdin.take().unwrap();
        use std::io::Write;
        for line in &script {
            writeln!(stdin, "{line}").unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 6, "{stdout}");
    let parsed: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).expect("each stdout line must parse as JSON"))
        .collect();
    assert_eq!(parsed[0]["result"]["serverInfo"]["name"], "grove-mcp");
    assert_eq!(parsed[1]["id"], 2);
    assert_eq!(parsed[2]["result"]["tools"].as_array().unwrap().len(), 38);
    let add_text = parsed[3]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(add_text.contains("A-01"), "{add_text}");
    let resources = parsed[4]["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 3, "{resources:?}");
    let check = &parsed[5];
    assert_eq!(check["result"]["isError"], false);
    assert!(
        check["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("ok")
    );
}
