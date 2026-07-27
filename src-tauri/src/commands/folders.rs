use std::{path::PathBuf, sync::Arc};

use chrono::Utc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::{
    indexer,
    models::{FollowedFolder, ImageAsset},
    state::AppState,
    watcher::FolderWatcher,
};

#[tauri::command]
pub fn list_folders(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<FollowedFolder>, String> {
    state.database.folders().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn add_folder(
    path: String,
    state: State<'_, Arc<AppState>>,
    folder_watcher: State<'_, FolderWatcher>,
    app: AppHandle,
) -> Result<FollowedFolder, String> {
    let canonical = PathBuf::from(&path)
        .canonicalize()
        .map_err(|error| format!("Impossible d’ouvrir le dossier: {error}"))?;
    if !canonical.is_dir() {
        return Err("Le chemin sélectionné n’est pas un dossier".into());
    }
    app.asset_protocol_scope()
        .allow_directory(&canonical, true)
        .map_err(|error| format!("Impossible d’autoriser l’affichage du dossier: {error}"))?;
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
    folder_watcher.watch(folder.clone())?;
    Ok(folder)
}

#[tauri::command(rename_all = "camelCase")]
pub fn remove_folder(
    folder_id: String,
    state: State<'_, Arc<AppState>>,
    folder_watcher: State<'_, FolderWatcher>,
) -> Result<(), String> {
    if let Some(folder) = state
        .database
        .folder(&folder_id)
        .map_err(|error| error.to_string())?
    {
        folder_watcher.unwatch(folder)?;
    }
    state
        .database
        .remove_folder(&folder_id)
        .map_err(|error| error.to_string())?;
    state.vectors.write().remove_folder(&folder_id);
    Ok(())
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

#[tauri::command(rename_all = "camelCase")]
pub fn pending_images(
    folder_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ImageAsset>, String> {
    indexer::pending_assets(&state, folder_id.as_deref()).map_err(|error| error.to_string())
}
