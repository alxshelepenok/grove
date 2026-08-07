use crate::cli::{load, run_cli, CliCtx, COMMAND_NAMES, HELP, SESSION_MUTATE_COMMANDS};
use crate::json::{emit_jval, julia_num_repr, parse_json, JVal, Json};
use crate::ops::{OpResult, EXIT_OK};

pub const MCP_PROTOCOL_VERSIONS: [&str; 3] = ["2024-11-05", "2025-03-26", "2025-06-18"];
pub const MCP_SERVER_NAME: &str = "grove-mcp";
pub const MCP_SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ERR_PARSE: i64 = -32700;
pub const ERR_INVALID_REQUEST: i64 = -32600;
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERR_INVALID_PARAMS: i64 = -32602;
pub const ERR_SERVER_NOT_INITIALIZED: i64 = -32002;
pub const ERR_SERVER: i64 = -32000;

pub struct McpServer {
    pub root: String,
    pub session: String,
    pub protocol_version: String,
    pub initialized: bool,
}

impl McpServer {
    pub fn new(root: String, session: String) -> McpServer {
        McpServer {
            root,
            session,
            protocol_version: MCP_PROTOCOL_VERSIONS[MCP_PROTOCOL_VERSIONS.len() - 1].to_string(),
            initialized: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum PropType {
    Str,
    Int,
    Bool,
}

struct Prop {
    key: &'static str,
    cli: &'static str,
    typ: PropType,
    desc: &'static str,
    choices: &'static [&'static str],
}

struct ToolSpec {
    cmd: &'static str,
    props: &'static [Prop],
    required: &'static [&'static str],
}

const CYNEFIN: [&str; 4] = ["clear", "complicated", "complex", "chaotic"];
const KINDS: [&str; 8] = ["g", "w", "d", "q", "b", "t", "y", "a"];
const LABELS: [&str; 9] = [
    "blocks",
    "implements",
    "asks",
    "tests",
    "targets",
    "produces",
    "causes",
    "supersedes",
    "distills",
];

const TOOL_SPECS: &[ToolSpec] = &[
    ToolSpec {
        cmd: "init",
        props: &[
            Prop { key: "id_stride", cli: "id-stride", typ: PropType::Int, desc: "additive gap between successive numeric id suffixes", choices: &[] },
            Prop { key: "id_offset", cli: "id-offset", typ: PropType::Int, desc: "first suffix when a family allocator is empty", choices: &[] },
            Prop { key: "id_width", cli: "id-width", typ: PropType::Int, desc: "minimum digit padding for new ids", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "add",
        props: &[
            Prop { key: "kind", cli: "", typ: PropType::Str, desc: "node kind", choices: &KINDS },
            Prop { key: "title", cli: "title", typ: PropType::Str, desc: "node title", choices: &[] },
            Prop { key: "area", cli: "area", typ: PropType::Str, desc: "owning area A-NN (required for kind g)", choices: &[] },
            Prop { key: "type", cli: "type", typ: PropType::Str, desc: "work item type (w)", choices: &["feature", "refactor", "bug", "spike"] },
            Prop { key: "cynefin", cli: "cynefin", typ: PropType::Str, desc: "cynefin class (w, q, b)", choices: &CYNEFIN },
            Prop { key: "goals", cli: "goals", typ: PropType::Str, desc: "comma-separated goal ids (w)", choices: &[] },
            Prop { key: "theme", cli: "theme", typ: PropType::Str, desc: "theme id T-NN (w)", choices: &[] },
            Prop { key: "surface", cli: "surface", typ: PropType::Str, desc: "comma-separated paths (w, y, a)", choices: &[] },
            Prop { key: "from", cli: "from", typ: PropType::Str, desc: "comma-separated provenance ids (y)", choices: &[] },
            Prop { key: "tags", cli: "tags", typ: PropType::Str, desc: "comma-separated glossary terms (y)", choices: &[] },
            Prop { key: "why", cli: "why", typ: PropType::Str, desc: "anchor rationale (y; xor surface)", choices: &[] },
            Prop { key: "status", cli: "status", typ: PropType::Str, desc: "initial status override", choices: &[] },
            Prop { key: "fitness", cli: "fitness", typ: PropType::Str, desc: "legacy fitness label (g)", choices: &[] },
            Prop { key: "fitness_kind", cli: "fitness-kind", typ: PropType::Str, desc: "structured fitness kind (g)", choices: &["count", "ratio", "boolean", "metric", "manual"] },
            Prop { key: "fitness_target", cli: "fitness-target", typ: PropType::Str, desc: "structured fitness target (g)", choices: &[] },
            Prop { key: "supersedes", cli: "supersedes", typ: PropType::Str, desc: "comma-separated superseded decision ids (d)", choices: &[] },
            Prop { key: "targets", cli: "targets", typ: PropType::Str, desc: "comma-separated target ids (q, b)", choices: &[] },
            Prop { key: "tests", cli: "tests", typ: PropType::Str, desc: "comma-separated question ids (b)", choices: &[] },
        ],
        required: &["kind", "title"],
    },
    ToolSpec {
        cmd: "set",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "node id", choices: &[] },
            Prop { key: "key", cli: "", typ: PropType::Str, desc: "attribute key: status|cynefin|type|title|fitness|fitness_kind|area|requires_coverage", choices: &[] },
            Prop { key: "value", cli: "", typ: PropType::Str, desc: "new value", choices: &[] },
        ],
        required: &["id", "key", "value"],
    },
    ToolSpec {
        cmd: "field",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "node id", choices: &[] },
            Prop { key: "field", cli: "", typ: PropType::Str, desc: "field name (ac, hypothesis, evidence_strategy, evidence, outcome, goals, surface, ...)", choices: &[] },
            Prop { key: "op", cli: "", typ: PropType::Str, desc: "field operation", choices: &["add", "rm", "clear"] },
            Prop { key: "value", cli: "", typ: PropType::Str, desc: "entry text (add) or 1-based index (rm)", choices: &[] },
        ],
        required: &["id", "field", "op"],
    },
    ToolSpec {
        cmd: "link",
        props: &[
            Prop { key: "from", cli: "", typ: PropType::Str, desc: "source node id", choices: &[] },
            Prop { key: "label", cli: "", typ: PropType::Str, desc: "edge label", choices: &LABELS },
            Prop { key: "to", cli: "", typ: PropType::Str, desc: "target node id", choices: &[] },
        ],
        required: &["from", "label", "to"],
    },
    ToolSpec {
        cmd: "unlink",
        props: &[
            Prop { key: "from", cli: "", typ: PropType::Str, desc: "source node id", choices: &[] },
            Prop { key: "label", cli: "", typ: PropType::Str, desc: "edge label", choices: &LABELS },
            Prop { key: "to", cli: "", typ: PropType::Str, desc: "target node id", choices: &[] },
        ],
        required: &["from", "label", "to"],
    },
    ToolSpec {
        cmd: "evidence",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "text", cli: "", typ: PropType::Str, desc: "evidence line to append", choices: &[] },
        ],
        required: &["id", "text"],
    },
    ToolSpec {
        cmd: "fitness",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "goal", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
            Prop { key: "delta", cli: "", typ: PropType::Int, desc: "per-goal delta (+N, 0, or -N)", choices: &[] },
        ],
        required: &["id", "goal", "delta"],
    },
    ToolSpec {
        cmd: "archive",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "distill",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
            Prop { key: "null", cli: "null", typ: PropType::Bool, desc: "write a null-distill attestation", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec { cmd: "render", props: &[], required: &[] },
    ToolSpec {
        cmd: "repair",
        props: &[
            Prop { key: "confirm", cli: "confirm", typ: PropType::Bool, desc: "accept current lock contents", choices: &[] },
        ],
        required: &["confirm"],
    },
    ToolSpec { cmd: "ready", props: &[], required: &[] },
    ToolSpec { cmd: "next", props: &[], required: &[] },
    ToolSpec {
        cmd: "packet",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "cone", cli: "cone", typ: PropType::Bool, desc: "append multi-hop structural context on blocks", choices: &[] },
            Prop { key: "cone_depth", cli: "cone-depth", typ: PropType::Int, desc: "cone BFS hops (default 4)", choices: &[] },
            Prop { key: "cone_max", cli: "cone-max", typ: PropType::Int, desc: "cone node cap (default 50)", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "deps",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "node id", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "impact",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "node id", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec { cmd: "path", props: &[], required: &[] },
    ToolSpec { cmd: "triage", props: &[], required: &[] },
    ToolSpec {
        cmd: "dor",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "show",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "node id", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "list",
        props: &[
            Prop { key: "kind", cli: "", typ: PropType::Str, desc: "node kind", choices: &KINDS },
            Prop { key: "status", cli: "status", typ: PropType::Str, desc: "status filter", choices: &[] },
            Prop { key: "cynefin", cli: "cynefin", typ: PropType::Str, desc: "cynefin filter", choices: &CYNEFIN },
        ],
        required: &["kind"],
    },
    ToolSpec { cmd: "graph", props: &[], required: &[] },
    ToolSpec { cmd: "check", props: &[], required: &[] },
    ToolSpec { cmd: "status", props: &[], required: &[] },
    ToolSpec { cmd: "stats", props: &[], required: &[] },
    ToolSpec {
        cmd: "diff",
        props: &[
            Prop { key: "since", cli: "since", typ: PropType::Str, desc: "git ref to diff against (default HEAD)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "log",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "optional node id filter", choices: &[] },
            Prop { key: "limit", cli: "limit", typ: PropType::Int, desc: "row cap (default 200; 0 = unlimited)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "renumber",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "current node id", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "new id", choices: &[] },
        ],
        required: &["id", "to"],
    },
    ToolSpec {
        cmd: "resume",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "handoff",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "new owner session token", choices: &[] },
        ],
        required: &["id", "to"],
    },
    ToolSpec {
        cmd: "revert",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "undo",
        props: &[
            Prop { key: "steps", cli: "steps", typ: PropType::Int, desc: "number of mutations to revert (default 1)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "gate",
        props: &[
            Prop { key: "theta", cli: "theta", typ: PropType::Int, desc: "surface overflow threshold (default 0)", choices: &[] },
            Prop { key: "n", cli: "n", typ: PropType::Int, desc: "done-count threshold (default 5)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "revalidate",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "discovery id Y-NN", choices: &[] },
            Prop { key: "surface", cli: "surface", typ: PropType::Str, desc: "comma-separated fresh anchor paths", choices: &[] },
            Prop { key: "from", cli: "from", typ: PropType::Str, desc: "comma-separated provenance ids", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "glossary",
        props: &[
            Prop { key: "old", cli: "", typ: PropType::Str, desc: "existing glossary term", choices: &[] },
            Prop { key: "new", cli: "", typ: PropType::Str, desc: "replacement term", choices: &[] },
        ],
        required: &["old", "new"],
    },
    ToolSpec { cmd: "projects", props: &[], required: &[] },
    ToolSpec {
        cmd: "promote",
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "discovery id Y-NN", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "target project (directory or registry name)", choices: &[] },
        ],
        required: &["id", "to"],
    },
];

fn spec_for(cmd: &str) -> Option<&'static ToolSpec> {
    TOOL_SPECS.iter().find(|s| s.cmd == cmd)
}

fn jstr(s: &str) -> String {
    emit_jval(&JVal::Str(s.to_string()))
}

fn emit_id(id: &Json) -> String {
    match id {
        Json::Int(i) => i.to_string(),
        Json::Float(f) => julia_num_repr(*f),
        Json::Str(s) => jstr(s),
        _ => "null".to_string(),
    }
}

fn result_response(id: &Json, result: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{}}}",
        emit_id(id),
        result
    )
}

fn error_response(id: &Json, code: i64, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":{}}}}}",
        emit_id(id),
        code,
        jstr(message)
    )
}

