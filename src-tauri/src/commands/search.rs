use std::{path::PathBuf, sync::Arc};

#[cfg(debug_assertions)]
use std::time::Instant;

use tauri::State;

use crate::{
    indexer,
    models::{ImageAsset, SearchPage, SearchRequest},
    state::AppState,
};
#[cfg(debug_assertions)]
use crate::tracing;

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

async fn execute_search_page(
    request: SearchRequest,
    diagnostic_id: Option<String>,
    state: Arc<AppState>,
) -> Result<SearchPage, String> {
    #[cfg(debug_assertions)]
    let received_at = Instant::now();

    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(debug_assertions)]
        let diagnostic_label = diagnostic_id.as_deref().unwrap_or("-");
        #[cfg(debug_assertions)]
        let queue_wait_ms = received_at.elapsed().as_secs_f64() * 1_000.0;
        #[cfg(debug_assertions)]
        let execution_started_at = Instant::now();
        #[cfg(debug_assertions)]
        let mode = request.mode.as_deref().unwrap_or_else(|| {
            if request.query.trim().is_empty() {
                "browse"
            } else if request.query_vector.is_some() {
                "hybrid"
            } else {
                "lexical"
            }
        });

        let limit = request.limit.unwrap_or(2_000).min(50_000);
        let offset = request.offset.unwrap_or(0).min(50_000);

        #[cfg(debug_assertions)]
        tracing::event(
            "search.command.start",
            format!(
                "id={diagnostic_label} mode={mode} query={:?} folder_id={:?} exclude_image_id={:?} limit={limit} offset={offset} vector_dimensions={} spawn_blocking_queue_ms={queue_wait_ms:.2}",
                request.query,
                request.folder_id,
                request.exclude_image_id,
                request.query_vector.as_ref().map_or(0, Vec::len),
            ),
        );

        let result = indexer::search_page_with_diagnostics(
            &state,
            &request.query,
            request.query_vector.as_deref(),
            request.folder_id.as_deref(),
            request.mode.as_deref(),
            request.exclude_image_id.as_deref(),
            limit,
            offset,
            diagnostic_id.as_deref(),
        );

        #[cfg(debug_assertions)]
        tracing::event(
            "search.command.complete",
            format!(
                "id={diagnostic_label} mode={mode} query={:?} results={} total_available={} success={} rust_execution_ms={:.2} command_total_ms={:.2}",
                request.query,
                result.as_ref().map_or(0, |page| page.items.len()),
                result.as_ref().map_or(0, |page| page.total),
                result.is_ok(),
                execution_started_at.elapsed().as_secs_f64() * 1_000.0,
                received_at.elapsed().as_secs_f64() * 1_000.0,
            ),
        );

        result
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn search_images(
    request: SearchRequest,
    diagnostic_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ImageAsset>, String> {
    execute_search_page(request, diagnostic_id, Arc::clone(state.inner()))
        .await
        .map(|page| page.items)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn search_image_page(
    request: SearchRequest,
    diagnostic_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<SearchPage, String> {
    execute_search_page(request, diagnostic_id, Arc::clone(state.inner())).await
}
