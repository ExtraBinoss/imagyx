use std::{path::PathBuf, sync::Arc};

use tauri::{State, ipc::Response};

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
) -> Result<Response, String> {
    let source = PathBuf::from(&path);
    let state = Arc::clone(state.inner());
    let bytes = tauri::async_runtime::spawn_blocking(move || {
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
            .map(|bytes| bytes.as_ref().clone())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;

    Ok(Response::new(bytes))
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