fn help_description(cmd: &str) -> String {
    for raw in HELP.lines() {
        let line = raw.trim_start();
        if !line.starts_with(cmd) {
            continue;
        }
        let rest = &line[cmd.len()..];
        if !rest.is_empty() && !rest.starts_with(' ') {
            continue;
        }
        let b = line.as_bytes();
        let mut i = 0;
        let mut last_gap_end = None;
        while i + 1 < b.len() {
            if b[i] == b' ' && b[i + 1] == b' ' {
                let mut j = i;
                while j < b.len() && b[j] == b' ' {
                    j += 1;
                }
                last_gap_end = Some(j);
                i = j;
            } else {
                i += 1;
            }
        }
        if let Some(s) = last_gap_end {
            let d = line[s..].trim();
            if !d.is_empty() {
                return d.to_string();
            }
        }
        let d = line.trim();
        if !d.is_empty() {
            return d.to_string();
        }
    }
    format!("grove {cmd}")
}

fn tool_description(cmd: &str) -> String {
    help_description(cmd)
}

fn prop_schema_json(prop: &Prop) -> String {
    let typ = match prop.typ {
        PropType::Str => "string",
        PropType::Int => "integer",
        PropType::Bool => "boolean",
    };
    let mut s = format!(
        "{{\"type\":\"{}\",\"description\":{}}}",
        typ,
        jstr(prop.desc)
    );
    if !prop.choices.is_empty() {
        let vals: Vec<String> = prop.choices.iter().map(|c| jstr(c)).collect();
        s.insert_str(s.len() - 1, &format!(",\"enum\":[{}]", vals.join(",")));
    }
    s
}

