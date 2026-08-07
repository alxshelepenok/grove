use grove_desktop_lib::templates::{ui_dir, Templates};
use grove_desktop_lib::views::render_view;

fn main() {
    let mut root = ".".to_string();
    let mut packet_id: Option<String> = None;
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--root=") {
            root = v.to_string();
        } else if let Some(v) = arg.strip_prefix("--packet=") {
            packet_id = Some(v.to_string());
        }
    }
    let root = grove_core::abspath(&root);
    let tpl = Templates::load(&ui_dir()).expect("templates load");
    for level in [
        "overview",
        "areas",
        "discovery",
        "goals",
        "work",
        "themes",
        "graph",
    ] {
        let html = render_view(&tpl, &root, level, &serde_json::json!({})).expect("render");
        println!("=== {level} ({} bytes) ===", html.len());
        println!("{html}");
    }
    if let Some(id) = packet_id {
        let html = render_view(&tpl, &root, "packet", &serde_json::json!({ "id": id }))
            .expect("render packet");
        println!("=== packet {id} ({} bytes) ===", html.len());
        println!("{html}");
    }
}
