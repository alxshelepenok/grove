use grove_core::parse_fixture;
use grove_desktop_lib::templates::{ui_dir, Templates};
use grove_desktop_lib::views::{areas, discovery, goals, graph, overview, project, themes, work};

mod common;

const FIXTURE: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Ship the API"
  area: A-01
  fitness_target: 4
  fitness_current: 1

g G-02 status=verified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Dogfood the loop"
  area: A-01
  fitness_target: 3
  fitness_current: 3

t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Scaling"

w W-01 type=feature status=proposed cynefin=complex t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Themed alpha"
  goals: G-01
  theme: T-01

w W-02 type=feature status=ready cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Themed beta"
  goals: G-01
  theme: T-01

w W-03 type=refactor status=progress cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Unthemed gamma"
  goals: G-02

w W-04 type=bug status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Unthemed delta"
  goals: G-01

q Q-01 status=open cynefin=complex t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Open question"
q Q-02 status=answered cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Answered question"
  tags: normalization

b B-01 status=proposed cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Proposed bet"
b B-02 status=testing cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Testing bet"
b B-03 status=validated cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Validated bet"
  tags: other

y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Active probe"
  tags: normalization
  surface: packages/core/src/lib.rs
  invariant:
    | Volatile values are masked at capture time

y Y-02 status=stale t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Stale probe"
  tags: normalization

e W-01 blocks W-02 t_created=2026-07-27T00:00:00Z
e W-02 blocks W-03 t_created=2026-07-27T00:00:00Z
e Q-01 targets G-01 t_created=2026-07-27T00:00:00Z
e B-01 targets G-01 t_created=2026-07-27T00:00:00Z
e Q-02 targets G-02 t_created=2026-07-27T00:00:00Z
e B-02 targets G-02 t_created=2026-07-27T00:00:00Z
e W-09 blocks W-03 t_created=2026-07-27T00:00:00Z

:archive
w W-09 type=feature status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Archived work"
"#;

const GLOSSARY: &str = r#"# Glossary

| Term | Definition | Source |
| --- | --- | --- |
| normalization | Masking volatile values at capture time | D-01 |
| causal inversion | Dependency opposing information flow | W-02 |
"#;

fn fixture_state() -> grove_core::State {
    parse_fixture(FIXTURE).expect("fixture parses")
}

fn templates() -> Templates {
    Templates::load(&ui_dir()).expect("templates load")
}

#[test]
fn goals_model_shapes_fitness_and_work() {
    let st = fixture_state();
    let m = goals::model(&st);
    let goals = m["goals"].as_array().unwrap();
    assert_eq!(goals.len(), 2);

    assert_eq!(goals[0]["id"], "G-01");
    assert_eq!(goals[0]["percent"], 25);
    assert_eq!(goals[0]["fitness_label"], "1 / 4");
    assert_eq!(goals[0]["has_bar"], true);
    assert_eq!(goals[0]["works"], 3);

    assert_eq!(goals[1]["id"], "G-02");
    assert_eq!(goals[1]["percent"], 100);
    assert_eq!(goals[1]["fitness_label"], "3 / 3");
    assert_eq!(goals[1]["works"], 1);

    for g in goals {
        assert!(g.get("chi").is_none(), "chi dropped from goal model");
    }
    assert!(m.get("chi_hint").is_none(), "chi_hint dropped from page model");
}

