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

#[derive(Clone, Copy)]
struct Annotations {
    read_only: bool,
    destructive: bool,
    idempotent: bool,
}

const RO: Annotations = Annotations {
    read_only: true,
    destructive: false,
    idempotent: true,
};
const MUT: Annotations = Annotations {
    read_only: false,
    destructive: false,
    idempotent: false,
};
const DES: Annotations = Annotations {
    read_only: false,
    destructive: true,
    idempotent: false,
};
const IDEM: Annotations = Annotations {
    read_only: false,
    destructive: false,
    idempotent: true,
};

struct ToolSpec {
    cmd: &'static str,
    title: &'static str,
    desc: &'static str,
    ann: Annotations,
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
        title: "Initialise project",
        desc: "Initialise a grove project: create .grove/state.lock, index.md and glossary.md under the project root. Run once per project; every other tool fails until the lock exists, and a second run refuses rather than overwrites. id-stride, id-offset and id-width tune numeric id allocation for new nodes. Returns the initialised .grove path.",
        ann: MUT,
        props: &[
            Prop { key: "id_stride", cli: "id-stride", typ: PropType::Int, desc: "additive gap between successive numeric id suffixes", choices: &[] },
            Prop { key: "id_offset", cli: "id-offset", typ: PropType::Int, desc: "first suffix when a family allocator is empty", choices: &[] },
            Prop { key: "id_width", cli: "id-width", typ: PropType::Int, desc: "minimum digit padding for new ids", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "add",
        title: "Add node",
        desc: "Create one node of the given kind and return its assigned id (for example W-12); nothing else is printed. Only kind and title are always required; other fields apply per kind (goals and theme for w, surface or why for y, fitness for g, supersedes for d, targets for q and b) and invalid combinations are rejected on write. To modify an existing node use set or field; to connect nodes use link.",
        ann: MUT,
        props: &[
            Prop { key: "kind", cli: "", typ: PropType::Str, desc: "node kind", choices: &KINDS },
            Prop { key: "title", cli: "title", typ: PropType::Str, desc: "short node title, stamped verbatim", choices: &[] },
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
            Prop { key: "fitness", cli: "fitness", typ: PropType::Str, desc: "retired legacy fitness label (g; writes are rejected, use fitness_kind + fitness_target)", choices: &[] },
            Prop { key: "fitness_kind", cli: "fitness-kind", typ: PropType::Str, desc: "structured fitness kind (g; some fitness value is required for kind g)", choices: &["count", "ratio", "boolean", "metric", "manual"] },
            Prop { key: "fitness_target", cli: "fitness-target", typ: PropType::Str, desc: "structured fitness target (g)", choices: &[] },
            Prop { key: "supersedes", cli: "supersedes", typ: PropType::Str, desc: "comma-separated superseded decision ids (d)", choices: &[] },
            Prop { key: "targets", cli: "targets", typ: PropType::Str, desc: "comma-separated target ids (q, b)", choices: &[] },
            Prop { key: "tests", cli: "tests", typ: PropType::Str, desc: "comma-separated question ids (b)", choices: &[] },
        ],
        required: &["kind", "title"],
    },
    ToolSpec {
        cmd: "set",
        title: "Set node attribute",
        desc: "Apply one guarded transition of a scalar attribute on a node: status, cynefin, type, title, fitness_kind, fitness_target, area or requires_coverage. Illegal transitions (a status skip, a work item whose DoR is not met) are rejected with the reason and a pointer to dor; success is silent. For list-valued fields such as ac or evidence use field.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "target node id, e.g. W-12, G-03 or D-40", choices: &[] },
            Prop { key: "key", cli: "", typ: PropType::Str, desc: "attribute key: status|cynefin|type|title|fitness_kind|fitness_target|area|requires_coverage", choices: &[] },
            Prop { key: "value", cli: "", typ: PropType::Str, desc: "new value for the attribute key", choices: &[] },
        ],
        required: &["id", "key", "value"],
    },
    ToolSpec {
        cmd: "field",
        title: "Edit node field",
        desc: "Edit one list-valued field of a node (ac, hypothesis, evidence_strategy, evidence, outcome, goals, surface, and so on): op add appends value, rm removes the entry at the 1-based index given in value, clear empties the field. Success is silent. For scalar attributes use set; for done-work proof on a work item prefer evidence.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "target node id, e.g. W-12, G-03 or D-40", choices: &[] },
            Prop { key: "field", cli: "", typ: PropType::Str, desc: "field name (ac, hypothesis, evidence_strategy, evidence, outcome, goals, surface, ...)", choices: &[] },
            Prop { key: "op", cli: "", typ: PropType::Str, desc: "field operation", choices: &["add", "rm", "clear"] },
            Prop { key: "value", cli: "", typ: PropType::Str, desc: "entry text (add) or 1-based index (rm)", choices: &[] },
        ],
        required: &["id", "field", "op"],
    },
    ToolSpec {
        cmd: "link",
        title: "Link nodes",
        desc: "Create a directed edge from one node to another with a label: blocks, implements, asks, tests, targets, produces, causes, supersedes or distills. Edges feed the ready, next, path, deps and impact analytics and are checked by invariants, so invalid combinations are rejected; success is silent. To remove an edge use unlink.",
        ann: MUT,
        props: &[
            Prop { key: "from", cli: "", typ: PropType::Str, desc: "source node id", choices: &[] },
            Prop { key: "label", cli: "", typ: PropType::Str, desc: "edge label", choices: &LABELS },
            Prop { key: "to", cli: "", typ: PropType::Str, desc: "target node id", choices: &[] },
        ],
        required: &["from", "label", "to"],
    },
    ToolSpec {
        cmd: "unlink",
        title: "Unlink nodes",
        desc: "Remove one directed edge identified by its from node, label and to node. Refuses when the removal would break graph invariants; success is silent. To create an edge use link.",
        ann: MUT,
        props: &[
            Prop { key: "from", cli: "", typ: PropType::Str, desc: "source node id", choices: &[] },
            Prop { key: "label", cli: "", typ: PropType::Str, desc: "edge label", choices: &LABELS },
            Prop { key: "to", cli: "", typ: PropType::Str, desc: "target node id", choices: &[] },
        ],
        required: &["from", "label", "to"],
    },
    ToolSpec {
        cmd: "evidence",
        title: "Append evidence",
        desc: "Append one evidence line to a work item's evidence field: the canonical way to record done-work proof, which dor, gate and distill read. Success is silent. Equivalent to field with field=evidence and op=add, but self-documenting.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "text", cli: "", typ: PropType::Str, desc: "evidence line to append", choices: &[] },
        ],
        required: &["id", "text"],
    },
    ToolSpec {
        cmd: "fitness",
        title: "Set fitness delta",
        desc: "Record how much one work item contributes toward one goal: the per-goal delta, where +N advances the goal, 0 is neutral and -N regresses it. Typically set at creation and re-set when scope changes (the last write wins); the deltas surface in dor breakdowns and execution packets. Success is silent; an unknown work item or goal id fails with `missing: <id>` (exit 5). Use set for scalar attributes and field for list fields; this tool only edits the delta.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "goal", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
            Prop { key: "delta", cli: "", typ: PropType::Int, desc: "per-goal delta (+N, 0, or -N)", choices: &[] },
        ],
        required: &["id", "goal", "delta"],
    },
    ToolSpec {
        cmd: "archive",
        title: "Archive goal",
        desc: "Archive a verified goal together with its exclusive subgraph (w, d, q, b, t) by setting their archived flag; archived nodes stay in the lock but leave active views, so this is a soft removal, not a deletion, and hard to reverse. Requires distillation first: a linked Discovery or a null-distill attestation from distill; gate reports whether the goal is due.",
        ann: DES,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "distill",
        title: "Distillation worksheet",
        desc: "Print the distillation worksheet for a verified goal: what its subgraph produced and what should survive in Discoveries before archive. Refuses with the current status when the goal is not verified. Read-only unless null=true, which appends a null-distill attestation to the audit journal; state.lock itself is not mutated. Run this before archive when no Discovery captures the goal.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "goal id G-NN", choices: &[] },
            Prop { key: "null", cli: "null", typ: PropType::Bool, desc: "write a null-distill attestation", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "render",
        title: "Render index",
        desc: "Regenerate index.md from the current state.lock. Idempotent, safe to re-run and silent on success; most mutating tools already auto-render, so use it after out-of-band edits or when index.md looks stale.",
        ann: IDEM,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "repair",
        title: "Repair lock checksum",
        desc: "Accept whatever is currently in state.lock and recompute its checksum; confirm=true is required. Last resort when check reports a checksum mismatch after a manual edit or merge: it blesses the file as-is, so inspect the contents first.",
        ann: IDEM,
        props: &[
            Prop { key: "confirm", cli: "confirm", typ: PropType::Bool, desc: "accept current lock contents", choices: &[] },
        ],
        required: &["confirm"],
    },
    ToolSpec {
        cmd: "ready",
        title: "List ready work items",
        desc: "List work items in status ready, one line per item (id, title, and a [crit] marker when the item sits on the critical path), critical-path first. Use this for the whole queue; use next when you want a single recommendation, or packet for one item's full context.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "next",
        title: "Propose next work item",
        desc: "Propose the single next work item to execute and return its full execution packet: the same markdown bundle packet produces, prefixed by the skill banner. The start-of-session default; ready shows the whole queue instead, and packet fetches an arbitrary work item.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "packet",
        title: "Execution packet",
        desc: "Full execution packet for one work item as markdown: record, goals and fitness contribution, hypotheses, linked decisions, blocking questions and the outcome of every blocker. Fetch this before starting or resuming any work item; next returns the same bundle only for its single proposal, and show prints the bare record without execution context. cone=true appends multi-hop structural context over blocks edges (cone-depth default 4, cone-max default 50 nodes); deps returns just the blocker ids. An unknown id fails with `not found` (exit 5).",
        ann: RO,
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
        title: "Upstream blockers",
        desc: "Transitive predecessors over blocks edges: every node that must finish before the given node can start, returned as one id per line in dependency order. impact is the downstream mirror; path shows the whole critical chain.",
        ann: RO,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "target node id, e.g. W-12, G-03 or D-40", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "impact",
        title: "Downstream impact",
        desc: "Transitive successors over blocks edges: every node the given node blocks from starting, returned as one id per line. deps is the upstream mirror.",
        ann: RO,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "target node id, e.g. W-12, G-03 or D-40", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "path",
        title: "Critical path",
        desc: "Print the critical path: the longest chain of unfinished blocks edges, as one id per line in chain order. Use it to see the current bottleneck end to end; deps and impact cover a single node's neighborhood.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "triage",
        title: "Triage discovery need",
        desc: "Rank open work items by discovery need in a table (coverage, chi-square, fragility, suggestion). Read-only advisory input for deciding which work item needs a Discovery next; gate is the pass-or-fail check on the project.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "dor",
        title: "DoR breakdown",
        desc: "Definition-of-Ready breakdown for one work item: one line per conjunct with its current pass or fail and a final result line. Run it before resume to confirm a work item is actually startable; set status=progress consults the same conjuncts.",
        ann: RO,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "show",
        title: "Show node record",
        desc: "Dump one node's full record as plain text: kind, status, timestamps, every populated field and incident edges. An unknown id fails with no output (exit code 5). list filters many nodes by kind or status; packet wraps a work item's record in execution context.",
        ann: RO,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "target node id, e.g. W-12, G-03 or D-40", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "list",
        title: "List nodes",
        desc: "List nodes filtered by required kind (g, w, d, q, b, t, y or a), one tab-separated line per node: id, status, title. Optional status and cynefin filters (cynefin = the clear/complicated/complex/chaotic complexity class) narrow the set; when nothing matches, nothing is printed. show dumps one record in full; status summarises the whole project.",
        ann: RO,
        props: &[
            Prop { key: "kind", cli: "", typ: PropType::Str, desc: "node kind", choices: &KINDS },
            Prop { key: "status", cli: "status", typ: PropType::Str, desc: "status filter", choices: &[] },
            Prop { key: "cynefin", cli: "cynefin", typ: PropType::Str, desc: "cynefin filter", choices: &CYNEFIN },
        ],
        required: &["kind"],
    },
    ToolSpec {
        cmd: "graph",
        title: "Graph as mermaid",
        desc: "Render the whole graph as a fenced mermaid flowchart block: one node per id with status classes and labelled edges. Read-only; deps and impact give a single node's neighborhood.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "check",
        title: "Check invariants",
        desc: "Verify the state.lock checksum and all structural invariants. Returns ok on success, otherwise the first failing invariant; on a checksum mismatch after a deliberate edit see repair.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "status",
        title: "Status summary",
        desc: "One-screen markdown project summary: work in progress, alignment triggers and invariant notes, prefixed by the embedded-skill banner. stats is the historical counterpart; check is pass-or-fail on invariants.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "stats",
        title: "Workflow statistics",
        desc: "Read-only telemetry computed from the journal and the lock, in metric sections: record and mutation counts, cycle time, DoR first-pass rate, bets, discovery, undo and surprise. status summarises current state instead.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "diff",
        title: "Structural diff",
        desc: "Structural diff of nodes and edges against a git ref (since, default HEAD): grove structures, not text hunks. Requires the project root to be a git repository; outside one it fails with a `not a git repository` error naming the root. log shows who changed what and when.",
        ann: RO,
        props: &[
            Prop { key: "since", cli: "since", typ: PropType::Str, desc: "git ref to diff against (default HEAD)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "log",
        title: "Journal timeline",
        desc: "Timeline of node and edge timestamps plus raw journal records, newest first, one line each; optional id filter and limit (default 200 rows, 0 for unlimited). stats aggregates the same history into metrics.",
        ann: RO,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "optional node id filter", choices: &[] },
            Prop { key: "limit", cli: "limit", typ: PropType::Int, desc: "row cap (default 200; 0 = unlimited)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "renumber",
        title: "Renumber node",
        desc: "Change a node's id and rewrite every reference to it across the graph. Refuses while the old id appears in done-work evidence; success is silent. Journal-recorded, but ids quoted in external documents will dangle.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "current node id", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "new id", choices: &[] },
        ],
        required: &["id", "to"],
    },
    ToolSpec {
        cmd: "resume",
        title: "Resume work item",
        desc: "Adopt this session's token on a progress work item, taking ownership of it: on success the item stays in progress with the session id and timestamp stamped on it (visible as session= in show output) and nothing is printed. Refuses when the item is not in progress. handoff transfers ownership to another session; revert drops the claim and returns the work item to ready.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "handoff",
        title: "Hand off work item",
        desc: "Transfer ownership of a progress work item to another session token; only the current holder can. resume is how the receiving session picks the item up.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "new owner session token", choices: &[] },
        ],
        required: &["id", "to"],
    },
    ToolSpec {
        cmd: "revert",
        title: "Revert to ready",
        desc: "Return a progress work item to ready and clear its session claim (holder or stale claim only); refuses when the item is not in progress. For rolling back graph mutations use undo.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "work item id W-NN", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "undo",
        title: "Undo mutations",
        desc: "Roll back the last N mutations (steps, default 1) by truncating the journal and replaying it. Destructive to audit history: the undone journal records are gone for good, and success is silent. For session claims use revert instead.",
        ann: DES,
        props: &[
            Prop { key: "steps", cli: "steps", typ: PropType::Int, desc: "number of mutations to revert (default 1)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "gate",
        title: "Distillation gate",
        desc: "Report whether the project passes the distillation gate: baseline, treewidth delta, work items done since baseline and what would distill (thresholds theta default 0 and n default 5). Appends a gate record to the audit journal but never mutates state.lock; distill is the worksheet and archive is the action.",
        ann: MUT,
        props: &[
            Prop { key: "theta", cli: "theta", typ: PropType::Int, desc: "surface overflow threshold (default 0)", choices: &[] },
            Prop { key: "n", cli: "n", typ: PropType::Int, desc: "done-count threshold (default 5)", choices: &[] },
        ],
        required: &[],
    },
    ToolSpec {
        cmd: "revalidate",
        title: "Revalidate discovery",
        desc: "Move a stale Discovery back to active by paying a fresh anchor: new surface paths and/or provenance ids. Success is silent; the Discovery's new status and revalidation log are visible via show. An unknown id fails with `not found` (exit 5). To copy a Discovery into another project use promote.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "discovery id Y-NN", choices: &[] },
            Prop { key: "surface", cli: "surface", typ: PropType::Str, desc: "comma-separated fresh anchor paths", choices: &[] },
            Prop { key: "from", cli: "from", typ: PropType::Str, desc: "comma-separated provenance ids", choices: &[] },
        ],
        required: &["id"],
    },
    ToolSpec {
        cmd: "glossary",
        title: "Rename glossary term",
        desc: "Atomically rename one glossary term in glossary.md and rewrite the Discovery tags that reference it: both halves change together or not at all. Refuses when the term is unknown.",
        ann: MUT,
        props: &[
            Prop { key: "old", cli: "", typ: PropType::Str, desc: "existing glossary term", choices: &[] },
            Prop { key: "new", cli: "", typ: PropType::Str, desc: "replacement term", choices: &[] },
        ],
        required: &["old", "new"],
    },
    ToolSpec {
        cmd: "projects",
        title: "List projects",
        desc: "List the project registry, one line per project: name, path and last-opened time. Entries are created and refreshed automatically as grove commands run inside a project; this server is bound to a single root at startup.",
        ann: RO,
        props: &[],
        required: &[],
    },
    ToolSpec {
        cmd: "promote",
        title: "Promote discovery",
        desc: "Copy a Discovery into another project (registry name or directory) with origin provenance; the copy arrives as proposed and the target project's state is written, unlike revalidate, which refreshes in place.",
        ann: MUT,
        props: &[
            Prop { key: "id", cli: "", typ: PropType::Str, desc: "discovery id Y-NN", choices: &[] },
            Prop { key: "to", cli: "to", typ: PropType::Str, desc: "target project (directory or registry name)", choices: &[] },
        ],
        required: &["id", "to"],
    },
    ToolSpec {
        cmd: "skill",
        title: "Embedded skill",
        desc: "Print the embedded agent skill (the SKILL.md workflow guide with frontmatter), or install it as a skill directory when install names one. The same content is also readable as the grove://skill resource.",
        ann: IDEM,
        props: &[Prop {
            key: "install",
            cli: "install",
            typ: PropType::Str,
            desc: "directory to install the skill into (omit to print)",
            choices: &[],
        }],
        required: &[],
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

fn tool_description(spec: &ToolSpec) -> String {
    if spec.desc.is_empty() {
        help_description(spec.cmd)
    } else {
        spec.desc.to_string()
    }
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
        "{{\"name\":{},\"title\":{},\"description\":{},\"inputSchema\":{{\"type\":\"object\",\"properties\":{{{}}},\"required\":[{}],\"additionalProperties\":false}},\"annotations\":{{\"readOnlyHint\":{},\"destructiveHint\":{},\"idempotentHint\":{},\"openWorldHint\":false}}}}",
        jstr(spec.cmd),
        jstr(spec.title),
        jstr(&tool_description(spec)),
        props,
        required.join(","),
        spec.ann.read_only,
        spec.ann.destructive,
        spec.ann.idempotent
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
        jstr("grove skill"),
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

fn resource_read(server: &McpServer, uri: &str) -> Result<String, (i64, String)> {
    if uri == "grove://skill" || uri.starts_with("grove://skill/") {
        let page = uri
            .strip_prefix("grove://skill")
            .unwrap_or_default()
            .trim_start_matches('/');
        let page = if page.is_empty() { "SKILL.md" } else { page };
        let text = if page == "SKILL.md" {
            crate::skill::stamped_skill_md()
        } else {
            crate::skill::skill_page(page)
                .ok_or_else(|| (ERR_INVALID_PARAMS, format!("unknown skill page: {page}")))?
                .to_string()
        };
        return Ok(format!(
            "{{\"contents\":[{{\"uri\":{},\"mimeType\":{},\"text\":{}}}]}}",
            jstr(uri),
            jstr("text/markdown"),
            jstr(&text)
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

const MCP_INSTRUCTIONS: &str = "Grove is a graph-driven workflow protocol. Read the resource grove://skill (the SKILL.md root; per-page reads grove://skill/<page>) before driving work items. Start every session with the status and next tools; keep check green.";

fn mcp_instructions() -> String {
    format!(
        "{} Binary v{}; embedded skill v{}.",
        MCP_INSTRUCTIONS,
        MCP_SERVER_VERSION,
        crate::skill::skill_version()
    )
}

fn initialize_result_json(version: &str) -> String {
    format!(
        "{{\"protocolVersion\":{},\"capabilities\":{{\"tools\":{{\"listChanged\":false}},\"resources\":{{\"listChanged\":false,\"subscribe\":false}}}},\"serverInfo\":{{\"name\":{},\"version\":{}}},\"instructions\":{}}}",
        jstr(version),
        jstr(MCP_SERVER_NAME),
        jstr(MCP_SERVER_VERSION),
        jstr(&mcp_instructions())
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
