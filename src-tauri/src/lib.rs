mod commands;
mod db;
mod indexer;
mod ml;
mod models;
mod paths;
mod state;

use std::{path::PathBuf, sync::Arc, time::Duration};

use paths::AppPaths;
use state::AppState;
use tauri::Manager;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("pictures directory is unavailable")]
    PicturesDirectoryUnavailable,
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
    #[error("model error: {0}")]
    Model(String),
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

            for folder in state.database.folders()? {
                app.asset_protocol_scope()
                    .allow_directory(&folder.path, true)?;
            }

            app.manage(Arc::clone(&state));
            start_model_preparation(app.handle().clone(), Arc::clone(&state));
            start_background_refresh(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::list_folders,
            commands::add_folder,
            commands::remove_folder,
            commands::index_folder,
            commands::search_images,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Imagyx");
}

fn start_model_preparation(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = state.ml.lock().prepare(&app, &state.model_progress) {
            eprintln!("Imagyx model preparation failed: {error}");
        }
    });
}

fn start_background_refresh(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(45));
        loop {
            interval.tick().await;
            let Ok(folders) = state.database.folders() else {
                continue;
            };
            for folder in folders {
                let state = Arc::clone(&state);
                let app = app.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    indexer::index_folder(&state, &app, &folder)
                })
                .await;
            }
        }
    });
}