#[test]
fn goals_fragment_contains_numbers_and_ids() {
    let st = fixture_state();
    let html = templates().render("goals", &goals::model(&st)).unwrap();
    assert!(html.contains("<th>ID</th>"));
    assert!(html.contains("<th>Title</th>"));
    assert!(!html.contains("<th>Goal</th>"), "merged goal column split");
    assert!(html.contains(r#"<td><span class="text-mono">G-01</span></td>"#));
    assert!(html.contains(r#"<td class="cell-title">Ship the API</td>"#));
    assert!(html.contains("1 / 4"));
    assert!(html.contains("width: 25%;"));
    assert!(html.contains("width: 100%;"));
    assert!(html.contains("badge-warning"));
    assert!(html.contains(r#"<td class="cell-works">3</td>"#));
    assert!(html.contains(r#"<td class="cell-works">1</td>"#));
    assert!(html.contains("Track fitness progress across all goals."));
    assert!(!html.contains("cell-chi"), "chi column removed");
    assert!(!html.contains('\u{3c7}'), "greek chi removed");
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn goals_fragment_boolean_wording_and_bar_markup() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
g G-01 status=partial fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Boolean on"
  fitness_current: true

g G-02 status=partial fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Boolean off"
"#,
    )
    .expect("fixture parses");
    let m = goals::model(&st);
    assert_eq!(m["goals"][0]["fitness_label"], "boolean: true");
    assert_eq!(m["goals"][0]["percent"], 100);
    assert_eq!(m["goals"][1]["fitness_label"], "boolean: false");
    assert_eq!(m["goals"][1]["percent"], 0);
    let html = templates().render("goals", &m).unwrap();
    assert!(html.contains("boolean: true"));
    assert!(html.contains("boolean: false"));
    assert!(
        html.contains(r#"<span class="fitness-bar"><span class="fitness-bar-fill" style="width: 100%;"></span></span>"#),
        "full bar uses segmented-bar style classes"
    );
    assert!(
        html.contains(r#"<span class="fitness-bar"><span class="fitness-bar-fill" style="width: 0%;"></span></span>"#),
        "empty bar uses segmented-bar style classes"
    );
    assert!(!html.contains("progress-indicator"), "old bar classes gone");
}

#[test]
fn goals_fragment_empty_state() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
"#,
    )
    .expect("fixture parses");
    let m = goals::model(&st);
    assert_eq!(m["empty"], true);
    let html = templates().render("goals", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains(r#"<p class="empty-state-title">No goals</p>"#));
    assert!(html.contains("The lock has no goal nodes yet."));
    assert!(!html.contains("alert-info"), "alert replaced by empty-state");
    assert!(html.contains(r#"id="add-node-modal""#));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn goals_rail_badge_markup_and_refresh_wiring() {
    let html = std::fs::read_to_string(ui_dir().join("index.html")).unwrap();
    let goals_item = html
        .split(r#"<li class="side-rail-item" data-level="goals">"#)
        .nth(1)
        .and_then(|rest| rest.split("</li>").next())
        .expect("goals rail item present");
    assert!(goals_item.contains(
        r#"<span class="badge badge-count goals-badge" id="goals-badge" hidden></span>"#
    ));
    assert!(goals_item.contains("nav-icon-target"));
    assert!(goals_item.contains("tooltip-bubble"), "tooltip kept");

    let js = std::fs::read_to_string(ui_dir().join("js").join("main.js")).unwrap();
    assert_eq!(
        js.matches("grove_status_metrics").count(),
        1,
        "badge reuses the status metrics payload, no second invoke"
    );
    assert!(js.contains(r#"document.getElementById("goals-badge")"#));
}

const AREAS_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:0000000000000000000000000000000000000000000000000000000000000000
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Evals"
  surface: src/evals.jl

a A-02 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "API"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01
  fitness_target: 4
  fitness_current: 1

g G-02 status=partial fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal two"
  area: A-02
  fitness_current: true

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Shared work"
  goals: G-01, G-02

w W-02 type=bug status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Done evals work"
  goals: G-01

q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Open question"
q Q-02 status=answered cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Answered question"

b B-01 status=validated cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Validated bet"
d D-01 status=accepted t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Accepted decision"

y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Active probe"
  surface: src/evals.jl

e Q-01 targets W-01 t_created=2026-07-27T00:00:00Z
e Q-02 targets W-01 t_created=2026-07-27T00:00:00Z
e B-01 targets W-01 t_created=2026-07-27T00:00:00Z
e W-01 implements D-01 t_created=2026-07-27T00:00:00Z
"#;

fn areas_temp_root() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    let (guard, _home) = common::isolated_grove_home("areas");
    let dir = std::env::temp_dir().join(format!(
        "grove-desktop-areas-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".grove")).unwrap();
    std::fs::write(dir.join(".grove").join("state.lock"), AREAS_LOCK).unwrap();
    let r = grove_core::run_cli(&[
        "repair".to_string(),
        "--confirm".to_string(),
        format!("--root={}", dir.display()),
    ]);
    assert_eq!(r.code, grove_core::EXIT_OK, "repair failed: {}", r.err);
    (guard, dir)
}

fn count_for(area: &serde_json::Value, status: &str) -> u64 {
    area["counts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["status"] == status)
        .map(|r| r["count"].as_u64().unwrap())
        .unwrap_or(0)
}

#[test]
fn areas_model_counts_shared_node_in_both() {
    let (guard, dir) = areas_temp_root();
    let root = dir.to_string_lossy().into_owned();
    let st = grove_desktop_lib::views::load_state(&root).unwrap();
    let m = areas::model(&st);
    let rows = m["areas"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(m["empty"], false);

    let a1 = &rows[0];
    assert_eq!(a1["id"], "A-01");
    assert_eq!(a1["title"], "Evals");
    assert_eq!(a1["status"], "present");
    assert_eq!(a1["status_variant"], "neutral");
    let a1_goals: Vec<&str> = a1["goals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect();
    assert_eq!(a1_goals, ["G-01"]);
    assert_eq!(a1["goals"][0]["title"], "Goal one");
    assert_eq!(a1["goals"][0]["fitness_label"], "1 / 4");
    for dropped in ["status", "status_variant", "has_bar", "percent"] {
        assert!(
            a1["goals"][0].get(dropped).is_none(),
            "goal still exposes {dropped}"
        );
    }
    assert_eq!(count_for(a1, "proposed"), 1);
    assert_eq!(count_for(a1, "done"), 1);
    assert_eq!(a1["c"]["b"], 1);
    assert_eq!(a1["c"]["q"], 1);
    assert_eq!(a1["c"]["d"], 1);
    assert_eq!(a1["c"]["y"], 1);
    assert_eq!(a1["c_total"], 4);
    assert_eq!(a1["v"]["q"], 1);
    assert_eq!(a1["v"]["b"], 0);
    assert_eq!(a1["v"]["w"], 1);
    assert_eq!(a1["v"]["surf"], 0);
    assert_eq!(a1["v_total"], 2);
    assert_eq!(a1["has_y"], true);
    assert_eq!(a1["has_surf"], false);
    assert_eq!(a1["surface"][0], "src/evals.jl");
    assert_eq!(a1["has_surface"], true);
    assert_eq!(a1["has_tags"], false);

    let a2 = &rows[1];
    assert_eq!(a2["id"], "A-02");
    let a2_goals: Vec<&str> = a2["goals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_str().unwrap())
        .collect();
    assert_eq!(a2_goals, ["G-02"]);
    assert_eq!(a2["goals"][0]["title"], "Goal two");
    assert_eq!(a2["goals"][0]["fitness_label"], "boolean: true");
    for dropped in ["status", "status_variant", "has_bar", "percent"] {
        assert!(
            a2["goals"][0].get(dropped).is_none(),
            "goal still exposes {dropped}"
        );
    }
    assert_eq!(count_for(a2, "proposed"), 1);
    assert_eq!(count_for(a2, "done"), 0);
    assert_eq!(a2["c"]["b"], 1);
    assert_eq!(a2["c"]["q"], 1);
    assert_eq!(a2["c"]["d"], 1);
    assert_eq!(a2["c"]["y"], 0);
    assert_eq!(a2["c_total"], 3);
    assert_eq!(a2["v"]["q"], 1);
    assert_eq!(a2["v"]["w"], 1);
    assert_eq!(a2["v_total"], 2);
    assert_eq!(a2["has_y"], false);
    assert_eq!(a2["has_surface"], false);
    assert_eq!(a2["has_tags"], false);

    assert_eq!(a1["v"]["w"], a2["v"]["w"]);
    assert_eq!(a1["v"]["q"], a2["v"]["q"]);

    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "areas",
        &serde_json::json!({}),
    )
    .unwrap();
    assert!(html.contains(r#"class="view view-areas""#));
    assert!(html.contains(r#"id="area-A-01""#));
    assert!(html.contains(r#"id="area-A-02""#));
    drop(guard);
}

#[test]
fn areas_fragment_renders_cards_health_and_links() {
    let st = parse_fixture(AREAS_LOCK).expect("areas fixture parses");
    let html = templates().render("areas", &areas::model(&st)).unwrap();
    assert!(html.contains(r#"class="view view-areas""#));
    assert!(html.contains(r#"id="area-A-01""#));
    assert!(html.contains(r#"id="area-A-02""#));
    assert!(html.contains("Evals"));
    assert!(html.contains("API"));
    assert!(html.contains(r#"<span class="badge badge-neutral capitalize" id="A-01">present</span>"#));
    assert!(html.contains(r#"<span class="overview-row-title">Goal one</span>"#));
    assert!(html.contains(
        r#"<span class="text-muted overview-row-note word-spacing-tight">(1 / 4)</span>"#
    ));
    assert!(html.contains(
        r#"<span class="text-muted overview-row-note word-spacing-tight">(boolean: true)</span>"#
    ));
    assert!(!html.contains("fitness-bar"), "goal progress bars removed");
    assert!(!html.contains("area-goal-row"), "old goal row markup removed");
    assert!(
        !html.contains(r#"<span class="badge badge-warning">unverified</span>"#),
        "goal status badges removed"
    );
    assert!(html.contains(r#"<span class="badge badge-neutral capitalize">proposed 1</span>"#));
    assert!(html.contains(r#"<span class="badge badge-success capitalize">done 1</span>"#));
    assert!(html.contains(
        r#"<span class="area-health-key">Content (C): <span class="area-health-total">4</span></span>"#
    ));
    assert!(html.contains(
        r#"<span class="area-health-key">Uncertainty (V): <span class="area-health-total">2</span></span>"#
    ));
    assert!(
        html.contains("Validated assumptions (1), answered questions (1), accepted decisions (1), active discoveries (1).</span>"),
        "A-01 content row includes conditional discoveries"
    );
    assert!(
        html.contains("Validated assumptions (1), answered questions (1), accepted decisions (1).</span>"),
        "A-02 content row omits discoveries when none active"
    );
    assert!(
        html.contains("Open questions (1), pending assumptions (0), work below DoR (1).</span>"),
        "uncertainty row omits surfaces when none uncovered"
    );
    assert!(!html.contains("uncovered surfaces"));
    assert!(html.contains(r#"<span class="area-health-total">4</span>"#));
    assert!(html.contains(r#"<span class="area-health-total">3</span>"#));
    assert!(html.contains(r#"<span class="area-health-total">2</span>"#));
    assert!(html.contains("src/evals.jl"));
    assert!(
        !html.contains("Relevance view, not a partition"),
        "verbose area note removed from the copy pass"
    );
    assert!(html.contains(r#"data-action="goto" data-level="goals""#));
    assert!(html.contains(r#"data-action="goto" data-level="themes""#));
    assert!(html.contains(r#"id="add-node-modal""#));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn areas_fragment_empty_state() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

g G-01 status=unverified t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01
"#,
    )
    .expect("fixture parses");
    let m = areas::model(&st);
    assert_eq!(m["empty"], true);
    let html = templates().render("areas", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains(r#"<p class="empty-state-title">No areas</p>"#));
    assert!(html.contains("The lock has no area nodes yet."));
    assert!(!html.contains("alert-info"), "alert replaced by empty-state");
    assert!(html.contains(r#"id="add-node-modal""#));
}

#[test]
fn areas_fragment_card_sections_render_compact_empty_states() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
"#,
    )
    .expect("fixture parses");
    let m = areas::model(&st);
    assert_eq!(m["empty"], false);
    assert_eq!(m["areas"][0]["goals_empty"], true);
    assert_eq!(m["areas"][0]["work_empty"], true);
    let html = templates().render("areas", &m).unwrap();
    assert_eq!(
        html.matches(r#"<div class="empty-state empty-state-compact">"#).count(),
        2,
        "goals and work sections each render a compact empty-state"
    );
    assert!(html.contains(r#"width="32" height="32""#), "compact ghost icon");
    assert!(html.contains("No goals in this area."));
    assert!(html.contains("No work linked to this area&#x27;s goals."));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn themes_model_clusters_and_finds_critical_path() {
    let st = fixture_state();
    let m = themes::model(&st);
    let themes = m["themes"].as_array().unwrap();
    assert_eq!(themes.len(), 2);

    assert_eq!(themes[0]["id"], "T-01");
    assert_eq!(themes[0]["title"], "Scaling");
    let themed: Vec<&str> = themes[0]["works"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].as_str().unwrap())
        .collect();
    assert_eq!(themed, ["W-01", "W-02"]);

    assert_eq!(themes[1]["unthemed"], true);
    let unthemed: Vec<&str> = themes[1]["works"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["id"].as_str().unwrap())
        .collect();
    assert_eq!(unthemed, ["W-03", "W-04"]);

    let chain: Vec<&str> = m["critical_path"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(chain, ["W-01", "W-02", "W-03"]);
    assert_eq!(m["critical_len"], 3);

    let questions: Vec<&str> = m["questions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(questions, ["Q-01"]);
    let assumptions: Vec<&str> = m["assumptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(assumptions, ["B-01", "B-02"]);
    assert_eq!(m["questions_count"], 1);
    assert_eq!(m["assumptions_count"], 2);
    assert_eq!(m["cloud_empty"], false);
}

#[test]
fn themes_fragment_shows_cloud_and_excludes_closed() {
    let st = fixture_state();
    let m = themes::model(&st);
    let html = templates().render("themes", &m).unwrap();
    assert!(html.contains("Open questions and assumptions"));
    assert!(html.contains("Scaling"));
    assert!(html.contains("Unthemed"));
    assert!(html.contains("W-01"));
    assert!(html.contains("Q-01"));
    assert!(html.contains("B-02"));
    assert!(html.contains("complex"));
    assert!(!html.contains("Q-02"));
    assert!(!html.contains("B-03"));
    for item in m["questions"].as_array().unwrap().iter().chain(m["assumptions"].as_array().unwrap()) {
        assert!(item.get("kind").is_none(), "kind badge field dropped");
        assert!(item.get("kind_variant").is_none(), "kind badge field dropped");
    }
    assert!(!html.contains(">B</span>"), "kind badge removed");
    assert!(!html.contains("cloud-item"), "cloud rows use compact-row classes");
    assert!(html.contains(r#"<li class="overview-row">"#));
    assert!(html.contains(r#"<span class="overview-row-title">Open question</span>"#));
    assert!(html.contains("overview-row-note"), "cynefin badge pinned to row end");
    for label in ["Questions", "Assumptions"] {
        assert!(
            html.contains(&format!(r#"<span class="area-section-label">{label}</span>"#)),
            "section head {label}"
        );
    }
    let q_pos = html.find(r#"<span class="area-section-label">Questions</span>"#).unwrap();
    let a_pos = html.find(r#"<span class="area-section-label">Assumptions</span>"#).unwrap();
    assert!(q_pos < a_pos, "questions section precedes assumptions");
    let q_id_pos = html.find(r#"<span class="overview-row-id text-mono">Q-01</span>"#).unwrap();
    let b_id_pos = html.find(r#"<span class="overview-row-id text-mono">B-01</span>"#).unwrap();
    assert!(q_pos < q_id_pos && q_id_pos < a_pos, "Q-01 inside the questions section");
    assert!(a_pos < b_id_pos, "B-01 inside the assumptions section");

    let table_count = html.matches(r#"<table class="theme-works-table interactive">"#).count();
    assert_eq!(table_count, 2, "one works table per theme card incl. Unthemed");
    assert_eq!(
        html.matches(r#"<div class="table-scroll">"#).count(),
        2,
        "each works table renders inside a scroll container"
    );
    assert!(!html.contains(r#"<ul class="theme-works">"#), "ul/li work list replaced");
    let css = std::fs::read_to_string(ui_dir().join("css").join("views").join("themes.css")).unwrap();
    assert!(
        css.contains(".theme-card .table-scroll") && css.contains("max-height: 360px"),
        "theme card list height capped with scroll"
    );
    assert!(
        css.contains(".theme-works-table thead th") && css.contains("position: static"),
        "in-card thead is not sticky"
    );
    let overview_css = std::fs::read_to_string(ui_dir().join("css").join("views").join("overview.css")).unwrap();
    assert!(
        overview_css.contains("#overview-goals .overview-rows") && overview_css.contains("#overview-discovery .overview-rows"),
        "overview list scroll caps present"
    );
    for col in [r#"<th class="col-id">ID</th>"#, "<th>Title</th>", r#"<th class="col-status">Status</th>"#] {
        assert_eq!(html.matches(col).count(), 2, "column {col} in both tables");
    }
    for id in ["W-01", "W-02", "W-03", "W-04"] {
        assert!(html.contains(&format!(r#"<tr data-id="{id}">"#)), "row carries data-id {id}");
    }
    assert!(html.contains(r#"<td class="cell-title">Themed alpha</td>"#));
    assert!(html.contains(r#"<div class="theme-counts">"#), "counts chips stay in the card header");
    let counts_pos = html.find(r#"<div class="theme-counts">"#).unwrap();
    let table_pos = html.find(r#"<table class="theme-works-table interactive">"#).unwrap();
    assert!(counts_pos < table_pos, "counts render above the works table");
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn themes_fragment_empty_cloud_renders_ghost_empty_state() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Scaling"
"#,
    )
    .expect("fixture parses");
    let m = themes::model(&st);
    assert_eq!(m["cloud_empty"], true);
    let html = templates().render("themes", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state empty-state-compact">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains("No open questions or proposed/testing assumptions."));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn themes_fragment_compact_empty_states() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Scaling"
"#,
    )
    .expect("fixture parses");
    let m = themes::model(&st);
    assert_eq!(m["critical_len"], 0);
    let html = templates().render("themes", &m).unwrap();
    assert_eq!(
        html.matches(r#"<div class="empty-state empty-state-compact">"#).count(),
        3,
        "critical path card, workless theme row, and question cloud each render a compact empty-state"
    );
    assert!(html.contains(r#"width="32" height="32""#), "compact ghost icon");
    assert!(html.contains("No open blocks edges."));
    assert!(html.contains("No work items under this theme."));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn glossary_parser_reads_term_definition_source() {
    let terms = discovery::parse_glossary(GLOSSARY);
    assert_eq!(
        terms,
        vec![
            discovery::GlossaryTerm {
                term: "normalization".to_string(),
                definition: "Masking volatile values at capture time".to_string(),
                source: "D-01".to_string(),
            },
            discovery::GlossaryTerm {
                term: "causal inversion".to_string(),
                definition: "Dependency opposing information flow".to_string(),
                source: "W-02".to_string(),
            },
        ]
    );
}

#[test]
fn discovery_model_counts_c_per_term_and_unattributed() {
    let st = fixture_state();
    let m = discovery::model(&st, GLOSSARY);
    let terms = m["terms"].as_array().unwrap();
    assert_eq!(terms.len(), 2);

    assert_eq!(terms[0]["term"], "normalization");
    assert_eq!(terms[0]["c"], 2);
    assert_eq!(terms[0]["c_ids"], "Q-02, Y-01");

    assert_eq!(terms[1]["term"], "causal inversion");
    assert_eq!(terms[1]["c"], 0);

    assert_eq!(m["unattributed"], 1);
    assert_eq!(m["unattributed_ids"], "B-03");
    assert_eq!(m["c_total"], 3);

    let active = m["discoveries"].as_array().unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["id"], "Y-01");
    assert_eq!(active[0]["status_variant"], "success");
    assert_eq!(
        active[0]["invariant"],
        "Volatile values are masked at capture time"
    );
    assert_eq!(active[0]["surface"][0], "packages/core/src/lib.rs");
    assert_eq!(active[0]["has_surface"], true);

    let inactive = m["inactive_discoveries"].as_array().unwrap();
    assert_eq!(inactive.len(), 1);
    assert_eq!(inactive[0]["id"], "Y-02");
    assert_eq!(inactive[0]["status"], "stale");
    assert_eq!(inactive[0]["status_variant"], "danger");
}

#[test]
fn discovery_fragment_shows_terms_and_discovery_contours() {
    let st = fixture_state();
    let html = templates()
        .render("discovery", &discovery::model(&st, GLOSSARY))
        .unwrap();
    assert!(html.contains("normalization"));
    assert!(html.contains("causal inversion"));
    assert!(html.contains("Masking volatile values at capture time"));
    assert!(html.contains("Y-01"));
    assert!(html.contains("Active probe"));
    assert!(html.contains("Volatile values are masked at capture time"));
    assert!(html.contains("packages/core/src/lib.rs"));
    assert!(html.contains("badge-success"));
    assert!(html.contains("Y-02"));
    assert!(html.contains("badge-danger"));
    assert!(html.contains("C(term)"));
}

#[test]
fn discovery_fragment_empty_discoveries_render_empty_state() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
"#,
    )
    .expect("fixture parses");
    let m = discovery::model(&st, GLOSSARY);
    assert_eq!(m["discoveries_empty"], true);
    let html = templates().render("discovery", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains(r#"<p class="empty-state-title">No active discoveries</p>"#));
    assert!(html.contains("No discovery is active in the lock."));
    assert!(!html.contains("alert-info"), "alert replaced by empty-state");
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn graph_model_excludes_archived_by_default() {
    let st = fixture_state();
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 15);
    assert_eq!(m["edge_count"], 6);
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    assert!(nodes.iter().all(|n| n["id"] != "W-09"));
    assert!(nodes.iter().all(|n| n["archived"] == false));
    let edges = m["graph"]["edges"].as_array().unwrap();
    assert!(edges.iter().all(|e| e["from"] != "W-09" && e["to"] != "W-09"));

    let w1 = nodes.iter().find(|n| n["id"] == "W-01").unwrap();
    assert_eq!(w1["kind"], "w");
    assert_eq!(w1["status"], "proposed");
    assert_eq!(w1["title"], "Themed alpha");
    assert_eq!(w1["wtype"], "feature");

    let labels: Vec<&str> = edges.iter().map(|e| e["label"].as_str().unwrap()).collect();
    assert!(labels.contains(&"blocks"));
    assert!(labels.contains(&"targets"));
}

#[test]
fn graph_model_includes_archived_on_request() {
    let st = fixture_state();
    let m = graph::model(&st, true, graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 16);
    assert_eq!(m["edge_count"], 7);
    assert_eq!(m["include_archived"], true);
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    let w9 = nodes.iter().find(|n| n["id"] == "W-09").unwrap();
    assert_eq!(w9["archived"], true);
    assert_eq!(w9["status"], "done");
}

#[test]
fn graph_fragment_carries_canvas_and_json_model() {
    let st = fixture_state();
    let html = templates()
        .render("graph", &graph::model(&st, false, graph::ROOT_TITLE))
        .unwrap();
    assert!(html.contains(r#"<canvas id="graph-canvas">"#));
    assert!(html.contains(r#"type="application/json" id="graph-data""#));
    assert!(html.contains("W-01"));
    assert!(html.contains("Reheat"));
    assert!(html.contains("Include archived"));
    assert!(html.contains("Explore nodes and edges as an interactive graph."));
    assert!(
        !html.contains("15 of 16 nodes"),
        "node and edge counters removed from the static description"
    );
}

fn contains_edges(m: &serde_json::Value) -> Vec<(String, String)> {
    m["graph"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["virtual"] == true)
        .map(|e| {
            assert_eq!(e["label"], "contains");
            (
                e["from"].as_str().unwrap().to_string(),
                e["to"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn graph_model_synthesizes_root_and_contains_edges() {
    let st = fixture_state();
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 15);
    assert_eq!(m["edge_count"], 6);
    assert_eq!(m["virtual_node_count"], 1);
    assert_eq!(m["virtual_edge_count"], 12);

    let nodes = m["graph"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 16);
    let root = nodes.iter().find(|n| n["id"] == "Project").unwrap();
    assert_eq!(root["kind"], "root");
    assert_eq!(root["title"], "Project");
    assert_eq!(root["archived"], false);
    assert_eq!(root["virtual"], true);

    let edges = m["graph"]["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 18);
    let real: Vec<&serde_json::Value> = edges.iter().filter(|e| e.get("virtual").is_none()).collect();
    assert_eq!(real.len(), 6);
    let labels: Vec<&str> = real.iter().map(|e| e["label"].as_str().unwrap()).collect();
    assert!(labels.contains(&"blocks"));
    assert!(labels.contains(&"targets"));

    let contains = contains_edges(&m);
    for expected in [
        ("Project", "A-01"),
        ("A-01", "G-01"),
        ("A-01", "G-02"),
        ("G-01", "W-01"),
        ("G-01", "W-02"),
        ("G-01", "W-04"),
        ("G-02", "W-03"),
    ] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    for n in nodes.iter().filter(|n| ["a", "g", "w"].contains(&n["kind"].as_str().unwrap())) {
        let id = n["id"].as_str().unwrap();
        assert!(
            contains.iter().any(|(_, to)| to == id),
            "{id} has no containment parent"
        );
    }
}

#[test]
fn graph_model_root_titled_project() {
    let st = fixture_state();
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    let root = nodes.iter().find(|n| n["id"] == "Project").unwrap();
    assert_eq!(root["id"], "Project");
    assert_eq!(root["kind"], "root");
    assert_eq!(root["title"], "Project");
}

const GRAPH_ORPHAN_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Owned goal"
  area: A-01

g G-02 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Orphan goal"

g G-03 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Lost goal"
  area: A-99

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Owned work"
  goals: G-01

w W-02 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Orphan work"

w W-03 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Lost work"
  goals: G-88
"#;

#[test]
fn graph_model_orphans_attach_to_root() {
    let st = parse_fixture(GRAPH_ORPHAN_LOCK).expect("fixture parses");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["virtual_edge_count"], 7);
    let contains = contains_edges(&m);
    for expected in [
        ("Project", "A-01"),
        ("A-01", "G-01"),
        ("Project", "G-02"),
        ("Project", "G-03"),
        ("G-01", "W-01"),
        ("Project", "W-02"),
        ("Project", "W-03"),
    ] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
}

const GRAPH_ARCHIVE_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Live area"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Live goal"
  area: A-01

g G-02 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal of gone area"
  area: A-02

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Live work"
  goals: G-01

w W-02 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work of gone goal"
  goals: G-03

:archive
a A-02 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Gone area"

g G-03 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Gone goal"
  area: A-02

w W-03 type=feature status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Gone work"
  goals: G-01
"#;

#[test]
fn graph_model_archived_filter_moves_virtual_edges() {
    let st = parse_fixture(GRAPH_ARCHIVE_LOCK).expect("fixture parses");

    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 5);
    assert_eq!(m["virtual_edge_count"], 5);
    let contains = contains_edges(&m);
    for expected in [
        ("Project", "A-01"),
        ("A-01", "G-01"),
        ("Project", "G-02"),
        ("G-01", "W-01"),
        ("Project", "W-02"),
    ] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    for hidden in ["A-02", "G-03", "W-03"] {
        assert!(
            contains
                .iter()
                .all(|(from, to)| from != hidden && to != hidden),
            "contains edge touches hidden {hidden}"
        );
    }
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    assert!(nodes.iter().any(|n| n["id"] == "Project"));

    let m = graph::model(&st, true, graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 8);
    assert_eq!(m["virtual_edge_count"], 8);
    let contains = contains_edges(&m);
    for expected in [
        ("Project", "A-01"),
        ("Project", "A-02"),
        ("A-01", "G-01"),
        ("A-02", "G-02"),
        ("A-02", "G-03"),
        ("G-01", "W-01"),
        ("G-03", "W-02"),
        ("G-01", "W-03"),
    ] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    assert_eq!(contains.iter().filter(|(from, _)| from == "Project").count(), 2);
}

#[test]
fn graph_model_theme_contains_themed_work() {
    let st = fixture_state();
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    let contains = contains_edges(&m);
    for expected in [("T-01", "W-01"), ("T-01", "W-02")] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    assert!(
        contains.iter().all(|(from, to)| from != "Project" || to != "T-01"),
        "T-01 has works and must not attach to Project"
    );
}

const GRAPH_THEME_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Alone theme"

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Ghost themed work"
  theme: T-99
"#;

#[test]
fn graph_model_workless_theme_attaches_to_root() {
    let st = parse_fixture(GRAPH_THEME_LOCK).expect("fixture parses");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["virtual_edge_count"], 2);
    let contains = contains_edges(&m);
    for expected in [("Project", "T-01"), ("Project", "W-01")] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    assert!(
        contains.iter().all(|(from, _)| from != "T-99"),
        "ghost theme T-99 gets no contains edge"
    );
}

const GRAPH_DISCOVERY_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area one"
  surface: src/a.rs

a A-02 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area two"
  surface: src/b.rs

y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Both areas"
  surface: src/a.rs, src/b.rs

y Y-02 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area two only"
  surface: src/b.rs

y Y-03 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "No match"
  surface: src/zzz.rs

y Y-04 status=stale t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Stale match"
  surface: src/a.rs
"#;

#[test]
fn graph_model_discovery_matches_area_or_root() {
    let st = parse_fixture(GRAPH_DISCOVERY_LOCK).expect("fixture parses");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["virtual_edge_count"], 6);
    let contains = contains_edges(&m);
    for expected in [
        ("Project", "A-01"),
        ("Project", "A-02"),
        ("A-01", "Y-01"), // smallest area id wins when several areas match
        ("A-02", "Y-02"),
        ("Project", "Y-03"),
        ("Project", "Y-04"), // stale discoveries never match an area
    ] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
}

const GRAPH_EDGELESS_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Alone question"

b B-01 status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Alone bet"

d D-01 status=accepted t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Alone decision"

q Q-02 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Linked question"

b B-02 status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Linked bet"

e Q-02 targets B-02 t_created=2026-07-27T00:00:00Z
"#;

#[test]
fn graph_model_edgeless_qbd_attach_to_root() {
    let st = parse_fixture(GRAPH_EDGELESS_LOCK).expect("fixture parses");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    assert_eq!(m["virtual_edge_count"], 3);
    let contains = contains_edges(&m);
    for expected in [("Project", "Q-01"), ("Project", "B-01"), ("Project", "D-01")] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    for linked in ["Q-02", "B-02"] {
        assert!(
            contains.iter().all(|(_, to)| to != linked),
            "{linked} has a real edge and must not attach to Project"
        );
    }
}

fn graph_clusters(m: &serde_json::Value) -> std::collections::BTreeMap<String, String> {
    m["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["id"].as_str().unwrap().to_string(),
                n["cluster"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn graph_root_node_keeps_id_and_uses_given_project_title() {
    let st = fixture_state();
    let m = graph::model(&st, false, "g14-lock");
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    let root = nodes.iter().find(|n| n["id"] == "Project").expect("root node");
    assert_eq!(root["id"], "Project", "ROOT_ID unchanged");
    assert_eq!(root["title"], "g14-lock", "root title follows the project name");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    let nodes = m["graph"]["nodes"].as_array().unwrap();
    let root = nodes.iter().find(|n| n["id"] == "Project").expect("root node");
    assert_eq!(root["title"], graph::ROOT_TITLE, "default title fallback");
}

const GRAPH_CLUSTER_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area one"
  surface: src/a.rs

a A-02 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area two"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01

g G-02 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal two"
  area: A-01

g G-03 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal three"
  area: A-02

g G-04 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Orphan goal"

t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Theme"

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work one"
  goals: G-01

w W-02 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Two goals"
  goals: G-01, G-02

w W-03 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Theme only"
  theme: T-01

w W-04 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Orphan work"

y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Matching probe"
  surface: src/a.rs

y Y-02 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Unmatched probe"
  surface: src/zzz.rs

q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Edgeless question"
"#;

#[test]
fn graph_model_cluster_groups_area_subtrees() {
    let st = parse_fixture(GRAPH_CLUSTER_LOCK).expect("fixture parses");
    let clusters = graph_clusters(&graph::model(&st, false, graph::ROOT_TITLE));
    for id in ["A-01", "G-01", "G-02", "W-01", "W-02", "Y-01"] {
        assert_eq!(clusters[id], "A-01", "{id} belongs to the A-01 subtree");
    }
    for id in ["A-02", "G-03"] {
        assert_eq!(clusters[id], "A-02", "{id} belongs to the A-02 subtree");
    }
}

#[test]
fn graph_model_cluster_sends_root_attached_nodes_to_root() {
    let st = parse_fixture(GRAPH_CLUSTER_LOCK).expect("fixture parses");
    let clusters = graph_clusters(&graph::model(&st, false, graph::ROOT_TITLE));
    for id in ["Project", "G-04", "W-03", "W-04", "T-01", "Y-02", "Q-01"] {
        assert_eq!(
            clusters[id],
            graph::ROOT_CLUSTER,
            "{id} hangs directly off Project"
        );
    }
}

const GRAPH_CLUSTER_MULTI_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area one"

a A-02 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area two"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal in area two"
  area: A-02

g G-02 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal in area one"
  area: A-01

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Cross-area work"
  goals: G-01, G-02
"#;

#[test]
fn graph_model_cluster_work_follows_smallest_goal() {
    let st = parse_fixture(GRAPH_CLUSTER_MULTI_LOCK).expect("fixture parses");
    let clusters = graph_clusters(&graph::model(&st, false, graph::ROOT_TITLE));
    assert_eq!(clusters["W-01"], "A-02");
    assert_eq!(clusters["G-01"], "A-02");
    assert_eq!(clusters["G-02"], "A-01");
}

#[test]
fn graph_model_cluster_stable_under_kind_filter() {
    let st = parse_fixture(GRAPH_CLUSTER_LOCK).expect("fixture parses");
    let all = graph_clusters(&graph::model(&st, false, graph::ROOT_TITLE));
    for kind in ["a", "g", "w", "y"] {
        let filtered = graph_clusters(&graph::model_filtered(&st, false, kind, "", graph::ROOT_TITLE));
        for (id, cluster) in &filtered {
            assert_eq!(all[id], *cluster, "{id} changed cluster under kind={kind}");
        }
    }
}

#[test]
fn graph_model_cluster_stable_under_focus() {
    let st = parse_fixture(GRAPH_CLUSTER_LOCK).expect("fixture parses");
    let all = graph_clusters(&graph::model(&st, false, graph::ROOT_TITLE));
    let focused = graph_clusters(&graph::model_filtered(&st, false, "all", "G-01", graph::ROOT_TITLE));
    assert!(focused.len() > 2);
    for (id, cluster) in &focused {
        assert_eq!(all[id], *cluster, "{id} changed cluster under focus");
    }
}

fn graph_node_ids(m: &serde_json::Value) -> Vec<String> {
    let mut ids: Vec<String> = m["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

#[test]
fn graph_model_kind_filter_keeps_containment_ancestors() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, false, "g", "", graph::ROOT_TITLE);
    assert_eq!(m["kind"], "g");
    assert_eq!(m["node_count"], 3);
    assert_eq!(m["edge_count"], 0);
    assert_eq!(m["virtual_edge_count"], 3);
    assert_eq!(graph_node_ids(&m), ["A-01", "G-01", "G-02", "Project"]);
    let contains = contains_edges(&m);
    for expected in [("Project", "A-01"), ("A-01", "G-01"), ("A-01", "G-02")] {
        assert!(
            contains.contains(&(expected.0.to_string(), expected.1.to_string())),
            "missing contains edge {} -> {}",
            expected.0,
            expected.1
        );
    }
    let filters = m["filters"].as_array().unwrap();
    let g_tab = filters.iter().find(|f| f["status"] == "g").unwrap();
    assert_eq!(g_tab["active"], true);
    assert_eq!(g_tab["count"], 2);
    assert_eq!(g_tab["label"], "Goals (G)");
    let all_tab = filters.iter().find(|f| f["status"] == "all").unwrap();
    assert_eq!(all_tab["active"], false);
    assert_eq!(all_tab["count"], 15);
    assert_eq!(all_tab["label"], "All");
}

#[test]
fn graph_model_filter_labels_follow_status_bar_names() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal"
w W-01 type=feature status=ready cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work"
t T-01 status=open t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Theme"
q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Question"
b B-01 status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bet"
y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Probe"
d D-01 status=accepted t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Decision"
"#,
    )
    .expect("fixture parses");
    let m = graph::model(&st, false, graph::ROOT_TITLE);
    let filters = m["filters"].as_array().unwrap();
    let labels: Vec<(&str, &str)> = filters
        .iter()
        .map(|f| {
            (
                f["status"].as_str().unwrap(),
                f["label"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        labels,
        [
            ("all", "All"),
            ("a", "Areas (A)"),
            ("g", "Goals (G)"),
            ("w", "Work (W)"),
            ("t", "Themes (T)"),
            ("q", "Questions (Q)"),
            ("b", "Assumptions (B)"),
            ("y", "Discovery (Y)"),
            ("d", "Decisions (D)"),
        ]
    );
}

#[test]
fn graph_model_kind_filter_work_keeps_goal_theme_area_chain() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, false, "w", "", graph::ROOT_TITLE);
    assert_eq!(m["node_count"], 8);
    assert_eq!(m["edge_count"], 2);
    assert_eq!(m["virtual_edge_count"], 9);
    assert_eq!(
        graph_node_ids(&m),
        ["A-01", "G-01", "G-02", "Project", "T-01", "W-01", "W-02", "W-03", "W-04"]
    );
    let filters = m["filters"].as_array().unwrap();
    let statuses: Vec<&str> = filters
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["all", "a", "g", "w", "t", "q", "b", "y"]);
}

#[test]
fn graph_model_focus_keeps_subtree_neighbors_ancestors() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, false, "all", "G-01", graph::ROOT_TITLE);
    assert_eq!(m["focus"], "G-01");
    assert_eq!(m["focus_label"], "G-01 - Ship the API (g)");
    assert_eq!(m["node_count"], 8);
    assert_eq!(m["edge_count"], 3);
    assert_eq!(m["virtual_edge_count"], 7);
    assert_eq!(
        graph_node_ids(&m),
        ["A-01", "B-01", "G-01", "Project", "Q-01", "T-01", "W-01", "W-02", "W-04"]
    );
}

#[test]
fn graph_model_kind_tabs_reflect_archived_toggle() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, true, "all", "", graph::ROOT_TITLE);
    let filters = m["filters"].as_array().unwrap();
    let w_tab = filters.iter().find(|f| f["status"] == "w").unwrap();
    assert_eq!(w_tab["count"], 5);
    let all_tab = filters.iter().find(|f| f["status"] == "all").unwrap();
    assert_eq!(all_tab["count"], 16);
}

#[test]
fn graph_model_focus_outside_kind_resets_to_all() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, false, "y", "G-01", graph::ROOT_TITLE);
    assert_eq!(m["kind"], "all");
    assert_eq!(m["focus"], "G-01");
    assert_eq!(
        graph_node_ids(&m),
        ["A-01", "B-01", "G-01", "Project", "Q-01", "T-01", "W-01", "W-02", "W-04"]
    );
}

#[test]
fn graph_model_focus_unknown_id_ignored() {
    let st = fixture_state();
    let m = graph::model_filtered(&st, false, "all", "Z-99", graph::ROOT_TITLE);
    assert_eq!(m["focus"], "");
    assert_eq!(m["node_count"], 15);
    assert_eq!(m["virtual_edge_count"], 12);
}

#[test]
fn graph_fragment_shows_kind_tabs_and_focus_select() {
    let st = fixture_state();
    let html = templates()
        .render("graph", &graph::model(&st, false, graph::ROOT_TITLE))
        .unwrap();
    assert!(html.contains(r#"role="tablist""#));
    assert!(html.contains(r#"data-action="filter" data-status="all""#));
    for kind in ["a", "g", "w", "t", "q", "b", "y"] {
        assert!(
            html.contains(&format!(r#"data-action="filter" data-status="{kind}""#)),
            "missing {kind} tab"
        );
    }
    assert!(
        !html.contains(r#"data-action="filter" data-status="d""#),
        "no d nodes in the fixture, so no d tab"
    );
    assert!(html.contains(
        r#"data-action="filter" data-status="all" aria-selected="true">All<span class="filter-tab-count">15</span>"#
    ));
    for (kind, label) in [
        ("a", "Areas (A)"),
        ("g", "Goals (G)"),
        ("w", "Work (W)"),
        ("t", "Themes (T)"),
        ("q", "Questions (Q)"),
        ("b", "Assumptions (B)"),
        ("y", "Discovery (Y)"),
    ] {
        assert!(
            html.contains(&format!(r#"data-status="{kind}" aria-selected="false">{label}<span"#)),
            "missing label {label}"
        );
    }
    assert!(html.contains(r#">Work (W)<span class="filter-tab-count">4</span>"#));
    assert!(html.contains(r#"id="graph-focus""#));
    assert!(html.contains("Focus a node..."));
    assert!(html.contains("W-01 - Themed alpha (w)"));
    assert!(
        !html.contains("titled Project"),
        "verbose legend caption removed from the copy pass"
    );
    assert!(!html.contains("the project name"));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn graph_fragment_focus_shows_selected_label_and_clear() {
    let st = fixture_state();
    let html = templates()
        .render("graph", &graph::model_filtered(&st, false, "all", "G-01", graph::ROOT_TITLE))
        .unwrap();
    assert!(html.contains(r#"id="graph-focus-clear""#));
    assert!(html.contains("G-01 - Ship the API (g)"));
    assert!(html.contains(r#"data-kind="all" data-focus="G-01""#));
}

#[test]
fn icon_helper_inlines_svg_with_size() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"
"#,
    )
    .expect("fixture parses");
    let tpl = templates();
    let html = tpl.render("discovery", &discovery::model(&st, "")).unwrap();
    assert!(html.contains("No active discoveries"));
    assert!(
        !html.contains("No glossary terms"),
        "glossary empty-state removed from the copy pass"
    );
    assert!(html.contains("class=\"icon empty-state-icon\""));
    assert!(html.contains("width=\"48\""));
    assert!(html.contains("height=\"48\""));
}

#[test]
fn packet_fragment_renders_mutation_forms() {
    let model = serde_json::json!({
        "works": [],
        "selected": "W-01",
        "packet": {
            "id": "W-01",
            "title": "Themed alpha",
            "status": "ready",
            "markdown": "# packet",
        },
    });
    let html = templates().render("packet", &model).unwrap();
    assert!(html.contains(r#"data-packet-id="W-01""#));
    assert!(html.contains(r#"id="packet-evidence-text""#));
    assert!(html.contains(r#"id="packet-evidence-submit""#));
    assert!(html.contains(r#"id="packet-evidence-error""#));
    assert!(html.contains(r#"id="packet-status-select""#));
    assert!(html.contains(r#"<option value="ready" selected>ready</option>"#));
    assert!(html.contains(r#"id="packet-status-submit""#));
    assert!(html.contains(r#"id="packet-link-label""#));
    assert!(html.contains(r#"<option value="blocks" selected>blocks</option>"#));
    assert!(html.contains("distills"));
    assert!(html.contains(r#"id="packet-link-target""#));
    assert!(html.contains(r#"id="packet-link-add""#));
    assert!(html.contains(r#"id="packet-link-remove""#));
    assert!(html.contains(r#"id="add-node-modal""#));
}

#[test]
fn packet_fragment_trigger_shows_selected_text_or_placeholder() {
    let selected = serde_json::json!({
        "works": [{"id": "W-01", "label": "W-01 - Themed alpha"}],
        "selected": "W-01",
        "selectedText": "W-01 - Themed alpha",
    });
    let html = templates().render("packet", &selected).unwrap();
    assert!(html.contains(
        r#"<div class="searchable-select-trigger">W-01 - Themed alpha</div>"#
    ));
    assert!(!html.contains("Select a work item..."));

    let bare = serde_json::json!({
        "works": [{"id": "W-01", "label": "W-01 - Themed alpha"}],
        "selected": "",
    });
    let html = templates().render("packet", &bare).unwrap();
    assert!(html.contains(
        r#"<div class="searchable-select-trigger">Select a work item...</div>"#
    ));
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains("Pick a work item to render its execution packet."));
    assert!(!html.contains(r#"<p class="empty-state-title">"#), "prompt keeps no title");
}

fn packet_temp_root() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    let (guard, _home) = common::isolated_grove_home("packet");
    let dir = std::env::temp_dir().join(format!(
        "grove-desktop-packet-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".grove")).unwrap();
    std::fs::write(
        dir.join(".grove").join("state.lock"),
        FIXTURE.replace(
            "sha256:fixture",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        ),
    )
    .unwrap();
    let r = grove_core::run_cli(&[
        "repair".to_string(),
        "--confirm".to_string(),
        format!("--root={}", dir.display()),
    ]);
    assert_eq!(r.code, grove_core::EXIT_OK, "repair failed: {}", r.err);
    (guard, dir)
}

#[test]
fn packet_view_trigger_shows_routed_work() {
    let (guard, dir) = packet_temp_root();
    let root = dir.to_string_lossy().into_owned();
    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "packet",
        &serde_json::json!({ "id": "W-01" }),
    )
    .unwrap();
    assert!(html.contains(
        r#"<div class="searchable-select-trigger">W-01 - Themed alpha</div>"#
    ));
    assert!(!html.contains("Select a work item..."));

    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "packet",
        &serde_json::json!({}),
    )
    .unwrap();
    assert!(html.contains(
        r#"<div class="searchable-select-trigger">Select a work item...</div>"#
    ));
    drop(guard);
}

#[test]
fn add_modal_markup_covers_all_kinds() {
    let html = templates()
        .render("add-modal", &serde_json::json!({}))
        .unwrap();
    assert!(html.contains(r#"id="add-node-modal""#));
    assert!(html.contains(r#"id="add-node-kind""#));
    assert!(html.contains(r#"id="add-node-title""#));
    assert!(html.contains(r#"id="add-node-submit""#));
    assert!(html.contains(r#"id="add-node-cancel""#));
    assert!(html.contains(r#"id="add-node-error""#));
    for kind in ["w", "q", "b", "d", "y", "g", "t", "a"] {
        assert!(
            html.contains(&format!(r#"value="{kind}""#)),
            "kind option {kind}"
        );
        assert!(
            html.contains(&format!(r#"data-add-kind="{kind}""#)),
            "kind section {kind}"
        );
    }
    assert!(html.contains(r#"id="add-w-type""#));
    assert!(html.contains(r#"id="add-w-cynefin""#));
    assert!(html.contains(r#"id="add-w-goals""#));
    assert!(html.contains(r#"id="add-w-goal-deltas""#));
    assert!(html.contains(r#"id="add-w-tags""#));
    assert!(html.contains(r#"id="add-y-from""#));
    assert!(html.contains(r#"id="add-g-area""#));
    assert!(html.contains(r#"id="add-a-surface""#));

    let st = fixture_state();
    let goals_html = templates().render("goals", &goals::model(&st)).unwrap();
    assert!(goals_html.contains(r#"id="add-node-modal""#));
}

const OVERVIEW_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:0000000000000000000000000000000000000000000000000000000000000000
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01
  fitness_target: 4
  fitness_current: 1

g G-02 status=verified fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal two"
  area: A-01
  fitness_current: true

w W-01 type=feature status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work one"
  goals: G-01

w W-02 type=bug status=ready cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work two"

w W-03 type=bug status=progress cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work three"

w W-04 type=bug status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work four"

w W-05 type=bug status=rejected cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Work five"

q Q-01 status=open cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Question one"
q Q-02 status=answered cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Question two"

b B-01 status=proposed cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bet one"
b B-02 status=testing cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bet two"
b B-03 status=validated cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bet three"
b B-04 status=invalidated_blocking cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bet four"

y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Probe one"
y Y-02 status=stale t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Probe two"

:archive
w W-09 type=bug status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Archived work"
"#;

fn overview_fixture_model() -> serde_json::Value {
    let st = fixture_state();
    overview::model(&st, &[], "2026-07-27T00:00:00Z")
}

fn overview_temp_root(tag: &str) -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    let (guard, _home) = common::isolated_grove_home(tag);
    let dir = std::env::temp_dir().join(format!(
        "grove-desktop-overview-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".grove")).unwrap();
    std::fs::write(dir.join(".grove").join("state.lock"), OVERVIEW_LOCK).unwrap();
    let r = grove_core::run_cli(&[
        "repair".to_string(),
        "--confirm".to_string(),
        format!("--root={}", dir.display()),
    ]);
    assert_eq!(r.code, grove_core::EXIT_OK, "repair failed: {}", r.err);
    (guard, dir)
}

fn work_segment(m: &serde_json::Value, status: &str) -> serde_json::Value {
    m["work"]["segments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["status"] == status)
        .unwrap_or_else(|| panic!("work segment {status}"))
        .clone()
}

#[test]
fn overview_model_content_health_and_flat_trend() {
    let m = overview_fixture_model();
    assert_eq!(m["content"]["c"], 3);
    assert_eq!(m["content"]["v"], 6);
    assert_eq!(m["content"]["ratio"], "0.50");
    assert_eq!(m["content"]["trend_points"], 1);
    assert_eq!(m["content"]["trend_delta"], 0);
    assert_eq!(m["content"]["trend_dir"], "flat");
    assert_eq!(m["content"]["trend_variant"], "neutral");
    assert_eq!(m["content"]["trend_label"], "0 over last 1 point");
    assert_eq!(m["content"]["spark_c"], "0.0,14.0 120.0,14.0");
    assert_eq!(m["content"]["spark_v"], "0.0,1.0 120.0,1.0");
}

#[test]
fn overview_model_work_segments() {
    let m = overview_fixture_model();
    assert_eq!(m["work"]["open"], 3);
    let segs = m["work"]["segments"].as_array().unwrap();
    let order: Vec<&str> = segs.iter().map(|s| s["status"].as_str().unwrap()).collect();
    assert_eq!(order, ["proposed", "ready", "progress", "done", "rejected"]);
    assert_eq!(work_segment(&m, "proposed")["count"], 1);
    assert_eq!(work_segment(&m, "proposed")["valueText"], "1");
    assert_eq!(work_segment(&m, "proposed")["variant"], "neutral");
    assert_eq!(work_segment(&m, "ready")["count"], 1);
    assert_eq!(work_segment(&m, "ready")["variant"], "info");
    assert_eq!(work_segment(&m, "progress")["count"], 1);
    assert_eq!(work_segment(&m, "progress")["variant"], "accent");
    assert_eq!(work_segment(&m, "done")["count"], 1);
    assert_eq!(work_segment(&m, "done")["variant"], "success");
    assert_eq!(work_segment(&m, "rejected")["count"], 0);
    assert_eq!(work_segment(&m, "rejected")["variant"], "danger");
    assert_eq!(work_segment(&m, "rejected")["widthPct"], 0);
    let pct = work_segment(&m, "done")["widthPct"].as_f64().unwrap();
    assert!((pct - 25.0).abs() < 1e-9, "done widthPct {pct}");
    assert!(m["work"].get("rows").is_none());
    assert!(m["work"].get("wip").is_none());
}

#[test]
fn overview_model_goal_rows() {
    let m = overview_fixture_model();
    let goals = m["goals"].as_array().unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals[0]["id"], "G-01");
    assert_eq!(goals[0]["title"], "Ship the API");
    assert_eq!(goals[0]["fitness_label"], "1 / 4");
    assert_eq!(goals[1]["id"], "G-02");
    assert_eq!(goals[1]["title"], "Dogfood the loop");
    assert_eq!(goals[1]["fitness_label"], "3 / 3");
    for g in goals {
        for dropped in ["status", "status_variant", "has_bar", "percent"] {
            assert!(g.get(dropped).is_none(), "goal still exposes {dropped}");
        }
    }
    assert_eq!(m["goals_count"], 2);
    assert_eq!(m["goals_empty"], false);
}

#[test]
fn overview_model_recent_discoveries_capped_and_sorted() {
    let m = overview_fixture_model();
    let items = m["discovery_items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "Y-01");
    assert_eq!(items[0]["title"], "Active probe");
    assert_eq!(items[1]["id"], "Y-02");
    assert_eq!(m["discovery_empty"], false);
    assert!(m["discovery"].is_null(), "old discovery counts gone");

    let lock = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture
y Y-01 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-20T00:00:00Z "P1"
y Y-02 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-21T00:00:00Z "P2"
y Y-03 status=stale t_created=2026-07-27T00:00:00Z t_updated=2026-07-22T00:00:00Z "P3"
y Y-04 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-23T00:00:00Z "P4"
y Y-05 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-24T00:00:00Z "P5"
y Y-06 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-25T00:00:00Z "P6"
y Y-07 status=active t_created=2026-07-27T00:00:00Z t_updated=2026-07-26T00:00:00Z "P7"
"#;
    let st = parse_fixture(lock).expect("fixture parses");
    let m = overview::model(&st, &[], "2026-07-27T00:00:00Z");
    let ids: Vec<&str> = m["discovery_items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["Y-07", "Y-06", "Y-05", "Y-04", "Y-03", "Y-02"]);
}

#[test]
fn overview_model_nav_cards() {
    let m = overview_fixture_model();
    let cards = m["nav_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 7);
    let routes: Vec<&str> = cards
        .iter()
        .map(|c| c["route"].as_str().unwrap())
        .collect();
    assert_eq!(
        routes,
        ["areas", "discovery", "goals", "work", "themes", "graph", "packet"]
    );
    let icons = ui_dir().join("icons");
    for c in cards {
        assert!(!c["title"].as_str().unwrap().is_empty());
        assert!(!c["description"].as_str().unwrap().is_empty());
        let icon = c["icon_name"].as_str().unwrap();
        assert!(
            icons.join(format!("{icon}.svg")).exists(),
            "missing icon {icon}.svg"
        );
    }
}

#[test]
fn overview_status_bar_model_counts() {
    let st = fixture_state();
    let m = overview::status_bar_model(&st, Some(std::time::SystemTime::now()), Some(2048));
    assert_eq!(m["c"], 3);
    assert_eq!(m["v"], 6);
    assert_eq!(m["g"], 2);
    assert_eq!(m["ready"], 1);
    assert_eq!(m["done"], 1);
    assert_eq!(m["lock"], "2.0 KB");
    assert_eq!(m["updated"], "just now");
}

#[test]
fn overview_status_bar_lock_size_label() {
    assert_eq!(overview::lock_size_label(None), "n/a");
    assert_eq!(overview::lock_size_label(Some(512)), "512 B");
    assert_eq!(overview::lock_size_label(Some(1024)), "1.0 KB");
    assert_eq!(overview::lock_size_label(Some(1536)), "1.5 KB");
    assert_eq!(overview::lock_size_label(Some(3 * 1024 * 1024)), "3.0 MB");
}

#[test]
fn overview_status_bar_updated_label_edges() {
    let now = std::time::SystemTime::now();
    let at = |secs: u64| Some(now - std::time::Duration::from_secs(secs));
    assert_eq!(overview::updated_label(now, at(5)), "just now");
    assert_eq!(overview::updated_label(now, at(45)), "45s ago");
    assert_eq!(overview::updated_label(now, at(12 * 60)), "12m ago");
    assert_eq!(overview::updated_label(now, at(3 * 3600)), "3h ago");
    assert_eq!(overview::updated_label(now, at(2 * 86400)), "2d ago");
    assert_eq!(overview::updated_label(now, None), "n/a");

    let st = fixture_state();
    assert_eq!(overview::status_bar_model(&st, None, None)["updated"], "n/a");
    assert_eq!(overview::status_bar_model(&st, None, None)["lock"], "n/a");
}

#[test]
fn overview_trend_windows_last_30_and_direction() {
    let series: Vec<(String, i64, i64)> = (0..35)
        .map(|i| (format!("2026-07-{:02}T00:00:00Z", i + 1), i, 0))
        .collect();
    let t = overview::trend_from_series(&series);
    assert_eq!(t.points, 30);
    assert_eq!(t.delta, 29);
    assert_eq!(t.dir, "up");
    assert_eq!(t.variant, "success");
    assert_eq!(t.label, "+29 over last 30 points");
    assert_eq!(t.spark_c.split(' ').count(), 30);
    assert!(t.spark_c.starts_with("0.0,23.2"), "{}", t.spark_c);
    assert!(t.spark_c.ends_with("120.0,1.0"), "{}", t.spark_c);

    let down: Vec<(String, i64, i64)> = (0..5).map(|i| ("t".to_string(), 10 - i, 0)).collect();
    let t = overview::trend_from_series(&down);
    assert_eq!(t.delta, -4);
    assert_eq!(t.dir, "down");
    assert_eq!(t.variant, "danger");
    assert_eq!(t.label, "-4 over last 5 points");

    let single = overview::trend_from_series(&[("t".to_string(), 2, 2)]);
    assert_eq!(single.points, 1);
    assert_eq!(single.dir, "flat");
    assert_eq!(single.label, "0 over last 1 point");
    assert_eq!(single.spark_c, "0.0,1.0 120.0,1.0");

    let empty = overview::trend_from_series(&[]);
    assert_eq!(empty.points, 0);
    assert_eq!(empty.label, "no history");
}

#[test]
fn overview_fragment_renders_cards_and_deep_links() {
    let m = overview_fixture_model();
    let html = templates().render("overview", &m).unwrap();
    assert!(html.contains(r#"class="view view-overview""#));
    for id in [
        "overview-content",
        "overview-work",
        "overview-goals",
        "overview-discovery",
    ] {
        assert!(html.contains(&format!(r#"id="{id}""#)), "card {id}");
    }
    let order: Vec<usize> = [
        r#"class="overview-grid""#,
        r#"id="overview-content""#,
        r#"id="overview-work""#,
        r#"id="overview-goals""#,
        r#"id="overview-discovery""#,
        r#"class="nav-cards""#,
    ]
    .iter()
    .map(|n| html.find(n).unwrap_or_else(|| panic!("missing {n}")))
    .collect();
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "bento 2x2 grid: content+work row 1, goals+discovery row 2, nav-cards last"
    );
    assert!(
        !html.contains("overview-col"),
        "column wrappers removed in favor of the 2x2 grid"
    );
    for level in ["discovery", "goals", "themes"] {
        assert!(
            html.contains(&format!(r#"data-action="goto" data-level="{level}""#)),
            "goto {level}"
        );
    }
    assert!(html.contains(r#"class="overview-spark""#));
    assert!(html.contains("Content (C)"));
    assert!(html.contains("Uncertainty (V)"));
    assert!(html.contains("Content (C) / Uncertainty (V) ratio"));
    assert!(
        html.contains(r#"<span class="overview-k">Content (C) trend</span><span class="overview-v">0 over last 1 point</span>"#),
        "trend renders as plain text"
    );
    assert!(!html.contains(r#">0 over last 1 point</span></span>"#), "no trend badge");
    assert!(
        html.contains(r#"<span class="overview-figure-label"><span class="overview-swatch overview-swatch-c" aria-hidden="true"></span>Content (C)</span>"#),
        "C label carries decorative spark-c swatch"
    );
    assert!(
        html.contains(r#"<span class="overview-k"><span class="overview-swatch overview-swatch-v" aria-hidden="true"></span>Uncertainty (V)</span>"#),
        "V label carries decorative spark-v swatch"
    );
    assert_eq!(html.matches("overview-swatch ").count(), 2, "exactly two swatches");
    assert!(
        html.contains(r#"<span class="overview-k">Content (C) / Uncertainty (V) ratio</span>"#),
        "ratio row has no swatch"
    );
    assert!(html.contains("0.50"));
    assert_eq!(
        html.matches("segmented-bar-legend-item").count(),
        5,
        "legend keeps zero-count statuses"
    );
    assert_eq!(
        html.matches("segmented-bar-segment").count(),
        4,
        "zero-count status gets no track segment"
    );
    assert!(html.contains("segmented-bar-c-accent"));
    assert!(html.contains("Ship the API"));
    assert!(html.contains("(1 / 4)"));
    assert!(!html.contains("fitness-bar"), "goal progress bars removed");
    assert!(!html.contains(r#"<span class="badge badge-warning">unverified</span>"#));
    assert!(html.contains("Active probe"));
    assert!(html.contains("Stale probe"));
    assert!(!html.contains("invalidated blocking B"));
    assert_eq!(html.matches(r#"class="nav-card""#).count(), 7, "nav cards");
    for route in [
        "areas",
        "discovery",
        "goals",
        "work",
        "themes",
        "graph",
        "packet",
    ] {
        assert!(
            html.contains(r#"class="nav-card" title="#)
                && html.contains(&format!(r#"data-level="{route}""#)),
            "nav card {route}"
        );
    }
    assert!(html.contains("Coverage and health per knowledge area."));
    assert!(html.contains(r#"id="add-node-modal""#));
}

#[test]
fn overview_fragment_discovery_empty_state() {
    let mut m = overview_fixture_model();
    m["discovery_items"] = serde_json::json!([]);
    m["discovery_empty"] = serde_json::json!(true);
    let html = templates().render("overview", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state empty-state-compact">"#));
    assert!(html.contains(r#"width="32" height="32""#), "compact ghost icon");
    assert!(html.contains("No discoveries yet."));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn overview_fragment_goals_empty_state() {
    let mut m = overview_fixture_model();
    m["goals"] = serde_json::json!([]);
    m["goals_empty"] = serde_json::json!(true);
    let html = templates().render("overview", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state empty-state-compact">"#));
    assert!(html.contains("No goals in the lock."));
}

#[test]
fn overview_render_on_temp_root_matches_hand_built_lock() {
    let (guard, dir) = overview_temp_root("render");
    let root = dir.to_string_lossy().into_owned();
    let st = grove_desktop_lib::views::load_state(&root).unwrap();
    let m = overview::model(&st, &[], "2026-07-27T00:00:00Z");

    assert_eq!(m["content"]["c"], 3);
    assert_eq!(m["content"]["v"], 6);
    assert_eq!(m["content"]["ratio"], "0.50");
    assert_eq!(m["work"]["open"], 3);
    assert_eq!(work_segment(&m, "proposed")["count"], 1);
    assert_eq!(work_segment(&m, "ready")["count"], 1);
    assert_eq!(work_segment(&m, "progress")["count"], 1);
    assert_eq!(work_segment(&m, "done")["count"], 1);
    assert_eq!(work_segment(&m, "rejected")["count"], 1);

    let goals = m["goals"].as_array().unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals[0]["id"], "G-01");
    assert_eq!(goals[0]["fitness_label"], "1 / 4");
    assert_eq!(goals[1]["id"], "G-02");
    assert_eq!(goals[1]["fitness_label"], "boolean: true");

    let items = m["discovery_items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["id"], "Y-01");
    assert_eq!(items[1]["id"], "Y-02");

    let sb = overview::status_bar_model(&st, Some(std::time::SystemTime::now()), Some(100));
    assert_eq!(sb["c"], 3);
    assert_eq!(sb["v"], 6);
    assert_eq!(sb["g"], 2);
    assert_eq!(sb["ready"], 1);
    assert_eq!(sb["done"], 1);
    assert_eq!(sb["updated"], "just now");

    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "overview",
        &serde_json::json!({}),
    )
    .unwrap();
    assert!(html.contains(r#"class="view view-overview""#));
    assert!(html.contains("Goal one"));
    assert!(html.contains("Goal two"));
    assert!(html.contains("Probe one"));
    assert!(html.contains("Probe two"));
    assert!(!html.contains("Archived work"));
    drop(guard);
}

const WORK_LOCK: &str = r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:0000000000000000000000000000000000000000000000000000000000000000
a A-01 status=present t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Area"

g G-01 status=unverified fitness_kind=count t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01
  fitness_target: 2
  fitness_current: 1

g G-02 status=partial fitness_kind=boolean t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal two"
  area: A-01
  fitness_current: true

w W-01 status=ready cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Untyped ready work"
  goals: G-01
  fitness: G-01=+1
  ac:
    | first acceptance
  evidence_strategy:
    | run the tests

w W-02 type=feature status=proposed cynefin=complex t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Feature proposed"
  goals: G-01, G-02

w W-03 type=bug status=progress cynefin=complicated t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Bug in progress"
  goals: G-02

w W-04 type=refactor status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Refactor done"
  goals: G-01

w W-05 type=spike status=rejected cynefin=chaotic t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Spike rejected"
  goals: G-01

e W-01 blocks W-03 t_created=2026-07-27T00:00:00Z

:archive
w W-09 type=feature status=done cynefin=clear t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Archived work"
  goals: G-01
"#;

fn work_temp_root() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
    let (guard, _home) = common::isolated_grove_home("work");
    let dir = std::env::temp_dir().join(format!(
        "grove-desktop-work-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join(".grove")).unwrap();
    std::fs::write(dir.join(".grove").join("state.lock"), WORK_LOCK).unwrap();
    let r = grove_core::run_cli(&[
        "repair".to_string(),
        "--confirm".to_string(),
        format!("--root={}", dir.display()),
    ]);
    assert_eq!(r.code, grove_core::EXIT_OK, "repair failed: {}", r.err);
    (guard, dir)
}

fn work_ids(m: &serde_json::Value) -> Vec<&str> {
    m["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect()
}

fn work_filter<'a>(m: &'a serde_json::Value, status: &str) -> &'a serde_json::Value {
    m["filters"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["status"] == status)
        .unwrap_or_else(|| panic!("filter tab {status}"))
}

#[test]
fn work_model_mirrors_render_work_semantics() {
    let (guard, dir) = work_temp_root();
    let root = dir.to_string_lossy().into_owned();
    let st = grove_desktop_lib::views::load_state(&root).unwrap();
    let m = work::model(&st, "all", false);
    assert_eq!(work_ids(&m), ["W-01", "W-02", "W-03", "W-04", "W-05"]);
    assert_eq!(m["total"], 5);
    assert_eq!(m["shown"], 5);
    assert_eq!(m["empty"], false);
    assert_eq!(m["filter"], "all");
    assert_eq!(m["include_archived"], false);

    let rows = m["rows"].as_array().unwrap();
    let w1 = &rows[0];
    assert_eq!(w1["wtype"], "nothing");
    assert_eq!(w1["title"], "Untyped ready work");
    assert_eq!(w1["goals"], "G-01");
    assert_eq!(w1["cynefin"], "clear");
    assert_eq!(w1["dor"], "\u{22a4}");
    assert_eq!(w1["status"], "ready");
    assert_eq!(w1["status_variant"], "info");
    assert_eq!(w1["critical"], "\u{2605}");
    assert_eq!(w1["archived"], false);

    let w2 = &rows[1];
    assert_eq!(w2["wtype"], "feature");
    assert_eq!(w2["goals"], "G-01, G-02");
    assert_eq!(w2["dor"], "\u{22a5}");
    assert_eq!(w2["status_variant"], "neutral");
    assert_eq!(w2["critical"], "");

    let w3 = &rows[2];
    assert_eq!(w3["status_variant"], "accent");
    assert_eq!(w3["critical"], "\u{2605}");
    assert_eq!(w3["dor"], "\u{22a5}");

    assert_eq!(rows[3]["status_variant"], "success");
    assert_eq!(rows[3]["critical"], "");
    assert_eq!(rows[4]["status_variant"], "danger");
    assert_eq!(rows[4]["cynefin"], "chaotic");

    let filters = m["filters"].as_array().unwrap();
    assert_eq!(filters.len(), 6);
    assert_eq!(work_filter(&m, "all")["count"], 5);
    assert_eq!(work_filter(&m, "all")["active"], true);
    assert_eq!(work_filter(&m, "all")["variant"], "neutral");
    assert_eq!(work_filter(&m, "proposed")["count"], 1);
    assert_eq!(work_filter(&m, "proposed")["active"], false);
    assert_eq!(work_filter(&m, "proposed")["variant"], "neutral");
    assert_eq!(work_filter(&m, "ready")["count"], 1);
    assert_eq!(work_filter(&m, "ready")["variant"], "info");
    assert_eq!(work_filter(&m, "progress")["count"], 1);
    assert_eq!(work_filter(&m, "progress")["variant"], "accent");
    assert_eq!(work_filter(&m, "done")["count"], 1);
    assert_eq!(work_filter(&m, "done")["variant"], "success");
    assert_eq!(work_filter(&m, "rejected")["count"], 1);
    assert_eq!(work_filter(&m, "rejected")["variant"], "danger");

    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "work",
        &serde_json::json!({}),
    )
    .unwrap();
    assert!(html.contains(r#"class="view view-work""#));
    assert!(html.contains(r#"<tr data-id="W-01">"#));

    let html = grove_desktop_lib::views::render_view(
        &templates(),
        &root,
        "work",
        &serde_json::json!({ "status": "progress" }),
    )
    .unwrap();
    assert!(html.contains(r#"<tr data-id="W-03">"#));
    assert!(!html.contains(r#"<tr data-id="W-01">"#));
    assert!(html.contains("Track work items from proposal to completion."));
    drop(guard);
}

#[test]
fn work_model_filters_status_and_archived_toggle() {
    let st = parse_fixture(WORK_LOCK).expect("work fixture parses");

    let m = work::model(&st, "progress", false);
    assert_eq!(work_ids(&m), ["W-03"]);
    assert_eq!(m["shown"], 1);
    assert_eq!(m["total"], 5);
    assert_eq!(m["filter"], "progress");
    assert_eq!(work_filter(&m, "progress")["active"], true);
    assert_eq!(work_filter(&m, "all")["active"], false);
    for f in m["filters"].as_array().unwrap() {
        assert!(f.get("label").is_none(), "work tabs keep bare status text");
    }

    let m = work::model(&st, "bogus", false);
    assert_eq!(m["filter"], "all");
    assert_eq!(work_ids(&m).len(), 5);

    let m = work::model(&st, "all", true);
    assert_eq!(
        work_ids(&m),
        ["W-01", "W-02", "W-03", "W-04", "W-05", "W-09"]
    );
    assert_eq!(m["include_archived"], true);
    let rows = m["rows"].as_array().unwrap();
    let w9 = &rows[5];
    assert_eq!(w9["archived"], true);
    assert_eq!(w9["status"], "done");
    assert_eq!(w9["title"], "Archived work");

    let m = work::model(&st, "done", true);
    assert_eq!(work_ids(&m), ["W-04", "W-09"]);

    let m = work::model(&st, "done", false);
    assert_eq!(work_ids(&m), ["W-04"]);

    let m = work::model(&st, "rejected", false);
    assert_eq!(work_ids(&m), ["W-05"]);
}

#[test]
fn work_fragment_renders_table_filter_tabs_and_rows() {
    let st = parse_fixture(WORK_LOCK).expect("work fixture parses");
    let html = templates()
        .render("work", &work::model(&st, "all", false))
        .unwrap();
    assert!(html.contains(r#"class="view view-work""#));
    for h in [
        "ID", "Type", "Title", "Goals", "Cynefin", "DoR", "Status", "Critical",
    ] {
        assert!(html.contains(&format!(">{h}</th>")), "header {h}");
    }
    for s in ["all", "proposed", "ready", "progress", "done", "rejected"] {
        assert!(
            html.contains(&format!(r#"data-action="filter" data-status="{s}""#)),
            "tab {s}"
        );
    }
    assert_eq!(html.matches(r#"role="tab""#).count(), 6);
    assert!(html.contains(
        r#"data-action="filter" data-status="all" aria-selected="true">all<span class="filter-tab-count">5</span>"#
    ));
    for s in ["proposed", "ready", "progress", "done", "rejected"] {
        assert!(
            html.contains(&format!(
                r#"data-status="{s}" aria-selected="false">{s}<span class="filter-tab-count">1</span>"#
            )),
            "tab {s} count"
        );
    }
    let i_header = html.find(r#"class="page-header""#).unwrap();
    let i_bar = html.find(r#"class="filter-bar""#).unwrap();
    let i_table = html.find(r#"class="work-table"#).unwrap();
    assert!(
        i_header < i_bar && i_bar < i_table,
        "filter row sits under the header above the table"
    );
    let bar_end = html[i_bar..].find("</div>").map(|e| i_bar + e).unwrap();
    let bar = &html[i_bar..bar_end];
    assert!(bar.contains(r#"role="tablist""#));
    assert!(bar.contains(r#"aria-label="Work status""#));
    assert!(bar.contains(r#"id="work-archived""#));
    assert!(bar.contains("Include archived"));
    assert!(!html.contains("page-actions"));
    assert!(!html.contains("work-toolbar"));
    assert!(!html.contains("work-filters"));
    assert!(!html.contains("work-chip"));
    assert!(html.contains(r#"<tr data-id="W-01">"#));
    assert!(html.contains(r#"<tr data-id="W-04">"#));
    assert!(html.contains(r#"<td><span class="text-mono">W-01</span></td>"#));
    assert!(!html.contains(r#"data-action="goto""#));
    assert!(
        !html.contains("5 of 5 shown"),
        "shown counters removed from the static description"
    );
    assert!(html.contains("nothing"));
    assert!(html.contains("\u{22a4}"));
    assert!(html.contains("\u{22a5}"));
    assert!(html.contains("\u{2605}"));
    assert!(html.contains(r#"<span class="badge badge-info capitalize" id="W-01">ready</span>"#));
    assert!(html.contains(r#"<span class="badge badge-neutral capitalize" id="W-02">proposed</span>"#));
    assert!(html.contains(r#"<span class="badge badge-accent capitalize" id="W-03">progress</span>"#));
    assert!(html.contains(r#"<span class="badge badge-success capitalize" id="W-04">done</span>"#));
    assert!(html.contains(r#"<span class="badge badge-danger capitalize" id="W-05">rejected</span>"#));
    assert!(!html.contains("W-09"));
    assert!(!html.contains("Archived work"));
    assert!(html.contains(r#"id="add-node-modal""#));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn work_fragment_empty_state() {
    let st = parse_fixture(
        r#"@grove 1
# AUTO-GENERATED. Do not edit. Use `grove` CLI.
# checksum: sha256:fixture

g G-01 status=unverified t_created=2026-07-27T00:00:00Z t_updated=2026-07-27T00:00:00Z "Goal one"
  area: A-01
"#,
    )
    .expect("fixture parses");
    let m = work::model(&st, "all", false);
    assert_eq!(m["empty"], true);
    assert_eq!(m["total"], 0);
    let html = templates().render("work", &m).unwrap();
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains(r#"<p class="empty-state-title">No work items</p>"#));
    assert!(html.contains("No work items match this filter."));
    assert!(!html.contains("alert-info"), "alert replaced by empty-state");
    assert!(html.contains(r#"role="tablist""#));
    assert!(html.contains(
        r#"data-action="filter" data-status="all" aria-selected="true">all<span class="filter-tab-count">0</span>"#
    ));
    assert!(html.contains(r#"id="work-archived""#));
    assert!(html.contains(r#"id="add-node-modal""#));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn project_fragment_renders_no_project_state() {
    let html = project::render(&templates()).unwrap();
    assert!(html.contains(r#"class="view view-project""#));
    assert!(html.contains(r#"<div class="empty-state">"#));
    assert!(html.contains(r#"class="icon empty-state-icon""#));
    assert!(html.contains(r#"viewBox="0 0 72 75""#), "ghost icon svg inlined");
    assert!(html.contains(r#"<p class="empty-state-title">No project selected</p>"#));
    assert!(html.contains("Open an existing project or create a new one"));
    assert_eq!(
        html.matches(r#"data-action="project-picker""#).count(),
        2,
        "open and create buttons"
    );
    assert!(html.contains(r#"data-action="project-picker" data-mode="open""#));
    assert!(html.contains(r#"data-action="project-picker" data-mode="create""#));
    assert!(html.contains(">Open project</button>"));
    assert!(html.contains(">Create project</button>"));
    assert!(!html.contains('\u{b7}'));
    assert!(!html.contains('\u{2013}'));
    assert!(!html.contains('\u{2014}'));
}

#[test]
fn project_rail_button_and_gating_wiring() {
    let html = std::fs::read_to_string(ui_dir().join("index.html")).unwrap();
    assert_eq!(
        html.matches(r#"<li class="side-rail-item" data-level="#).count(),
        8,
        "eight route items"
    );
    let item = html
        .split(r#"<li class="side-rail-item side-rail-action side-rail-project">"#)
        .nth(1)
        .and_then(|rest| rest.split("</li>").next())
        .expect("project rail item present");
    assert!(item.contains(r#"id="project-open""#));
    assert!(item.contains(r#"aria-label="Project""#));
    assert!(item.contains(r#"data-action="project-picker""#));
    assert!(item.contains(r#"id="project-avatar""#), "W-50 avatar swap container");
    assert!(item.contains("nav-icon-tree"));

    let js = std::fs::read_to_string(ui_dir().join("js").join("main.js")).unwrap();
    assert!(js.contains("grove:project-picker"), "picker hook dispatched");
    assert!(js.contains("grove:project-changed"), "lifecycle hook observed");
    assert!(js.contains("refreshProjectState"));
    assert!(js.contains("aria-disabled"));
    assert!(js.contains("side-rail-item.disabled"), "clicks blocked centrally");

    let tree = std::fs::read_to_string(ui_dir().join("icons").join("tree.svg")).unwrap();
    assert!(tree.contains(r#"viewBox="0 0 19 25""#));
    assert!(tree.contains(r#"fill="currentColor""#));
    assert!(!tree.contains(r#"fill="black""#), "normalized to currentColor");
    let manifest =
        std::fs::read_to_string(ui_dir().join("..").join("tools").join("icons.manifest.json"))
            .unwrap();
    assert!(manifest.contains("\"tree\""), "tree in the icon manifest");
}

#[test]
fn command_menu_component_wiring() {
    let html = std::fs::read_to_string(ui_dir().join("index.html")).unwrap();
    assert!(html.contains(r#"href="/css/components/command-menu.css""#));
    assert!(html.contains(r#"href="/css/components/search-input.css""#));

    let css = std::fs::read_to_string(
        ui_dir()
            .join("css")
            .join("components")
            .join("command-menu.css"),
    )
    .unwrap();
    for token in [
        ".command-menu-panel",
        ".command-menu-input",
        ".command-menu-browse",
        ".command-menu-section-title",
        ".command-menu-item",
        ".command-menu-item.active",
        ".command-menu-item-hint",
        ".command-menu-item-avatar",
        ".command-menu-empty",
        "--wv-color-accent-bg",
        "--wv-color-text-tertiary",
        "width: 460px",
        "width: 32px",
    ] {
        assert!(css.contains(token), "command-menu.css: {token}");
    }
    assert!(
        !css.contains("--wv-font-size-xxs"),
        "no xxs anywhere in the menu"
    );
    assert!(
        !css.contains(".command-menu-icon-clock"),
        "clock header icon dropped"
    );
    for slug in ["search", "folder-open", "add", "cancel", "tree"] {
        assert!(
            css.contains(&format!(".command-menu-icon-{slug}"))
                && css.contains(&format!(r#"url("/icons/{slug}.svg")"#)),
            "icon class + mask for {slug}"
        );
        assert!(
            ui_dir().join("icons").join(format!("{slug}.svg")).exists(),
            "{slug}.svg present"
        );
    }
    let cancel = std::fs::read_to_string(ui_dir().join("icons").join("cancel.svg")).unwrap();
    assert!(cancel.contains(r#"fill="currentColor""#));
    let manifest =
        std::fs::read_to_string(ui_dir().join("..").join("tools").join("icons.manifest.json"))
            .unwrap();
    assert!(manifest.contains("\"cancel\""), "cancel in the icon manifest");

    let search_css = std::fs::read_to_string(
        ui_dir()
            .join("css")
            .join("components")
            .join("search-input.css"),
    )
    .unwrap();
    for token in [
        ".search-input",
        ".search-input-icon",
        "position: absolute",
        "padding-left",
    ] {
        assert!(search_css.contains(token), "search-input.css: {token}");
    }

    let js = std::fs::read_to_string(
        ui_dir().join("js").join("utils").join("command-menu.js"),
    )
    .unwrap();
    for token in [
        "openCommandMenu",
        "searchInput",
        "search-input",
        "setSections",
        "showForm",
        "showError",
        "close",
        "ArrowDown",
        "ArrowUp",
        "Enter",
        "Escape",
        "listbox",
        "aria-selected",
    ] {
        assert!(js.contains(token), "command-menu.js: {token}");
    }
    for banned in ["project", "grove_", "Project"] {
        assert!(!js.contains(banned), "component stays pure: {banned}");
    }
}

#[test]
fn project_picker_wiring() {
    let picker = std::fs::read_to_string(
        ui_dir()
            .join("js")
            .join("integration")
            .join("project-picker.js"),
    )
    .unwrap();
    for token in [
        "openProjectPicker",
        "openCommandMenu",
        "grove_projects_list",
        "grove_project_open",
        "grove_project_create",
        "grove_project_close",
        "grove_pick_directory",
        "grove:project-changed",
        "Open project...",
        "Create project...",
        "Close current project",
        "Recent projects",
        "current",
        "folder-open",
        "browse: pickDirectory",
        "if (currentPath)",
        ".grove/state.lock",
    ] {
        assert!(picker.contains(token), "project-picker.js: {token}");
    }
    assert!(
        !picker.contains(r#"icon: "clock""#),
        "recents header carries no icon"
    );

    let main = std::fs::read_to_string(ui_dir().join("js").join("main.js")).unwrap();
    assert!(main.contains("project-picker.js"), "main.js imports the picker");
    assert!(main.contains("openProjectPicker"));
    assert!(
        !main.contains("project picker requested"),
        "stub listener replaced"
    );

    let icon = std::fs::read_to_string(ui_dir().join("icons").join("folder-open.svg")).unwrap();
    assert!(icon.contains(r#"viewBox="0 0 24 24""#));
    assert!(icon.contains(r#"fill="currentColor""#));
    assert!(!icon.contains(r#"fill="black""#), "normalized to currentColor");
    let manifest =
        std::fs::read_to_string(ui_dir().join("..").join("tools").join("icons.manifest.json"))
            .unwrap();
    assert!(manifest.contains("\"folder-open\""), "folder-open in the icon manifest");
}

#[test]
fn directory_picker_wiring() {
    let cargo = std::fs::read_to_string(
        ui_dir().join("..").join("src-tauri").join("Cargo.toml"),
    )
    .unwrap();
    assert!(
        cargo.contains("tauri-plugin-dialog"),
        "dialog plugin dependency declared"
    );

    let lib = std::fs::read_to_string(
        ui_dir().join("..").join("src-tauri").join("src").join("lib.rs"),
    )
    .unwrap();
    assert!(
        lib.contains("tauri_plugin_dialog::init()"),
        "plugin registered on the builder"
    );
    assert!(
        lib.contains("commands::grove_pick_directory"),
        "command in the invoke handler"
    );

    let cmds = std::fs::read_to_string(
        ui_dir()
            .join("..")
            .join("src-tauri")
            .join("src")
            .join("commands")
            .join("mod.rs"),
    )
    .unwrap();
    for token in [
        "grove_pick_directory",
        "DialogExt",
        "pick_folder",
        "spawn_blocking",
        "Option<String>",
    ] {
        assert!(cmds.contains(token), "commands/mod.rs: {token}");
    }
    assert!(
        !cmds.contains("blocking_pick_folder"),
        "directory picker must not block the main thread"
    );

    let picker = std::fs::read_to_string(
        ui_dir()
            .join("js")
            .join("integration")
            .join("project-picker.js"),
    )
    .unwrap();
    assert!(
        picker.contains(r#"tauriBridge.invoke("grove_pick_directory")"#),
        "picker invokes the command"
    );

    let menu = std::fs::read_to_string(
        ui_dir().join("js").join("utils").join("command-menu.js"),
    )
    .unwrap();
    for token in ["browseBtn", "command-menu-browse", "form.browse"] {
        assert!(menu.contains(token), "command-menu.js: {token}");
    }
    assert!(
        !menu.contains("grove_pick_directory"),
        "component stays host-agnostic"
    );
}


#[test]
fn tree_avatar_module_and_wiring() {
    assert!(
        !ui_dir().join("js").join("utils").join("avatar.js").exists(),
        "js generator removed in favor of the rust command"
    );

    let src_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let avatar = std::fs::read_to_string(src_dir.join("avatar.rs")).unwrap();
    for token in [
        "hash_code",
        "get_digit",
        "get_unit",
        "pick_tone",
        "hsl_yiq",
        "tree_avatar_svg",
        "DEEP_TONES",
        "LIGHT_TONES",
        "FAMILY_HUES",
        "viewBox",
        "stroke-width",
        "stroke-linecap",
        "<path",
        "hsl(",
    ] {
        assert!(avatar.contains(token), "avatar.rs: {token}");
    }
    assert!(
        !avatar.contains("export const"),
        "no js API left in the rust module"
    );
    for banned in ['\u{b7}', '\u{2013}', '\u{2014}'] {
        assert!(!avatar.contains(banned), "avatar.rs ascii: {banned}");
    }

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap();
    assert!(lib.contains("pub mod avatar;"), "avatar module declared");
    assert!(
        lib.contains("commands::grove_project_avatar"),
        "command registered in the invoke handler"
    );

    let commands = std::fs::read_to_string(src_dir.join("commands").join("mod.rs")).unwrap();
    assert!(
        commands.contains("pub fn grove_project_avatar(name: String) -> String"),
        "command signature takes the project name only"
    );
    assert!(
        commands.contains("crate::avatar::tree_avatar_svg(&name)"),
        "command delegates to the generator"
    );

    let html = std::fs::read_to_string(ui_dir().join("index.html")).unwrap();
    assert!(
        html.contains(
            r#"<span class="project-avatar" id="project-avatar"><span class="nav-icon nav-icon-tree"></span></span>"#
        ),
        "rootless default tile unchanged"
    );

    let main = std::fs::read_to_string(ui_dir().join("js").join("main.js")).unwrap();
    assert!(!main.contains("utils/avatar.js"), "main.js drops the js generator");
    assert!(!main.contains("treeAvatarSvg"), "main.js drops the js api");
    assert!(main.contains("renderProjectAvatar"));
    assert!(main.contains(r#"document.getElementById("project-avatar")"#));
    assert!(
        main.contains(r#"tauriBridge.invoke("grove_project_avatar", { name })"#),
        "rail avatar invokes the rust command"
    );
    assert!(main.contains("avatarToken"), "stale renders dropped by token");
    assert!(main.contains("nav-icon nav-icon-tree"), "rootless restores the tree tile");

    let picker = std::fs::read_to_string(
        ui_dir()
            .join("js")
            .join("integration")
            .join("project-picker.js"),
    )
    .unwrap();
    assert!(!picker.contains("utils/avatar.js"), "picker drops the js generator");
    assert!(!picker.contains("treeAvatarSvg"), "picker drops the js api");
    assert!(
        picker.contains(r#"tauriBridge.invoke("grove_project_avatar", { name })"#),
        "recents invoke the rust command per row"
    );
    assert!(
        picker.contains("avatar: await projectAvatar(String(r.name ?? \"\"))"),
        "recents seeded by entry name when sections are built"
    );
    assert!(
        picker.contains(r#"badge: r.path === currentPath ? "current" : undefined"#),
        "current badge flow unchanged"
    );

    let rail_css = std::fs::read_to_string(
        ui_dir().join("css").join("components").join("side-rail.css"),
    )
    .unwrap();
    assert!(rail_css.contains(".project-avatar svg"), "rail svg pinned by css");
    assert!(rail_css.contains("width: 28px"), "rail avatar at 28px");

    let menu_css = std::fs::read_to_string(
        ui_dir().join("css").join("components").join("command-menu.css"),
    )
    .unwrap();
    assert!(
        menu_css.contains(".command-menu-item-avatar svg"),
        "recents svg pinned by css"
    );
    assert!(
        menu_css.contains(".command-menu-item-avatar svg {\n  width: 32px;\n  height: 32px;"),
        "recents svg full-bleed at 32px"
    );
    assert!(
        menu_css.contains(".command-menu-item-avatar:has(svg)"),
        "avatar tiles drop the inset frame"
    );

    let menu = std::fs::read_to_string(
        ui_dir().join("js").join("utils").join("command-menu.js"),
    )
    .unwrap();
    assert!(menu.contains("tile.dataset.avatar"), "avatar tiles keep the marker");
    assert!(
        menu.contains(r#"typeof item.avatar === "string""#),
        "string avatars swap the tile content"
    );
}


#[test]
fn dev_debug_tooling_wiring() {
    let main = std::fs::read_to_string(ui_dir().join("js").join("main.js")).unwrap();
    assert_eq!(
        main.matches(r#"document.addEventListener("keydown""#).count(),
        1,
        "Ctrl+Shift+P listener registered once"
    );
    for token in [
        "initCommandPalette",
        "ctrlKey",
        "shiftKey",
        r#"e.key === "P""#,
        "preventDefault",
        "modal-overlay.active",
        r#"openProjectPicker("default", { navigate: loadView })"#,
        "./views/debug.js",
        "isDevEnv() && DEV_ROUTES.has(level)",
    ] {
        assert!(main.contains(token), "main.js: {token}");
    }
    assert!(
        !main.contains("openDevPalette") && !main.contains("devMenu"),
        "standalone dev palette removed"
    );

    let picker =
        std::fs::read_to_string(ui_dir().join("js").join("integration").join("project-picker.js"))
            .unwrap();
    for token in [
        r#"window.workspace?.appEnv === "development""#,
        "Development actions",
        "Debug view",
        "debug-view",
        "opts.navigate",
    ] {
        assert!(picker.contains(token), "project-picker.js: {token}");
    }
    let actions_pos = picker.find(r#"{ title: "Actions", items: actions }"#).unwrap();
    let dev_pos = picker.find(r#""Development actions""#).unwrap();
    let recents_pos = picker.find(r#""Recent projects""#).unwrap();
    assert!(
        actions_pos < dev_pos && dev_pos < recents_pos,
        "Development actions sits between Actions and Recent projects"
    );

    let routes = main
        .split("const ROUTES = new Set([")
        .nth(1)
        .and_then(|rest| rest.split("]);").next())
        .expect("ROUTES set present");
    assert!(!routes.contains("debug"), "debug stays out of ROUTES");
    let dev_routes = main
        .split("const DEV_ROUTES = new Set([")
        .nth(1)
        .and_then(|rest| rest.split("]);").next())
        .expect("DEV_ROUTES set present");
    assert!(dev_routes.contains(r#""debug""#), "debug gated dev-side only");

    let debug = std::fs::read_to_string(ui_dir().join("js").join("views").join("debug.js")).unwrap();
    for token in [
        "renderDebug",
        "grove_project_current",
        "grove_projects_list",
        "grove_status_metrics",
        "grove_session_present",
        "escapeHtml",
        "view-debug",
        "<h1>Debug</h1>",
        "appEnv",
        "Registry recents",
        "present",
        "absent",
        "none",
        "Development-only diagnostics",
    ] {
        assert!(debug.contains(token), "debug.js: {token}");
    }
    assert!(
        !debug.contains("session:"),
        "diagnostics expose presence only, never the token"
    );
    for banned in ['\u{b7}', '\u{2013}', '\u{2014}'] {
        assert!(!debug.contains(banned), "debug.js ascii: {banned}");
        assert!(!main.contains(banned), "main.js ascii: {banned}");
    }

    let css = std::fs::read_to_string(
        ui_dir()
            .join("css")
            .join("components")
            .join("command-menu.css"),
    )
    .unwrap();
    assert!(css.contains(".command-menu-icon-bug"));
    assert!(css.contains(r#"url("/icons/bug.svg")"#));
    let icon = std::fs::read_to_string(ui_dir().join("icons").join("bug.svg")).unwrap();
    assert!(icon.contains(r#"fill="currentColor""#));
    let manifest =
        std::fs::read_to_string(ui_dir().join("..").join("tools").join("icons.manifest.json"))
            .unwrap();
    assert!(manifest.contains("\"bug\""), "bug in the icon manifest");

    let html = std::fs::read_to_string(ui_dir().join("index.html")).unwrap();
    assert!(!html.contains("debug"), "index.html carries no debug markup");
    assert!(!html.contains("Debug"));
    assert!(
        !ui_dir().join("views").join("debug.hbs").exists(),
        "no server-rendered debug template"
    );
}
