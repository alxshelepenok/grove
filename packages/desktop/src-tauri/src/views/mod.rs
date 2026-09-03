pub mod areas;
pub mod cone;
pub mod discovery;
pub mod goals;
pub mod graph;
pub mod overview;
pub mod packet;
pub mod project;
pub mod themes;
pub mod work;

use crate::templates::Templates;
use grove_core::{load, CliCtx, Kind, State};

pub const LEVELS: [&str; 9] = [
    "overview",
    "areas",
    "discovery",
    "goals",
    "work",
    "themes",
    "graph",
    "cone",
    "packet",
];

pub fn load_state(root: &str) -> Result<State, String> {
    let ctx = CliCtx::new(root.to_string());
    load(&ctx, true).map_err(|r| {
        let msg = if r.err.trim().is_empty() { r.out } else { r.err };
        msg.trim().to_string()
    })
}

pub fn render_view(
    tpl: &Templates,
    root: &str,
    level: &str,
    params: &serde_json::Value,
) -> Result<String, String> {
    match level {
        "overview" => overview::render(tpl, root),
        "areas" => areas::render(tpl, root),
        "discovery" => discovery::render(tpl, root),
        "goals" => goals::render(tpl, root),
        "work" => work::render(tpl, root, params),
        "themes" => themes::render(tpl, root),
        "graph" => graph::render(tpl, root, params),
        "cone" => cone::render(tpl, root, params),
        "packet" => packet::render(tpl, root, params),
        other => Err(format!(
            "unknown view level: {other} (expected one of {})",
            LEVELS.join(", ")
        )),
    }
}

pub fn status_variant(kind: Kind, status: &str) -> &'static str {
    match kind {
        Kind::G => match status {
            "unverified" => "warning",
            "partial" => "info",
            "verified" => "success",
            _ => "neutral",
        },
        Kind::W => match status {
            "ready" => "info",
            "progress" => "accent",
            "done" => "success",
            "rejected" => "danger",
            _ => "neutral",
        },
        Kind::Q => match status {
            "open" => "warning",
            "answered" => "success",
            _ => "neutral",
        },
        Kind::B => match status {
            "testing" => "info",
            "validated" => "success",
            "invalidated_acceptable" => "warning",
            "invalidated_blocking" => "danger",
            _ => "neutral",
        },
        Kind::T => match status {
            "done" => "success",
            _ => "info",
        },
        Kind::Y => match status {
            "proposed" => "warning",
            "active" => "success",
            "stale" => "danger",
            _ => "neutral",
        },
        _ => "neutral",
    }
}
