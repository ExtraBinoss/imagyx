use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use arboard::{Clipboard, ImageData};
use chrono::Utc;
use rayon::prelude::*;
use rusqlite::Connection;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow,
};
use uuid::Uuid;

use crate::{
    indexer, ml,
    models::{AppInfo, FollowedFolder, ImageAsset, ImageEmbedding, ImageExplanation, ModelDownloadProgress, QueryConcept, RuntimeStats, SearchRequest, SemanticMatch},
    state::AppState,
    watcher::FolderWatcher,
};

const SPOTLIGHT_WIDTH: f64 = 780.0;
const SPOTLIGHT_COMPACT_HEIGHT: f64 = 126.0;
const SPOTLIGHT_EXPANDED_HEIGHT: f64 = 580.0;

pub(crate) fn layout_spotlight_window(window: &WebviewWindow, expanded: bool) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "Écran actif indisponible".to_owned())?;
    let scale = monitor.scale_factor();
    let logical_height = if expanded { SPOTLIGHT_EXPANDED_HEIGHT } else { SPOTLIGHT_COMPACT_HEIGHT };
    let width = (SPOTLIGHT_WIDTH * scale).round().max(1.0) as u32;
    let height = (logical_height * scale).round().max(1.0) as u32;

    window
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;

    let work_area = monitor.work_area();
    let left = i64::from(work_area.position.x);
    let top = i64::from(work_area.position.y);
    let available_width = i64::from(work_area.size.width);
    let available_height = i64::from(work_area.size.height);
    let x = left + ((available_width - i64::from(width)) / 2).max(0);
    let desired_y = top + (available_height as f64 * 0.25).round() as i64;
    let maximum_y = top + (available_height - i64::from(height)).max(0);
    let y = desired_y.min(maximum_y).max(top);

    window
        .set_position(PhysicalPosition::new(clamp_i32(x), clamp_i32(y)))
        .map_err(|error| error.to_string())
}