fn tool_json(spec: &ToolSpec) -> String {
    let mut props = String::new();
    for (i, p) in spec.props.iter().enumerate() {
        if i > 0 {
            props.push(',');
        }
        props.push_str(&format!("{}:{}", jstr(p.key), prop_schema_json(p)));
    }
    let required: Vec<String> = spec.required.iter().map(|r| jstr(r)).collect();
    format!(
        "{{\"name\":{},\"description\":{},\"inputSchema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}],\"additionalProperties\":false}}}}",
        jstr(spec.cmd),
        jstr(&tool_description(spec.cmd)),
        props,
        required.join(",")
    )
}

fn tools_list_json() -> String {
    let tools: Vec<String> = TOOL_SPECS.iter().map(tool_json).collect();
    format!("{{\"tools\":[{}]}}", tools.join(","))
}

fn coerce_value(v: &Json, typ: PropType) -> Result<String, String> {
    match v {
        Json::Str(s) => {
            if typ == PropType::Bool && s != "true" && s != "false" {
                return Err(format!("expected boolean, got string {}", jstr(s)));
            }
            Ok(s.clone())
        }
        Json::Int(i) => Ok(i.to_string()),
        Json::Float(f) => Ok(julia_num_repr(*f)),
        Json::Bool(b) => match typ {
            PropType::Int => Err("expected integer, got boolean".to_string()),
            _ => Ok(if *b { "true" } else { "false" }.to_string()),
        },
        _ => Err("expected string, number, or boolean".to_string()),
    }
}

