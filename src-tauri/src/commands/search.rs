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
    let state = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(cached) = state
            .database
            .thumbnail_path(&image_id, &path)
            .map_err(|error| error.to_string())?
        {
            if PathBuf::from(&cached).is_file() {
                return Ok(cached);
            }
        }

        let known = state
            .database
            .image_path_is_known(&image_id, &path)
            .map_err(|error| error.to_string())?;
        if !known {
            return Err("L’image ne fait pas partie de la bibliothèque".into());
        }
        let thumbnail = state
            .thumbnails
            .get_or_create(&image_id, &source, modified_at)
            .map_err(|error| error.to_string())?;
        let thumbnail = thumbnail.to_string_lossy().into_owned();
        state
            .database
            .save_thumbnail_path(&image_id, &path, &thumbnail)
            .map_err(|error| error.to_string())?;
        Ok(thumbnail)
    })
    .await
    .map_err(|error| error.to_string())?
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
            request.offset.unwrap_or(0),
        )
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}
