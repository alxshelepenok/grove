pub mod avatar;
pub mod bridge;
mod commands;
pub mod projects;
pub mod templates;
pub mod triggers;
pub mod views;

use grove_core::abspath;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use tauri::Manager;

pub struct ProjectState {
    pub root: Mutex<Option<String>>,
    pub session: Mutex<String>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl ProjectState {
    pub fn new(root: Option<String>) -> ProjectState {
        let session = root.as_deref().map(desktop_session).unwrap_or_default();
        ProjectState {
            root: Mutex::new(root),
            session: Mutex::new(session),
        }
    }

    pub fn current_root(&self) -> Result<String, String> {
        lock(&self.root).clone().ok_or_else(|| {
            "no_project: no project is open; open or create a project first".to_string()
        })
    }

    pub fn session(&self) -> String {
        lock(&self.session).clone()
    }

    pub fn open(&self, root: String) {
        *lock(&self.session) = desktop_session(&root);
        *lock(&self.root) = Some(root);
    }

    pub fn close(&self) {
        *lock(&self.root) = None;
        *lock(&self.session) = String::new();
    }
}

impl Default for ProjectState {
    fn default() -> ProjectState {
        ProjectState::new(None)
    }
}

pub struct AppState {
    pub project: ProjectState,
    pub templates: templates::Templates,
}

pub fn desktop_session(root: &str) -> String {
    grove_core::derive_default_session_token(root)
}

const fn env_for_profile(debug_assertions: bool) -> &'static str {
    if debug_assertions {
        "development"
    } else {
        "production"
    }
}

pub fn app_env() -> &'static str {
    env_for_profile(cfg!(debug_assertions))
}

const fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn init_script() -> String {
    format!(
        r#"window.__GROVE_APP_ENV__ = "{}";window.__GROVE_PLATFORM__ = "{}";"#,
        app_env(),
        platform()
    )
}

fn discover_root(start: &str) -> Option<String> {
    let mut dir = abspath(start);
    loop {
        if Path::new(&dir).join(".grove").join("state.lock").is_file() {
            return Some(dir);
        }
        let parent = Path::new(&dir)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if parent == dir || parent.is_empty() {
            return None;
        }
        dir = parent;
    }
}

pub fn resolve_startup_root(
    args: &[String],
    env_project: Option<&str>,
    registry: &[grove_core::ProjectEntry],
    cwd: &str,
) -> Option<String> {
    for arg in args {
        if let Some(v) = arg.strip_prefix("--root=") {
            return Some(abspath(v));
        }
    }
    if let Some(p) = env_project {
        if !p.trim().is_empty() {
            if let Ok(root) = grove_core::resolve_project_target(p) {
                return Some(root);
            }
        }
    }
    let mut recent: Option<&grove_core::ProjectEntry> = None;
    for e in registry {
        if !Path::new(&e.path).join(".grove").join("state.lock").is_file() {
            continue;
        }
        if recent.map(|r| r.last_opened >= e.last_opened) != Some(true) {
            recent = Some(e);
        }
    }
    if let Some(e) = recent {
        return Some(e.path.clone());
    }
    discover_root(cwd)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env_project = std::env::var("GROVE_PROJECT").ok();
    let registry = grove_core::registry_load(&grove_core::registry_path()).unwrap_or_default();
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    let project = ProjectState::new(resolve_startup_root(
        &args,
        env_project.as_deref(),
        &registry,
        &cwd,
    ));
    match project.current_root() {
        Ok(r) => eprintln!("grove-desktop: root={r} session={}", project.session()),
        Err(_) => eprintln!("grove-desktop: no project root resolved; starting without a project"),
    }
    let templates = match templates::Templates::load(&templates::ui_dir()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("grove-desktop: {e}");
            std::process::exit(1);
        }
    };

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .append_invoke_initialization_script(&init_script())
        .manage(AppState { project, templates })
        .invoke_handler(tauri::generate_handler![
            commands::grove_view,
            commands::grove_read,
            commands::grove_write,
            commands::grove_status_metrics,
            commands::grove_projects_list,
            commands::grove_project_current,
            commands::grove_project_open,
            commands::grove_project_create,
            commands::grove_project_close,
            commands::grove_project_remove,
            commands::grove_pick_directory,
            commands::grove_session_present,
            commands::grove_project_avatar
        ]);

    #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }));
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running grove desktop");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_for_profile_maps_both_profiles() {
        assert_eq!(env_for_profile(true), "development");
        assert_eq!(env_for_profile(false), "production");
    }

    #[test]
    fn app_env_matches_compile_time_profile() {
        #[cfg(debug_assertions)]
        assert_eq!(app_env(), "development");
        #[cfg(not(debug_assertions))]
        assert_eq!(app_env(), "production");
    }

    #[test]
    fn init_script_injects_app_env_and_platform_before_page_scripts() {
        assert_eq!(
            init_script(),
            format!(
                r#"window.__GROVE_APP_ENV__ = "{}";window.__GROVE_PLATFORM__ = "{}";"#,
                app_env(),
                platform()
            )
        );
    }

    #[test]
    fn platform_matches_compile_time_target() {
        #[cfg(windows)]
        assert_eq!(platform(), "windows");
        #[cfg(target_os = "macos")]
        assert_eq!(platform(), "macos");
        #[cfg(all(not(windows), not(target_os = "macos")))]
        assert_eq!(platform(), "linux");
    }

    #[test]
    fn session_presence_tracks_the_open_project() {
        let project = ProjectState::new(None);
        assert!(project.session().is_empty());
        project.open("some-root".to_string());
        assert!(!project.session().is_empty());
        project.close();
        assert!(project.session().is_empty());
    }
}
