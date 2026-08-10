use crate::bridge::{run_read, run_write};
use crate::views::{load_state, render_view};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn grove_view(
    state: State<'_, AppState>,
    level: String,
    params: Option<serde_json::Value>,
) -> Result<String, String> {
    let root = match state.project.current_root() {
        Ok(root) => root,
        Err(_) => return crate::views::project::render(&state.templates),
    };
    render_view(
        &state.templates,
        &root,
        &level,
        &params.unwrap_or_else(|| serde_json::json!({})),
    )
}

#[tauri::command]
pub fn grove_read(
    state: State<'_, AppState>,
    cmd: String,
    args: Option<Vec<String>>,
) -> Result<String, String> {
    let root = state.project.current_root()?;
    run_read(&root, &cmd, &args.unwrap_or_default())
}

#[tauri::command]
pub fn grove_write(
    state: State<'_, AppState>,
    cmd: String,
    args: Option<Vec<String>>,
) -> Result<String, String> {
    let root = state.project.current_root()?;
    let session = state.project.session();
    run_write(&root, &session, &cmd, &args.unwrap_or_default())
}

#[tauri::command]
pub fn grove_status_metrics(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let root = state.project.current_root()?;
    let st = load_state(&root)?;
    let lock_meta = std::fs::metadata(grove_core::CliCtx::new(root).lockpath()).ok();
    let lock_mtime = lock_meta.as_ref().and_then(|m| m.modified().ok());
    let lock_bytes = lock_meta.as_ref().map(|m| m.len());
    Ok(crate::views::overview::status_bar_model(
        &st, lock_mtime, lock_bytes,
    ))
}

#[tauri::command]
pub fn grove_projects_list(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(crate::projects::list(&state.project))
}

#[tauri::command]
pub fn grove_project_current(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(crate::projects::current(&state.project).unwrap_or(serde_json::Value::Null))
}

#[tauri::command]
pub fn grove_project_open(
    state: State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    crate::projects::open(&state.project, &path)
}

#[tauri::command]
pub fn grove_project_create(
    state: State<'_, AppState>,
    path: String,
    name: String,
) -> Result<serde_json::Value, String> {
    crate::projects::create(&state.project, &path, &name)
}

#[tauri::command]
pub fn grove_project_close(state: State<'_, AppState>) {
    crate::projects::close(&state.project);
}

#[tauri::command]
pub fn grove_project_remove(
    state: State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    Ok(crate::projects::remove(&state.project, &path))
}

#[tauri::command]
pub async fn grove_pick_directory(app: tauri::AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path.map(|p| p.to_string()));
    });
    tauri::async_runtime::spawn_blocking(move || rx.recv().unwrap_or(None))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
pub fn grove_session_present(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(!state.project.session().is_empty())
}

#[tauri::command]
pub fn grove_project_avatar(name: String) -> String {
    crate::avatar::tree_avatar_svg(&name)
}