fn clamp_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<AppState>>) -> AppInfo {
    let model_progress = state.model_progress.read().clone();
    let runtime_stats = collect_runtime_stats(&state);
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
pub fn get_platform() -> String { std::env::consts::OS.to_owned() }

#[tauri::command]
pub fn get_runtime_stats(state: State<'_, Arc<AppState>>) -> RuntimeStats { collect_runtime_stats(&state) }

#[tauri::command]
pub fn update_runtime_stats(stats: RuntimeStats, state: State<'_, Arc<AppState>>) { *state.runtime_stats.write() = stats; }

#[tauri::command]
pub fn update_model_progress(progress: ModelDownloadProgress, state: State<'_, Arc<AppState>>) { *state.model_progress.write() = progress; }

#[tauri::command(rename_all = "camelCase")]
pub async fn prepare_local_model(model_key: String, app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || ml::prepare_local_model(&app, &state, &model_key).map(|path| path.to_string_lossy().into_owned()))
        .await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn reset_embeddings(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let connection = Connection::open(state.database.path()).map_err(|error| error.to_string())?;
    connection.execute("DELETE FROM embeddings", []).map_err(|error| error.to_string())?;
    state.refresh_vectors().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_folders(state: State<'_, Arc<AppState>>) -> Result<Vec<FollowedFolder>, String> { state.database.folders().map_err(|error| error.to_string()) }

#[tauri::command]
pub fn add_folder(path: String, state: State<'_, Arc<AppState>>, folder_watcher: State<'_, FolderWatcher>, app: AppHandle) -> Result<FollowedFolder, String> {
    let canonical = PathBuf::from(&path).canonicalize().map_err(|error| format!("Impossible d’ouvrir le dossier: {error}"))?;
    if !canonical.is_dir() { return Err("Le chemin sélectionné n’est pas un dossier".into()); }
    app.asset_protocol_scope().allow_directory(&canonical, true).map_err(|error| format!("Impossible d’autoriser l’affichage du dossier: {error}"))?;
    let canonical_string = canonical.to_string_lossy().into_owned();
    if let Some(existing) = state.database.folders().map_err(|e| e.to_string())?.into_iter().find(|folder| folder.path == canonical_string) { return Ok(existing); }
    let folder = FollowedFolder {
        id: Uuid::new_v4().to_string(),
        name: canonical.file_name().and_then(|name| name.to_str()).unwrap_or("Images").to_owned(),
        path: canonical_string, image_count: 0, created_at: Utc::now().timestamp_millis(),
    };
    state.database.add_folder(&folder).map_err(|e| e.to_string())?;
    folder_watcher.watch(folder.clone())?;
    Ok(folder)
}

#[tauri::command(rename_all = "camelCase")]
pub fn remove_folder(folder_id: String, state: State<'_, Arc<AppState>>, folder_watcher: State<'_, FolderWatcher>) -> Result<(), String> {
    if let Some(folder) = state.database.folder(&folder_id).map_err(|e| e.to_string())? { folder_watcher.unwatch(folder)?; }
    state.database.remove_folder(&folder_id).map_err(|e| e.to_string())?;
    state.refresh_vectors().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn index_folder(folder_id: String, state: State<'_, Arc<AppState>>, app: AppHandle) -> Result<(), String> {
    let folder = state.database.folder(&folder_id).map_err(|e| e.to_string())?.ok_or_else(|| "Dossier inconnu".to_owned())?;
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || indexer::index_folder(&state, &app, &folder)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn pending_images(folder_id: Option<String>, state: State<'_, Arc<AppState>>) -> Result<Vec<ImageAsset>, String> {
    indexer::pending_assets(&state, folder_id.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn prepare_ai_images(image_ids: Vec<String>, state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let wanted: HashSet<&str> = image_ids.iter().map(String::as_str).collect();
        let by_id: HashMap<String, ImageAsset> = state.database.images(None).map_err(|e| e.to_string())?.into_iter()
            .filter(|image| wanted.contains(image.id.as_str())).map(|image| (image.id.clone(), image)).collect();
        image_ids.par_iter().map(|image_id| {
            let image = by_id.get(image_id).ok_or_else(|| format!("Image inconnue: {image_id}"))?;
            state.thumbnails.get_or_create(&image.id, Path::new(&image.path), image.modified_at)
                .map(|path| path.to_string_lossy().into_owned()).map_err(|e| e.to_string())
        }).collect::<Result<Vec<_>, _>>()
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn save_embeddings(embeddings: Vec<ImageEmbedding>, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let rows = embeddings.into_iter().map(|entry| (entry.image_id, entry.vector)).collect::<Vec<_>>();
    state.database.save_embeddings(&rows).map_err(|e| e.to_string())?;
    state.refresh_vectors().map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn explain_results(image_ids: Vec<String>, concepts: Vec<QueryConcept>, state: State<'_, Arc<AppState>>) -> Vec<ImageExplanation> {
    if image_ids.is_empty() || concepts.is_empty() { return Vec::new(); }
    let wanted: HashSet<&str> = image_ids.iter().map(String::as_str).collect();
    state.vectors.read().par_iter().filter(|entry| wanted.contains(entry.image_id.as_str())).map(|entry| {
        let mut matches = concepts.iter().filter(|concept| concept.vector.len() == entry.vector.len()).map(|concept| SemanticMatch {
            label: concept.label.clone(), score: ((indexer::cosine_similarity(&concept.vector, &entry.vector) + 1.0) / 2.0).clamp(0.0, 1.0), source: "semantic".into(),
        }).collect::<Vec<_>>();
        matches.sort_by(|a, b| b.score.total_cmp(&a.score)); matches.truncate(8);
        ImageExplanation { image_id: entry.image_id.clone(), matches }
    }).collect()
}

#[tauri::command(rename_all = "camelCase")]
pub fn top_image_tags(concepts: Vec<QueryConcept>, limit: Option<usize>, state: State<'_, Arc<AppState>>) -> Vec<String> {
    if concepts.is_empty() { return Vec::new(); }
    let vectors = state.vectors.read();
    let counts = vectors.par_iter().take(2_000).filter_map(|entry| {
        concepts.iter()
            .filter(|concept| concept.vector.len() == entry.vector.len())
            .map(|concept| (concept.label.clone(), indexer::cosine_similarity(&concept.vector, &entry.vector)))
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .map(|best| best.0)
    }).fold(HashMap::<String, usize>::new, |mut map, label| {
        *map.entry(label).or_default() += 1;
        map
    }).reduce(HashMap::<String, usize>::new, |mut left, right| {
        for (label, count) in right { *left.entry(label).or_default() += count; }
        left
    });
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_label, left_count), (right_label, right_count)| right_count.cmp(left_count).then_with(|| left_label.cmp(right_label)));
    ranked.truncate(limit.unwrap_or(10).clamp(1, 20));
    ranked.into_iter().map(|(label, _)| label).collect()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_thumbnail(image_id: String, path: String, modified_at: i64, state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let source = PathBuf::from(&path);
    let allowed = state.database.folders().map_err(|e| e.to_string())?.iter().any(|folder| source.starts_with(&folder.path));
    if !allowed { return Err("L’image ne se trouve pas dans un dossier suivi".into()); }
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || state.thumbnails.get_or_create(&image_id, &source, modified_at).map(|path| path.to_string_lossy().into_owned()))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_images(request: SearchRequest, state: State<'_, Arc<AppState>>) -> Result<Vec<ImageAsset>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || indexer::search(&state, &request.query, request.query_vector.as_deref(), request.folder_id.as_deref(), request.limit.unwrap_or(2_000).min(50_000)))
        .await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_file_manager(path: String, reveal: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let canonical = managed_path(&state, &path)?;

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer");
        if reveal && canonical.is_file() { command.arg(format!("/select,{}", canonical.display())); }
        else { command.arg(&canonical); }
        command.spawn().map_err(|error| format!("Impossible d’ouvrir Explorer: {error}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if reveal && canonical.is_file() { command.arg("-R"); }
        command.arg(&canonical);
        command.spawn().map_err(|error| format!("Impossible d’ouvrir Finder: {error}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let target = if canonical.is_file() { canonical.parent().unwrap_or(&canonical) } else { canonical.as_path() };
        Command::new("xdg-open").arg(target).spawn().map_err(|error| format!("Impossible d’ouvrir le gestionnaire de fichiers: {error}"))?;
    }

    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn copy_image_to_clipboard(path: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let canonical = managed_path(&state, &path)?;
        if !canonical.is_file() { return Err("Le chemin ne correspond pas à une image".to_owned()); }
        let decoded = image::open(&canonical).map_err(|error| format!("Impossible de décoder l’image: {error}"))?.into_rgba8();
        let (width, height) = decoded.dimensions();
        let mut clipboard = Clipboard::new().map_err(|error| format!("Presse-papiers indisponible: {error}"))?;
        clipboard.set_image(ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(decoded.into_raw()),
        }).map_err(|error| format!("Impossible de copier l’image: {error}"))
    }).await.map_err(|error| error.to_string())?
}

#[tauri::command(rename_all = "camelCase")]
pub fn open_in_imagyx(image_id: String, app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let exists = state.database.images(None).map_err(|error| error.to_string())?.iter().any(|image| image.id == image_id);
    if !exists { return Err("Image inconnue".to_owned()); }
    let main = app.get_webview_window("main").ok_or_else(|| "Fenêtre principale indisponible".to_owned())?;
    main.show().map_err(|error| error.to_string())?;
    let _ = main.unminimize();
    main.set_focus().map_err(|error| error.to_string())?;
    main.emit("open-image-requested", image_id).map_err(|error| error.to_string())?;
    if let Some(spotlight) = app.get_webview_window("spotlight") {
        let _ = spotlight.emit("spotlight-will-hide", ());
        let _ = spotlight.hide();
    }
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_spotlight_expanded(expanded: bool, app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("spotlight").ok_or_else(|| "Fenêtre Spotlight indisponible".to_owned())?;
    layout_spotlight_window(&window, expanded)
}

#[tauri::command]
pub fn hide_spotlight(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("spotlight").ok_or_else(|| "Fenêtre Spotlight indisponible".to_owned())?;
    let _ = window.emit("spotlight-will-hide", ());
    window.hide().map_err(|error| error.to_string())
}

fn managed_path(state: &AppState, path: &str) -> Result<PathBuf, String> {
    let canonical = PathBuf::from(path).canonicalize().map_err(|error| format!("Chemin inaccessible: {error}"))?;
    let folders = state.database.folders().map_err(|error| error.to_string())?;
    let allowed = folders.iter().any(|folder| canonical.starts_with(Path::new(&folder.path)));
    if !allowed { return Err("Ce chemin ne fait pas partie d’un dossier suivi".to_owned()); }
    Ok(canonical)
}

fn collect_runtime_stats(state: &AppState) -> RuntimeStats {
    let resources = state.system_monitor.snapshot();
    let mut stats = state.runtime_stats.read().clone();
    stats.system_cpu_percent = resources.system_cpu_percent; stats.process_cpu_percent = resources.process_cpu_percent;
    stats.memory_used_bytes = resources.memory_used_bytes; stats.memory_total_bytes = resources.memory_total_bytes;
    stats.process_memory_bytes = resources.process_memory_bytes; stats.model_cache_bytes = directory_size(&state.paths.models);
    stats.thumbnail_cache_items = state.thumbnails.cached_items(); stats.updated_at = Utc::now().timestamp_millis(); stats
}

fn directory_size(root: &Path) -> u64 {
    fs::read_dir(root).map_or(0, |entries| entries.filter_map(Result::ok).map(|entry| match entry.metadata() {
        Ok(metadata) if metadata.is_file() => metadata.len(), Ok(metadata) if metadata.is_dir() => directory_size(&entry.path()), _ => 0,
    }).sum())
}
