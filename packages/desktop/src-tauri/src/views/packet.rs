use super::load_state;
use crate::bridge::run_read;
use crate::templates::Templates;
use grove_core::{listnodes, Kind};
use serde_json::{json, Value};

pub fn render(tpl: &Templates, root: &str, params: &Value) -> Result<String, String> {
    let st = load_state(root)?;
    let works: Vec<Value> = listnodes(&st, Kind::W, false)
        .into_iter()
        .map(|w| {
            json!({
                "id": w.id,
                "label": format!("{} - {}", w.id, w.title),
            })
        })
        .collect();
    let id = params
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();
    let selected_text = works
        .iter()
        .find(|w| w["id"].as_str() == Some(id.as_str()))
        .and_then(|w| w["label"].as_str())
        .map(String::from);
    let mut model = json!({
        "works": works,
        "selected": id,
    });
    if let Some(text) = selected_text {
        model["selectedText"] = json!(text);
    }
    if !id.is_empty() {
        match run_read(root, "packet", &[id.clone()]) {
            Ok(out) => {
                let markdown = serde_json::from_str::<Value>(&out)
                    .ok()
                    .and_then(|v| v.get("packet_markdown")?.as_str().map(String::from))
                    .unwrap_or(out);
                let title = st
                    .nodes
                    .get(&id)
                    .map(|n| n.title.clone())
                    .unwrap_or_default();
                let status = st
                    .nodes
                    .get(&id)
                    .map(|n| n.status.clone())
                    .unwrap_or_default();
                model["packet"] = json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "markdown": markdown,
                });
            }
            Err(e) => {
                model["error"] = json!(e);
            }
        }
    }
    tpl.render("packet", &model)
}