fn validate_args(spec: &ToolSpec, args: &[(String, Json)]) -> Result<(), String> {
    let mut unknown: Vec<&str> = Vec::new();
    for (k, _) in args {
        if !spec.props.iter().any(|p| p.key == k) {
            unknown.push(k);
        }
    }
    if !unknown.is_empty() {
        let allowed: Vec<&str> = spec.props.iter().map(|p| p.key).collect();
        return Err(format!(
            "unknown argument(s): {}; allowed: {}",
            unknown.join(", "),
            allowed.join(", ")
        ));
    }
    let missing: Vec<&str> = spec
        .required
        .iter()
        .filter(|r| !args.iter().any(|(k, _)| k == *r))
        .copied()
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "missing required argument(s): {}",
            missing.join(", ")
        ));
    }
    Ok(())
}

fn build_argv(
    server: &McpServer,
    spec: &ToolSpec,
    args: &[(String, Json)],
) -> Result<Vec<String>, String> {
    let mut positional: Vec<String> = Vec::new();
    let mut flags: Vec<String> = Vec::new();
    for prop in spec.props {
        let Some((_, v)) = args.iter().rev().find(|(k, _)| k == prop.key) else {
            continue;
        };
        let s = coerce_value(v, prop.typ).map_err(|m| format!("argument `{}`: {m}", prop.key))?;
        if prop.cli.is_empty() {
            positional.push(s);
        } else if prop.typ == PropType::Bool {
            if s == "true" {
                flags.push(format!("--{}", prop.cli));
            }
        } else {
            flags.push(format!("--{}={s}", prop.cli));
        }
    }
    let mut argv = vec![spec.cmd.to_string()];
    if spec.cmd == "glossary" {
        argv.push("rename".to_string());
    }
    if spec.cmd == "set" {
        argv.push(positional[0].clone());
        argv.push(format!("{}={}", positional[1], positional[2]));
    } else {
        argv.extend(positional);
    }
    argv.extend(flags);
    argv.push(format!("--root={}", server.root));
    if SESSION_MUTATE_COMMANDS.contains(&spec.cmd) {
        argv.push(format!("--session={}", server.session));
    }
    Ok(argv)
}

