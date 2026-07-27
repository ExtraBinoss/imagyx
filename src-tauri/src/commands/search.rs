use std::{path::PathBuf, sync::Arc};

use tauri::State;

use crate::{
    indexer,
    models::{ImageAsset, SearchRequest},
    state::AppState,
};

#[tauri::command(rename_all = "camelCase")]
pub async fn get_thumbnail(
    image_id: String,
    path: String,
    modified_at: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let source = PathBuf::from(&path);
    let allowed = state
        .database
        .folders()
        .map_err(|error| error.to_string())?
        .iter()
        .any(|folder| source.starts_with(&folder.path));
    if !allowed {
        return Err("L’image ne se trouve pas dans un dossier suivi".into());
    }
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        state
            .thumbnails
            .get_or_create(&image_id, &source, modified_at)
            .map(|path| path.to_string_lossy().into_owned())
    })
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
            request.query_vector.as_deref(),
            request.folder_id.as_deref(),
            request.limit.unwrap_or(2_000).min(50_000),
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}
