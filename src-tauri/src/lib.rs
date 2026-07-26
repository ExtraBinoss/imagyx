mod commands;
mod db;
mod indexer;
mod ml;
mod models;
mod paths;
mod state;
mod system_stats;
mod thumbnails;
mod watcher;

use std::{path::PathBuf, sync::Arc};

use paths::AppPaths;
use state::AppState;
use tauri::Manager;
use thiserror::Error;
use watcher::FolderWatcher;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("pictures directory is unavailable")]
    PicturesDirectoryUnavailable,
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("model error: {0}")]
    Model(String),
    #[error("filesystem watcher error: {0}")]
    Watcher(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let paths = AppPaths::discover()?;
            let state = Arc::new(AppState::new(paths)?);
            let folders = state.database.folders()?;
            app.asset_protocol_scope().allow_directory(&state.paths.thumbnails, true)?;
            app.asset_protocol_scope().allow_directory(&state.paths.models, true)?;
            for folder in &folders {
                app.asset_protocol_scope().allow_directory(&folder.path, true)?;
            }
            let folder_watcher = FolderWatcher::start(app.handle().clone(), Arc::clone(&state), folders)?;
            app.manage(folder_watcher);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_runtime_stats,
            commands::update_runtime_stats,
            commands::update_model_progress,
            commands::prepare_local_model,
            commands::list_folders,
            commands::add_folder,
            commands::remove_folder,
            commands::index_folder,
            commands::pending_images,
            commands::prepare_ai_images,
            commands::save_embeddings,
            commands::get_thumbnail,
            commands::search_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Imagyx");
}