fn tool_result_json(r: &OpResult) -> String {
    let mut text = r.out.trim_end().to_string();
    let errt = r.err.trim_end();
    if !errt.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("--- stderr ---\n");
        text.push_str(errt);
    }
    if r.code != EXIT_OK {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&format!("(grove exit code: {})", r.code));
    }
    format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":{}}}],\"isError\":{}}}",
        jstr(&text),
        r.code != EXIT_OK
    )
}

fn tool_error_json(message: &str) -> String {
    format!(
        "{{\"content\":[{{\"type\":\"text\",\"text\":{}}}],\"isError\":true}}",
        jstr(message)
    )
}

fn resources_list_json(server: &McpServer) -> String {
    let ctx = CliCtx::new(server.root.clone());
    let mut items: Vec<String> = vec![format!(
        "{{\"uri\":{},\"name\":{},\"mimeType\":{}}}",
        jstr("grove://skill"),
        jstr("grove protocol primer"),
        jstr("text/markdown")
    )];
    if let Ok(st) = load(&ctx, true) {
        for (nid, n) in &st.nodes {
            for (cmd, mt) in [("packet", "text/markdown"), ("show", "text/plain")] {
                items.push(format!(
                    "{{\"uri\":{},\"name\":{},\"mimeType\":{}}}",
                    jstr(&format!("grove://{cmd}/{nid}")),
                    jstr(&format!("{nid} {}", n.title)),
                    jstr(mt)
                ));
            }
        }
    }
    format!("{{\"resources\":[{}]}}", items.join(","))
}

const SKILL_PRIMER: &str = include_str!("../assets/skill-primer.md");

fn resource_read(server: &McpServer, uri: &str) -> Result<String, (i64, String)> {
    if uri == "grove://skill" {
        return Ok(format!(
            "{{\"contents\":[{{\"uri\":{},\"mimeType\":{},\"text\":{}}}]}}",
            jstr(uri),
            jstr("text/markdown"),
            jstr(SKILL_PRIMER)
        ));
    }
    let Some(path) = uri.strip_prefix("grove://") else {
        return Err((
            ERR_INVALID_PARAMS,
            format!("unsupported resource uri: {uri}"),
        ));
    };
    let Some((cmd, nid)) = path.split_once('/') else {
        return Err((ERR_INVALID_PARAMS, format!("malformed resource uri: {uri}")));
    };
    if nid.is_empty() || (cmd != "packet" && cmd != "show") {
        return Err((ERR_INVALID_PARAMS, format!("malformed resource uri: {uri}")));
    }
    let argv = vec![
        cmd.to_string(),
        nid.to_string(),
        format!("--root={}", server.root),
    ];
    let r = run_cli(&argv);
    if r.code != EXIT_OK {
        let msg = r.err.trim_end();
        let msg = if msg.is_empty() { r.out.trim_end() } else { msg };
        return Err((
            ERR_SERVER,
            format!("grove {cmd} {nid} failed (exit {}): {msg}", r.code),
        ));
    }
    let mt = if cmd == "packet" {
        "text/markdown"
    } else {
        "text/plain"
    };
    Ok(format!(
        "{{\"contents\":[{{\"uri\":{},\"mimeType\":{},\"text\":{}}}]}}",
        jstr(uri),
        jstr(mt),
        jstr(&r.out)
    ))
}

fn negotiate_version(params: Option<&Json>) -> String {
    let requested = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str());
    match requested {
        Some(v) if MCP_PROTOCOL_VERSIONS.contains(&v) => v.to_string(),
        _ => MCP_PROTOCOL_VERSIONS[MCP_PROTOCOL_VERSIONS.len() - 1].to_string(),
    }
}

fn initialize_result_json(version: &str) -> String {
    format!(
        "{{\"protocolVersion\":{},\"capabilities\":{{\"tools\":{{\"listChanged\":false}},\"resources\":{{\"listChanged\":false,\"subscribe\":false}}}},\"serverInfo\":{{\"name\":{},\"version\":{}}}}}",
        jstr(version),
        jstr(MCP_SERVER_NAME),
        jstr(MCP_SERVER_VERSION)
    )
}

