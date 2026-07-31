use std::{
    fs,
    path::Path,
    sync::Arc,
};

use chrono::Utc;
use tauri::{AppHandle, Emitter, State};

use crate::{
    ml,
    models::{AppInfo, FolderIndexCoverage, ModelDownloadProgress, RuntimeStats},
    preferences::ShortcutPreferences,
    state::AppState,
    tracing, tray,
};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<AppState>>, app: AppHandle) -> AppInfo {
    let model_progress = state.model_progress.read().clone();
    let runtime_stats = state.runtime_stats.read().clone();
    let background_state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _trace = tracing::span("runtime_stats.initial_collect");
        let stats = collect_runtime_stats(&background_state);
        *background_state.runtime_stats.write() = stats.clone();
        tray::update_runtime_stats(&app, &stats);
        let _ = app.emit("runtime-stats", stats);
    });
    AppInfo {
        root_dir: state.paths.root.to_string_lossy().into_owned(),
        models_dir: state.paths.models.to_string_lossy().into_owned(),
        database_path: state.database.path().to_string_lossy().into_owned(),
        thumbnails_dir: state.paths.thumbnails.to_string_lossy().into_owned(),
        ai_backend: runtime_stats.backend_effective.clone(),
        ai_ready: model_progress.stage == "ready",
        model_progress,
        runtime_stats,
    }
}

#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_owned()
}

#[tauri::command]
pub async fn get_index_coverage(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<FolderIndexCoverage>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.database.folder_index_coverage())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_runtime_stats(
    state: State<'_, Arc<AppState>>,
) -> Result<RuntimeStats, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let _trace = tracing::span("runtime_stats.collect");
        collect_runtime_stats(&state)
    })
    .await
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_runtime_stats(
    stats: RuntimeStats,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) {
    *state.runtime_stats.write() = stats.clone();
    tray::update_runtime_stats(&app, &stats);
    let _ = app.emit("runtime-stats", stats);
}

#[tauri::command]
pub fn update_model_progress(
    progress: ModelDownloadProgress,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) {
    *state.model_progress.write() = progress.clone();
    let _ = app.emit("model-download-progress", progress);
}

#[tauri::command]
pub fn get_spotlight_shortcut(preferences: State<'_, ShortcutPreferences>) -> String {
    preferences.value()
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_spotlight_shortcut(
    shortcut: String,
    app: AppHandle,
    preferences: State<'_, ShortcutPreferences>,
) -> Result<String, String> {
    let updated = preferences.update(&app, &shortcut)?;
    let _ = app.emit("spotlight-shortcut-updated", updated.clone());
    Ok(updated)
}

#[tauri::command]
pub fn get_launch_on_startup(preferences: State<'_, ShortcutPreferences>) -> bool {
    preferences.launch_on_startup()
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_launch_on_startup(
    enabled: bool,
    app: AppHandle,
    preferences: State<'_, ShortcutPreferences>,
) -> Result<bool, String> {
    let autolaunch = app.autolaunch();
    let previously_enabled = autolaunch.is_enabled().map_err(|error| error.to_string())?;
    if enabled {
        autolaunch.enable().map_err(|error| error.to_string())?;
    } else {
        autolaunch.disable().map_err(|error| error.to_string())?;
    }
    if let Err(error) = preferences.set_launch_on_startup(enabled) {
        if previously_enabled {
            let _ = autolaunch.enable();
        } else {
            let _ = autolaunch.disable();
        }
        return Err(error);
    }
    Ok(enabled)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn prepare_local_model(
    model_key: String,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        ml::prepare_local_model(&app, &state, &model_key)
            .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn reset_embeddings(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .database
        .reset_embeddings()
        .map_err(|error| error.to_string())?;
    state.vectors.write().clear();
    Ok(())
}

fn collect_runtime_stats(state: &AppState) -> RuntimeStats {
    let resources = state.system_monitor.snapshot();
    let mut stats = state.runtime_stats.read().clone();
    stats.system_cpu_percent = resources.system_cpu_percent;
    stats.process_cpu_percent = resources.process_cpu_percent;
    stats.memory_used_bytes = resources.memory_used_bytes;
    stats.memory_total_bytes = resources.memory_total_bytes;
    stats.process_memory_bytes = resources.process_memory_bytes;
    stats.model_cache_bytes = directory_size(&state.paths.models);
    stats.thumbnail_cache_items = state.thumbnails.cached_items();
    stats.updated_at = Utc::now().timestamp_millis();
    stats
}

fn directory_size(root: &Path) -> u64 {
    fs::read_dir(root).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .map(|entry| match entry.metadata() {
                Ok(metadata) if metadata.is_file() => metadata.len(),
                Ok(metadata) if metadata.is_dir() => directory_size(&entry.path()),
                _ => 0,
            })
            .sum()
    })
}
