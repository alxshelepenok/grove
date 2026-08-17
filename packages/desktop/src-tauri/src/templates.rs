use handlebars::{
    handlebars_helper, Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext,
};
use serde_json::json;
use std::path::{Path, PathBuf};

handlebars_helper!(eq: |a: Json, b: Json| a == b);
handlebars_helper!(gt: |a: Json, b: Json| {
    a.as_f64().zip(b.as_f64()).is_some_and(|(x, y)| x > y)
});
handlebars_helper!(json: |v: Json| serde_json::to_string(v)
    .unwrap_or_default()
    .replace("</", "<\\/"));

struct EmptyCtx;

impl HelperDef for EmptyCtx {
    fn call_inner<'reg: 'rc, 'rc>(
        &self,
        _: &Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc Context,
        _: &mut RenderContext<'reg, 'rc>,
    ) -> Result<handlebars::ScopedJson<'rc>, handlebars::RenderError> {
        Ok(handlebars::ScopedJson::Derived(json!({})))
    }
}

struct IconHelper {
    dir: PathBuf,
}

impl IconHelper {
    fn rewrite_svg(svg: &str, size: u64, class: &str) -> String {
        let trimmed = svg.trim();
        let Some(start) = trimmed.find("<svg") else {
            return String::new();
        };
        let Some(end) = trimmed[start..].find('>') else {
            return String::new();
        };
        let attrs = &trimmed[start + 4..start + end];
        let mut cleaned = String::with_capacity(attrs.len());
        let mut rest = attrs;
        for attr in ["width", "height"] {
            while let Some(pos) = rest.find(&format!("{attr}=\"")) {
                let value_start = pos + attr.len() + 2;
                let Some(value_end) = rest[value_start..].find('"') else {
                    break;
                };
                cleaned.push_str(&rest[..pos]);
                rest = &rest[value_start + value_end + 1..];
            }
        }
        cleaned.push_str(rest);
        format!(
            "<svg{} class=\"{class}\" width=\"{size}\" height=\"{size}\">{}",
            cleaned.trim_end(),
            &trimmed[start + end + 1..]
        )
    }
}

impl HelperDef for IconHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        _r: &'reg Handlebars<'reg>,
        _ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let name = h
            .param(0)
            .and_then(|p| p.value().as_str())
            .unwrap_or_default();
        let svg = std::fs::read_to_string(self.dir.join(format!("{name}.svg")))
            .unwrap_or_default();
        if svg.is_empty() {
            eprintln!("grove-desktop: unknown icon: {name}");
            return Ok(());
        }
        let size = h
            .hash_get("size")
            .and_then(|v| {
                v.value()
                    .as_u64()
                    .or_else(|| v.value().as_str()?.parse::<u64>().ok())
            })
            .filter(|n| *n > 0)
            .unwrap_or(18);
        let extra = h
            .hash_get("class")
            .and_then(|v| v.value().as_str())
            .unwrap_or_default();
        let extra = if extra
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-')
        {
            extra
        } else {
            ""
        };
        let class = if extra.is_empty() {
            "icon".to_string()
        } else {
            format!("icon {extra}")
        };
        out.write(&IconHelper::rewrite_svg(&svg, size, &class))?;
        Ok(())
    }
}

pub struct Templates {
    reg: Handlebars<'static>,
}

pub fn ui_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("GROVE_DESKTOP_UI_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("ui");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("ui")
}

impl Templates {
    pub fn load(ui_dir: &Path) -> Result<Templates, String> {
        let mut reg = Handlebars::new();
        reg.register_helper("eq", Box::new(eq));
        reg.register_helper("gt", Box::new(gt));
        reg.register_helper("emptyCtx", Box::new(EmptyCtx));
        reg.register_helper("json", Box::new(json));
        reg.register_helper(
            "icon",
            Box::new(IconHelper {
                dir: ui_dir.join("icons"),
            }),
        );
        let views_dir = ui_dir.join("views");
        for entry in std::fs::read_dir(views_dir.join("partials"))
            .map_err(|e| format!("cannot read {}: {e}", views_dir.display()))?
            .chain(
                std::fs::read_dir(&views_dir)
                    .map_err(|e| format!("cannot read {}: {e}", views_dir.display()))?,
            )
        {
            let path = entry
                .map_err(|e| format!("cannot read {}: {e}", views_dir.display()))?
                .path();
            if path.extension().and_then(|e| e.to_str()) != Some("hbs") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            reg.register_template_file(&name, &path)
                .map_err(|e| format!("cannot register {}: {e}", path.display()))?;
        }
        Ok(Templates { reg })
    }

    pub fn render(&self, view: &str, data: &serde_json::Value) -> Result<String, String> {
        self.reg
            .render(view, data)
            .map_err(|e| format!("render {view} failed: {e}"))
    }
}
