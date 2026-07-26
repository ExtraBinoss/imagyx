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
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
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
    let spotlight_shortcut = Shortcut::new(Some(Modifiers::CONTROL), Code::Numpad9);
    let handler_shortcut = spotlight_shortcut.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if shortcut != &handler_shortcut || event.state() != ShortcutState::Pressed { return; }
                    let Some(window) = app.get_webview_window("spotlight") else { return; };
                    let visible = window.is_visible().unwrap_or(false);
                    if visible {
                        let _ = window.hide();
                    } else {
                        let _ = window.center();
                        let _ = window.show();
                        let _ = window.set_focus();
                        let _ = window.emit("spotlight-opened", ());
                    }
                })
                .build(),
        )
        .setup(move |app| {
            if let Err(error) = app.global_shortcut().register(spotlight_shortcut) {
                eprintln!("Impossible d’enregistrer Ctrl+Numpad9: {error}");
            }

            let paths = AppPaths::discover()?;
            let state = Arc::new(AppState::new(paths)?);
            let folders = state.database.folders()?;
            app.asset_protocol_scope().allow_directory(&state.paths.thumbnails, true)?;
            app.asset_protocol_scope().allow_directory(&state.paths.models, true)?;
            for folder in &folders { app.asset_protocol_scope().allow_directory(&folder.path, true)?; }
            let folder_watcher = FolderWatcher::start(app.handle().clone(), Arc::clone(&state), folders)?;
            app.manage(folder_watcher);
            app.manage(state);

            if let Some(main) = app.get_webview_window("main") {
                let window = main.clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });
            }
            if let Some(spotlight) = app.get_webview_window("spotlight") {
                let window = spotlight.clone();
                spotlight.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_platform,
            commands::get_runtime_stats,
            commands::update_runtime_stats,
            commands::update_model_progress,
            commands::prepare_local_model,
            commands::reset_embeddings,
            commands::list_folders,
            commands::add_folder,
            commands::remove_folder,
            commands::index_folder,
            commands::pending_images,
            commands::prepare_ai_images,
            commands::save_embeddings,
            commands::explain_results,
            commands::get_thumbnail,
            commands::search_images,
            commands::open_in_file_manager,
            commands::copy_image_to_clipboard,
            commands::open_in_imagyx,
            commands::hide_spotlight,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Imagyx");
}
