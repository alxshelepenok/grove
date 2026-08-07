use crate::templates::Templates;

pub fn render(tpl: &Templates) -> Result<String, String> {
    tpl.render("project", &serde_json::json!({}))
}
