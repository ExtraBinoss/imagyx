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
    let permit = Arc::clone(&state.thumbnail_workers)
        .acquire_owned()
        .await
        .map_err(|error| error.to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let known = state
            .database
            .image_path_is_known(&image_id, &path)
            .map_err(|error| error.to_string())?;
        if !known {
            return Err("L’image ne fait pas partie de la bibliothèque".into());
        }

        state
            .thumbnails
            .get_or_create(&image_id, &source, modified_at)
            .map(|thumbnail| thumbnail.to_string_lossy().into_owned())
            .map_err(|error| error.to_string())
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
