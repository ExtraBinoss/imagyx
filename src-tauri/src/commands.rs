use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::{
    indexer,
    ml::MlRuntime,
    models::{AppInfo, FollowedFolder, ImageAsset, SearchRequest},
    state::AppState,
};

#[tauri::command]
pub fn get_app_info(state: State<'_, Arc<AppState>>) -> AppInfo {
    let ml = state.ml.lock();
    AppInfo {
        root_dir: state.paths.root.to_string_lossy().into_owned(),
        models_dir: state.paths.models.to_string_lossy().into_owned(),
        database_path: state.database.path().to_string_lossy().into_owned(),
        thumbnails_dir: state.paths.thumbnails.to_string_lossy().into_owned(),
        ai_backend: MlRuntime::backend_label().to_owned(),
        ai_ready: ml.is_ready() || ml.cache_has_models(),
    }
}

#[tauri::command]
pub fn list_folders(state: State<'_, Arc<AppState>>) -> Result<Vec<FollowedFolder>, String> {
    state.database.folders().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn add_folder(path: String, state: State<'_, Arc<AppState>>) -> Result<FollowedFolder, String> {
    let canonical = PathBuf::from(&path)
        .canonicalize()
        .map_err(|error| format!("Impossible d’ouvrir le dossier: {error}"))?;
    if !canonical.is_dir() {
        return Err("Le chemin sélectionné n’est pas un dossier".into());
    }
    let canonical_string = canonical.to_string_lossy().into_owned();
    if let Some(existing) = state
        .database
        .folders()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|folder| folder.path == canonical_string)
    {
        return Ok(existing);
    }
    let folder = FollowedFolder {
        id: Uuid::new_v4().to_string(),
        name: canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Images")
            .to_owned(),
        path: canonical_string,
        image_count: 0,
        created_at: Utc::now().timestamp_millis(),
    };
    state
        .database
        .add_folder(&folder)
        .map_err(|error| error.to_string())?;
    Ok(folder)
}

#[tauri::command(rename_all = "camelCase")]
pub fn remove_folder(folder_id: String, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state
        .database
        .remove_folder(&folder_id)
        .map_err(|error| error.to_string())?;
    state.refresh_vectors().map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn index_folder(
    folder_id: String,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let folder = state
        .database
        .folder(&folder_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Dossier inconnu".to_owned())?;
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || indexer::index_folder(&state, &app, &folder))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_images(
    request: SearchRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ImageAsset>, String> {
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        indexer::search(
            &state,
            &request.query,
            request.folder_id.as_deref(),
            request.limit.unwrap_or(250).min(1_000),
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}