pub fn handle_message(server: &mut McpServer, line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let msg = match parse_json(trimmed) {
        Ok(v) => v,
        Err(e) => {
            return Some(error_response(
                &Json::Null,
                ERR_PARSE,
                &format!("parse error: {e}"),
            ))
        }
    };
    if matches!(msg, Json::Arr(_)) {
        return Some(error_response(
            &Json::Null,
            ERR_INVALID_REQUEST,
            "invalid request: JSON-RPC batches are not used by MCP",
        ));
    }
    if !matches!(msg, Json::Obj(_)) {
        return Some(error_response(
            &Json::Null,
            ERR_INVALID_REQUEST,
            "invalid request: expected a JSON-RPC object",
        ));
    }
    let id = msg.get("id").cloned();
    let method = match msg.get("method").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => {
            return Some(error_response(
                id.as_ref().unwrap_or(&Json::Null),
                ERR_INVALID_REQUEST,
                "invalid request: missing `method`",
            ))
        }
    };
    let params = msg.get("params").cloned();
    let Some(id) = id else {
        if method == "notifications/initialized" {
            server.initialized = true;
        }
        return None;
    };
    match method.as_str() {
        "initialize" => {
            let version = negotiate_version(params.as_ref());
            server.protocol_version = version.clone();
            Some(result_response(&id, &initialize_result_json(&version)))
        }
        "notifications/initialized" => {
            server.initialized = true;
            Some(result_response(&id, "{}"))
        }
        "ping" => Some(result_response(&id, "{}")),
        "tools/list" => {
            if !server.initialized {
                return Some(error_response(
                    &id,
                    ERR_SERVER_NOT_INITIALIZED,
                    "server not initialized: send `initialize` then `notifications/initialized`",
                ));
            }
            Some(result_response(&id, &tools_list_json()))
        }
        "tools/call" => {
            if !server.initialized {
                return Some(error_response(
                    &id,
                    ERR_SERVER_NOT_INITIALIZED,
                    "server not initialized: send `initialize` then `notifications/initialized`",
                ));
            }
            Some(handle_tools_call(server, &id, params.as_ref()))
        }
        "resources/list" => {
            if !server.initialized {
                return Some(error_response(
                    &id,
                    ERR_SERVER_NOT_INITIALIZED,
                    "server not initialized: send `initialize` then `notifications/initialized`",
                ));
            }
            Some(result_response(&id, &resources_list_json(server)))
        }
        "resources/read" => {
            if !server.initialized {
                return Some(error_response(
                    &id,
                    ERR_SERVER_NOT_INITIALIZED,
                    "server not initialized: send `initialize` then `notifications/initialized`",
                ));
            }
            let uri = params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            if uri.is_empty() {
                return Some(error_response(
                    &id,
                    ERR_INVALID_PARAMS,
                    "resources/read: missing params.uri",
                ));
            }
            match resource_read(server, &uri) {
                Ok(result) => Some(result_response(&id, &result)),
                Err((code, msg)) => Some(error_response(&id, code, &msg)),
            }
        }
        _ => Some(error_response(
            &id,
            ERR_METHOD_NOT_FOUND,
            &format!("method not found: {method}"),
        )),
    }
}

fn handle_tools_call(server: &McpServer, id: &Json, params: Option<&Json>) -> String {
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("");
    if name.is_empty() {
        return error_response(id, ERR_INVALID_PARAMS, "tools/call: missing params.name");
    }
    let Some(spec) = spec_for(name) else {
        let known: Vec<&str> = COMMAND_NAMES.iter().copied().collect();
        return error_response(
            id,
            ERR_INVALID_PARAMS,
            &format!("unknown tool: {name}; known tools: {}", known.join(", ")),
        );
    };
    let empty: Vec<(String, Json)> = Vec::new();
    let args = match params.and_then(|p| p.get("arguments")) {
        None | Some(Json::Null) => &empty,
        Some(Json::Obj(o)) => o,
        Some(_) => {
            return error_response(id, ERR_INVALID_PARAMS, "tools/call: arguments must be an object")
        }
    };
    if let Err(m) = validate_args(spec, args) {
        return result_response(id, &tool_error_json(&m));
    }
    let argv = match build_argv(server, spec, args) {
        Ok(a) => a,
        Err(m) => return result_response(id, &tool_error_json(&m)),
    };
    let r = run_cli(&argv);
    result_response(id, &tool_result_json(&r))
}
