use std::{collections::{HashMap, HashSet}, fs, path::{Path, PathBuf}, sync::Arc};

use chrono::Utc;
use rayon::prelude::*;
use rusqlite::Connection;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::{
    indexer, ml,
    models::{AppInfo, FollowedFolder, ImageAsset, ImageEmbedding, ImageExplanation, ModelDownloadProgress, QueryConcept, RuntimeStats, SearchRequest, SemanticMatch},
    state::AppState,
    watcher::FolderWatcher,
};

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
